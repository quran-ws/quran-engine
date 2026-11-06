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
    /// the engine's own limits. A `max` under `min` is `min`: the floor is the settled page,
    /// which a pinch never shrinks inside the screen, and a ceiling set under it asks for no
    /// zoom at all rather than for a smaller page.
    pub fn zoom_about(self, fx: f32, fy: f32, factor: f32, min: f32, max: f32) -> View {
        let (min, max) =
            (if min > 0.0 { min } else { defaults::MIN_ZOOM }, if max > 0.0 { max } else { defaults::MAX_ZOOM });
        let scale = (self.scale * factor).clamp(min, max.max(min));
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

/// The band of a laid-out page a host keeps drawn, in layout pixels: the space
/// [`Page::layout_draw_list_in`] takes its band in. A point `y` of the band is on the screen
/// at `offset_y + scale·y`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct InkBand {
    pub top: f32,
    pub height: f32,
}

impl View {
    /// The band of a page `content_h` layout pixels tall that a host draws for this view and a
    /// viewport `viewport_w` × `viewport_h` pixels: the whole page when it fits, otherwise a
    /// band two screens tall around the reader.
    ///
    /// A host draws the band once into a picture or a bitmap and moves that picture as the
    /// reader pans. The band's top moves in steps of half its slack rather than with the
    /// finger, so a pan inside one step changes neither the band nor the picture, and a pan
    /// across a step moves the band by exactly one step. A band with less than 2 px of slack
    /// has no step of a whole pixel, so it follows the reader, and its slack is shared above
    /// and below the screen.
    ///
    /// Two limits bound the band, both in viewport pixels, because a texture is measured on
    /// the screen and not in the layout. `max_height` is the tallest band the host can draw:
    /// 0 or less takes [`defaults::MAX_INK_HEIGHT`]. It is a hard limit, and a screen taller
    /// than it gets a band of exactly `max_height`. `max_pixels` is how many pixels the host
    /// keeps for the band, its width times its height: 0 or less means no such limit. It is a
    /// soft limit: it can shrink the band from two screens down to one screen, never below,
    /// because a band shorter than the screen leaves part of the screen with no ink. The page
    /// is drawn whole when it fits under both limits, read the same way.
    ///
    /// One case passes both limits: a view with no scale, a viewport with no height, or a
    /// non-finite scale, height or vertical offset has no screen to put a band around, so
    /// the band is the whole page, however tall it is.
    pub fn ink_band(
        self,
        content_h: f32,
        viewport_w: f32,
        viewport_h: f32,
        max_height: f32,
        max_pixels: f32,
    ) -> InkBand {
        let content_h = if content_h.is_finite() { content_h.max(0.0) } else { 0.0 };
        let whole = InkBand { top: 0.0, height: content_h };
        let scale = self.scale;
        let screen_known = scale > 0.0 && scale.is_finite() && viewport_h > 0.0 && viewport_h.is_finite();
        if !(screen_known && self.offset_y.is_finite()) {
            return whole;
        }
        // the hard limit, at least one pixel
        let hard = if max_height > 0.0 { max_height } else { defaults::MAX_INK_HEIGHT }.max(1.0);
        // In viewport pixels: the screen, held under the hard limit.
        let screen = viewport_h.clamp(1.0, hard);
        // the pixel budget, which never takes the band below one screen
        let budget = if max_pixels > 0.0 && viewport_w > 0.0 { max_pixels / viewport_w } else { f32::INFINITY };
        let ceiling = hard.min(budget.max(screen));
        if content_h * scale <= ceiling {
            return whole;
        }
        // A band two screens tall, or as tall as the ceiling allows, and a step of half the
        // slack. The screen at the top of the band is `-offset_y` down the page. A step under
        // one pixel would redraw on every frame of a pan anyway, so such a band follows the
        // reader instead, with its slack shared either side: a step larger than half the slack
        // would leave the edge of the screen with no ink.
        let tall = (screen * 2.0).min(ceiling);
        let half = (tall - screen) / 2.0;
        let top = if half >= 1.0 { ((-self.offset_y - half) / half).floor() * half } else { -self.offset_y - half };
        InkBand { top: top / scale, height: tall / scale }
    }
}

/// How many pages a released drag turns, in reading order: +1 the page after this one, -1 the
/// page before it, 0 when the drag is not a swipe at all.
///
/// A muṣḥaf is read right to left, so a flick to the right turns to the next page. That is the
/// whole of the difference from [`swipe_direction`], and the reason this exists: every host was
/// turning the physical direction into a page of its own accord, and a host that got the sign
/// the wrong way round turned the book backwards.
pub fn swipe_pages(dx: f32, dy: f32, vx: f32, vy: f32) -> i32 {
    swipe_direction(dx, dy, vx, vy)
}

/// Which way a released drag went: +1 the finger moved right, -1 left, 0 when it is not a swipe.
/// Mostly sideways, and either far enough or fast enough. A host turning pages wants
/// [`swipe_pages`], which reads this in the muṣḥaf's own order.
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

    /// The band of the current layout to draw for this view: [`View::ink_band`] on the
    /// layout's content height, or on the page's height when the page has no layout.
    pub fn layout_ink_band(
        &self,
        view: View,
        viewport_w: f32,
        viewport_h: f32,
        max_height: f32,
        max_pixels: f32,
    ) -> InkBand {
        let content_h = self.current_layout().map_or(self.height(), |l| l.content_h);
        view.ink_band(content_h, viewport_w, viewport_h, max_height, max_pixels)
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
use crate::meta::{MarkerInfo, SurahHeader};

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

    /// The surah headings in viewport px through the current layout: where each is drawn.
    ///
    /// Without a layout this returns the page-unit boxes. The layout moves a heading's line,
    /// so a host that maps the page-unit boxes itself must apply that move. This call has
    /// already applied it.
    pub fn surah_headers_view(&self) -> Vec<SurahHeader> {
        let Some(l) = self.current_layout() else { return self.surah_headers() };
        let n_words = self.data().words.len() as u32;
        self.surah_headers_cached()
            .iter()
            .map(|&h| {
                let p = match l.reflow.as_ref() {
                    Some(_) => l.groups.get((n_words + h.decoration) as usize).copied(),
                    None => l.groups.get(h.line as usize).copied(),
                }
                .unwrap_or(crate::Placement::IDENTITY);
                // The frame spans the row the heading is on. On a reflowed page that is the
                // row's own width, not the printed text block scaled by the banner's growth,
                // which was wider than the row as soon as the reader zoomed in. The banner's
                // growth still sets the frame's height.
                let (px0, y0) = p.apply(h.x0, h.y0);
                let (px1, y1) = p.apply(h.x1, h.y1);
                let (x0, x1) = match l.reflow.as_ref() {
                    Some(flow) => (0.0, flow.row_w),
                    None => (px0, px1),
                };
                let (tx0, ty0) = p.apply(h.title_x0, h.title_y0);
                let (tx1, ty1) = p.apply(h.title_x1, h.title_y1);
                SurahHeader {
                    x0: l.offset_x + x0 * l.scale,
                    y0: l.offset_y + y0 * l.scale,
                    x1: l.offset_x + x1 * l.scale,
                    y1: l.offset_y + y1 * l.scale,
                    title_x0: l.offset_x + tx0 * l.scale,
                    title_y0: l.offset_y + ty0 * l.scale,
                    title_x1: l.offset_x + tx1 * l.scale,
                    title_y1: l.offset_y + ty1 * l.scale,
                    ..h
                }
            })
            .collect()
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
