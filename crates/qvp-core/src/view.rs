//! The reader's own pan and zoom, on top of a layout.
//!
//! A layout says where the page's ink goes in viewport pixels. On top of that a reader pinches
//! and drags, and every host was writing the same arithmetic for it: clamp the scale, zoom
//! about the point between two fingers, keep the page from drifting off its own edges. That
//! arithmetic lives here, so the platforms cannot drift apart
//! (`docs/standards/API-DESIGN.md`).
//!
//! A host still owns the gestures. It reads a pinch from the platform's recognizer and asks
//! for the view that pinch produces.
use crate::{defaults, Page};

/// Pan and zoom over a laid-out page: a point `p` in layout pixels draws at
/// `offset + scale·p`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub scale: f32,
    pub offset_x: f32,
    pub offset_y: f32,
}

impl Default for View {
    fn default() -> Self {
        View { scale: 1.0, offset_x: 0.0, offset_y: 0.0 }
    }
}

impl View {
    /// Zoom by `factor` about `(fx, fy)` in viewport pixels, with the scale held between
    /// `min` and `max`: the point under the fingers stays under them. `min`/`max` of 0 take
    /// the engine's own limits.
    pub fn zoom_about(self, fx: f32, fy: f32, factor: f32, min: f32, max: f32) -> View {
        let (min, max) =
            (if min > 0.0 { min } else { defaults::MIN_ZOOM }, if max > 0.0 { max } else { defaults::MAX_ZOOM });
        let scale = (self.scale * factor).clamp(min.min(max), max.max(min));
        // the ratio actually applied, which is not `factor` once the clamp bites
        let k = if self.scale > 0.0 { scale / self.scale } else { 1.0 };
        View { scale, offset_x: fx - (fx - self.offset_x) * k, offset_y: fy - (fy - self.offset_y) * k }
    }

    /// Move by a drag, in viewport pixels.
    pub fn pan(self, dx: f32, dy: f32) -> View {
        View { offset_x: self.offset_x + dx, offset_y: self.offset_y + dy, ..self }
    }

    /// Hold the content against the viewport: an axis the content does not fill is centred,
    /// and on an axis it overflows the content keeps the viewport covered, so a drag can
    /// never open a blank strip beside the page.
    pub fn clamp(self, content_w: f32, content_h: f32, viewport_w: f32, viewport_h: f32) -> View {
        let axis = |offset: f32, content: f32, viewport: f32| -> f32 {
            let drawn = content * self.scale;
            if drawn <= viewport {
                (viewport - drawn) / 2.0
            } else {
                offset.min(0.0).max(viewport - drawn)
            }
        };
        View {
            scale: self.scale,
            offset_x: axis(self.offset_x, content_w, viewport_w),
            offset_y: axis(self.offset_y, content_h, viewport_h),
        }
    }
}

/// A released drag that is a page swipe: +1 or -1, or 0 when it is not one. Mostly sideways,
/// and either far enough or fast enough.
pub fn swipe_direction(dx: f32, dy: f32, vx: f32, vy: f32) -> i32 {
    let _ = vy;
    if dx.abs() <= dy.abs() * defaults::SWIPE_AXIS_RATIO {
        return 0;
    }
    if dx.abs() <= defaults::SWIPE_DISTANCE && vx.abs() <= defaults::SWIPE_VELOCITY {
        return 0;
    }
    if dx > 0.0 {
        1
    } else {
        -1
    }
}

