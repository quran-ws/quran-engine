//! The ABI plumbing every module shares: the panic guard, the string and array helpers.

use std::cell::RefCell;

/// Run one entry point's body and turn a panic in the engine into the function's error
/// value (0, -1 or null) instead of aborting the host process. The C ABI is the boundary
/// between a page file the host downloaded and the app that renders it; a malformed file
/// must never take the app down. On wasm32 there is no unwinding, so a panic still traps
/// there.
pub(crate) fn guard<T: FfiDefault>(body: impl FnOnce() -> T) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(v) => v,
        Err(_) => T::ffi_default(),
    }
}

/// The value an entry point returns when the engine panicked: null for pointers, -1 for
/// signed results (never a valid index), 0 for everything else.
pub(crate) trait FfiDefault {
    fn ffi_default() -> Self;
}
impl FfiDefault for () {
    fn ffi_default() {}
}
impl FfiDefault for u8 {
    fn ffi_default() -> u8 {
        0
    }
}
impl FfiDefault for u32 {
    fn ffi_default() -> u32 {
        0
    }
}
impl FfiDefault for f32 {
    fn ffi_default() -> f32 {
        0.0
    }
}
impl FfiDefault for i32 {
    fn ffi_default() -> i32 {
        -1
    }
}
impl FfiDefault for i64 {
    fn ffi_default() -> i64 {
        -1
    }
}
impl<T> FfiDefault for *mut T {
    fn ffi_default() -> *mut T {
        std::ptr::null_mut()
    }
}
impl<T> FfiDefault for *const T {
    fn ffi_default() -> *const T {
        std::ptr::null()
    }
}

thread_local! {
    pub(crate) static STR_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    pub(crate) static LAYOUT_BUF: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

#[repr(C)]
pub struct QvpStr {
    pub ptr: *const u8,
    pub len: u32,
}

pub(crate) unsafe fn out_str(out: *mut QvpStr, s: &str) {
    STR_BUF.with(|b| {
        let mut b = b.borrow_mut();
        b.clear();
        b.extend_from_slice(s.as_bytes());
        *out = QvpStr { ptr: b.as_ptr(), len: b.len() as u32 };
    });
}

pub(crate) unsafe fn in_str<'a>(ptr: *const u8, len: u32) -> &'a str {
    if ptr.is_null() || len == 0 {
        return "";
    }
    std::str::from_utf8(std::slice::from_raw_parts(ptr, len as usize)).unwrap_or("")
}

/// Copy up to `cap` items into `out` and return how many there are in all. Every array
/// output goes through here, so a short buffer always gets a truncated copy and the count
/// to call again with. A record output casts `out` to a pointer to fixed arrays, so `cap`
/// counts records.
pub(crate) unsafe fn fill<T: Copy>(out: *mut T, cap: u32, items: &[T]) -> u32 {
    if !out.is_null() && cap > 0 {
        let n = items.len().min(cap as usize);
        std::ptr::copy_nonoverlapping(items.as_ptr(), out, n);
    }
    items.len() as u32
}

/// One placement as the `{dx, dy, kx, ky}` record the header describes.
pub(crate) fn placement(p: &qvp_core::Placement) -> [f32; 4] {
    [p.dx, p.dy, p.kx, p.ky]
}

#[cfg(test)]
mod tests {
    use super::*;
    use qvp_core::Page;

    #[test]
    fn a_short_buffer_gets_a_truncated_copy_and_the_full_count() {
        let records = [[1u32, 2], [3, 4], [5, 6]];
        let mut out = [0u32; 4];
        let n = unsafe { fill(out.as_mut_ptr().cast::<[u32; 2]>(), 2, &records) };
        assert_eq!((n, out), (3, [1, 2, 3, 4]));
        assert_eq!(unsafe { fill(std::ptr::null_mut::<[u32; 2]>(), 0, &records) }, 3);
    }

    #[test]
    fn a_panic_inside_an_entry_point_becomes_the_error_value() {
        let signed: i32 = guard(|| panic!("engine bug"));
        let unsigned: u32 = guard(|| panic!("engine bug"));
        let pointer: *mut Page = guard(|| panic!("engine bug"));
        assert_eq!((signed, unsigned), (-1, 0));
        assert!(pointer.is_null());
        assert_eq!(guard(|| 7u32), 7);
    }
}
