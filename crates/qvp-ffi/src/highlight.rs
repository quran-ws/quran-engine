//! Highlights, selection and memorisation: bands over words, the selected range, masks and reveal.

use crate::abi::{fill, guard, out_str, QvpStr};
use crate::convert::{hstyle, target, vb};
use crate::types::*;
use qvp_core::*;

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
