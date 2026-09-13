//! Line bands, hit boxes and gap-aware hit testing.
use crate::{Page, NONE};

/// Vertical band of a printed line (page units): pitch-derived, plus the ink extent.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LineBand {
    pub line: u32,
    pub line_number: u8,
    pub y0: f32,
    pub y1: f32,
    pub mid: f32,
    pub ink_y0: f32,
    pub ink_y1: f32,
}

/// HitExact box of a word: ink box grown sideways to meet its neighbours and vertically to
/// the full line band, so every point on a printed line belongs to exactly one word.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitArea {
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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitOptions {
    /// refuse a hit further than this from the chosen word's ink (page units)
    pub max_distance: f32,
    /// share of a gap awarded to the preceding (right-hand) word
    pub gap_bias: f32,
    /// try the exact outline first (reports `path`)
    pub prefer_exact: bool,
}

impl Default for HitOptions {
    fn default() -> Self {
        HitOptions { max_distance: f32::INFINITY, gap_bias: crate::defaults::GAP_BIAS, prefer_exact: true }
    }
}

/// Gap-aware hit result. `exact` is true when the point is inside the word's ink outline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Hit {
    pub word: u32,
    pub path: u32,
    pub decoration: u32,
    pub line: u32,
    pub distance: f32,
    pub is_exact: bool,
}

impl Page {
    /// Line-spacing-derived vertical bands of every printed line, in page units.
    pub fn line_bands(&self) -> Vec<LineBand> {
        let q = self.quant();
        let p = self.line_spacing();
        self.data()
            .lines
            .iter()
            .enumerate()
            .map(|(i, l)| {
                let mid = self.line_centre(i);
                LineBand {
                    line: i as u32,
                    line_number: l.line_number,
                    y0: mid - p / 2.0,
                    y1: mid + p / 2.0,
                    mid,
                    ink_y0: l.bbox.y0 as f32 / q,
                    ink_y1: l.bbox.y1 as f32 / q,
                }
            })
            .collect()
    }

    /// Gap-aware hit boxes of every word (page units, pre-layout).
    pub fn hit_areas(&self, gap_bias: f32) -> Vec<HitArea> {
        let q = self.quant();
        let d = self.data();
        let bands = self.line_bands();
        let mut out = Vec::with_capacity(d.words.len());
        for (li, l) in d.lines.iter().enumerate() {
            let ws = &self.line_words[li];
            let band = bands[li];
            let (lx0, lx1) = (l.bbox.x0 as f32 / q, l.bbox.x1 as f32 / q);
            for (k, &(_, wi)) in ws.iter().enumerate() {
                let w = &d.words[wi as usize];
                let (ix0, ix1) = (w.bbox.x0 as f32 / q, w.bbox.x1 as f32 / q);
                // left neighbour = smaller x (next word in reading order); right = preceding
                let x0 = if k == 0 {
                    lx0.min(ix0)
                } else {
                    let left = &d.words[ws[k - 1].1 as usize];
                    let gap = ix0 - left.bbox.x1 as f32 / q;
                    if gap > 0.0 {
                        ix0 - gap * gap_bias
                    } else {
                        ix0
                    }
                };
                let x1 = if k + 1 == ws.len() {
                    lx1.max(ix1)
                } else {
                    let right = &d.words[ws[k + 1].1 as usize];
                    let gap = right.bbox.x0 as f32 / q - ix1;
                    if gap > 0.0 {
                        ix1 + gap * (1.0 - gap_bias)
                    } else {
                        ix1
                    }
                };
                out.push(HitArea {
                    word: wi,
                    line: li as u32,
                    x0,
                    y0: band.y0,
                    x1,
                    y1: band.y1,
                    ink_x0: ix0,
                    ink_y0: w.bbox.y0 as f32 / q,
                    ink_x1: ix1,
                    ink_y1: w.bbox.y1 as f32 / q,
                });
            }
        }
        out.sort_by_key(|h| h.word);
        out
    }

    /// Nearest-with-direction hit test in page units (no dead zones on a printed line).
    pub fn hit_test(&self, x: f32, y: f32, opt: &HitOptions) -> Option<Hit> {
        if opt.prefer_exact {
            if let Some(h) = self.hit_test_exact(x, y) {
                if h.path != NONE || h.decoration != NONE {
                    let line = if h.word != NONE {
                        self.data().words[h.word as usize].line_index as u32
                    } else {
                        self.geometry().table[self.data().decorations[h.decoration as usize].first_path as usize].line
                    };
                    return Some(Hit {
                        word: h.word,
                        path: h.path,
                        decoration: h.decoration,
                        line,
                        distance: 0.0,
                        is_exact: h.path != NONE,
                    });
                }
            }
        }
        self.hit_test_gap(x, y, opt)
    }

