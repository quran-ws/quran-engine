//! Vertical layout: line spacing, leading that fills a screen, padding.
//!
//! The point of these knobs is a phone: a printed mushaf page fitted to the width of a
//! tall screen leaves empty paper above and below, and the leading here spends it, so
//! the page fills the screen. Expansion only — the printed pitch is the floor, and the
//! text width is never a knob at all (the page is always fitted to the viewport width).
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
    /// added between every pair of consecutive lines. Values below 1.0 are
    /// clamped to 1.0: spacing only ever opens up, never tightens.
    pub line_spacing: f32,
    /// Extra leading between lines in page units, added after the multiplier.
    /// Negative values are clamped to 0.
    pub line_gap: f32,
    /// Choose the delta so the page fills `viewport_h - pad_top - pad_bottom`
    /// (overrides line_spacing / line_gap). A short page (fewer lines than
    /// `nominal_lines`) has no height of its own to fill: it takes the rows a
    /// full page gets, `(viewport_h - pads) / nominal_lines` each, centred.
    /// Never below the printed pitch — a page that cannot fit at it reports a
    /// `content_h` taller than the viewport.
    pub fill_height: bool,
    /// Line grid the mushaf is designed on (15 for KFGQPC Hafs). Short pages
    /// (fewer lines) stay centred, as printed.
    pub nominal_lines: u32,
    /// Printed side margins to cut, in page units (0 = keep the print's margins). The
    /// ink then spans the padded viewport width instead of the full page width.
    pub crop_left: f32,
    pub crop_right: f32,
    /// Upper bound on the content width as a multiple of the page's aspect ratio applied
    /// to the viewport height: the page is never wider than
    /// `viewport_h · page_w / page_h · max_aspect_slack`, so a landscape screen does not
    /// stretch the lines to its full width. 0 turns the bound off.
    pub max_aspect_slack: f32,
}

