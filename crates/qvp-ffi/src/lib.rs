//! C ABI over `qvp-core`. This is the single binding surface used by the
//! wasm build (web), Swift (via a module map), Kotlin (JNI shim), Dart FFI and
//! the React Native JSI shim. See `include/qvp.h`.
#![allow(clippy::missing_safety_doc)]

use qvp_core::qvp_format::{DecoKind, Family, Mark, PathKind};
use qvp_core::{Hit, LayoutSpec, Page, Selector, NONE};
use std::ffi::c_char;

#[repr(C)]
pub struct QvpPageInfo {
    pub width: f32,
    pub height: f32,
    pub page: u32,
    pub n_lines: u32,
    pub n_ayahs: u32,
    pub n_words: u32,
    pub n_paths: u32,
    pub n_decos: u32,
}

#[repr(C)]
pub struct QvpGeometry {
    pub ops: *const u8,
    pub ops_len: u32,
    pub pts: *const f32,
    pub pts_len: u32,
    /// `n_paths` records of 8 × u32: op_start, op_count, pt_start, pt_count, flags, word, line, reserved
    pub table: *const u32,
    pub n_paths: u32,
}

#[repr(C)]
pub struct QvpWordInfo {
    pub sura: u16,
    pub ayah: u16,
    pub word: u16,
    pub line_no: u16,
    pub ayah_idx: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub text: *const u8,
    pub text_len: u32,
    pub first_path: u32,
    pub n_paths: u32,
}

