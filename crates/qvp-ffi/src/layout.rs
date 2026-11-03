//! Layout: lay the page out for a viewport and read what the layout draws, and where.

use crate::abi::{fill, guard, placement, LAYOUT_BUF};
use crate::types::*;
use qvp_core::*;

pub(crate) unsafe fn layout_spec(spec: *const QvpLayoutSpec) -> LayoutSpec {
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
        banner_zoom: s.banner_zoom,
        surah_frames: s.surah_frames != 0,
        // The reflow knobs reach the core on a printed page too. A zoom of 1 or less lays the page
        // out as printed, and the knobs are what the zoom control reflows it with when a pinch
        // leaves the print, so a host's fill holds from the first step.
        reflow: Some(qvp_core::ReflowSpec {
            zoom: s.reflow_zoom.max(0.0),
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
                3 => qvp_core::Breaks::Printed,
                _ => qvp_core::ReflowSpec::default().breaks,
            },
            gaps: if s.reflow_gaps == 0 { qvp_core::GapMode::Printed } else { qvp_core::GapMode::Uniform },
            relax: if s.reflow_relax < 0.0 { qvp_core::defaults::REFLOW_RELAX } else { s.reflow_relax.min(1.0) },
            word_gap: if s.reflow_word_gap > 0.0 { s.reflow_word_gap } else { 1.0 },
            // 0 is the default; a negative value reaches the core as it is, which reads any
            // value at or below 0 as no cap.
            max_stretch: if s.reflow_max_stretch == 0.0 {
                qvp_core::defaults::REFLOW_MAX_STRETCH
            } else {
                s.reflow_max_stretch
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
        let v: Vec<[f32; 4]> = l.groups.iter().map(placement).collect();
        fill(out.cast(), cap, &v)
    })
}
/// Paths to draw a second time, at another placement: `n × {first_path, n_paths, dx, dy, kx,
/// ky}` as floats, the first two whole numbers. A sajdah line whose words ended up on two rows
/// is drawn over each of them. Returns the number of repeats, or the count needed.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_repeats(page: *const Page, out: *mut f32, cap: u32) -> u32 {
    guard(|| {
        let Some(flow) = (*page).current_layout().and_then(|l| l.reflow.as_ref()) else { return 0 };
        let v: Vec<[f32; 6]> = flow
            .repeats
            .iter()
            .map(|r| {
                let p = &r.placement;
                [r.first_path as f32, r.n_paths as f32, p.dx, p.dy, p.kx, p.ky]
            })
            .collect();
        fill(out.cast(), cap, &v)
    })
}
/// The group of every path, for [`qvp_layout_groups`]. Empty without reflow, where a path's
/// group is its line (`qvp_path_line`). Returns the number of paths, or the count needed.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_path_groups(page: *const Page, out: *mut u32, cap: u32) -> u32 {
    guard(|| {
        let Some(l) = (*page).current_layout() else { return 0 };
        fill(out, cap, &l.path_group)
    })
}
/// Paths the current layout does not draw: sheet furniture, and a native surah frame once
/// the page has reflowed. Returns how many, or the count needed.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_omitted_paths(page: *const Page, out: *mut u32, cap: u32) -> u32 {
    guard(|| {
        let Some(l) = (*page).current_layout() else { return 0 };
        fill(out, cap, &l.omitted_paths)
    })
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
        let v: Vec<[u32; 2]> = (*page).layout_draw_list_in(band).iter().map(|d| [d.path, d.placement]).collect();
        fill(out.cast(), cap, &v)
    })
}

/// Every placement the current layout draws under: one per group, then one for each repeat.
/// Returns the number of placements, or the count needed.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_placements(page: *const Page, out: *mut f32, cap: u32) -> u32 {
    guard(|| {
        let Some(l) = (*page).current_layout() else { return 0 };
        let v: Vec<[f32; 4]> = l.placements().iter().map(placement).collect();
        fill(out.cast(), cap, &v)
    })
}
/// The words of a reflowed row, in reading order. Returns the number of words on the row,
/// or the count needed when `cap` is too small; 0 when the page is not reflowed.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_row_words(page: *const Page, row: u32, out: *mut u32, cap: u32) -> u32 {
    guard(|| {
        let Some(flow) = (*page).current_layout().and_then(|l| l.reflow.as_ref()) else { return 0 };
        let Some(words) = flow.row_words.get(row as usize) else { return 0 };
        fill(out, cap, words)
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
/// The page's height laid out for `spec` at the printed pitch, in viewport px: `content_h` before
/// fill-height adds any leading, padding included. What a host asks before it sizes the canvas.
#[no_mangle]
pub unsafe extern "C" fn qvp_layout_printed_height(page: *const Page, spec: *const QvpLayoutSpec) -> f32 {
    guard(|| (*page).printed_height(&layout_spec(spec)))
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