impl Page {
    /// The view that puts a point inside a word at a chosen place on the screen, then holds
    /// the content against the viewport.
    ///
    /// This is what a pinch needs when the layout changes under it. Reflow moves a word to
    /// another row, so a host cannot keep the page still by remembering pixels; it remembers
    /// the word under the fingers and the point inside it (`nx`, `ny` from 0 to 1), and asks
    /// for the view that brings that point back.
    ///
    /// A word that moved to another row cannot hold both axes on a page whose width is fixed:
    /// the word's new row decides its x. The vertical place is the one this keeps.
    pub fn view_anchor(&self, view: View, word: u32, inside: (f32, f32), to: (f32, f32), viewport: (f32, f32)) -> View {
        let ((nx, ny), (to_x, to_y), (viewport_w, viewport_h)) = (inside, to, viewport);
        if word as usize >= self.data().words.len() {
            return view;
        }
        let (x0, y0, x1, y1) = self.word_bounds_view(word);
        let (px, py) = (x0 + nx.clamp(0.0, 1.0) * (x1 - x0), y0 + ny.clamp(0.0, 1.0) * (y1 - y0));
        let (content_w, content_h) = match self.current_layout() {
            Some(l) => (l.content_w, l.content_h),
            None => (self.width(), self.height()),
        };
        View { scale: view.scale, offset_x: to_x - view.scale * px, offset_y: to_y - view.scale * py }
            .clamp(content_w, content_h, viewport_w, viewport_h)
    }

    /// Where a point of the viewport lands in the laid-out page, for a host turning a touch
    /// into a hit test: the inverse of `offset + scale·p`.
    pub fn view_to_layout(&self, view: View, vx: f32, vy: f32) -> (f32, f32) {
        if view.scale == 0.0 {
            return (vx, vy);
        }
        ((vx - view.offset_x) / view.scale, (vy - view.offset_y) / view.scale)
    }
}

// ───────────── geometry through the current layout ─────────────
//
// Everything above answers in layout pixels. The calls below answer the same questions the
// page-unit ones do, but where the ink actually is: a host drawing its own overlay never has
// to place ink itself, and a reflowed page cannot catch it out.

use crate::highlight::{BandHeight, ViewBox};
use crate::hit::HitArea;
use crate::meta::MarkerInfo;

impl Page {
    /// Band boxes for a word list in viewport pixels, through the current layout. The
    /// page-unit twin is [`Page::word_bands`].
    pub fn word_bands_view(&self, words: &[u32], height: BandHeight, pad_x: f32, pad_y: f32) -> Vec<ViewBox> {
        let (scale, ox, oy) = match self.current_layout() {
            Some(l) => (l.scale, l.offset_x, l.offset_y),
            None => (1.0, 0.0, 0.0),
        };
        let reflowed = self.current_layout().map(|l| l.is_reflowed()).unwrap_or(false);
        let boxes = if reflowed {
            self.reflow_bands(words, height, pad_x, pad_y)
        } else {
            self.word_bands(words, height, pad_x, pad_y)
        };
        boxes
            .into_iter()
            .map(|b| {
                // a printed band still carries its line's own shift; a reflow band is placed
                let dy = if reflowed {
                    0.0
                } else {
                    self.current_layout().and_then(|l| l.line_dy.get(b.line as usize).copied()).unwrap_or(0.0)
                };
                ViewBox {
                    highlight: 0,
                    line: b.line,
                    x0: ox + b.x0 * scale,
                    y0: oy + (b.y0 + dy) * scale,
                    x1: ox + b.x1 * scale,
                    y1: oy + (b.y1 + dy) * scale,
                    color: 0,
                    radius: 0.0,
                }
            })
            .collect()
    }