#[repr(C)]
pub struct QvpAyahInfo {
    pub sura: u16,
    pub ayah: u16,
    pub part: u8,
    pub parts: u8,
    pub flags: u8,
    pub first_word: u32,
    pub n_words: u32,
    pub marker_deco: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

#[repr(C)]
pub struct QvpLineInfo {
    pub line_no: u8,
    pub first_word: u32,
    pub n_words: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

#[repr(C)]
pub struct QvpDecoInfo {
    pub kind: u8,
    pub sura: u16,
    pub ayah: u16,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub text: *const u8,
    pub text_len: u32,
    pub first_path: u32,
    pub n_paths: u32,
}

#[repr(C)]
pub struct QvpHit {
    pub word: u32,
    pub path: u32,
    pub deco: u32,
}

#[repr(C)]
pub struct QvpLayoutSpec {
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub pad_top: f32,
    pub pad_bottom: f32,
    pub pad_left: f32,
    pub pad_right: f32,
    pub line_spacing: f32,
    pub fill_height: u32,
    pub nominal_lines: u32,
}

#[repr(C)]
pub struct QvpLayout {
    pub scale: f32,
    pub ox: f32,
    pub oy: f32,
    pub content_h: f32,
    pub pitch: f32,
    pub n_lines: u32,
    /// per line: dy (page units), slot_top, slot_bottom (viewport px) — valid until the next qvp_layout call
    pub lines: *const f32,
}

// ───────────── memory (wasm hosts have no malloc) ─────────────

#[no_mangle]
pub extern "C" fn qvp_alloc(len: usize) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

#[no_mangle]
pub unsafe extern "C" fn qvp_dealloc(ptr: *mut u8, len: usize) {
    if !ptr.is_null() {
        drop(Vec::from_raw_parts(ptr, 0, len.max(1)));
    }
}

// ───────────── page lifecycle ─────────────

/// Load a page from QVP bytes (copied). Returns NULL on error.
#[no_mangle]
pub unsafe extern "C" fn qvp_page_load(bytes: *const u8, len: usize) -> *mut Page {
    if bytes.is_null() {
        return std::ptr::null_mut();
    }
    let slice = std::slice::from_raw_parts(bytes, len);
    match Page::load(slice) {
        Ok(p) => Box::into_raw(Box::new(p)),
        Err(_) => std::ptr::null_mut(),
    }
}

#[no_mangle]
pub unsafe extern "C" fn qvp_page_free(page: *mut Page) {
    if !page.is_null() {
        drop(Box::from_raw(page));
    }
}

#[no_mangle]
pub unsafe extern "C" fn qvp_page_info(page: *const Page, out: *mut QvpPageInfo) {
    let p = &*page;
    let d = p.data();
    *out = QvpPageInfo {
        width: p.width(),
        height: p.height(),
        page: d.header.page as u32,
        n_lines: d.lines.len() as u32,
        n_ayahs: d.ayahs.len() as u32,
        n_words: d.words.len() as u32,
        n_paths: d.paths.len() as u32,
        n_decos: d.decos.len() as u32,
    };
}

#[no_mangle]
pub unsafe extern "C" fn qvp_geometry(page: *const Page, out: *mut QvpGeometry) {
    let g = (*page).geometry();
    *out = QvpGeometry {
        ops: g.ops.as_ptr(),
        ops_len: g.ops.len() as u32,
        pts: g.pts.as_ptr(),
        pts_len: g.pts.len() as u32,
        table: g.table.as_ptr() as *const u32,
        n_paths: g.table.len() as u32,
    };
}

#[no_mangle]
pub unsafe extern "C" fn qvp_word_info(page: *const Page, idx: u32, out: *mut QvpWordInfo) -> i32 {
    let p = &*page;
    let d = p.data();
    let Some(w) = d.words.get(idx as usize) else { return 0 };
    let q = p.quant();
    let t = p.word_text(idx);
    *out = QvpWordInfo {
        sura: w.sura,
        ayah: w.ayah,
        word: w.word,
        line_no: d.lines[w.line_idx as usize].line_no as u16,
        ayah_idx: w.ayah_idx as u32,
        x0: w.bbox.x0 as f32 / q,
        y0: w.bbox.y0 as f32 / q,
        x1: w.bbox.x1 as f32 / q,
        y1: w.bbox.y1 as f32 / q,
        text: t.as_ptr(),
        text_len: t.len() as u32,
        first_path: w.first_path,
        n_paths: w.n_paths as u32,
    };
    1
}

#[no_mangle]
pub unsafe extern "C" fn qvp_ayah_info(page: *const Page, idx: u32, out: *mut QvpAyahInfo) -> i32 {
    let p = &*page;
    let Some(a) = p.data().ayahs.get(idx as usize) else { return 0 };
    let q = p.quant();
    *out = QvpAyahInfo {
        sura: a.sura,
        ayah: a.ayah,
        part: a.part,
        parts: a.parts,
        flags: a.flags,
        first_word: a.first_word as u32,
        n_words: a.n_words as u32,
        marker_deco: if a.marker_deco == u16::MAX { NONE } else { a.marker_deco as u32 },
        x0: a.bbox.x0 as f32 / q,
        y0: a.bbox.y0 as f32 / q,
        x1: a.bbox.x1 as f32 / q,
        y1: a.bbox.y1 as f32 / q,
    };
    1
}

#[no_mangle]
pub unsafe extern "C" fn qvp_line_info(page: *const Page, idx: u32, out: *mut QvpLineInfo) -> i32 {
    let p = &*page;
    let Some(l) = p.data().lines.get(idx as usize) else { return 0 };
    let q = p.quant();
    *out = QvpLineInfo {
        line_no: l.line_no,
        first_word: l.first_word as u32,
        n_words: l.n_words as u32,
        x0: l.bbox.x0 as f32 / q,
        y0: l.bbox.y0 as f32 / q,
        x1: l.bbox.x1 as f32 / q,
        y1: l.bbox.y1 as f32 / q,
    };
    1
}

#[no_mangle]
pub unsafe extern "C" fn qvp_deco_info(page: *const Page, idx: u32, out: *mut QvpDecoInfo) -> i32 {
    let p = &*page;
    let Some(d) = p.data().decos.get(idx as usize) else { return 0 };
    let q = p.quant();
    let t = p.deco_text(idx);
    *out = QvpDecoInfo {
        kind: d.kind as u8,
        sura: d.sura,
        ayah: d.ayah,
        x0: d.bbox.x0 as f32 / q,
        y0: d.bbox.y0 as f32 / q,
        x1: d.bbox.x1 as f32 / q,
        y1: d.bbox.y1 as f32 / q,
        text: t.as_ptr(),
        text_len: t.len() as u32,
        first_path: d.first_path,
        n_paths: d.n_paths as u32,
    };
    1
}

// ───────────── hit testing ─────────────

/// Returns 1 and fills `out` when something is under (x, y) in page units.
#[no_mangle]
pub unsafe extern "C" fn qvp_hit_test(page: *const Page, x: f32, y: f32, out: *mut QvpHit) -> i32 {
    match (*page).hit_test(x, y) {
        Some(Hit { word, path, deco }) => {
            *out = QvpHit { word, path, deco };
            1
        }
        None => 0,
    }
}

/// Word index for (sura, ayah, word) or -1.
#[no_mangle]
pub unsafe extern "C" fn qvp_find_word(page: *const Page, sura: u16, ayah: u16, word: u16) -> i32 {
    (*page).find_word(sura, ayah, word).map(|i| i as i32).unwrap_or(-1)
}

// ───────────── layout ─────────────

/// Compute and store a layout (line spacing / fill height / padding).
/// `out.lines` points at n_lines × 3 floats: dy, slot_top, slot_bottom.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout(page: *mut Page, spec: *const QvpLayoutSpec, out: *mut QvpLayout) {
    let s = &*spec;
    let p = &mut *page;
    let l = p.layout(&LayoutSpec {
        viewport_w: s.viewport_w,
        viewport_h: s.viewport_h,
        pad_top: s.pad_top,
        pad_bottom: s.pad_bottom,
        pad_left: s.pad_left,
        pad_right: s.pad_right,
        line_spacing: s.line_spacing,
        fill_height: s.fill_height != 0,
        nominal_lines: s.nominal_lines,
    });
    let mut buf = Vec::with_capacity(l.line_dy.len() * 3);
    for (i, dy) in l.line_dy.iter().enumerate() {
        buf.push(*dy);
        buf.push(l.line_slots[i].0);
        buf.push(l.line_slots[i].1);
    }
    let lines = buf.as_ptr();
    LAYOUT_BUF.with(|b| *b.borrow_mut() = buf);
    *out = QvpLayout { scale: l.scale, ox: l.ox, oy: l.oy, content_h: l.content_h, pitch: l.pitch, n_lines: l.line_dy.len() as u32, lines };
}

thread_local! {
    static LAYOUT_BUF: std::cell::RefCell<Vec<f32>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Hit-test in viewport px through the current layout.
#[no_mangle]
pub unsafe extern "C" fn qvp_hit_test_view(page: *const Page, vx: f32, vy: f32, out: *mut QvpHit) -> i32 {
    match (*page).hit_test_view(vx, vy) {
        Some(Hit { word, path, deco }) => {
            *out = QvpHit { word, path, deco };
            1
        }
        None => 0,
    }
}

// ───────────── style ─────────────

pub const QVP_SEL_PATH: u8 = 0;
pub const QVP_SEL_WORD: u8 = 1;
pub const QVP_SEL_AYAH: u8 = 2;
pub const QVP_SEL_LINE: u8 = 3;
pub const QVP_SEL_MARK: u8 = 4;
pub const QVP_SEL_FAMILY: u8 = 5;
pub const QVP_SEL_KIND: u8 = 6;
pub const QVP_SEL_DECO: u8 = 7;

fn selector(kind: u8, a: u32, b: u32, c: u32) -> Option<Selector> {
    Some(match kind {
        QVP_SEL_PATH => Selector::Path(a),
        QVP_SEL_WORD => Selector::Word(a as u16, b as u16, c as u16),
        QVP_SEL_AYAH => Selector::Ayah(a as u16, b as u16),
        QVP_SEL_LINE => Selector::Line(a as u8),
        QVP_SEL_MARK => Selector::Mark(Mark::from_u8(a as u8)),
        QVP_SEL_FAMILY => Selector::Family(Family::from_u8(a as u8)),
        QVP_SEL_KIND => Selector::Kind(PathKind::from_u8(a as u8)),
        QVP_SEL_DECO => Selector::Deco(DecoKind::from_u8(a as u8)),
        _ => return None,
    })
}

/// Set (`on` != 0) or remove a style. `rgba` is 0xRRGGBBAA; alpha 0 hides.
#[no_mangle]
pub unsafe extern "C" fn qvp_style(page: *mut Page, sel_kind: u8, a: u32, b: u32, c: u32, rgba: u32, on: i32) -> i32 {
    let Some(sel) = selector(sel_kind, a, b, c) else { return 0 };
    let p = &mut *page;
    if on != 0 {
        p.style.set(sel, rgba);
    } else {
        p.style.unset(sel);
    }
    1
}

#[no_mangle]
pub unsafe extern "C" fn qvp_style_clear(page: *mut Page) {
    (*page).style.clear();
}

#[no_mangle]
pub unsafe extern "C" fn qvp_style_default(page: *mut Page, rgba: u32) {
    (*page).style.default_ink = rgba;
}

/// Full display list: one 0xRRGGBBAA per path. Pointer valid until the next
/// engine call on this page.
#[no_mangle]
pub unsafe extern "C" fn qvp_paint(page: *mut Page) -> *const u32 {
    (*page).paint().as_ptr()
}

/// Overlay display list: writes up to `cap` (index, rgba) pairs into `out`,
/// returns the number of styled paths (may exceed `cap`).
#[no_mangle]
pub unsafe extern "C" fn qvp_styled(page: *const Page, out: *mut u32, cap: u32) -> u32 {
    let v = (*page).styled();
    if !out.is_null() {
        let o = std::slice::from_raw_parts_mut(out, (cap as usize) * 2);
        for (i, (idx, c)) in v.iter().enumerate().take(cap as usize) {
            o[i * 2] = *idx;
            o[i * 2 + 1] = *c;
        }
    }
    v.len() as u32
}

// ───────────── names (for UIs / debugging) ─────────────

#[no_mangle]
pub extern "C" fn qvp_mark_name(mark: u8) -> *const c_char {
    Mark::from_u8(mark).as_str().as_ptr() as *const c_char
}
#[no_mangle]
pub extern "C" fn qvp_mark_name_len(mark: u8) -> u32 {
    Mark::from_u8(mark).as_str().len() as u32
}
#[no_mangle]
pub extern "C" fn qvp_family_name(f: u8) -> *const c_char {
    Family::from_u8(f).as_str().as_ptr() as *const c_char
}
#[no_mangle]
pub extern "C" fn qvp_family_name_len(f: u8) -> u32 {
    Family::from_u8(f).as_str().len() as u32
}
#[no_mangle]
pub extern "C" fn qvp_kind_name(k: u8) -> *const c_char {
    PathKind::from_u8(k).as_str().as_ptr() as *const c_char
}
#[no_mangle]
pub extern "C" fn qvp_kind_name_len(k: u8) -> u32 {
    PathKind::from_u8(k).as_str().len() as u32
}
#[no_mangle]
pub extern "C" fn qvp_version() -> u32 {
    qvp_core::qvp_format::VERSION as u32
}
