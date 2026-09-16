//! C ABI over `qvp-core`. The single binding surface for wasm (web), Swift,
//! Kotlin (JNI shim), Dart FFI and the React Native shim. See `include/qvp.h`.
//!
//! Conventions: colours are 0xRRGGBBAA; `QVP_NONE` (0xFFFFFFFF) means absent;
//! strings come back through `QvpStr` (pointer + length, UTF-8, not NUL
//! terminated) and stay valid until the next call that returns a string on the
//! same thread; array outputs take a caller buffer + capacity and return the
//! total count (which may exceed the capacity — call again with a bigger buffer).
#![allow(clippy::missing_safety_doc)]

use qvp_core::qvp_format::{Category, DecoKind, Family, Mark, PathKind};
use qvp_core::*;
use std::cell::RefCell;
use std::ffi::c_char;

/// Run one entry point's body and turn a panic in the engine into the function's error
/// value (0, -1 or null) instead of aborting the host process. The C ABI is the boundary
/// between a page file the host downloaded and the app that renders it; a malformed file
/// must never take the app down. On wasm32 there is no unwinding, so a panic still traps
/// there.
fn guard<T: FfiDefault>(body: impl FnOnce() -> T) -> T {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(body)) {
        Ok(v) => v,
        Err(_) => T::ffi_default(),
    }
}

