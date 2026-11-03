//! Converters from the header's structs and codes to the core's types, and back.

use crate::types::*;
use qvp_core::qvp_format::{Category, DecoKind, Family, Mark, PathKind};
use qvp_core::*;

pub(crate) unsafe fn target(t: *const QvpTarget) -> Target {
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

pub(crate) unsafe fn selector(s: *const QvpSelector) -> Option<Selector> {
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

pub(crate) unsafe fn hstyle(s: *const QvpHighlightStyle) -> HighlightStyle {
    let s = &*s;
    HighlightStyle {
        mode: match s.mode {
            0 => HighlightMode::Ink,
            2 => HighlightMode::Both,
            _ => HighlightMode::Band,
        },
        height: band_height(s.height),
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

/// The header's band height code: 1 the ink, anything else the line spacing.
pub(crate) fn band_height(height: u8) -> BandHeight {
    if height == 1 {
        BandHeight::Ink
    } else {
        BandHeight::LineSpacing
    }
}

/// One box of a highlight or a mask in viewport px, with its handle and animated colour.
pub(crate) fn vb(b: &ViewBox) -> QvpBox {
    QvpBox { id: b.highlight, line: b.line, x0: b.x0, y0: b.y0, x1: b.x1, y1: b.y1, color: b.color, radius: b.radius }
}

/// A raw band box: no highlight id, no colour, no radius. Page units or viewport px
/// depending on the call.
pub(crate) fn raw_band(line: u32, x0: f32, y0: f32, x1: f32, y1: f32) -> QvpBox {
    QvpBox { id: 0, line, x0, y0, x1, y1, color: 0, radius: 0.0 }
}

/// One surah heading, page units or viewport px depending on the call.
pub(crate) fn header(h: &SurahHeader) -> QvpSurahHeader {
    QvpSurahHeader {
        decoration: h.decoration,
        surah: h.surah,
        _pad: 0,
        line: h.line,
        x0: h.x0,
        y0: h.y0,
        x1: h.x1,
        y1: h.y1,
        title_x0: h.title_x0,
        title_y0: h.title_y0,
        title_x1: h.title_x1,
        title_y1: h.title_y1,
    }
}

/// One ayah medallion, page units or viewport px depending on the call.
pub(crate) fn ayah_mark(m: &MarkerInfo) -> QvpAyahMark {
    QvpAyahMark {
        decoration: m.decoration,
        surah: m.surah,
        ayah: m.ayah,
        line: m.line,
        cx: m.cx,
        cy: m.cy,
        r: m.r,
        ornament_path: m.ornament_path,
        numeral_path: m.numeral_path,
    }
}

/// One hit box, page units or viewport px depending on the call.
pub(crate) fn hit_area(h: &HitArea) -> QvpHitArea {
    QvpHitArea {
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
    }
}

/// One placement as the `{dx, dy, kx, ky}` record the header describes.
pub(crate) fn placement(p: &Placement) -> [f32; 4] {
    [p.dx, p.dy, p.kx, p.ky]
}
