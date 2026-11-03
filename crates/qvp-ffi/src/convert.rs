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

pub(crate) fn vb(b: &ViewBox) -> QvpBox {
    QvpBox { id: b.highlight, line: b.line, x0: b.x0, y0: b.y0, x1: b.x1, y1: b.y1, color: b.color, radius: b.radius }
}