/// The value an entry point returns when the engine panicked: null for pointers, -1 for
/// signed results (never a valid index), 0 for everything else.
trait FfiDefault {
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
    static STR_BUF: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
    static LAYOUT_BUF: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

#[repr(C)]
pub struct QvpStr {
    pub ptr: *const u8,
    pub len: u32,
}

unsafe fn out_str(out: *mut QvpStr, s: &str) {
    STR_BUF.with(|b| {
        let mut b = b.borrow_mut();
        b.clear();
        b.extend_from_slice(s.as_bytes());
        *out = QvpStr { ptr: b.as_ptr(), len: b.len() as u32 };
    });
}

unsafe fn in_str<'a>(ptr: *const u8, len: u32) -> &'a str {
    if ptr.is_null() || len == 0 {
        return "";
    }
    std::str::from_utf8(std::slice::from_raw_parts(ptr, len as usize)).unwrap_or("")
}

unsafe fn fill<T: Copy>(out: *mut T, cap: u32, items: &[T]) -> u32 {
    if !out.is_null() && cap > 0 {
        let n = items.len().min(cap as usize);
        std::ptr::copy_nonoverlapping(items.as_ptr(), out, n);
    }
    items.len() as u32
}

// ───────────── structs ─────────────

#[repr(C)]
pub struct QvpPageInfo {
    pub width: f32,
    pub height: f32,
    pub page: u32,
    pub n_lines: u32,
    pub n_ayahs: u32,
    pub n_words: u32,
    pub n_paths: u32,
    pub n_decorations: u32,
}

#[repr(C)]
pub struct QvpGeometry {
    pub ops: *const u8,
    pub ops_len: u32,
    pub pts: *const f32,
    pub pts_len: u32,
    /// `n_paths` records of 8 × u32: op_start, op_count, pt_start, pt_count, flags, word, line, extra
    pub table: *const u32,
    pub n_paths: u32,
}

#[repr(C)]
pub struct QvpWordInfo {
    pub surah: u16,
    pub ayah: u16,
    pub word: u16,
    pub line_number: u16,
    pub ayah_index: u32,
    pub line_index: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub text: QvpStr,
    pub first_path: u32,
    pub n_paths: u32,
}

#[repr(C)]
pub struct QvpAyahInfo {
    pub surah: u16,
    pub ayah: u16,
    pub fragment: u8,
    pub fragments: u8,
    pub flags: u8,
    pub _pad: u8,
    pub rubu_al_hizb: u16,
    pub first_word: u32,
    pub n_words: u32,
    pub ayah_mark_decoration: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

#[repr(C)]
pub struct QvpLineInfo {
    pub line_number: u8,
    pub is_header: u8,
    pub first_word: u32,
    pub n_words: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    /// line-spacing band and body centre (page units)
    pub band_y0: f32,
    pub band_y1: f32,
    pub centre: f32,
}

#[repr(C)]
pub struct QvpDecorationInfo {
    pub decoration: u8,
    pub _pad: u8,
    pub surah: u16,
    pub ayah: u16,
    pub _pad2: u16,
    pub line: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub text: QvpStr,
    pub first_path: u32,
    pub n_paths: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpHit {
    pub word: u32,
    pub path: u32,
    pub decoration: u32,
    pub line: u32,
    pub distance: f32,
    pub is_exact: u8,
}

#[repr(C)]
pub struct QvpHitOptions {
    pub max_distance: f32,
    pub gap_bias: f32,
    pub prefer_exact: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpBox {
    pub id: u32,
    pub line: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub color: u32,
    pub radius: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpHitArea {
    pub word: u32,
    pub line: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub ink_x0: f32,
    pub ink_y0: f32,
    pub ink_x1: f32,
    pub ink_y1: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpLineBand {
    pub line: u32,
    pub line_number: u32,
    pub y0: f32,
    pub y1: f32,
    pub mid: f32,
    pub ink_y0: f32,
    pub ink_y1: f32,
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
    pub fill_height: u8,
    pub grid_lines: u32,
    pub crop_left: f32,
    pub crop_right: f32,
    pub max_aspect_slack: f32,
    /// How much bigger the ink is than fit-to-width when the words are broken onto rows of
    /// the page's own width. 0 lays the page out as printed (no reflow).
    pub reflow_zoom: f32,
    /// 0 ragged, 1 justified, 2 centred, 255 the engine's own default
    pub reflow_fill: u8,
    /// how the words are broken onto rows: 0 greedy, 1 even, 255 the engine's own default
    pub reflow_breaks: u8,
    /// 0 the printed gap between the two words, 1 the page's median gap between letters
    pub reflow_gaps: u8,
    /// multiplier on every gap (1 = the gap `reflow_gaps` picked)
    pub reflow_word_gap: f32,
    /// how far a justified row's gaps may stretch, as a multiple of the gaps it started with
    /// (0 = the engine's default, a negative value = no cap)
    pub reflow_max_stretch: f32,
    /// how far a row much shorter than the row beside it is opened towards it, 0 to 1
    /// (a negative value = the engine's default)
    pub reflow_relax: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpGrid {
    pub lines: u32,
    pub line_spacing: f32,
}

#[repr(C)]
pub struct QvpLayout {
    pub scale: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub content_w: f32,
    pub content_h: f32,
    pub line_spacing: f32,
    pub n_lines: u32,
    /// n_lines × {dy, slot_top, slot_bottom}; valid until the next qvp_layout call on this thread
    pub lines: *const f32,
    /// The view transform that shows the whole content: draw at fit_x + fit_scale·view_x,
    /// fit_y + fit_scale·view_y; the host's pan and zoom go on top.
    pub fit_scale: f32,
    pub fit_x: f32,
    pub fit_y: f32,
    /// 1 when the words were broken onto rows of their own, 0 when the page is as printed.
    pub reflowed: u32,
    /// Rows the reflow produced (0 when not reflowed). `lines` then holds the rows.
    pub n_rows: u32,
}

/// kind: 0 Page, 1 Word(a), 2 Words(words,n), 3 Ayah(a,b), 4 AyahRange(a,b,c), 5 Line(a), 6 Surah(a), 7 Range(a,b)
#[repr(C)]
pub struct QvpTarget {
    pub target: u8,
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub words: *const u32,
    pub n_words: u32,
}

/// kind: 0 Page, 1 Path(a), 2 WordPath(a,b), 3 WordMark(a,b), 4 WordMarkNamed(a, mark b, nth c),
/// 5 WordBody(a), 6 WordMarks(a), 7 Word(a), 8 Ayah(a,b), 9 Line(a), 10 Mark(a), 11 Category(a),
/// 12 Family(a), 13 Kind(a), 14 Deco(a), 15 DecoIdx(a)
#[repr(C)]
pub struct QvpSelector {
    pub selector: u8,
    pub a: u32,
    pub b: u32,
    pub c: u32,
}

#[repr(C)]
pub struct QvpHighlightStyle {
    pub mode: u8,
    pub height: u8,
    pub ink: u32,
    pub band: u32,
    pub pad_x: f32,
    pub pad_y: f32,
    pub radius: f32,
    pub seam: f32,
    pub transition_ms: u32,
    pub layer: i32,
}

/// Colours with alpha 0 mean "leave alone".
#[repr(C)]
pub struct QvpTheme {
    pub ink: u32,
    pub diacritics: u32,
    pub dots: u32,
    pub waqf: u32,
    pub sifr: u32,
    pub ayah_mark: u32,
    pub numeral: u32,
    pub headers: u32,
    pub transition_ms: u32,
    /// pairs of (mark, colour)
    pub marks: *const u32,
    pub n_marks: u32,
}

#[repr(C)]
pub struct QvpSurah {
    pub number: u16,
    pub ayah_count: u16,
    pub has_banner: u8,
    pub has_basmalah: u8,
    pub place: u8,
    pub _pad: u8,
    pub banner_decoration: u32,
    pub arabic: QvpStr,
    pub latin: QvpStr,
    pub english: QvpStr,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpDivision {
    pub division: u8,
    pub line: u8,
    pub number: u16,
    pub surah: u16,
    pub ayah: u16,
    pub ayah_index: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpAyahMark {
    pub decoration: u32,
    pub surah: u16,
    pub ayah: u16,
    pub line: u32,
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
    pub ornament_path: u32,
    pub numeral_path: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpRosette {
    pub decoration: u32,
    pub surah: u16,
    pub ayah: u16,
    pub juz: u16,
    pub hizb: u16,
    pub nisf: u16,
    pub rubu_al_hizb: u16,
    pub rubu_al_hizb_in_hizb: u16,
    pub _pad: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpSajdah {
    pub decoration: u32,
    pub surah: u16,
    pub ayah: u16,
    pub sign_path: u32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpMatch {
    pub word: u32,
    pub index: u32,
    pub is_loose_match: u8,
}

#[repr(C)]
pub struct QvpCropBounds {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub n_words: u32,
    pub ayah_mark_decoration: u32,
}

#[repr(C)]
pub struct QvpAtlasSurah {
    pub number: u16,
    pub first_page: u16,
    pub ayah_count: u16,
    pub place: u8,
    pub _pad: u8,
    pub arabic: QvpStr,
    pub latin: QvpStr,
    pub english: QvpStr,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpAtlasRubuAlHizb {
    pub rubu_al_hizb: u16,
    pub surah: u16,
    pub ayah: u16,
    pub page: u16,
}

// ───────────── helpers ─────────────

unsafe fn target(t: *const QvpTarget) -> Target {
    let t = &*t;
    match t.target {
        1 => Target::Word(t.a),
        2 => Target::Words(if t.words.is_null() {
            vec![]
        } else {
            std::slice::from_raw_parts(t.words, t.n_words as usize).to_vec()
        }),
        3 => Target::Ayah(t.a as u16, t.b as u16),
        4 => Target::AyahRange(t.a as u16, t.b as u16, t.c as u16),
        5 => Target::Line(t.a as u8),
        6 => Target::Surah(t.a as u16),
        7 => Target::Range(t.a, t.b),
        _ => Target::Page,
    }
}

unsafe fn selector(s: *const QvpSelector) -> Option<Selector> {
    let s = &*s;
    Some(match s.selector {
        0 => Selector::Page,
        1 => Selector::Path(s.a),
        2 => Selector::WordPath(s.a, s.b as u16),
        3 => Selector::WordMark(s.a, s.b as u16),
        4 => Selector::WordMarkNamed(s.a, Mark::from_u8(s.b as u8), s.c as u16),
        5 => Selector::WordBody(s.a),
        6 => Selector::WordMarks(s.a),
        7 => Selector::Word(s.a),
        8 => Selector::Ayah(s.a as u16, s.b as u16),
        9 => Selector::Line(s.a as u8),
        10 => Selector::Mark(Mark::from_u8(s.a as u8)),
        11 => Selector::Category(Category::from_u8(s.a as u8)),
        12 => Selector::Family(Family::from_u8(s.a as u8)),
        13 => Selector::Kind(PathKind::from_u8(s.a as u8)),
        14 => Selector::Deco(DecoKind::from_u8(s.a as u8)),
        15 => Selector::DecoIdx(s.a),
        _ => return None,
    })
}

unsafe fn hstyle(s: *const QvpHighlightStyle) -> HighlightStyle {
    let s = &*s;
    HighlightStyle {
        mode: match s.mode {
            0 => HighlightMode::Ink,
            2 => HighlightMode::Both,
            _ => HighlightMode::Band,
        },
        height: if s.height == 1 { BandHeight::Ink } else { BandHeight::LineSpacing },
        ink: s.ink,
        band: s.band,
        pad_x: s.pad_x,
        pad_y: s.pad_y,
        radius: s.radius,
        seam: s.seam,
        transition_ms: s.transition_ms,
        layer: s.layer,
    }
}

fn vb(b: &ViewBox) -> QvpBox {
    QvpBox { id: b.highlight, line: b.line, x0: b.x0, y0: b.y0, x1: b.x1, y1: b.y1, color: b.color, radius: b.radius }
}

// ───────────── memory ─────────────

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

// ───────────── page ─────────────

#[no_mangle]
pub unsafe extern "C" fn qvp_page_load(bytes: *const u8, len: usize) -> *mut Page {
    guard(|| {
        if bytes.is_null() {
            return std::ptr::null_mut();
        }
        match Page::load(std::slice::from_raw_parts(bytes, len)) {
            Ok(p) => Box::into_raw(Box::new(p)),
            Err(_) => std::ptr::null_mut(),
        }
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_page_free(page: *mut Page) {
    guard(|| {
        if !page.is_null() {
            drop(Box::from_raw(page));
        }
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_page_info(page: *const Page, out: *mut QvpPageInfo) {
    guard(|| {
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
            n_decorations: d.decorations.len() as u32,
        };
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_geometry(page: *const Page, out: *mut QvpGeometry) {
    guard(|| {
        let g = (*page).geometry();
        *out = QvpGeometry {
            ops: g.ops.as_ptr(),
            ops_len: g.ops.len() as u32,
            pts: g.pts.as_ptr(),
            pts_len: g.pts.len() as u32,
            table: g.table.as_ptr() as *const u32,
            n_paths: g.table.len() as u32,
        };
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_word_info(page: *const Page, index: u32, out: *mut QvpWordInfo) -> i32 {
    guard(|| {
        let p = &*page;
        let d = p.data();
        let Some(w) = d.words.get(index as usize) else { return 0 };
        let q = p.quant();
        let t = p.word_text(index);
        *out = QvpWordInfo {
            surah: w.surah,
            ayah: w.ayah,
            word: w.word,
            line_number: d.lines[w.line_index as usize].line_number as u16,
            ayah_index: w.ayah_index as u32,
            line_index: w.line_index as u32,
            x0: w.bbox.x0 as f32 / q,
            y0: w.bbox.y0 as f32 / q,
            x1: w.bbox.x1 as f32 / q,
            y1: w.bbox.y1 as f32 / q,
            text: QvpStr { ptr: t.as_ptr(), len: t.len() as u32 },
            first_path: w.first_path,
            n_paths: w.n_paths as u32,
        };
        1
    })
}
/// form: 0 rasm_uthmani, 1 rasm_imlai, 2 qpc, 3 rasm, 4 search
#[no_mangle]
pub unsafe extern "C" fn qvp_word_form(page: *const Page, index: u32, form: u8, out: *mut QvpStr) -> i32 {
    guard(|| {
        let p = &*page;
        if index as usize >= p.data().words.len() {
            return 0;
        }
        let t = p.word_form(index, Form::from_u8(form));
        *out = QvpStr { ptr: t.as_ptr(), len: t.len() as u32 };
        1
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_ayah_info(page: *const Page, index: u32, out: *mut QvpAyahInfo) -> i32 {
    guard(|| {
        let p = &*page;
        let Some(a) = p.data().ayahs.get(index as usize) else { return 0 };
        let q = p.quant();
        *out = QvpAyahInfo {
            surah: a.surah,
            ayah: a.ayah,
            fragment: a.fragment,
            fragments: a.fragments,
            flags: a.flags,
            _pad: 0,
            rubu_al_hizb: a.rubu_al_hizb,
            first_word: a.first_word as u32,
            n_words: a.n_words as u32,
            ayah_mark_decoration: if a.ayah_mark_decoration == u16::MAX { NONE } else { a.ayah_mark_decoration as u32 },
            x0: a.bbox.x0 as f32 / q,
            y0: a.bbox.y0 as f32 / q,
            x1: a.bbox.x1 as f32 / q,
            y1: a.bbox.y1 as f32 / q,
        };
        1
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_line_info(page: *const Page, index: u32, out: *mut QvpLineInfo) -> i32 {
    guard(|| {
        let p = &*page;
        let Some(l) = p.data().lines.get(index as usize) else { return 0 };
        let q = p.quant();
        let spacing = p.line_spacing();
        let c = p.line_centre(index as usize);
        *out = QvpLineInfo {
            line_number: l.line_number,
            is_header: p.line_is_header(index as usize) as u8,
            first_word: l.first_word as u32,
            n_words: l.n_words as u32,
            x0: l.bbox.x0 as f32 / q,
            y0: l.bbox.y0 as f32 / q,
            x1: l.bbox.x1 as f32 / q,
            y1: l.bbox.y1 as f32 / q,
            band_y0: c - spacing / 2.0,
            band_y1: c + spacing / 2.0,
            centre: c,
        };
        1
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_decoration_info(page: *const Page, index: u32, out: *mut QvpDecorationInfo) -> i32 {
    guard(|| {
        let p = &*page;
        let Some(d) = p.data().decorations.get(index as usize) else { return 0 };
        let q = p.quant();
        let t = p.deco_text(index);
        *out = QvpDecorationInfo {
            decoration: d.kind as u8,
            _pad: 0,
            surah: d.surah,
            ayah: d.ayah,
            _pad2: 0,
            line: p.geometry().table[d.first_path as usize].line,
            x0: d.bbox.x0 as f32 / q,
            y0: d.bbox.y0 as f32 / q,
            x1: d.bbox.x1 as f32 / q,
            y1: d.bbox.y1 as f32 / q,
            text: QvpStr { ptr: t.as_ptr(), len: t.len() as u32 },
            first_path: d.first_path,
            n_paths: d.n_paths as u32,
        };
        1
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_find_word(page: *const Page, surah: u16, ayah: u16, word: u16) -> i32 {
    guard(|| guard(|| (*page).find_word(surah, ayah, word).map(|i| i as i32).unwrap_or(-1)))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_target_words(page: *const Page, t: *const QvpTarget, out: *mut u32, cap: u32) -> u32 {
    guard(|| {
        let v = (*page).target_words(&target(t));
        fill(out, cap, &v)
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_page_line_spacing(page: *const Page) -> f32 {
    guard(|| guard(|| (*page).line_spacing()))
}
/// The grid the page is laid out inside: the mushaf's line count and the printed line spacing.
#[no_mangle]
pub unsafe extern "C" fn qvp_page_grid(page: *const Page, out: *mut QvpGrid) {
    guard(|| {
        let g = (*page).grid();
        *out = QvpGrid { lines: g.lines, line_spacing: g.line_spacing };
    })
}

// ───────────── metadata ─────────────

#[no_mangle]
pub unsafe extern "C" fn qvp_surah_count(page: *const Page) -> u32 {
    guard(|| guard(|| (*page).surahs().len() as u32))
}
/// Strings point into a thread-local buffer valid until the next string-returning call.
#[no_mangle]
pub unsafe extern "C" fn qvp_surah_at(page: *const Page, i: u32, out: *mut QvpSurah) -> i32 {
    guard(|| {
        let v = (*page).surahs();
        let Some(s) = v.get(i as usize) else { return 0 };
        let joined = format!("{}\u{0}{}\u{0}{}", s.arabic, s.latin, s.english);
        STR_BUF.with(|b| {
            let mut b = b.borrow_mut();
            b.clear();
            b.extend_from_slice(joined.as_bytes());
            let base = b.as_ptr();
            let (a, l, e) = (s.arabic.len(), s.latin.len(), s.english.len());
            *out = QvpSurah {
                number: s.number,
                ayah_count: s.ayah_count,
                has_banner: s.has_banner as u8,
                has_basmalah: s.has_basmalah as u8,
                place: match s.revelation_place.as_str() {
                    "makkah" => 0,
                    "madinah" => 1,
                    _ => 255,
                },
                _pad: 0,
                banner_decoration: s.banner_decoration,
                arabic: QvpStr { ptr: base, len: a as u32 },
                latin: QvpStr { ptr: base.add(a + 1), len: l as u32 },
                english: QvpStr { ptr: base.add(a + l + 2), len: e as u32 },
            };
        });
        1
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_divisions(page: *const Page, out: *mut QvpDivision, cap: u32) -> u32 {
    guard(|| {
        let v: Vec<QvpDivision> = (*page)
            .divisions()
            .iter()
            .map(|d| QvpDivision {
                division: d.kind,
                line: d.line,
                number: d.n,
                surah: d.surah,
                ayah: d.ayah,
                ayah_index: d.ayah_index,
            })
            .collect();
        fill(out, cap, &v)
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_ayah_marks(page: *const Page, out: *mut QvpAyahMark, cap: u32) -> u32 {
    guard(|| {
        let v: Vec<QvpAyahMark> = (*page)
            .ayah_marks()
            .iter()
            .map(|m| QvpAyahMark {
                decoration: m.decoration,
                surah: m.surah,
                ayah: m.ayah,
                line: m.line,
                cx: m.cx,
                cy: m.cy,
                r: m.r,
                ornament_path: m.ornament_path,
                numeral_path: m.numeral_path,
            })
            .collect();
        fill(out, cap, &v)
    })
}
/// The ayah medallions in viewport px through the current layout: where each is drawn.
#[no_mangle]
pub unsafe extern "C" fn qvp_ayah_marks_view(page: *const Page, out: *mut QvpAyahMark, cap: u32) -> u32 {
    guard(|| {
        let v: Vec<QvpAyahMark> = (*page)
            .ayah_marks_view()
            .iter()
            .map(|m| QvpAyahMark {
                decoration: m.decoration,
                surah: m.surah,
                ayah: m.ayah,
                line: m.line,
                cx: m.cx,
                cy: m.cy,
                r: m.r,
                ornament_path: m.ornament_path,
                numeral_path: m.numeral_path,
            })
            .collect();
        fill(out, cap, &v)
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_rosettes(page: *const Page, out: *mut QvpRosette, cap: u32) -> u32 {
    guard(|| {
        let v: Vec<QvpRosette> = (*page)
            .rosettes()
            .iter()
            .map(|r| QvpRosette {
                decoration: r.decoration,
                surah: r.surah,
                ayah: r.ayah,
                juz: r.juz,
                hizb: r.hizb,
                nisf: r.nisf,
                rubu_al_hizb: r.rubu_al_hizb,
                rubu_al_hizb_in_hizb: r.rubu_al_hizb_in_hizb,
                _pad: 0,
            })
            .collect();
        fill(out, cap, &v)
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_sajdahs(page: *const Page, out: *mut QvpSajdah, cap: u32) -> u32 {
    guard(|| {
        let v: Vec<QvpSajdah> = (*page)
            .sajdahs()
            .iter()
            .map(|&(d, s, a, p)| QvpSajdah { decoration: d, surah: s, ayah: a, sign_path: p })
            .collect();
        fill(out, cap, &v)
    })
}
/// pairs of (surah, ayah) as u32 = surah<<16 | ayah
#[no_mangle]
pub unsafe extern "C" fn qvp_ayah_keys(page: *const Page, out: *mut u32, cap: u32) -> u32 {
    guard(|| {
        let v: Vec<u32> = (*page).ayah_keys().iter().map(|(s, a)| (*s as u32) << 16 | *a as u32).collect();
        fill(out, cap, &v)
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_ayah_word_count(page: *const Page, surah: u16, ayah: u16, is_complete: *mut u8) -> u32 {
    guard(|| {
        let (n, c) = (*page).ayah_word_count(surah, ayah);
        if !is_complete.is_null() {
            *is_complete = c as u8;
        }
        n
    })
}
/// Returns the number of words written, or -1 when the segment count does not match
/// (follow the ayah whole instead of drifting).
#[no_mangle]
pub unsafe extern "C" fn qvp_recite_map(
    page: *const Page,
    surah: u16,
    ayah: u16,
    n_segments: u32,
    out: *mut u32,
    cap: u32,
) -> i32 {
    guard(|| match (*page).recite_map(surah, ayah, n_segments) {
        Some(v) => fill(out, cap, &v) as i32,
        None => -1,
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_word_label(page: *const Page, word_index: u32, out: *mut QvpStr) {
    guard(|| {
        out_str(out, &(*page).word_label(word_index));
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_ayah_label(page: *const Page, ayah_index: u32, out: *mut QvpStr) {
    guard(|| {
        out_str(out, &(*page).ayah_label(ayah_index));
    })
}

// ───────────── text & search ─────────────

#[no_mangle]
pub unsafe extern "C" fn qvp_text(
    page: *const Page,
    t: *const QvpTarget,
    form: u8,
    word_sep: *const u8,
    word_sep_len: u32,
    line_sep: *const u8,
    line_sep_len: u32,
    out: *mut QvpStr,
) {
    guard(|| {
        let ws = if t.is_null() { (*page).target_words(&Target::Page) } else { (*page).target_words(&target(t)) };
        let s =
            (*page).text_of(&ws, Form::from_u8(form), in_str(word_sep, word_sep_len), in_str(line_sep, line_sep_len));
        out_str(out, &s);
    })
}
/// mode: 0 includes, 1 exact, 2 prefix
#[no_mangle]
pub unsafe extern "C" fn qvp_search(
    page: *const Page,
    query: *const u8,
    query_len: u32,
    form: u8,
    mode: u8,
    normalize: u8,
    loose_match: u8,
    limit: u32,
    out: *mut QvpMatch,
    cap: u32,
) -> u32 {
    guard(|| {
        let opt = SearchOptions {
            form: Form::from_u8(form),
            mode: match mode {
                1 => SearchMode::Exact,
                2 => SearchMode::Prefix,
                _ => SearchMode::Includes,
            },
            normalize: normalize != 0,
            loose: loose_match != 0,
            limit: if limit == 0 { usize::MAX } else { limit as usize },
        };
        let v: Vec<QvpMatch> = (*page)
            .search(in_str(query, query_len), &opt)
            .iter()
            .map(|m| QvpMatch { word: m.word, index: m.index as u32, is_loose_match: m.loose as u8 })
            .collect();
        fill(out, cap, &v)
    })
}
/// op (`QVP_ARABIC_*`): 0 strip marks, 1 fold, 2 normalize query (match fold), 3 loose key,
/// 4 search key. See docs/SEARCH-FOLD.md; 4 is additive, the rest are unchanged.
#[no_mangle]
pub unsafe extern "C" fn qvp_arabic(op: u8, s: *const u8, len: u32, out: *mut QvpStr) {
    guard(|| {
        let i = in_str(s, len);
        let r = match op {
            0 => strip_marks(i),
            1 => fold(i),
            3 => loose_key(i),
            4 => search_key(i),
            _ => normalize_query(i),
        };
        out_str(out, &r);
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_citation(page: *const Page, words: *const u32, n: u32, out: *mut QvpStr) {
    guard(|| {
        let ws = std::slice::from_raw_parts(words, n as usize);
        out_str(out, &(*page).citation(ws));
    })
}

/// Attach a JSON sidecar of derived text forms. Returns words updated, or -1 on a parse error.
#[no_mangle]
pub unsafe extern "C" fn qvp_attach_words(page: *mut Page, json: *const u8, len: u32) -> i32 {
    guard(|| {
        guard(|| match (*page).attach_words_json(std::slice::from_raw_parts(json, len as usize)) {
            Some(n) => n as i32,
            None => -1,
        })
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_has_form(page: *const Page, form: u8) -> u8 {
    guard(|| guard(|| (*page).has_form(Form::from_u8(form)) as u8))
}

// ───────────── hit testing ─────────────

/// An exact hit in the one hit struct: the word's line, distance 0, `is_exact` 1.
unsafe fn exact_hit(page: *const Page, h: HitExact) -> QvpHit {
    let line = if h.word == u32::MAX { u32::MAX } else { (*page).data().words[h.word as usize].line_index as u32 };
    QvpHit { word: h.word, path: h.path, decoration: h.decoration, line, distance: 0.0, is_exact: 1 }
}
#[no_mangle]
pub unsafe extern "C" fn qvp_hit_test_exact(page: *const Page, x: f32, y: f32, out: *mut QvpHit) -> i32 {
    guard(|| match (*page).hit_test_exact(x, y) {
        Some(h) => {
            *out = exact_hit(page, h);
            1
        }
        None => 0,
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_hit_test_exact_view(page: *const Page, view_x: f32, view_y: f32, out: *mut QvpHit) -> i32 {
    guard(|| match (*page).hit_test_exact_view(view_x, view_y) {
        Some(h) => {
            *out = exact_hit(page, h);
            1
        }
        None => 0,
    })
}
unsafe fn hopt(o: *const QvpHitOptions) -> HitOptions {
    if o.is_null() {
        return HitOptions::default();
    }
    let o = &*o;
    HitOptions {
        max_distance: if o.max_distance <= 0.0 { f32::INFINITY } else { o.max_distance },
        gap_bias: o.gap_bias,
        prefer_exact: o.prefer_exact != 0,
    }
}
fn hx(h: Hit) -> QvpHit {
    QvpHit {
        word: h.word,
        path: h.path,
        decoration: h.decoration,
        line: h.line,
        distance: h.distance,
        is_exact: h.is_exact as u8,
    }
}
/// Gap-aware: every point on a printed line resolves to a word. `opt` may be NULL.
#[no_mangle]
pub unsafe extern "C" fn qvp_hit_test(
    page: *const Page,
    x: f32,
    y: f32,
    opt: *const QvpHitOptions,
    out: *mut QvpHit,
) -> i32 {
    guard(|| match (*page).hit_test(x, y, &hopt(opt)) {
        Some(h) => {
            *out = hx(h);
            1
        }
        None => 0,
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_hit_test_view(
    page: *const Page,
    view_x: f32,
    view_y: f32,
    opt: *const QvpHitOptions,
    out: *mut QvpHit,
) -> i32 {
    guard(|| match (*page).hit_test_view(view_x, view_y, &hopt(opt)) {
        Some(h) => {
            *out = hx(h);
            1
        }
        None => 0,
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_line_bands(page: *const Page, out: *mut QvpLineBand, cap: u32) -> u32 {
    guard(|| {
        let v: Vec<QvpLineBand> = (*page)
            .line_bands()
            .iter()
            .map(|b| QvpLineBand {
                line: b.line,
                line_number: b.line_number as u32,
                y0: b.y0,
                y1: b.y1,
                mid: b.mid,
                ink_y0: b.ink_y0,
                ink_y1: b.ink_y1,
            })
            .collect();
        fill(out, cap, &v)
    })
}
/// The hit boxes in viewport px through the current layout: the partition a tap is resolved
/// with, so an overlay and a tap agree. On a reflowed page these are the rows' own boxes.
#[no_mangle]
pub unsafe extern "C" fn qvp_hit_areas_view(page: *const Page, gap_bias: f32, out: *mut QvpHitArea, cap: u32) -> u32 {
    guard(|| {
        let v: Vec<QvpHitArea> = (*page)
            .hit_areas_view(gap_bias)
            .iter()
            .map(|h| QvpHitArea {
                word: h.word,
                line: h.line,
                x0: h.x0,
                y0: h.y0,
                x1: h.x1,
                y1: h.y1,
                ink_x0: h.ink_x0,
                ink_y0: h.ink_y0,
                ink_x1: h.ink_x1,
                ink_y1: h.ink_y1,
            })
            .collect();
        fill(out, cap, &v)
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_hit_areas(page: *const Page, gap_bias: f32, out: *mut QvpHitArea, cap: u32) -> u32 {
    guard(|| {
        let v: Vec<QvpHitArea> = (*page)
            .hit_areas(gap_bias)
            .iter()
            .map(|h| QvpHitArea {
                word: h.word,
                line: h.line,
                x0: h.x0,
                y0: h.y0,
                x1: h.x1,
                y1: h.y1,
                ink_x0: h.ink_x0,
                ink_y0: h.ink_y0,
                ink_x1: h.ink_x1,
                ink_y1: h.ink_y1,
            })
            .collect();
        fill(out, cap, &v)
    })
}

// ───────────── the reader's pan and zoom ─────────────

/// Pan and zoom over a laid-out page: a point `p` in layout px draws at `offset + scale·p`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpView {
    pub scale: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl From<QvpView> for qvp_core::View {
    fn from(v: QvpView) -> Self {
        qvp_core::View { scale: v.scale, offset_x: v.offset_x, offset_y: v.offset_y }
    }
}
impl From<qvp_core::View> for QvpView {
    fn from(v: qvp_core::View) -> Self {
        QvpView { scale: v.scale, offset_x: v.offset_x, offset_y: v.offset_y }
    }
}

/// Zoom by `factor` about a point of the viewport, holding the scale between `min` and `max`
/// (0 for the engine's own limits): what the point under two fingers asks for.
#[no_mangle]
pub unsafe extern "C" fn qvp_view_zoom_about(
    view: *const QvpView,
    focal_x: f32,
    focal_y: f32,
    factor: f32,
    min: f32,
    max: f32,
    out: *mut QvpView,
) {
    guard(|| *out = qvp_core::View::from(*view).zoom_about(focal_x, focal_y, factor, min, max).into())
}
/// Move the view by a drag, in viewport px.
#[no_mangle]
pub unsafe extern "C" fn qvp_view_pan(view: *const QvpView, dx: f32, dy: f32, out: *mut QvpView) {
    guard(|| *out = qvp_core::View::from(*view).pan(dx, dy).into())
}
/// Hold the content against the viewport: centre an axis it does not fill, cover the viewport
/// on an axis it overflows, so no drag opens a blank strip beside the page.
#[no_mangle]
pub unsafe extern "C" fn qvp_view_clamp(
    view: *const QvpView,
    content_w: f32,
    content_h: f32,
    viewport_w: f32,
    viewport_h: f32,
    out: *mut QvpView,
) {
    guard(|| *out = qvp_core::View::from(*view).clamp(content_w, content_h, viewport_w, viewport_h).into())
}
/// The view that puts a point inside a word (`nx`, `ny` from 0 to 1) at a place on the screen,
/// then clamps. This is how a pinch holds its place when the layout reflows under it.
#[no_mangle]
pub unsafe extern "C" fn qvp_view_anchor(
    page: *const Page,
    view: *const QvpView,
    word: u32,
    nx: f32,
    ny: f32,
    to_x: f32,
    to_y: f32,
    viewport_w: f32,
    viewport_h: f32,
    out: *mut QvpView,
) {
    guard(|| *out = (*page).view_anchor((*view).into(), word, (nx, ny), (to_x, to_y), (viewport_w, viewport_h)).into())
}
/// Where a viewport point lands in the laid-out page, for turning a touch into a hit test.
#[no_mangle]
pub unsafe extern "C" fn qvp_view_to_layout(page: *const Page, view: *const QvpView, vx: f32, vy: f32, out: *mut f32) {
    guard(|| {
        let (x, y) = (*page).view_to_layout((*view).into(), vx, vy);
        *out = x;
        *out.add(1) = y;
    })
}
/// +1 or -1 when a released drag is a page swipe, 0 when it is not.
#[no_mangle]
pub unsafe extern "C" fn qvp_view_swipe(dx: f32, dy: f32, vx: f32, vy: f32) -> i32 {
    guard(|| qvp_core::swipe_direction(dx, dy, vx, vy))
}

/// Everything the current layout draws, in drawing order: `{path, placement}` pairs indexing
/// `qvp_layout_placements`. Omitted paths never appear and a repeated one appears twice, so one
/// loop draws any page. Returns the number of pairs, or the count needed.
///
/// `band_top`/`band_bottom` hold it to a band of the laid-out page, in viewport px: what keeps
/// a reflowed page smooth, since it is taller than the screen on purpose. `band_bottom` at or
/// below `band_top` means the whole page.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_draw_list(
    page: *const Page,
    band_top: f32,
    band_bottom: f32,
    out: *mut u32,
    cap: u32,
) -> u32 {
    guard(|| {
        let band = (band_bottom > band_top).then_some((band_top, band_bottom));
        let list = (*page).layout_draw_list_in(band);
        let n = list.len().min(cap as usize);
        for (i, d) in list.iter().take(n).enumerate() {
            *out.add(i * 2) = d.path;
            *out.add(i * 2 + 1) = d.placement;
        }
        list.len() as u32
    })
}

/// Every placement the current layout draws under: one per group, then one for each repeat.
/// Returns the number of placements, or the count needed.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_placements(page: *const Page, out: *mut f32, cap: u32) -> u32 {
    guard(|| {
        let Some(l) = (*page).current_layout() else { return 0 };
        let ps = l.placements();
        let n = ps.len().min(cap as usize);
        for (i, q) in ps.iter().take(n).enumerate() {
            *out.add(i * 4) = q.dx;
            *out.add(i * 4 + 1) = q.dy;
            *out.add(i * 4 + 2) = q.kx;
            *out.add(i * 4 + 3) = q.ky;
        }
        ps.len() as u32
    })
}

// ───────────── the reader's zoom control ─────────────

/// Where the reader's zoom control stands. All zero is the control a page opens on: stepped,
/// on the printed page.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpZoom {
    /// 0 stepped, 1 continuous, 2 magnify. Anything else is stepped.
    pub mode: u32,
    /// 0 is the printed page, 1 upwards the page's own steps. What stepped reads.
    pub step: u32,
    /// The reflow zoom in force, 1.0 at the printed page. What continuous reads.
    pub zoom: f32,
}

/// What a gesture produced: the control to keep, the view to draw with, and whether the page
/// was laid out again under it.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpZoomChange {
    pub zoom: QvpZoom,
    pub view: QvpView,
    pub relaid: u32,
}

impl From<QvpZoom> for qvp_core::Zoom {
    fn from(z: QvpZoom) -> Self {
        qvp_core::Zoom { mode: qvp_core::ZoomMode::from_u32(z.mode), step: z.step, zoom: z.zoom }
    }
}
impl From<qvp_core::Zoom> for QvpZoom {
    fn from(z: qvp_core::Zoom) -> Self {
        QvpZoom { mode: z.mode as u32, step: z.step, zoom: z.zoom }
    }
}
impl From<qvp_core::ZoomChange> for QvpZoomChange {
    fn from(c: qvp_core::ZoomChange) -> Self {
        QvpZoomChange { zoom: c.zoom.into(), view: c.view.into(), relaid: c.relaid as u32 }
    }
}

/// The same control under another policy, keeping the size the reader is already at.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_mode(
    page: *mut Page,
    spec: *const QvpLayoutSpec,
    zoom: *const QvpZoom,
    mode: u32,
    out: *mut QvpZoom,
) {
    guard(|| {
        let s = layout_spec(spec);
        *out = (*page).zoom_mode(&s, (*zoom).into(), qvp_core::ZoomMode::from_u32(mode)).into()
    })
}

/// One frame of a pinch. `factor` is the distance between the fingers against the distance
/// when they went down — the whole gesture every time, not the change since the last frame.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_pinch(
    page: *mut Page,
    spec: *const QvpLayoutSpec,
    zoom: *const QvpZoom,
    view: *const QvpView,
    factor: f32,
    focal_x: f32,
    focal_y: f32,
    out: *mut QvpZoomChange,
) {
    guard(|| {
        let s = layout_spec(spec);
        *out = (*page).zoom_pinch(&s, (*zoom).into(), (*view).into(), factor, (focal_x, focal_y)).into()
    })
}

/// The control moved straight to a step: a size button, a double tap, a reset. Step 0 is the
/// printed page.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_to_step(
    page: *mut Page,
    spec: *const QvpLayoutSpec,
    zoom: *const QvpZoom,
    step: u32,
    view: *const QvpView,
    out: *mut QvpZoomChange,
) {
    guard(|| {
        let s = layout_spec(spec);
        *out = (*page).zoom_to_step(&s, (*zoom).into(), step, (*view).into()).into()
    })
}

/// The same control on another page: what the reader was reading at, carried onto the page they
/// turned to. A step carries as a step, a free zoom as a size held inside what the page can reach.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_carried(
    page: *mut Page,
    spec: *const QvpLayoutSpec,
    zoom: *const QvpZoom,
    out: *mut QvpZoom,
) {
    guard(|| {
        let s = layout_spec(spec);
        *out = (*page).zoom_carried(&s, (*zoom).into()).into()
    })
}

/// The reflow zoom one step of this page's control means; step 0 is the printed page.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_at_step(page: *mut Page, spec: *const QvpLayoutSpec, step: u32) -> f32 {
    guard(|| {
        let s = layout_spec(spec);
        (*page).zoom_at_step(&s, step)
    })
}

/// 1 once the reader has zoomed in, by either road: a magnified view, or a page reflowed above
/// the printed size. `fit_scale` is the view scale the page is fitted at (0 = it is at it).
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_is_zoomed(zoom: *const QvpZoom, view: *const QvpView, fit_scale: f32) -> i32 {
    guard(|| qvp_core::Zoom::from(*zoom).is_zoomed((*view).into(), fit_scale) as i32)
}

/// What a sideways drag on this page means: 0 pan it, 1 turn the page.
#[no_mangle]
pub unsafe extern "C" fn qvp_sideways_drag(
    page: *const Page,
    zoom: *const QvpZoom,
    view: *const QvpView,
    fit_scale: f32,
) -> u32 {
    guard(|| (*page).sideways_drag((*zoom).into(), (*view).into(), fit_scale) as u32)
}

/// `spec` with this control's zoom in it: what the host lays out, draws and hit-tests with.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_spec(
    page: *mut Page,
    spec: *const QvpLayoutSpec,
    zoom: *const QvpZoom,
    out: *mut QvpLayoutSpec,
) {
    guard(|| {
        let s = layout_spec(spec);
        // the control owns one field of the spec and leaves the reader's spacing knobs alone
        let z = (*page).zoom_spec(&s, (*zoom).into()).reflow.map(|r| r.zoom).unwrap_or(0.0);
        *out = QvpLayoutSpec { reflow_zoom: z, ..*spec };
    })
}

// ───────────── layout ─────────────

unsafe fn layout_spec(spec: *const QvpLayoutSpec) -> LayoutSpec {
    let s = &*spec;
    LayoutSpec {
        viewport_w: s.viewport_w,
        viewport_h: s.viewport_h,
        pad_top: s.pad_top,
        pad_bottom: s.pad_bottom,
        pad_left: s.pad_left,
        pad_right: s.pad_right,
        line_spacing: s.line_spacing,
        fill_height: s.fill_height != 0,
        grid_lines: s.grid_lines,
        crop_left: s.crop_left,
        crop_right: s.crop_right,
        max_aspect_slack: s.max_aspect_slack,
        reflow: (s.reflow_zoom > 0.0).then(|| qvp_core::ReflowSpec {
            zoom: s.reflow_zoom,
            fill: match s.reflow_fill {
                0 => qvp_core::Fill::Ragged,
                1 => qvp_core::Fill::Justified,
                2 => qvp_core::Fill::Centred,
                _ => qvp_core::ReflowSpec::default().fill,
            },
            breaks: match s.reflow_breaks {
                0 => qvp_core::Breaks::Greedy,
                1 => qvp_core::Breaks::Even,
                2 => qvp_core::Breaks::Fitted,
                _ => qvp_core::ReflowSpec::default().breaks,
            },
            gaps: if s.reflow_gaps == 0 { qvp_core::GapMode::Printed } else { qvp_core::GapMode::Uniform },
            relax: if s.reflow_relax < 0.0 { qvp_core::defaults::REFLOW_RELAX } else { s.reflow_relax.min(1.0) },
            word_gap: if s.reflow_word_gap > 0.0 { s.reflow_word_gap } else { 1.0 },
            max_stretch: if s.reflow_max_stretch == 0.0 {
                qvp_core::defaults::REFLOW_MAX_STRETCH
            } else {
                s.reflow_max_stretch.max(0.0)
            },
        }),
    }
}
#[no_mangle]
pub unsafe extern "C" fn qvp_layout(page: *mut Page, spec: *const QvpLayoutSpec, out: *mut QvpLayout) {
    guard(|| {
        let l = (*page).layout(&layout_spec(spec));
        *out = read_layout(l);
    })
}

/// The layout the page already has, without computing one: what a host reads after a call that
/// laid the page out itself, such as the zoom control. Zero when the page has no layout yet.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_current(page: *const Page, out: *mut QvpLayout) -> u32 {
    guard(|| match (*page).current_layout() {
        Some(l) => {
            *out = read_layout(l);
            1
        }
        None => 0,
    })
}

unsafe fn read_layout(l: &qvp_core::Layout) -> QvpLayout {
    {
        // reflowed: the slots are the rows, and a printed line has no shift of its own
        let n = if l.is_reflowed() { l.line_slots.len() } else { l.line_dy.len() };
        let mut buf = Vec::with_capacity(n * 3);
        for i in 0..n {
            buf.push(l.line_dy.get(i).copied().unwrap_or(0.0));
            buf.push(l.line_slots[i].0);
            buf.push(l.line_slots[i].1);
        }
        let lines = buf.as_ptr();
        let n_lines = n as u32;
        let res = QvpLayout {
            scale: l.scale,
            offset_x: l.offset_x,
            offset_y: l.offset_y,
            content_w: l.content_w,
            content_h: l.content_h,
            line_spacing: l.line_spacing,
            n_lines,
            lines,
            fit_scale: l.fit_scale,
            fit_x: l.fit_x,
            fit_y: l.fit_y,
            reflowed: l.is_reflowed() as u32,
            n_rows: l.reflow.as_ref().map(|r| r.row_band.len() as u32).unwrap_or(0),
        };
        LAYOUT_BUF.with(|b| *b.borrow_mut() = buf);
        res
    }
}
/// Where each group of paths is placed: `n × {dx, dy, kx, ky}`, a point `p` of the group
/// being drawn at `(kx·p.x + dx, ky·p.y + dy)` in page units, before the layout's scale and
/// offset. Without reflow there is one group per printed line; with it, one per word then one
/// per decoration. Returns the number of groups, or the count needed when `cap` is too small.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_groups(page: *const Page, out: *mut f32, cap: u32) -> u32 {
    guard(|| {
        let Some(l) = (*page).current_layout() else { return 0 };
        let n = l.groups.len() as u32;
        if out.is_null() || cap < n {
            return n;
        }
        for (i, g) in l.groups.iter().enumerate() {
            *out.add(i * 4) = g.dx;
            *out.add(i * 4 + 1) = g.dy;
            *out.add(i * 4 + 2) = g.kx;
            *out.add(i * 4 + 3) = g.ky;
        }
        n
    })
}
/// Paths to draw a second time, at another placement: `n × {first_path, n_paths, dx, dy, kx,
/// ky}` as floats, the first two whole numbers. A sajdah line whose words ended up on two rows
/// is drawn over each of them. Returns the number of repeats, or the count needed.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_repeats(page: *const Page, out: *mut f32, cap: u32) -> u32 {
    guard(|| {
        let Some(flow) = (*page).current_layout().and_then(|l| l.reflow.as_ref()) else { return 0 };
        let n = flow.repeats.len() as u32;
        if out.is_null() || cap < n {
            return n;
        }
        for (i, r) in flow.repeats.iter().enumerate() {
            *out.add(i * 6) = r.first_path as f32;
            *out.add(i * 6 + 1) = r.n_paths as f32;
            *out.add(i * 6 + 2) = r.placement.dx;
            *out.add(i * 6 + 3) = r.placement.dy;
            *out.add(i * 6 + 4) = r.placement.kx;
            *out.add(i * 6 + 5) = r.placement.ky;
        }
        n
    })
}
/// The group of every path, for [`qvp_layout_groups`]. Empty without reflow, where a path's
/// group is its line (`qvp_path_line`). Returns the number of paths, or the count needed.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_path_groups(page: *const Page, out: *mut u32, cap: u32) -> u32 {
    guard(|| {
        let Some(l) = (*page).current_layout() else { return 0 };
        let n = l.path_group.len() as u32;
        if out.is_null() || cap < n {
            return n;
        }
        for (i, g) in l.path_group.iter().enumerate() {
            *out.add(i) = *g;
        }
        n
    })
}
/// Paths the current layout does not draw: on a reflowed page the running head and the page
/// number, which the print puts outside the page box. Returns how many, or the count needed.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_omitted_paths(page: *const Page, out: *mut u32, cap: u32) -> u32 {
    guard(|| {
        let Some(l) = (*page).current_layout() else { return 0 };
        let n = l.omitted_paths.len() as u32;
        if out.is_null() || cap < n {
            return n;
        }
        for (i, p) in l.omitted_paths.iter().enumerate() {
            *out.add(i) = *p;
        }
        n
    })
}
/// The words of a reflowed row, in reading order. Returns the number of words on the row,
/// or the count needed when `cap` is too small; 0 when the page is not reflowed.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_row_words(page: *const Page, row: u32, out: *mut u32, cap: u32) -> u32 {
    guard(|| {
        let Some(flow) = (*page).current_layout().and_then(|l| l.reflow.as_ref()) else { return 0 };
        let Some(words) = flow.row_words.get(row as usize) else { return 0 };
        let n = words.len() as u32;
        if out.is_null() || cap < n {
            return n;
        }
        for (i, w) in words.iter().enumerate() {
            *out.add(i) = *w;
        }
        n
    })
}
/// The row a word landed on in a reflowed layout, or `QVP_NONE`.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_word_row(page: *const Page, word: u32) -> u32 {
    guard(|| {
        (*page)
            .current_layout()
            .and_then(|l| l.reflow.as_ref())
            .and_then(|f| f.word_row.get(word as usize).copied())
            .unwrap_or(u32::MAX)
    })
}
/// The largest reflow `zoom` at which every word of the page still fits a row. A host clamps
/// its pinch to this; the sajdah line is not counted, it is drawn over a span of words.
#[no_mangle]
pub unsafe extern "C" fn qvp_reflow_max_zoom(page: *const Page, spec: *const QvpLayoutSpec) -> f32 {
    guard(|| {
        let s = layout_spec(spec);
        (*page).reflow_max_zoom(&s.reflow.unwrap_or_default())
    })
}
/// The zoom each step of a reader's zoom control lands on for this page: the engine searches a
/// band around each nominal zoom for the one whose rows come out best. Writes up to `n_out`
/// zooms to `out`, rising, and returns how many it wrote. `nominals` null takes the engine's
/// own, `band` of 0 its own band. The printed page, zoom 1, is not among them.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_levels(
    page: *mut Page,
    spec: *const QvpLayoutSpec,
    nominals: *const f32,
    n_nominals: u32,
    band: f32,
    out: *mut f32,
    n_out: u32,
) -> u32 {
    guard(|| {
        if page.is_null() || out.is_null() {
            return 0;
        }
        let s = layout_spec(spec);
        let noms: &[f32] = if nominals.is_null() || n_nominals == 0 {
            &[]
        } else {
            std::slice::from_raw_parts(nominals, n_nominals as usize)
        };
        let levels = (*page).zoom_levels(&s, noms, band);
        let n = levels.len().min(n_out as usize);
        std::ptr::copy_nonoverlapping(levels.as_ptr(), out, n);
        n as u32
    })
}
/// The zoom each step of this page's zoom control lands on, lowest first, read from the table
/// the engine carries. Writes up to `n_out` and returns how many it wrote. Zoom 1, the printed
/// page, is the step every control starts from and is not among them.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_steps(page: *mut Page, spec: *const QvpLayoutSpec, out: *mut f32, n_out: u32) -> u32 {
    guard(|| {
        if page.is_null() || out.is_null() {
            return 0;
        }
        let s = layout_spec(spec);
        let steps = (*page).zoom_steps(&s);
        let n = steps.len().min(n_out as usize);
        std::ptr::copy_nonoverlapping(steps.as_ptr(), out, n);
        n as u32
    })
}
/// Every zoom the search considers for one step of a zoom control, with what its rows cost:
/// what `qvp_zoom_levels` picks the least of. Writes up to `n_out` pairs to `out_zoom` and
/// `out_cost` and returns how many it wrote. `floor` bounds the search from below, so a
/// generator can keep the steps apart; 0 takes the engine's own floor.
#[no_mangle]
pub unsafe extern "C" fn qvp_zoom_level_candidates(
    page: *mut Page,
    spec: *const QvpLayoutSpec,
    nominal: f32,
    band: f32,
    floor: f32,
    out_zoom: *mut f32,
    out_cost: *mut f32,
    n_out: u32,
) -> u32 {
    guard(|| {
        if page.is_null() || out_zoom.is_null() || out_cost.is_null() {
            return 0;
        }
        let s = layout_spec(spec);
        let cand = (*page).zoom_level_candidates(&s, nominal, band, floor);
        let n = cand.len().min(n_out as usize);
        for (i, &(z, c)) in cand.iter().take(n).enumerate() {
            *out_zoom.add(i) = z;
            *out_cost.add(i) = c;
        }
        n as u32
    })
}
/// The `line_spacing` multiplier that makes the page fill the padded viewport of `spec` when
/// fitted to width; max <= 0 means unlimited. The padding is subtracted here, not by the host.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_line_spacing_to_fill(
    page: *const Page,
    spec: *const QvpLayoutSpec,
    max: f32,
) -> f32 {
    guard(|| (*page).line_spacing_to_fill(&layout_spec(spec), if max <= 0.0 { f32::INFINITY } else { max }))
}
/// The share of the padded viewport of `spec` left empty when the page is fitted to width.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_wasted_fraction(page: *const Page, spec: *const QvpLayoutSpec) -> f32 {
    guard(|| (*page).wasted_fraction(&layout_spec(spec)))
}
/// out: x0,y0,x1,y1 in viewport px through the current layout
#[no_mangle]
pub unsafe extern "C" fn qvp_word_bounds_view(page: *const Page, word_index: u32, out: *mut f32) -> i32 {
    guard(|| {
        if word_index as usize >= (*page).data().words.len() {
            return 0;
        }
        let (a, b, c, d) = (*page).word_bounds_view(word_index);
        *out = a;
        *out.add(1) = b;
        *out.add(2) = c;
        *out.add(3) = d;
        1
    })
}

// ───────────── styles ─────────────

#[no_mangle]
pub unsafe extern "C" fn qvp_style_add(
    page: *mut Page,
    layer: i32,
    sel: *const QvpSelector,
    rgba: u32,
    transition_ms: u32,
) -> u32 {
    guard(|| match selector(sel) {
        Some(s) => (*page).style_in(layer, s, Paint::fade(rgba, transition_ms)),
        None => 0,
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_style_add_target(
    page: *mut Page,
    layer: i32,
    t: *const QvpTarget,
    rgba: u32,
    transition_ms: u32,
) -> u32 {
    guard(|| (*page).style_target(layer, &target(t), Paint::fade(rgba, transition_ms)))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_style_remove(page: *mut Page, handle: u32) -> u32 {
    guard(|| guard(|| (*page).remove_style(handle) as u32))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_style_recolor(page: *mut Page, handle: u32, rgba: u32, transition_ms: u32) -> u32 {
    guard(|| guard(|| (*page).recolor_style(handle, Paint::fade(rgba, transition_ms)) as u32))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_style_clear(page: *mut Page) {
    guard(|| {
        (*page).styles.clear();
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_style_clear_layer(page: *mut Page, layer: i32) {
    guard(|| {
        (*page).styles.clear_layer(layer);
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_style_default_color(page: *mut Page, rgba: u32) {
    guard(|| {
        (*page).set_default_ink(rgba);
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_style_hide(page: *mut Page, sel: *const QvpSelector) -> u32 {
    guard(|| {
        guard(|| match selector(sel) {
            Some(s) => (*page).hide(s),
            None => 0,
        })
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_theme(page: *mut Page, t: *const QvpTheme) -> u32 {
    guard(|| {
        let t = &*t;
        let c = |v: u32| if v & 0xff == 0 { None } else { Some(v) };
        let marks = if t.marks.is_null() {
            vec![]
        } else {
            std::slice::from_raw_parts(t.marks, (t.n_marks * 2) as usize)
                .chunks(2)
                .map(|p| (Mark::from_u8(p[0] as u8), p[1]))
                .collect()
        };
        (*page).theme(&Theme {
            ink: c(t.ink),
            diacritics: c(t.diacritics),
            dots: c(t.dots),
            waqf: c(t.waqf),
            sifr: c(t.sifr),
            ayah_mark: c(t.ayah_mark),
            numeral: c(t.numeral),
            headers: c(t.headers),
            marks,
            transition_ms: t.transition_ms,
        })
    })
}
/// Handles currently holding rules.
#[no_mangle]
pub unsafe extern "C" fn qvp_style_handles(page: *const Page, out: *mut u32, cap: u32) -> u32 {
    guard(|| guard(|| fill(out, cap, &(*page).styles.handles())))
}

// ───────────── clock & display list ─────────────

/// Advance the clock; returns 1 while something is still animating (keep rendering).
#[no_mangle]
pub unsafe extern "C" fn qvp_tick(page: *mut Page, now_ms: f64) -> u8 {
    guard(|| guard(|| (*page).tick(now_ms) as u8))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_colors(page: *mut Page) -> *const u32 {
    guard(|| guard(|| (*page).colors().as_ptr()))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_styled_paths(page: *mut Page, out: *mut u32, cap: u32) -> u32 {
    guard(|| {
        let v = (*page).styled_paths();
        if !out.is_null() {
            for (i, (index, c)) in v.iter().enumerate().take(cap as usize) {
                *out.add(i * 2) = *index;
                *out.add(i * 2 + 1) = *c;
            }
        }
        v.len() as u32
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_color_of(page: *mut Page, pi: u32) -> u32 {
    guard(|| guard(|| (*page).color_of(pi)))
}

// ───────────── highlights ─────────────

#[no_mangle]
pub unsafe extern "C" fn qvp_highlight_add(
    page: *mut Page,
    t: *const QvpTarget,
    style: *const QvpHighlightStyle,
) -> u32 {
    guard(|| {
        let st = if style.is_null() { HighlightStyle::default() } else { hstyle(style) };
        (*page).highlight(&target(t), st)
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_highlight_move(page: *mut Page, handle: u32, t: *const QvpTarget) -> u8 {
    guard(|| guard(|| (*page).move_highlight(handle, &target(t)) as u8))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_highlight_restyle(page: *mut Page, handle: u32, style: *const QvpHighlightStyle) -> u8 {
    guard(|| guard(|| (*page).restyle_highlight(handle, hstyle(style)) as u8))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_highlight_remove(page: *mut Page, handle: u32) -> u8 {
    guard(|| guard(|| (*page).remove_highlight(handle) as u8))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_highlight_clear(page: *mut Page) {
    guard(|| {
        (*page).clear_highlights();
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_highlight_handles(page: *const Page, out: *mut u32, cap: u32) -> u32 {
    guard(|| guard(|| fill(out, cap, &(*page).highlight_handles())))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_highlight_words(page: *const Page, handle: u32, out: *mut u32, cap: u32) -> u32 {
    guard(|| guard(|| fill(out, cap, &(*page).highlight_words(handle))))
}
/// Band boxes of every highlight in viewport px (animated). Draw each highlight id as one path.
#[no_mangle]
pub unsafe extern "C" fn qvp_highlight_boxes_view(page: *const Page, out: *mut QvpBox, cap: u32) -> u32 {
    guard(|| {
        let v: Vec<QvpBox> = (*page).highlight_boxes_view().iter().map(vb).collect();
        fill(out, cap, &v)
    })
}
/// Raw band boxes (page units, no animation) for a word list.
/// Band boxes for a word list in viewport px through the current layout.
#[no_mangle]
pub unsafe extern "C" fn qvp_word_bands_view(
    page: *const Page,
    words: *const u32,
    n: u32,
    height: u8,
    pad_x: f32,
    pad_y: f32,
    out: *mut QvpBox,
    cap: u32,
) -> u32 {
    guard(|| {
        let ws = std::slice::from_raw_parts(words, n as usize);
        let v: Vec<QvpBox> = (*page)
            .word_bands_view(ws, if height == 1 { BandHeight::Ink } else { BandHeight::LineSpacing }, pad_x, pad_y)
            .iter()
            .map(|b| QvpBox { id: 0, line: b.line, x0: b.x0, y0: b.y0, x1: b.x1, y1: b.y1, color: 0, radius: 0.0 })
            .collect();
        fill(out, cap, &v)
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_word_bands(
    page: *const Page,
    words: *const u32,
    n: u32,
    height: u8,
    pad_x: f32,
    pad_y: f32,
    out: *mut QvpBox,
    cap: u32,
) -> u32 {
    guard(|| {
        let ws = std::slice::from_raw_parts(words, n as usize);
        let v: Vec<QvpBox> = (*page)
            .word_bands(ws, if height == 1 { BandHeight::Ink } else { BandHeight::LineSpacing }, pad_x, pad_y)
            .iter()
            .map(|b| QvpBox { id: 0, line: b.line, x0: b.x0, y0: b.y0, x1: b.x1, y1: b.y1, color: 0, radius: 0.0 })
            .collect();
        fill(out, cap, &v)
    })
}

// ───────────── selection ─────────────

/// Set the selection (whole-word range). anchor/focus = QVP_NONE clears.
#[no_mangle]
pub unsafe extern "C" fn qvp_select(page: *mut Page, anchor: u32, focus: u32) {
    guard(|| {
        let p = &mut *page;
        p.selection.anchor = (anchor != NONE).then_some(anchor);
        p.selection.focus = (focus != NONE).then_some(focus);
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_selection(page: *const Page, out: *mut u32, cap: u32) -> u32 {
    guard(|| guard(|| fill(out, cap, &(*page).selection.words())))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_selection_text(page: *const Page, form: u8, include_citation: u8, out: *mut QvpStr) {
    guard(|| {
        let p = &*page;
        let ws = p.selection.words();
        let mut s = p.text_of(&ws, Form::from_u8(form), " ", "\n");
        if include_citation != 0 && !ws.is_empty() {
            s = format!("{s} ({})", p.citation(&ws));
        }
        out_str(out, &s);
    })
}

// ───────────── memorisation ─────────────

#[no_mangle]
pub unsafe extern "C" fn qvp_mask(page: *mut Page, t: *const QvpTarget, mode: u8) {
    guard(|| {
        (*page).mask(
            &target(t),
            match mode {
                1 => MaskMode::Block,
                2 => MaskMode::Blur,
                _ => MaskMode::Hide,
            },
        );
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_mask_from(page: *mut Page, word_index: u32, mode: u8) {
    guard(|| {
        (*page).mask_from(
            word_index,
            match mode {
                1 => MaskMode::Block,
                2 => MaskMode::Blur,
                _ => MaskMode::Hide,
            },
        );
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_mask_options(
    page: *mut Page,
    block_color: u32,
    pad_x: f32,
    pad_y: f32,
    radius: f32,
    reverse: u8,
) {
    guard(|| {
        (*page).set_mask_options(block_color, pad_x, pad_y, radius, reverse != 0);
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_mask_transition(page: *mut Page, ms: u32) {
    guard(|| {
        (*page).set_mask_transition(ms);
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_unmask_next(page: *mut Page, n: u32) -> u32 {
    guard(|| guard(|| (*page).unmask_next(n as usize) as u32))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_mask_back(page: *mut Page, n: u32) -> u32 {
    guard(|| guard(|| (*page).mask_back(n as usize) as u32))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_unmask_word(page: *mut Page, word_index: u32) -> u8 {
    guard(|| guard(|| (*page).unmask_word(word_index) as u8))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_mask_word(page: *mut Page, word_index: u32) -> u8 {
    guard(|| guard(|| (*page).mask_word(word_index) as u8))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_unmask_all(page: *mut Page) {
    guard(|| {
        (*page).unmask_all();
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_mask_all(page: *mut Page) {
    guard(|| {
        (*page).mask_all();
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_unmask(page: *mut Page) {
    guard(|| {
        (*page).unmask();
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_mask_hidden(page: *const Page, out: *mut u32, cap: u32) -> u32 {
    guard(|| guard(|| fill(out, cap, &(*page).mask_hidden())))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_mask_words(page: *const Page, out: *mut u32, cap: u32) -> u32 {
    guard(|| guard(|| fill(out, cap, (*page).mask_words())))
}
/// Boxes to draw over hidden words in Block/Blur mode (viewport px).
#[no_mangle]
pub unsafe extern "C" fn qvp_mask_boxes_view(page: *const Page, out: *mut QvpBox, cap: u32) -> u32 {
    guard(|| {
        let v: Vec<QvpBox> = (*page).mask_boxes_view().iter().map(vb).collect();
        fill(out, cap, &v)
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_reveal_start(
    page: *mut Page,
    lit: u32,
    by_ayah: u8,
    grey: u32,
    ink: u32,
    ayah_marks: u8,
    transition_ms: u32,
) -> u32 {
    guard(|| (*page).reveal_start(lit, by_ayah != 0, grey, ink, ayah_marks != 0, transition_ms))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_reveal_goto(page: *mut Page, at: i64) -> u8 {
    guard(|| guard(|| (*page).reveal_goto(at) as u8))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_reveal_position(page: *const Page) -> i64 {
    guard(|| guard(|| (*page).reveal_position().unwrap_or(-2)))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_reveal_step_count(page: *const Page) -> u32 {
    guard(|| guard(|| (*page).reveal_step_count()))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_reveal_step_of(page: *const Page, word_index: u32) -> i64 {
    guard(|| guard(|| (*page).reveal_step_of(word_index).map(|s| s as i64).unwrap_or(-1)))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_reveal_stop(page: *mut Page) {
    guard(|| {
        (*page).reveal_stop();
    })
}

// ───────────── crop ─────────────

#[no_mangle]
pub unsafe extern "C" fn qvp_crop_bounds(
    page: *const Page,
    t: *const QvpTarget,
    pad: f32,
    keep_ayah_marks: u8,
    out: *mut QvpCropBounds,
) -> i32 {
    guard(|| match (*page).crop_bounds(&target(t), pad, keep_ayah_marks != 0) {
        Some(c) => {
            *out = QvpCropBounds {
                x0: c.x0,
                y0: c.y0,
                x1: c.x1,
                y1: c.y1,
                n_words: c.n_words,
                ayah_mark_decoration: c.ayah_mark_decoration,
            };
            1
        }
        None => 0,
    })
}
/// Standalone SVG with the current colours. background alpha 0 = transparent.
#[no_mangle]
pub unsafe extern "C" fn qvp_crop_svg(
    page: *mut Page,
    t: *const QvpTarget,
    pad: f32,
    keep_ayah_marks: u8,
    background: u32,
    out: *mut QvpStr,
) -> i32 {
    guard(|| {
        match (*page).crop_svg(
            &target(t),
            pad,
            keep_ayah_marks != 0,
            if background & 0xff == 0 { None } else { Some(background) },
        ) {
            Some(s) => {
                out_str(out, &s);
                1
            }
            None => 0,
        }
    })
}

// ───────────── atlas ─────────────

#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_load(bytes: *const u8, len: usize) -> *mut Atlas {
    guard(|| {
        guard(|| match Atlas::decode(std::slice::from_raw_parts(bytes, len)) {
            Ok(a) => Box::into_raw(Box::new(a)),
            Err(_) => std::ptr::null_mut(),
        })
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_free(a: *mut Atlas) {
    guard(|| {
        if !a.is_null() {
            drop(Box::from_raw(a));
        }
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_page_of(a: *const Atlas, surah: u16, ayah: u16) -> i32 {
    guard(|| guard(|| (*a).page_of(surah, ayah).map(|p| p as i32).unwrap_or(-1)))
}
/// out: first_surah, first_ayah, last_surah, last_ayah
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_page_range(a: *const Atlas, page: u16, out: *mut u16) -> i32 {
    guard(|| {
        guard(|| match (*a).page_range(page) {
            Some((f, l)) => {
                *out = f.0;
                *out.add(1) = f.1;
                *out.add(2) = l.0;
                *out.add(3) = l.1;
                1
            }
            None => 0,
        })
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_page_count(a: *const Atlas) -> u32 {
    guard(|| guard(|| (*a).pages.len() as u32))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_surah_count(a: *const Atlas) -> u32 {
    guard(|| guard(|| (*a).surahs.len() as u32))
}
unsafe fn atlas_surah_out(s: &qvp_core::qvp_format::atlas::AtlasSurah, out: *mut QvpAtlasSurah) {
    let joined = format!("{}\u{0}{}\u{0}{}", s.arabic, s.latin, s.english);
    STR_BUF.with(|b| {
        let mut b = b.borrow_mut();
        b.clear();
        b.extend_from_slice(joined.as_bytes());
        let base = b.as_ptr();
        let (ar, l, e) = (s.arabic.len(), s.latin.len(), s.english.len());
        *out = QvpAtlasSurah {
            number: s.n,
            first_page: s.first_page,
            ayah_count: s.ayah_count,
            place: s.place,
            _pad: 0,
            arabic: QvpStr { ptr: base, len: ar as u32 },
            latin: QvpStr { ptr: base.add(ar + 1), len: l as u32 },
            english: QvpStr { ptr: base.add(ar + l + 2), len: e as u32 },
        };
    });
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_surah(a: *const Atlas, n: u16, out: *mut QvpAtlasSurah) -> i32 {
    guard(|| {
        guard(|| match (*a).surah(n) {
            Some(s) => {
                atlas_surah_out(s, out);
                1
            }
            None => 0,
        })
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_surah_at(a: *const Atlas, i: u32, out: *mut QvpAtlasSurah) -> i32 {
    guard(|| {
        let atlas = &*a;
        match atlas.surahs.get(i as usize) {
            Some(s) => {
                atlas_surah_out(s, out);
                1
            }
            None => 0,
        }
    })
}
fn rubu_al_hizb_out(r: &qvp_core::qvp_format::atlas::AtlasRubuAlHizb) -> QvpAtlasRubuAlHizb {
    QvpAtlasRubuAlHizb { rubu_al_hizb: r.rubu_al_hizb, surah: r.surah, ayah: r.ayah, page: r.page }
}
/// kind: 0 juz, 1 hizb, 2 nisf, 3 rubu_al_hizb
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_division(a: *const Atlas, kind: u8, n: u16, out: *mut QvpAtlasRubuAlHizb) -> i32 {
    guard(|| {
        let r = match kind {
            0 => (*a).juz(n),
            1 => (*a).hizb(n),
            2 => (*a).nisf(n),
            _ => (*a).rubu_al_hizb(n),
        };
        match r {
            Some(r) => {
                *out = rubu_al_hizb_out(r);
                1
            }
            None => 0,
        }
    })
}
/// kind: 0 juz, 1 hizb, 2 nisf, 3 rubu_al_hizb → division number containing the ayah, or -1
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_division_of(a: *const Atlas, kind: u8, surah: u16, ayah: u16) -> i32 {
    guard(|| {
        let r = match kind {
            0 => (*a).juz_at(surah, ayah),
            1 => (*a).hizb_at(surah, ayah),
            2 => (*a).nisf_at(surah, ayah),
            _ => (*a).rubu_al_hizb_at(surah, ayah).map(|r| r.rubu_al_hizb),
        };
        r.map(|x| x as i32).unwrap_or(-1)
    })
}
/// out: first_page, last_page
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_pages_of_juz(a: *const Atlas, n: u16, out: *mut u16) -> i32 {
    guard(|| {
        guard(|| match (*a).pages_of_juz(n) {
            Some((f, l)) => {
                *out = f;
                *out.add(1) = l;
                1
            }
            None => 0,
        })
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_search_surahs(
    a: *const Atlas,
    text: *const u8,
    len: u32,
    out: *mut u16,
    cap: u32,
) -> u32 {
    guard(|| {
        let v: Vec<u16> = (*a).search_surahs(in_str(text, len)).iter().map(|s| s.n).collect();
        fill(out, cap, &v)
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_json(a: *const Atlas, out: *mut QvpStr) {
    guard(|| {
        out_str(out, &(*a).to_json());
    })
}

// ───────────── names ─────────────

#[no_mangle]
pub unsafe extern "C" fn qvp_mark_name(mark: u8, out: *mut QvpStr) {
    guard(|| {
        let s = Mark::from_u8(mark).as_str();
        *out = QvpStr { ptr: s.as_ptr(), len: s.len() as u32 };
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_family_name(f: u8, out: *mut QvpStr) {
    guard(|| {
        let s = Family::from_u8(f).as_str();
        *out = QvpStr { ptr: s.as_ptr(), len: s.len() as u32 };
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_kind_name(k: u8, out: *mut QvpStr) {
    guard(|| {
        let s = PathKind::from_u8(k).as_str();
        *out = QvpStr { ptr: s.as_ptr(), len: s.len() as u32 };
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_category_name(c: u8, out: *mut QvpStr) {
    guard(|| {
        let s = Category::from_u8(c).as_str();
        *out = QvpStr { ptr: s.as_ptr(), len: s.len() as u32 };
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_mark_from_name(s: *const u8, len: u32) -> u8 {
    guard(|| guard(|| Mark::from_svg(in_str(s, len)) as u8))
}
/// The name tables the engine owns. `qvp_name`, `qvp_name_id` and `qvp_name_count` cover
/// every one of them, so a wrapper never carries a table of its own.
pub const QVP_NAMES_MARK: u8 = 0;
pub const QVP_NAMES_KIND: u8 = 1;
pub const QVP_NAMES_FAMILY: u8 = 2;
pub const QVP_NAMES_CATEGORY: u8 = 3;
pub const QVP_NAMES_DECORATION: u8 = 4;
pub const QVP_NAMES_DIVISION: u8 = 5;
pub const QVP_NAMES_PLACE: u8 = 6;

const DIVISION_NAMES: [&str; 4] = ["juz", "hizb", "nisf", "rubu_al_hizb"];
const PLACE_NAMES: [&str; 2] = ["makkah", "madinah"];

/// How many ids a table has (ids run from 0; 255 is "unknown" everywhere).
fn name_count(table: u8) -> u32 {
    match table {
        QVP_NAMES_MARK => 36,
        QVP_NAMES_KIND => 8,
        QVP_NAMES_FAMILY => 8,
        QVP_NAMES_CATEGORY => 9,
        QVP_NAMES_DECORATION => 7,
        QVP_NAMES_DIVISION => 4,
        QVP_NAMES_PLACE => 2,
        _ => 0,
    }
}

fn name_of(table: u8, id: u8) -> &'static str {
    if id as u32 >= name_count(table) {
        return "";
    }
    match table {
        QVP_NAMES_MARK => Mark::from_u8(id).as_str(),
        QVP_NAMES_KIND => PathKind::from_u8(id).as_str(),
        QVP_NAMES_FAMILY => Family::from_u8(id).as_str(),
        QVP_NAMES_CATEGORY => Category::from_u8(id).as_str(),
        QVP_NAMES_DECORATION => DecoKind::from_u8(id).as_str(),
        QVP_NAMES_DIVISION => DIVISION_NAMES[id as usize],
        QVP_NAMES_PLACE => PLACE_NAMES[id as usize],
        _ => "",
    }
}

/// Number of ids in a name table (QVP_NAMES_*).
#[no_mangle]
pub extern "C" fn qvp_name_count(table: u8) -> u32 {
    guard(|| name_count(table))
}
/// The name of `id` in a table; empty for an id outside the table.
#[no_mangle]
pub unsafe extern "C" fn qvp_name(table: u8, id: u8, out: *mut QvpStr) {
    guard(|| {
        let s = name_of(table, id);
        *out = QvpStr { ptr: s.as_ptr(), len: s.len() as u32 };
    })
}
/// The id of a name in a table, or 255 when the table has no such name.
#[no_mangle]
pub unsafe extern "C" fn qvp_name_id(table: u8, s: *const u8, len: u32) -> u8 {
    guard(|| {
        let name = in_str(s, len);
        (0..name_count(table)).map(|i| i as u8).find(|&i| name_of(table, i) == name).unwrap_or(255)
    })
}
#[no_mangle]
pub extern "C" fn qvp_mark_category(mark: u8) -> u8 {
    guard(|| guard(|| Mark::from_u8(mark).category() as u8))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_version(out: *mut QvpStr) {
    guard(|| out_str(out, env!("CARGO_PKG_VERSION")))
}
/// The page format version this engine reads (`docs/FORMAT.md`).
#[no_mangle]
pub extern "C" fn qvp_format_version() -> u32 {
    guard(|| guard(|| qvp_core::qvp_format::VERSION as u32))
}
/// Keeps the symbol referenced on platforms that need a C string somewhere.
#[no_mangle]
pub extern "C" fn qvp_engine_name() -> *const c_char {
    guard(|| guard(|| c"qvp".as_ptr()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_panic_inside_an_entry_point_becomes_the_error_value() {
        let signed: i32 = guard(|| panic!("engine bug"));
        let unsigned: u32 = guard(|| panic!("engine bug"));
        let pointer: *mut Page = guard(|| panic!("engine bug"));
        assert_eq!((signed, unsigned), (-1, 0));
        assert!(pointer.is_null());
        assert_eq!(guard(|| 7u32), 7);
    }

    #[test]
    fn name_tables_round_trip() {
        fn name(table: u8, id: u8) -> String {
            let mut o = QvpStr { ptr: std::ptr::null(), len: 0 };
            unsafe { qvp_name(table, id, &mut o) };
            unsafe { String::from_utf8_lossy(std::slice::from_raw_parts(o.ptr, o.len as usize)).into_owned() }
        }
        for table in QVP_NAMES_MARK..=QVP_NAMES_PLACE {
            let n = qvp_name_count(table);
            assert!(n > 0);
            for id in 1..n as u8 {
                let s = name(table, id);
                assert!(!s.is_empty(), "table {table} id {id}");
                assert_eq!(unsafe { qvp_name_id(table, s.as_ptr(), s.len() as u32) }, id, "{s}");
            }
            assert_eq!(name(table, 200), "");
            assert_eq!(unsafe { qvp_name_id(table, b"nope".as_ptr(), 4) }, 255);
        }
        assert_eq!(name(QVP_NAMES_MARK, 7), "shaddah");
        assert_eq!(name(QVP_NAMES_DECORATION, 0), "ayah-mark");
        assert_eq!(name(QVP_NAMES_DIVISION, 3), "rubu_al_hizb");
        assert_eq!(name(QVP_NAMES_PLACE, 1), "madinah");
    }

    #[test]
    fn garbage_bytes_load_as_null() {
        let p = unsafe { qvp_page_load(b"not a page".as_ptr(), 10) };
        assert!(p.is_null());
    }
}
