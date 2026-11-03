//! Dress: another mushaf's ornaments placed on this page.

use crate::abi::{guard, in_str, QvpStr};
use crate::view::QvpView;
use qvp_core::*;

/// The handle a host holds: an ornament set a dressed page keeps a share of, so a new
/// layout re-places the border without the host handing the set back.
pub type Ornaments = std::sync::Arc<OrnamentSet>;

/// One ornament of a set. The strings live as long as the set.
#[repr(C)]
pub struct QvpOrnament {
    pub kind: u32,
    pub lineage: u32,
    pub n_parts: u32,
    pub has_slices: u32,
    pub redistributable: u32,
    pub _pad: u32,
    pub style: QvpStr,
    pub riwayah: QvpStr,
    pub license_id: QvpStr,
    pub license_status: QvpStr,
    pub attribution: QvpStr,
}

/// One printed colour of a design. Alpha 0 is the window the design leaves open.
#[repr(C)]
pub struct QvpOrnamentPart {
    pub color: u32,
    pub stroke: u32,
    pub name: QvpStr,
}

pub const QVP_DRAW_EVENODD: u32 = 1 << 8;
pub const QVP_DRAW_STROKE: u32 = 2 << 8;
/// `QvpDressSpec.line_art` bits: which kinds are drawn as line art.
pub const QVP_LINE_ART_AYAH_MARK: u32 = 1;
pub const QVP_LINE_ART_SURAH_HEADER: u32 = 2;
pub const QVP_LINE_ART_PAGE_FRAME: u32 = 4;
pub const QVP_LINE_ART_ALL: u32 = 7;

#[repr(C)]
pub struct QvpDressSpec {
    pub ayah_mark: u32,
    pub surah_header: u32,
    pub page_frame: u32,
    pub gap: f32,
    pub band_gap: f32,
    pub ayah_mark_scale: f32,
    pub page_frame_scale: f32,
    pub line_art: u32,
    /// (ornament, part, rgba) triples; NULL for none.
    pub colors: *const u32,
    pub n_colors: u32,
}

#[repr(C)]
pub struct QvpDressInfo {
    pub ayah_mark: u32,
    pub surah_header: u32,
    pub page_frame: u32,
    pub n_ayah_marks: u32,
    pub n_surah_headers: u32,
    pub n_frame_repeats: u32,
    pub frame_stretched: u32,
    pub n_draws: u32,
    pub revision: u32,
    pub view_box: [f32; 4],
    pub overflow: [f32; 4],
}

#[repr(C)]
pub struct QvpDressGeometry {
    pub ops: *const u8,
    pub ops_len: u32,
    pub pts: *const f32,
    pub pts_len: u32,
    /// `n_draws` records of 9 × u32: op_start, op_count, pt_start, pt_count, colour,
    /// flags, stroke_width (float bits), part, placement
    pub table: *const u32,
    pub n_draws: u32,
}

/// The set behind a handle. Taking the reference here keeps the raw deref in one place.
unsafe fn oset<'a>(s: *const Ornaments) -> &'a OrnamentSet {
    &*s
}

