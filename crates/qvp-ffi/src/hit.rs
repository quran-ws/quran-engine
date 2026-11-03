//! Hit testing: the word under a point, line bands and hit areas.

use crate::abi::{fill, guard};
use crate::types::*;
use qvp_core::*;

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
