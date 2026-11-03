//! Styles, the clock and the display list: layered colour rules and the colours they produce.

use crate::abi::{fill, guard};
use crate::convert::{selector, target};
use crate::types::*;
use qvp_core::qvp_format::Mark;
use qvp_core::*;

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
    guard(|| (*page).remove_style(handle) as u32)
}
#[no_mangle]
pub unsafe extern "C" fn qvp_style_recolor(page: *mut Page, handle: u32, rgba: u32, transition_ms: u32) -> u32 {
    guard(|| (*page).recolor_style(handle, Paint::fade(rgba, transition_ms)) as u32)
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
    guard(|| match selector(sel) {
        Some(s) => (*page).hide(s),
        None => 0,
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
    guard(|| fill(out, cap, &(*page).styles.handles()))
}

// ───────────── clock & display list ─────────────

/// Advance the clock; returns 1 while something is still animating (keep rendering).
#[no_mangle]
pub unsafe extern "C" fn qvp_tick(page: *mut Page, now_ms: f64) -> u8 {
    guard(|| (*page).tick(now_ms) as u8)
}
#[no_mangle]
pub unsafe extern "C" fn qvp_colors(page: *mut Page) -> *const u32 {
    guard(|| (*page).colors().as_ptr())
}
#[no_mangle]
pub unsafe extern "C" fn qvp_styled_paths(page: *mut Page, out: *mut u32, cap: u32) -> u32 {
    guard(|| {
        let v: Vec<[u32; 2]> = (*page).styled_paths().iter().map(|&(index, c)| [index, c]).collect();
        fill(out.cast(), cap, &v)
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_color_of(page: *mut Page, pi: u32) -> u32 {
    guard(|| (*page).color_of(pi))
}
