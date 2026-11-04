//! Vertical placement: the baseline every row sits on, and the band each row answers for.
use super::{Atom, RowSpec};
use crate::Page;
use std::collections::HashMap;

/// The heights of the rows, in page units, indexed by row: the baseline each row sits on and
/// the ink it holds above and below that baseline.
pub(super) struct RowHeights {
    pub(super) baseline: Vec<f32>,
    pub(super) ascent: Vec<f32>,
    pub(super) descent: Vec<f32>,
}

impl Page {
    /// The baseline of every row. `row_dx` is what [`Page::place_across`] placed, and
    /// `printed_y` the printed line of each row when the page keeps the printed line positions.
    ///
    /// Every word sits on its line's baseline, so a row places its words on one baseline of
    /// its own, the printed line spacing apart. Two rows are pushed further apart only where
    /// their ink would actually meet: a deep descender over a tall mark in the same column.
    /// The print allows ink to cross into a neighbouring line, and so does this.
    pub(super) fn place_down(
        &self,
        rows: &[Vec<&Atom>],
        headers: &HashMap<usize, u32>,
        row_dx: &[Vec<f32>],
        shape: &RowSpec,
        printed_y: Option<&[usize]>,
        omitted_header_paths: &[u32],
    ) -> RowHeights {
        let (row_w, pitch, top, banner_k) = (shape.row_w, shape.pitch, shape.top, shape.banner_k);
        let q = self.quant();
        let d = self.data();
        let baselines = &self.line_baseline;
        let ink_of = |a: &Atom, dx: f32| -> (f32, f32, f32, f32) {
            let w = &d.words[a.word as usize];
            let b = baselines[w.line_index as usize];
            (
                w.bbox.x0 as f32 / q + dx,
                w.bbox.x1 as f32 / q + dx,
                b - w.bbox.y0 as f32 / q, // ascent
                w.bbox.y1 as f32 / q - b, // descent
            )
        };
        let mut row_y: Vec<f32> = Vec::with_capacity(rows.len());
        let (mut ascents, mut descents) = (Vec::with_capacity(rows.len()), Vec::with_capacity(rows.len()));
        for (r, atoms) in rows.iter().enumerate() {
            let (mut asc, mut desc) = (0.0f32, 0.0f32);
            for (i, a) in atoms.iter().enumerate() {
                let (_, _, aa, dd) = ink_of(a, row_dx[r].get(i).copied().unwrap_or(0.0));
                asc = asc.max(aa);
                desc = desc.max(dd);
            }
            // A banner row is as tall as its own ink, which is the decorations drawn on it: a
            // banner line holds no words, so its line box is empty and says nothing about how
            // much room the surah name or the basmalah needs. The ink is measured at the size
            // the banner is drawn at, which is the size `place_header` fits to the row.
            if let Some(&li) = headers.get(&r) {
                let c = self.line_centre(li as usize);
                let k = self.header_scale(li as usize, row_w, banner_k, omitted_header_paths);
                let bbox = self.header_bbox(li as usize, omitted_header_paths);
                if !bbox.is_empty() {
                    asc = asc.max(k * (c - bbox.y0 as f32 / q));
                    desc = desc.max(k * (bbox.y1 as f32 / q - c));
                }
            }
            ascents.push(asc.max(0.0));
            descents.push(desc.max(0.0));
            // the printed page reproduced: keep the printed line positions
            if let Some(lines) = printed_y {
                let li = lines[r];
                let y = if headers.contains_key(&r) { self.line_centre(li) } else { baselines[li] };
                row_y.push(y);
                continue;
            }
            let y = match r.checked_sub(1) {
                None => top + ascents[r],
                Some(prev) => {
                    // the room the two rows' ink needs where it shares a column
                    let mut need: f32 = 0.0;
                    for (i, a) in rows[prev].iter().enumerate() {
                        let (ax0, ax1, _, adesc) = ink_of(a, row_dx[prev].get(i).copied().unwrap_or(0.0));
                        for (j, b) in atoms.iter().enumerate() {
                            let (bx0, bx1, basc, _) = ink_of(b, row_dx[r].get(j).copied().unwrap_or(0.0));
                            if ax0 < bx1 && bx0 < ax1 {
                                need = need.max(adesc + basc);
                            }
                        }
                    }
                    // A banner keeps the print's air only within a normal line pitch. Its
                    // row holds its own ink, not the blank slot the print set it inside.
                    if headers.contains_key(&prev) || headers.contains_key(&r) {
                        let air = headers
                            .get(&prev)
                            .map(|&li| {
                                self.header_scale(li as usize, row_w, banner_k, omitted_header_paths)
                                    * self.header_air(li as usize, omitted_header_paths).1
                            })
                            .or_else(|| {
                                headers.get(&r).map(|&li| {
                                    self.header_scale(li as usize, row_w, banner_k, omitted_header_paths)
                                        * self.header_air(li as usize, omitted_header_paths).0
                                })
                            })
                            .unwrap_or(0.0);
                        let beside = if headers.contains_key(&prev) { ascents[r] } else { descents[prev] };
                        let air = air.min((pitch - beside).max(0.0));
                        need = need.max(descents[prev] + ascents[r] + air);
                    }
                    row_y[prev] + pitch.max(need)
                }
            };
            row_y.push(y);
        }
        RowHeights { baseline: row_y, ascent: ascents, descent: descents }
    }

    /// The ink centre of every row and the vertical band it answers for, (top, bottom), in
    /// page units.
    ///
    /// Bands meet halfway between neighbouring rows, as the printed slots do — measured
    /// between the rows' ink, not their baselines. A baseline sits low in its row (the
    /// ascenders and marks are all above it), so a band built around one would hang below
    /// the words it belongs to.
    pub(super) fn row_bands(
        &self,
        headers: &HashMap<usize, u32>,
        heights: &RowHeights,
        pitch: f32,
        printed_y: Option<&[usize]>,
    ) -> (Vec<f32>, Vec<(f32, f32)>) {
        let RowHeights { baseline: row_y, ascent: ascents, descent: descents } = heights;
        let n_rows = row_y.len();
        // the row's ink centre; a page that came out as printed uses the printed line's own
        // centre, so its slots are the printed slots
        let centres: Vec<f32> = (0..n_rows)
            .map(|r| match printed_y {
                Some(lines) => self.line_centre(lines[r]),
                None => row_y[r] - (ascents[r] - descents[r]) / 2.0,
            })
            .collect();
        let half = pitch / 2.0;
        let mut bands = Vec::with_capacity(n_rows);
        for r in 0..n_rows {
            // beside a banner the printed gap is wide (the opening pages set the banner pitches
            // above the text) and it is not the text row's to claim, so the boundary stops half
            // a line spacing out. The printed slots do the same.
            let banner = |i: usize| headers.contains_key(&i);
            let top_b = match r.checked_sub(1) {
                Some(prev) => {
                    let mid = (centres[prev] + centres[r]) / 2.0;
                    if banner(r) || banner(prev) {
                        mid.max(centres[r] - half)
                    } else {
                        mid
                    }
                }
                None => centres[r] - half,
            };
            let bottom_b = match centres.get(r + 1) {
                Some(&next) => {
                    let mid = (centres[r] + next) / 2.0;
                    if banner(r) || banner(r + 1) {
                        mid.min(centres[r] + half)
                    } else {
                        mid
                    }
                }
                None => centres[r] + half,
            };
            bands.push((top_b, bottom_b));
        }
        (centres, bands)
    }
}
