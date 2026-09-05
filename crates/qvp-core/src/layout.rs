//! Vertical layout: line spacing, leading that fills a screen, padding.
use crate::Page;

/// How to place lines vertically. All lengths in *viewport pixels* except
/// `line_gap` (page units).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutSpec {
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub pad_top: f32,
    pub pad_bottom: f32,
    pub pad_left: f32,
    pub pad_right: f32,
    /// Multiplier on the printed line pitch (1.0 = as printed). The printed
    /// positions are kept; the same delta `pitch·(line_spacing − 1)` is
    /// added between every pair of consecutive lines.
    pub line_spacing: f32,
    /// Extra leading between lines in page units, added after the multiplier.
    pub line_gap: f32,
    /// Choose the delta so the page fills `viewport_h - pad_top - pad_bottom`
    /// (overrides line_spacing / line_gap).
    pub fill_height: bool,
    /// Line grid the mushaf is designed on (15 for KFGQPC Hafs). Short pages
    /// (fewer lines) stay centred, as printed.
    pub nominal_lines: u32,
}

impl Default for LayoutSpec {
    fn default() -> Self {
        LayoutSpec { viewport_w: 345.0, viewport_h: 550.0, pad_top: 0.0, pad_bottom: 0.0, pad_left: 0.0, pad_right: 0.0, line_spacing: 1.0, line_gap: 0.0, fill_height: false, nominal_lines: 15 }
    }
}

/// Result of [`Page::layout`]: page units → viewport px is
/// `vx = ox + x*scale`, `vy = oy + (y + line_dy[line])*scale`.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub scale: f32,
    pub ox: f32,
    pub oy: f32,
    /// Per-line vertical shift in page units.
    pub line_dy: Vec<f32>,
    /// Laid-out vertical extent of each line in viewport px: (top, bottom).
    /// Boundaries sit halfway between neighbouring lines' laid-out centres.
    pub line_slots: Vec<(f32, f32)>,
    /// Total content height in viewport px including padding.
    pub content_h: f32,
    /// Content width in viewport px (the padded page width).
    pub content_w: f32,
    /// Effective line pitch in page units (printed pitch + the added delta).
    pub pitch: f32,
}

/// Leading (page units, between consecutive lines) that makes a page fill a
/// viewport when fitted to width: `needed = pageW·viewH/viewW`,
/// `gap = (needed − pageH)/(lines − 1)`, clamped at 0 and `max`.
pub fn gap_to_fill(page_w: f32, page_h: f32, lines: u32, view_w: f32, view_h: f32, max: f32) -> f32 {
    if lines < 2 || view_w <= 0.0 {
        return 0.0;
    }
    let needed = page_w * view_h / view_w;
    ((needed - page_h) / (lines as f32 - 1.0)).clamp(0.0, max)
}

/// Fraction of the viewport left empty when the page is fitted to width.
pub fn wasted_fraction(page_w: f32, page_h: f32, view_w: f32, view_h: f32) -> f32 {
    if view_w <= 0.0 || view_h <= 0.0 {
        return 0.0;
    }
    let shown_h = page_h * view_w / page_w;
    ((view_h - shown_h) / view_h).clamp(0.0, 1.0)
}

impl Page {
    pub fn natural_pitch(&self) -> f32 {
        self.natural_pitch
    }
    pub fn line_centre(&self, li: usize) -> f32 {
        self.line_centre[li]
    }

    /// Compute and store a layout. Horizontal placement is as printed (scaled to
    /// the padded viewport width); vertical placement moves whole lines by `line_dy`.
    ///
    /// Printed lines are neither equally tall nor equally pitched, and ink often
    /// reaches into the neighbouring line, so lines are never re-spread onto a
    /// grid. Instead every line keeps its printed position and the *same* delta
    /// is inserted between each pair of consecutive lines: line `k` (0-based on
    /// the nominal grid) moves by `k·delta`. `delta = 0` reproduces the print.
    pub fn layout(&mut self, spec: &LayoutSpec) -> &Layout {
        let pw = self.width();
        let ph = self.height();
        let avail_w = (spec.viewport_w - spec.pad_left - spec.pad_right).max(1.0);
        let scale = avail_w / pw;
        let n = self.data.lines.len();
        let nominal = spec.nominal_lines.max(n as u32).max(2) as f32;
        let natural = self.natural_pitch;
        // never squeeze below 5% of the printed pitch
        let min_delta = -0.95 * natural;
        let delta = if spec.fill_height {
            let avail_h = (spec.viewport_h - spec.pad_top - spec.pad_bottom).max(1.0);
            ((avail_h / scale - ph) / (nominal - 1.0)).max(min_delta)
        } else {
            (natural * (spec.line_spacing - 1.0) + spec.line_gap).max(min_delta)
        };
        let pitch = natural + delta;
        // short pages: the printed page is already centred; centre the added
        // leading the same way so the page stays in the middle of the grid
        let slot0 = (nominal - n as f32) / 2.0;
        let top_units = spec.pad_top / scale;
        let mut line_dy = Vec::with_capacity(n);
        for l in self.data.lines.iter() {
            let k = slot0 + (l.line_no.max(1) as f32 - 1.0).min(n as f32 - 1.0);
            line_dy.push(top_units + k * delta);
        }
        // laid-out centres in page units; slot boundaries halfway between neighbours
        let centres: Vec<f32> = (0..n).map(|i| self.line_centre[i] + line_dy[i]).collect();
        let half = pitch / 2.0;
        let mut line_slots = Vec::with_capacity(n);
        for i in 0..n {
            let c = centres[i];
            let top = match i.checked_sub(1).map(|j| centres[j]) {
                Some(prev) if prev < c => (prev + c) / 2.0,
                _ => c - half,
            };
            let bottom = match centres.get(i + 1) {
                Some(&next) if next > c => (c + next) / 2.0,
                _ => c + half,
            };
            line_slots.push((top * scale, bottom * scale));
        }
        let content_h = spec.pad_top + (ph + (nominal - 1.0) * delta) * scale + spec.pad_bottom;
        self.layout = Some(Layout { scale, ox: spec.pad_left, oy: 0.0, line_dy, line_slots, content_h, content_w: spec.viewport_w, pitch });
        self.layout.as_ref().unwrap()
    }

    pub fn current_layout(&self) -> Option<&Layout> {
        self.layout.as_ref()
    }

    /// Word ink box in viewport px through the current layout (for scroll-into-view etc.).
    pub fn word_box_view(&self, wi: u32) -> (f32, f32, f32, f32) {
        let q = self.quant();
        let w = &self.data.words[wi as usize];
        let (x0, y0, x1, y1) = (w.bbox.x0 as f32 / q, w.bbox.y0 as f32 / q, w.bbox.x1 as f32 / q, w.bbox.y1 as f32 / q);
        match &self.layout {
            Some(l) => {
                let d = l.line_dy[w.line_idx as usize];
                (l.ox + x0 * l.scale, l.oy + (y0 + d) * l.scale, l.ox + x1 * l.scale, l.oy + (y1 + d) * l.scale)
            }
            None => (x0, y0, x1, y1),
        }
    }
}
