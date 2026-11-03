//! Crop: the bounds of a target and a standalone SVG of it.

use crate::abi::{guard, out_str, QvpStr};
use crate::convert::target;
use crate::types::*;
use qvp_core::*;

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
