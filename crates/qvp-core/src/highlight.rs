//! Highlights: ink recolouring and/or a band behind the words, one path per
//! highlight covering every printed line it occupies, with engine-driven
//! transitions (colour fades, band boxes slide between positions).
use crate::style::{ease_out, lerp_rgba, Handle, Paint, Selector, LAYER_HIGHLIGHT};
use crate::{Page, Rgba, Target};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum HighlightMode {
    Ink = 0,
    Band = 1,
    Both = 2,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum BandHeight {
    /// the line spacing (bands of adjacent lines meet)
    LineSpacing = 0,
    /// the words' own ink height plus pad_y
    Ink = 1,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HighlightStyle {
    pub mode: HighlightMode,
    pub ink: Rgba,
    pub band: Rgba,
    pub height: BandHeight,
    pub pad_x: f32,
    pub pad_y: f32,
    /// corner radius of band boxes (page units)
    pub radius: f32,
    /// vertical overlap between neighbouring line boxes (page units) to kill hairlines
    pub seam: f32,
    pub transition_ms: u32,
    pub layer: i32,
}

impl Default for HighlightStyle {
    fn default() -> Self {
        HighlightStyle {
            mode: HighlightMode::Band,
            ink: crate::defaults::HIGHLIGHT_INK,
            band: crate::defaults::HIGHLIGHT_BAND,
            height: BandHeight::LineSpacing,
            pad_x: crate::defaults::HIGHLIGHT_PAD_X,
            pad_y: crate::defaults::HIGHLIGHT_PAD_Y,
            radius: 0.0,
            seam: crate::defaults::HIGHLIGHT_SEAM,
            transition_ms: 0,
            layer: LAYER_HIGHLIGHT,
        }
    }
}

/// One box per printed line, page units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BandBox {
    pub line: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

/// A band box ready to draw: viewport px through the layout, with the current
/// animated colour (alpha already applied).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewBox {
    pub highlight: Handle,
    pub line: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub color: Rgba,
    pub radius: f32,
}

/// A corner radius never exceeds half the box's shorter side, so every renderer draws the same
/// rounded rectangle (a Canvas or CoreGraphics call would otherwise clamp on its own terms).
pub(crate) fn clamp_radius(radius: f32, w: f32, h: f32, scale: f32) -> f32 {
    radius.min(w * scale / 2.0).min(h * scale / 2.0).max(0.0)
}

#[derive(Clone, Debug)]
pub(crate) struct Highlight {
    pub handle: Handle,
    pub words: Vec<u32>,
    pub style: HighlightStyle,
    pub boxes: Vec<BandBox>,
    pub prev_boxes: Vec<BandBox>,
    pub t0: f64,
    pub color_from: Rgba,
    pub color_to: Rgba,
    /// true while fading in (created) / out (removed)
    pub removing: bool,
}

impl Page {
    /// Band boxes for a word list on a reflowed page, one per row (page units, placed).
    pub(crate) fn reflow_bands(&self, words: &[u32], height: BandHeight, pad_x: f32, pad_y: f32) -> Vec<BandBox> {
        let q = self.quant();
        let Some(flow) = self.current_layout().and_then(|l| l.reflow.as_ref()) else { return vec![] };
        let mut by_row: Vec<Option<BandBox>> = vec![None; flow.row_band.len()];
        for &wi in words {
            let r = flow.word_row[wi as usize];
            if r as usize >= by_row.len() {
                continue;
            }
            let w = &self.data().words[wi as usize];
            let p = flow.word_place[wi as usize];
            let (x0, x1) = (p.apply(w.bbox.x0 as f32 / q, 0.0).0 - pad_x, p.apply(w.bbox.x1 as f32 / q, 0.0).0 + pad_x);
            let (y0, y1) = match height {
                BandHeight::LineSpacing => flow.row_band[r as usize],
                BandHeight::Ink => {
                    (p.apply(0.0, w.bbox.y0 as f32 / q).1 - pad_y, p.apply(0.0, w.bbox.y1 as f32 / q).1 + pad_y)
                }
            };
            let b = by_row[r as usize].get_or_insert(BandBox { line: r, x0, y0, x1, y1 });
            b.x0 = b.x0.min(x0);
            b.x1 = b.x1.max(x1);
            b.y0 = b.y0.min(y0);
            b.y1 = b.y1.max(y1);
        }
        by_row.into_iter().flatten().collect()
    }

    /// Band boxes for a word list (page units, before seams).
    pub fn word_bands(&self, words: &[u32], height: BandHeight, pad_x: f32, pad_y: f32) -> Vec<BandBox> {
        let q = self.quant();
        let d = self.data();
        let bands = self.line_bands();
        let mut by_line: Vec<Option<BandBox>> = vec![None; d.lines.len()];
        for &wi in words {
            let w = &d.words[wi as usize];
            let li = w.line_index as usize;
            let (x0, x1) = (w.bbox.x0 as f32 / q - pad_x, w.bbox.x1 as f32 / q + pad_x);
            let (y0, y1) = match height {
                BandHeight::LineSpacing => (bands[li].y0, bands[li].y1),
                BandHeight::Ink => (w.bbox.y0 as f32 / q - pad_y, w.bbox.y1 as f32 / q + pad_y),
            };
            let b = by_line[li].get_or_insert(BandBox { line: li as u32, x0, y0, x1, y1 });
            b.x0 = b.x0.min(x0);
            b.x1 = b.x1.max(x1);
            b.y0 = b.y0.min(y0);
            b.y1 = b.y1.max(y1);
        }
        by_line.into_iter().flatten().collect()
    }

    /// Add a highlight. Returns a handle; `remove_highlight(handle)` removes it (animated when
    /// transition_ms > 0). Ink recolouring goes through the style engine under the same handle.
    pub fn highlight(&mut self, target: &Target, style: HighlightStyle) -> Handle {
        let words = self.target_words(target);
        let h = self.styles.new_handle();
        self.install_highlight(h, words, style);
        h
    }

    fn install_highlight(&mut self, h: Handle, words: Vec<u32>, style: HighlightStyle) {
        if matches!(style.mode, HighlightMode::Ink | HighlightMode::Both) {
            for &w in &words {
                self.styles.push_under(h, style.layer, Selector::Word(w), Paint::fade(style.ink, style.transition_ms));
            }
        }
        if matches!(style.mode, HighlightMode::Band | HighlightMode::Both) {
            let boxes = self.word_bands(&words, style.height, style.pad_x, style.pad_y);
            let from = style.band & !0xff; // fade in from transparent when animated
            self.highlights.push(Highlight {
                handle: h,
                words,
                style,
                boxes,
                prev_boxes: vec![],
                t0: self.clock_ms,
                color_from: from,
                color_to: style.band,
                removing: false,
            });
        } else {
            self.highlights.push(Highlight {
                handle: h,
                words,
                style,
                boxes: vec![],
                prev_boxes: vec![],
                t0: self.clock_ms,
                color_from: style.band,
                color_to: style.band,
                removing: false,
            });
        }
    }

    /// Move an existing highlight to a new target (the band slides, ink fades).
    pub fn move_highlight(&mut self, handle: Handle, target: &Target) -> bool {
        let words = self.target_words(target);
        let Some(i) = self.highlights.iter().position(|x| x.handle == handle && !x.removing) else { return false };
        let style = self.highlights[i].style;
        // ink rules: replace under the same handle
        self.styles.remove(handle);
        if matches!(style.mode, HighlightMode::Ink | HighlightMode::Both) {
            for &w in &words {
                self.styles.push_under(
                    handle,
                    style.layer,
                    Selector::Word(w),
                    Paint::fade(style.ink, style.transition_ms),
                );
            }
        }
        let hl = &mut self.highlights[i];
        if matches!(style.mode, HighlightMode::Band | HighlightMode::Both) {
            let new_boxes = self.word_bands(&words, style.height, style.pad_x, style.pad_y);
            let hl = &mut self.highlights[i];
            // snapshot the current interpolated boxes as the new "prev"
            hl.prev_boxes = current_boxes(hl, self.clock_ms);
            hl.boxes = new_boxes;
            hl.t0 = self.clock_ms;
            hl.color_from = hl.color_to;
        } else {
            hl.prev_boxes.clear();
        }
        self.highlights[i].words = words;
        true
    }

    /// Change a highlight's colours in place.
    pub fn restyle_highlight(&mut self, handle: Handle, style: HighlightStyle) -> bool {
        let Some(i) = self.highlights.iter().position(|x| x.handle == handle && !x.removing) else { return false };
        let words = self.highlights[i].words.clone();
        self.styles.remove(handle);
        if matches!(style.mode, HighlightMode::Ink | HighlightMode::Both) {
            for &w in &words {
                self.styles.push_under(
                    handle,
                    style.layer,
                    Selector::Word(w),
                    Paint::fade(style.ink, style.transition_ms),
                );
            }
        }
        let now = self.clock_ms;
        let boxes = if matches!(style.mode, HighlightMode::Band | HighlightMode::Both) {
            self.word_bands(&words, style.height, style.pad_x, style.pad_y)
        } else {
            vec![]
        };
        let hl = &mut self.highlights[i];
        hl.prev_boxes = current_boxes(hl, now);
        hl.color_from = current_color(hl, now);
        hl.style = style;
        hl.color_to = style.band;
        hl.boxes = boxes;
        hl.t0 = now;
        true
    }

    /// Remove a highlight (fades out over its transition, then disappears).
    pub fn remove_highlight(&mut self, handle: Handle) -> bool {
        self.styles.remove(handle);
        let Some(i) = self.highlights.iter().position(|x| x.handle == handle && !x.removing) else { return false };
        let now = self.clock_ms;
        let hl = &mut self.highlights[i];
        if hl.style.transition_ms == 0 {
            self.highlights.remove(i);
            return true;
        }
        hl.prev_boxes = current_boxes(hl, now);
        hl.color_from = current_color(hl, now);
        hl.color_to = hl.style.band & !0xff;
        hl.t0 = now;
        hl.removing = true;
        true
    }

    pub fn clear_highlights(&mut self) {
        let hs: Vec<Handle> = self.highlights.iter().map(|h| h.handle).collect();
        for h in hs {
            self.styles.remove(h);
        }
        self.highlights.clear();
    }

    pub fn highlight_handles(&self) -> Vec<Handle> {
        self.highlights.iter().filter(|h| !h.removing).map(|h| h.handle).collect()
    }
    pub fn highlight_words(&self, handle: Handle) -> Vec<u32> {
        self.highlights.iter().find(|h| h.handle == handle).map(|h| h.words.clone()).unwrap_or_default()
    }

    pub(crate) fn highlights_moving(&mut self) -> bool {
        let now = self.clock_ms;
        let mut moving = false;
        self.highlights.retain(|h| !(h.removing && now - h.t0 >= h.style.transition_ms as f64));
        for h in &self.highlights {
            let dur = h.style.transition_ms as f64;
            if dur > 0.0 && now - h.t0 < dur {
                moving = true;
            }
        }
        moving
    }

    /// Band boxes of every highlight in viewport px through the current layout,
    /// with seams applied between vertically adjacent boxes and animated colours.
    /// Draw each highlight as ONE path (nonzero fill) of these boxes, behind the ink.
    pub fn highlight_boxes_view(&self) -> Vec<ViewBox> {
        let now = self.clock_ms;
        let mut out = Vec::new();
        let reflowed = self.current_layout().map(|l| l.is_reflowed()).unwrap_or(false);
        let (scale, ox, oy, dy): (f32, f32, f32, Vec<f32>) = match self.current_layout() {
            Some(l) => (l.scale, l.offset_x, l.offset_y, l.line_dy.clone()),
            None => (1.0, 0.0, 0.0, vec![0.0; self.data().lines.len()]),
        };
        for h in &self.highlights {
            // a reflowed page has rows, not printed lines: rebuild the bands from where the
            // words actually landed (they slide with the reflow, not between it and the print)
            let boxes = match self.current_layout().and_then(|l| l.reflow.as_ref()) {
                Some(_) => self.reflow_bands(&h.words, h.style.height, h.style.pad_x, h.style.pad_y),
                None => current_boxes(h, now),
            };
            let color = current_color(h, now);
            if color & 0xff == 0 {
                continue;
            }
            let seam = h.style.seam * scale;
            let mut view: Vec<ViewBox> = boxes
                .iter()
                .map(|b| {
                    let li = b.line as usize;
                    // reflow bands are already placed; printed bands still take their line's shift
                    let d = if reflowed { 0.0 } else { dy.get(li).copied().unwrap_or(0.0) };
                    ViewBox {
                        highlight: h.handle,
                        line: b.line,
                        x0: ox + b.x0 * scale,
                        y0: oy + (b.y0 + d) * scale,
                        x1: ox + b.x1 * scale,
                        y1: oy + (b.y1 + d) * scale,
                        color,
                        radius: clamp_radius(h.style.radius * scale, b.x1 - b.x0, b.y1 - b.y0, scale),
                    }
                })
                .collect();
            view.sort_by(|a, b| a.y0.partial_cmp(&b.y0).unwrap());
            // seam only where two boxes actually meet
            for i in 0..view.len() {
                if i + 1 < view.len() && (view[i + 1].y0 - view[i].y1).abs() <= seam + 0.01 {
                    view[i].y1 += seam / 2.0;
                    view[i + 1].y0 -= seam / 2.0;
                }
            }
            out.extend(view);
        }
        out
    }
}

fn current_color(h: &Highlight, now: f64) -> Rgba {
    let dur = h.style.transition_ms as f64;
    if dur <= 0.0 {
        return h.color_to;
    }
    lerp_rgba(h.color_from, h.color_to, ease_out((now - h.t0) / dur))
}

fn current_boxes(h: &Highlight, now: f64) -> Vec<BandBox> {
    let dur = h.style.transition_ms as f64;
    if dur <= 0.0 || h.prev_boxes.is_empty() || now - h.t0 >= dur {
        return if h.removing {
            h.prev_boxes
                .clone()
                .into_iter()
                .chain(h.boxes.clone())
                .collect::<Vec<_>>()
                .into_iter()
                .take(h.prev_boxes.len().max(h.boxes.len()))
                .collect()
        } else {
            h.boxes.clone()
        };
    }
    let t = ease_out((now - h.t0) / dur) as f32;
    // pair boxes by order (top to bottom); unmatched ones lerp from/to a collapsed box
    let n = h.prev_boxes.len().max(h.boxes.len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let a = h.prev_boxes.get(i).or_else(|| h.boxes.get(i)).copied().unwrap();
        let b = h.boxes.get(i).or_else(|| h.prev_boxes.get(i)).copied().unwrap();
        let l = |p: f32, q: f32| p + (q - p) * t;
        out.push(BandBox {
            line: if t < 0.5 { a.line } else { b.line },
            x0: l(a.x0, b.x0),
            y0: l(a.y0, b.y0),
            x1: l(a.x1, b.x1),
            y1: l(a.y1, b.y1),
        });
    }
    out
}