impl Default for LayoutSpec {
    fn default() -> Self {
        LayoutSpec {
            viewport_w: 345.0,
            viewport_h: 550.0,
            pad_top: 0.0,
            pad_bottom: 0.0,
            pad_left: 0.0,
            pad_right: 0.0,
            line_spacing: 1.0,
            line_gap: 0.0,
            fill_height: false,
            nominal_lines: crate::defaults::NOMINAL_LINES,
            crop_left: 0.0,
            crop_right: 0.0,
            max_aspect_slack: 0.0,
        }
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
    /// Boundaries sit halfway between neighbouring lines' laid-out centres —
    /// except beside a header line (surah name, basmalah), where a boundary
    /// stops half a pitch from the centre. Slots never overlap.
    pub line_slots: Vec<(f32, f32)>,
    /// Total content height in viewport px including padding.
    pub content_h: f32,
    /// Content width in viewport px (the padded page width).
    pub content_w: f32,
    /// Effective line pitch in page units (printed pitch + the added delta).
    pub pitch: f32,
    /// The view transform that shows the whole laid-out content inside the viewport:
    /// shrink by `fit_scale` when the content is taller than the viewport (never enlarge),
    /// then offset by `fit_x`, `fit_y` so the content is centred. A host draws at
    /// `fit_x + fit_scale · vx`, `fit_y + fit_scale · vy`, and applies its own pan and
    /// zoom on top.
    pub fit_scale: f32,
    pub fit_x: f32,
    pub fit_y: f32,
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
    /// Leading (page units) that makes this page fill the padded viewport of `spec` when
    /// fitted to width; see [`gap_to_fill`].
    pub fn gap_to_fill(&self, spec: &LayoutSpec, max: f32) -> f32 {
        let view_w = spec.viewport_w - spec.pad_left - spec.pad_right;
        let view_h = spec.viewport_h - spec.pad_top - spec.pad_bottom;
        gap_to_fill(
            self.width(),
            self.height(),
            spec.nominal_lines.max(self.data.lines.len() as u32),
            view_w,
            view_h,
            max,
        )
    }

    pub fn layout(&mut self, spec: &LayoutSpec) -> &Layout {
        let pw = self.width();
        let ph = self.height();
        // The content width: the viewport, bounded by the page's aspect ratio when the
        // host asks for it, so a wide screen does not stretch the lines.
        let content_w = if spec.max_aspect_slack > 0.0 {
            spec.viewport_w.min(spec.viewport_h * pw / ph * spec.max_aspect_slack)
        } else {
            spec.viewport_w
        };
        let crop = (spec.crop_left.max(0.0), spec.crop_right.max(0.0));
        let avail_w = (content_w - spec.pad_left - spec.pad_right).max(1.0);
        let scale = avail_w / (pw - crop.0 - crop.1).max(1.0);
        let n = self.data.lines.len();
        let nominal = spec.nominal_lines.max(n as u32).max(2) as f32;
        let natural = self.natural_pitch;
        let avail_h = (spec.viewport_h - spec.pad_top - spec.pad_bottom).max(1.0);
        // a short page has no height of its own to fill: it takes the rows a
        // full page fills the viewport with
        let on_grid = spec.fill_height && (n as f32) < nominal;
        // leading only opens up: the printed pitch is the floor
        let delta = if on_grid {
            (avail_h / (nominal * scale) - natural).max(0.0)
        } else if spec.fill_height {
            ((avail_h / scale - ph) / (nominal - 1.0)).max(0.0)
        } else {
            (natural * (spec.line_spacing - 1.0) + spec.line_gap).max(0.0)
        };
        let pitch = natural + delta;
        // laid-out height in page units: the grid's rows, or the printed page
        // with the leading added
        let block_h = if on_grid { nominal * pitch } else { ph + (nominal - 1.0) * delta };
        // short pages: the printed page is already centred; centre the added
        // leading the same way so the page stays in the middle of the grid
        let slot0 = (nominal - n as f32) / 2.0;
        let top_units = spec.pad_top / scale + (block_h - ph - (nominal - 1.0) * delta) / 2.0;
        let mut line_dy = Vec::with_capacity(n);
        for l in self.data.lines.iter() {
            let k = slot0 + (l.line_no.max(1) as f32 - 1.0).min(n as f32 - 1.0);
            line_dy.push(top_units + k * delta);
        }
        // laid-out centres in page units; slot boundaries halfway between neighbours — except
        // beside a header line, where the printed gap (the opening pages' banner sits pitches
        // above the text) is not the line's to claim: there a boundary stops half a pitch out
        let centres: Vec<f32> = (0..n).map(|i| self.line_centre[i] + line_dy[i]).collect();
        let headers: Vec<bool> = (0..n).map(|i| self.line_is_header(i)).collect();
        let half = pitch / 2.0;
        let mut line_slots = Vec::with_capacity(n);
        for i in 0..n {
            let c = centres[i];
            let top = match i.checked_sub(1) {
                Some(j) if centres[j] < c => {
                    let mid = (centres[j] + c) / 2.0;
                    if headers[i] || headers[j] {
                        mid.max(c - half)
                    } else {
                        mid
                    }
                }
                _ => c - half,
            };
            let bottom = match centres.get(i + 1) {
                Some(&next) if next > c => {
                    let mid = (c + next) / 2.0;
                    if headers[i] || headers[i + 1] {
                        mid.min(c + half)
                    } else {
                        mid
                    }
                }
                _ => c + half,
            };
            line_slots.push((top * scale, bottom * scale));
        }
        let content_h = spec.pad_top + block_h * scale + spec.pad_bottom;
        let mut layout = Layout {
            scale,
            ox: spec.pad_left - crop.0 * scale,
            oy: 0.0,
            line_dy,
            line_slots,
            content_h,
            content_w,
            fit_scale: 0.0,
            fit_x: 0.0,
            fit_y: 0.0,
            pitch,
        };
        // Fit: shrink to the viewport height when the content is taller, never enlarge;
        // centre the result. Offsets are never negative.
        let fit_scale = if layout.content_h > spec.viewport_h && layout.content_h > 0.0 {
            spec.viewport_h / layout.content_h
        } else {
            1.0
        };
        layout.fit_scale = fit_scale;
        layout.fit_x = ((spec.viewport_w - layout.content_w * fit_scale) / 2.0).max(0.0);
        layout.fit_y = ((spec.viewport_h - layout.content_h * fit_scale) / 2.0).max(0.0);
        self.layout = Some(layout);
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
