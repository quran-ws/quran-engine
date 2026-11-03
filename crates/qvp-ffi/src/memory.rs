//! Memory: a heap for hosts without malloc, such as wasm.

use crate::abi::guard;

#[no_mangle]
pub extern "C" fn qvp_alloc(len: usize) -> *mut u8 {
    guard(|| {
        let mut v = Vec::<u8>::with_capacity(len.max(1));
        let p = v.as_mut_ptr();
        std::mem::forget(v);
        p
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_dealloc(ptr: *mut u8, len: usize) {
    guard(|| {
        if !ptr.is_null() {
            drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
        }
    })
}