    /// The hit boxes of every word in viewport pixels, through the current layout: the same
    /// partition [`Page::hit_test_view`] resolves a tap with, so an overlay and a tap agree.
    ///
    /// On a reflowed page these are the rows' own partition, not the printed lines' shifted:
    /// a row's words have other neighbours, and the gaps between them are other gaps.
    pub fn hit_areas_view(&self, gap_bias: f32) -> Vec<HitArea> {
        let Some(l) = self.current_layout() else { return self.hit_areas(gap_bias) };
        let (scale, ox, oy) = (l.scale, l.offset_x, l.offset_y);
        let Some(flow) = l.reflow.as_ref() else {
            // printed: the page-unit areas, each moved by its line's own shift
            return self
                .hit_areas(gap_bias)
                .into_iter()
                .map(|a| {
                    let dy = l.line_dy.get(a.line as usize).copied().unwrap_or(0.0);
                    HitArea {
                        x0: ox + a.x0 * scale,
                        y0: oy + (a.y0 + dy) * scale,
                        x1: ox + a.x1 * scale,
                        y1: oy + (a.y1 + dy) * scale,
                        ink_x0: ox + a.ink_x0 * scale,
                        ink_y0: oy + (a.ink_y0 + dy) * scale,
                        ink_x1: ox + a.ink_x1 * scale,
                        ink_y1: oy + (a.ink_y1 + dy) * scale,
                        ..a
                    }
                })
                .collect();
        };
        let q = self.quant();
        let mut out = Vec::with_capacity(self.data().words.len());
        for (r, words) in flow.row_words.iter().enumerate() {
            let (top, bottom) = flow.row_band[r];
            // placed ink of the row, right to left in reading order
            let ink: Vec<(u32, f32, f32, f32, f32)> = words
                .iter()
                .map(|&wi| {
                    let w = &self.data().words[wi as usize];
                    let p = flow.word_place[wi as usize];
                    let (x0, y0) = p.apply(w.bbox.x0 as f32 / q, w.bbox.y0 as f32 / q);
                    let (x1, y1) = p.apply(w.bbox.x1 as f32 / q, w.bbox.y1 as f32 / q);
                    (wi, x0, y0, x1, y1)
                })
                .collect();
            for (k, &(wi, ix0, iy0, ix1, iy1)) in ink.iter().enumerate() {
                // the row's right edge for the first word, its left edge for the last, and
                // otherwise the gap to the neighbour, split by `gap_bias` towards the
                // preceding (right-hand) word
                let x1 = match k.checked_sub(1) {
                    None => flow.row_w.max(ix1),
                    Some(prev) => {
                        let gap = ink[prev].1 - ix1;
                        if gap > 0.0 {
                            ix1 + gap * (1.0 - gap_bias)
                        } else {
                            ix1
                        }
                    }
                };
                let x0 = match ink.get(k + 1) {
                    None => 0.0f32.min(ix0),
                    Some(next) => {
                        let gap = ix0 - next.3;
                        if gap > 0.0 {
                            ix0 - gap * gap_bias
                        } else {
                            ix0
                        }
                    }
                };
                out.push(HitArea {
                    word: wi,
                    line: r as u32,
                    x0: ox + x0 * scale,
                    y0: oy + top * scale,
                    x1: ox + x1 * scale,
                    y1: oy + bottom * scale,
                    ink_x0: ox + ix0 * scale,
                    ink_y0: oy + iy0 * scale,
                    ink_x1: ox + ix1 * scale,
                    ink_y1: oy + iy1 * scale,
                });
            }
        }
        out
    }

    /// The ayah medallions of the page in viewport pixels, through the current layout: where
    /// each one is drawn, for a tap target or a marker a host draws itself. The page-unit twin
    /// is [`Page::ayah_marks`].
    pub fn ayah_marks_view(&self) -> Vec<MarkerInfo> {
        let Some(l) = self.current_layout() else { return self.ayah_marks() };
        let n_words = self.data().words.len() as u32;
        self.ayah_marks()
            .into_iter()
            .map(|m| {
                let p = match l.reflow.as_ref() {
                    Some(_) => l.groups.get((n_words + m.decoration) as usize).copied(),
                    None => l.groups.get(m.line as usize).copied(),
                }
                .unwrap_or(crate::Placement::IDENTITY);
                let (cx, cy) = p.apply(m.cx, m.cy);
                MarkerInfo {
                    cx: l.offset_x + cx * l.scale,
                    cy: l.offset_y + cy * l.scale,
                    r: m.r * p.kx.max(p.ky) * l.scale,
                    ..m
                }
            })
            .collect()
    }
}