    /// Resolve a point to a line by its pitch band, then to a word with the gap split by `gap_bias`.
    pub fn hit_test_gap(&self, x: f32, y: f32, opt: &HitOptions) -> Option<Hit> {
        let bands = self.line_bands();
        // nearest band vertically (bands tile the page without gaps at natural pitch)
        let mut best_line = None;
        let mut best_dy = f32::MAX;
        for b in &bands {
            if self.data().lines[b.line as usize].n_words == 0 {
                continue;
            }
            let dy = if y < b.y0 {
                b.y0 - y
            } else if y > b.y1 {
                y - b.y1
            } else {
                0.0
            };
            if dy < best_dy {
                best_dy = dy;
                best_line = Some(b.line as usize);
            }
        }
        let li = best_line?;
        let q = self.quant();
        let d = self.data();
        let ws = &self.line_words[li];
        if ws.is_empty() {
            return None;
        }
        // choose by gap-split thresholds
        let mut chosen = ws[ws.len() - 1].1; // rightmost (first in reading order)
        for k in 0..ws.len() {
            let w = &d.words[ws[k].1 as usize];
            let (ix0, ix1) = (w.bbox.x0 as f32 / q, w.bbox.x1 as f32 / q);
            if x >= ix0 && x <= ix1 {
                chosen = ws[k].1;
                break;
            }
            if k + 1 < ws.len() {
                let right = &d.words[ws[k + 1].1 as usize];
                let rx0 = right.bbox.x0 as f32 / q;
                if x > ix1 && x < rx0 {
                    let gap = rx0 - ix1;
                    let threshold = rx0 - gap * opt.gap_bias;
                    chosen = if x >= threshold { ws[k + 1].1 } else { ws[k].1 };
                    break;
                }
            }
            if k == 0 && x < ix0 {
                chosen = ws[0].1;
                break;
            }
        }
        let w = &d.words[chosen as usize];
        let (ix0, iy0, ix1, iy1) =
            (w.bbox.x0 as f32 / q, w.bbox.y0 as f32 / q, w.bbox.x1 as f32 / q, w.bbox.y1 as f32 / q);
        let dx = if x < ix0 {
            ix0 - x
        } else if x > ix1 {
            x - ix1
        } else {
            0.0
        };
        let dy = if y < iy0 {
            iy0 - y
        } else if y > iy1 {
            y - iy1
        } else {
            0.0
        };
        let distance = (dx * dx + dy * dy).sqrt();
        if distance > opt.max_distance {
            return None;
        }
        Some(Hit { word: chosen, path: NONE, decoration: NONE, line: li as u32, distance, is_exact: false })
    }

    /// Gap-aware hit test in viewport px through the current layout.
    pub fn hit_test_view(&self, vx: f32, vy: f32, opt: &HitOptions) -> Option<Hit> {
        let Some(l) = self.current_layout() else { return self.hit_test(vx, vy, opt) };
        let x = (vx - l.offset_x) / l.scale;
        let y = (vy - l.offset_y) / l.scale;
        // exact first through the layout
        if opt.prefer_exact {
            if let Some(h) = self.hit_test_exact_view(vx, vy) {
                if h.path != NONE || h.decoration != NONE {
                    let line = if h.word != NONE {
                        self.data().words[h.word as usize].line_index as u32
                    } else {
                        self.geometry().table[self.data().decorations[h.decoration as usize].first_path as usize].line
                    };
                    return Some(Hit {
                        word: h.word,
                        path: h.path,
                        decoration: h.decoration,
                        line,
                        distance: 0.0,
                        is_exact: h.path != NONE,
                    });
                }
            }
        }
        // find the line whose laid-out slot contains y, then hit within that line's page space
        let mut li = None;
        for (i, (top, bottom)) in l.line_slots.iter().enumerate() {
            let (t, b) = (top / l.scale, bottom / l.scale);
            if y >= t && y <= b && self.data().lines[i].n_words > 0 {
                li = Some(i);
                break;
            }
        }
        let li = li?;
        let py = y - l.line_dy[li];
        let mut o = *opt;
        o.prefer_exact = false;
        let mut h = self.hit_test_gap(x, py, &o)?;
        if h.line as usize != li {
            // the band search picked a neighbour: force this line
            let ws = &self.line_words[li];
            if ws.is_empty() {
                return None;
            }
            h.line = li as u32;
        }
        Some(h)
    }
}