#[no_mangle]
pub unsafe extern "C" fn qvp_ornaments_load(bytes: *const u8, len: usize) -> *mut Ornaments {
    guard(|| {
        if bytes.is_null() {
            return std::ptr::null_mut();
        }
        match OrnamentSet::decode(std::slice::from_raw_parts(bytes, len)) {
            Ok(s) => Box::into_raw(Box::new(std::sync::Arc::new(s))),
            Err(_) => std::ptr::null_mut(),
        }
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_ornaments_free(s: *mut Ornaments) {
    guard(|| {
        if !s.is_null() {
            drop(Box::from_raw(s));
        }
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_ornament_count(s: *const Ornaments) -> u32 {
    guard(|| {
        if s.is_null() {
            return 0;
        }
        oset(s).ornaments.len() as u32
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_ornament_find(s: *const Ornaments, kind: u32, style: *const u8, len: u32) -> i32 {
    guard(|| {
        if s.is_null() {
            return -1;
        }
        let Some(kind) = u8::try_from(kind).ok().and_then(OrnamentKind::from_u8) else { return -1 };
        oset(s).find(kind, in_str(style, len)).map(|i| i as i32).unwrap_or(-1)
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_ornament_info(s: *const Ornaments, i: u32, out: *mut QvpOrnament) -> i32 {
    guard(|| {
        if s.is_null() || out.is_null() {
            return 0;
        }
        let Some(o) = oset(s).ornaments.get(i as usize) else { return 0 };
        let q = |t: &str| QvpStr { ptr: t.as_ptr(), len: t.len() as u32 };
        *out = QvpOrnament {
            kind: o.kind as u32,
            lineage: o.lineage as u32,
            n_parts: o.parts.len() as u32,
            has_slices: o.slices.is_some() as u32,
            redistributable: o.license.redistributable as u32,
            _pad: 0,
            style: q(&o.style),
            riwayah: q(&o.riwayah),
            license_id: q(&o.license.id),
            license_status: q(&o.license.status),
            attribution: q(&o.license.attribution),
        };
        1
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_ornament_part(s: *const Ornaments, i: u32, part: u32, out: *mut QvpOrnamentPart) -> i32 {
    guard(|| {
        if s.is_null() || out.is_null() {
            return 0;
        }
        let Some(p) = oset(s).ornaments.get(i as usize).and_then(|o| o.parts.get(part as usize)) else { return 0 };
        *out = QvpOrnamentPart {
            color: p.color,
            stroke: p.stroke as u32,
            name: QvpStr { ptr: p.name.as_ptr(), len: p.name.len() as u32 },
        };
        1
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_dress(page: *mut Page, set: *const Ornaments, spec: *const QvpDressSpec) -> i32 {
    guard(|| {
        if page.is_null() || set.is_null() || spec.is_null() {
            return 0;
        }
        let s = &*spec;
        // A triple whose part does not fit in a u16 names no part, so it is skipped.
        let colors = if s.colors.is_null() {
            vec![]
        } else {
            let Some(n) = (s.n_colors as usize).checked_mul(3) else { return 0 };
            std::slice::from_raw_parts(s.colors, n)
                .as_chunks::<3>()
                .0
                .iter()
                .filter_map(|&[o, part, rgba]| Some((o, u16::try_from(part).ok()?, rgba)))
                .collect()
        };
        let spec = DressSpec {
            ayah_mark: s.ayah_mark,
            surah_header: s.surah_header,
            page_frame: s.page_frame,
            gap: s.gap,
            band_gap: s.band_gap,
            ayah_mark_scale: s.ayah_mark_scale,
            page_frame_scale: s.page_frame_scale,
            line_art: s.line_art & LINE_ART_ALL,
            colors,
        };
        (*page).dress(&*set, &spec).is_some() as i32
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_undress(page: *mut Page) {
    guard(|| {
        if !page.is_null() {
            (*page).undress()
        }
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_dress_info(page: *const Page, out: *mut QvpDressInfo) -> i32 {
    guard(|| {
        if page.is_null() || out.is_null() {
            return 0;
        }
        let Some(d) = (*page).dress_of() else { return 0 };
        let (l, t, r, b) = (*page).dress_overflow();
        *out = QvpDressInfo {
            ayah_mark: d.ayah_mark,
            surah_header: d.surah_header,
            page_frame: d.page_frame,
            n_ayah_marks: d.n_ayah_marks,
            n_surah_headers: d.n_surah_headers,
            n_frame_repeats: d.n_frame_repeats,
            frame_stretched: d.frame_stretched as u32,
            n_draws: d.table.len() as u32,
            revision: d.revision,
            view_box: d.view_box,
            overflow: [l, t, r, b],
        };
        1
    })
}
/// Hold a view against the viewport for the dressed box (the content plus the dress
/// overflow), so a drag reaches the border and never goes past it. An undressed page gets
/// the plain `qvp_view_clamp` of its content.
#[no_mangle]
pub unsafe extern "C" fn qvp_dress_clamp(
    page: *const Page,
    view: *const QvpView,
    viewport_w: f32,
    viewport_h: f32,
    out: *mut QvpView,
) {
    guard(|| {
        if page.is_null() || view.is_null() || out.is_null() {
            return;
        }
        *out = (*page).dress_clamp((*view).into(), viewport_w, viewport_h).into()
    })
}
/// The view for a host that scrolls the page itself: the dressed box at `scale`, centred
/// across `viewport_w` (or at x = 0 when wider) with its top at y = 0. `out_size` gets the
/// dressed box's width and height at that scale, the size to give the canvas. An undressed
/// page gets its content centred at the top and the content's size.
#[no_mangle]
pub unsafe extern "C" fn qvp_dress_view(
    page: *const Page,
    scale: f32,
    viewport_w: f32,
    out: *mut QvpView,
    out_size: *mut f32,
) {
    guard(|| {
        if page.is_null() || out.is_null() {
            return;
        }
        let (v, w, h) = (*page).dress_view(scale, viewport_w);
        *out = v.into();
        if !out_size.is_null() {
            *out_size = w;
            *out_size.add(1) = h;
        }
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_dress_geometry(page: *const Page, out: *mut QvpDressGeometry) {
    guard(|| {
        if out.is_null() {
            return;
        }
        // A null page is not dressed: it gets the empty geometry.
        *out = match if page.is_null() { None } else { (*page).dress_of() } {
            Some(d) => QvpDressGeometry {
                ops: d.ops.as_ptr(),
                ops_len: d.ops.len() as u32,
                pts: d.pts.as_ptr(),
                pts_len: d.pts.len() as u32,
                table: d.table.as_ptr() as *const u32,
                n_draws: d.table.len() as u32,
            },
            None => QvpDressGeometry {
                ops: std::ptr::null(),
                ops_len: 0,
                pts: std::ptr::null(),
                pts_len: 0,
                table: std::ptr::null(),
                n_draws: 0,
            },
        }
    })
}
