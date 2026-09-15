//! Vertical layout: line spacing, leading that fills a screen, padding.
//!
//! The point of these knobs is a phone: a printed mushaf page fitted to the width of a
//! tall screen leaves empty paper above and below, and the leading here fills it, so
//! the page fills the screen. Expansion only: the printed spacing is the floor, and the
//! text width is never a knob at all (the page is always fitted to the viewport width).
use crate::reflow::{Placement, ReflowSpec, Reflowed};
use crate::Page;

/// How to place lines vertically. All lengths in *viewport pixels*.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LayoutSpec {
    pub viewport_w: f32,
    pub viewport_h: f32,
    pub pad_top: f32,
    pub pad_bottom: f32,
    pub pad_left: f32,
    pub pad_right: f32,
    /// Multiplier on the printed line spacing (1.0 = as printed). The printed
    /// positions are kept; the same delta `line_spacing_printed·(line_spacing − 1)`
    /// is added between every pair of consecutive lines. Values below 1.0 are
    /// clamped to 1.0: spacing only ever opens up, never tightens.
    pub line_spacing: f32,
    /// Choose the delta so the page fills `viewport_h - pad_top - pad_bottom`
    /// (overrides line_spacing). A short page (fewer lines than the grid) has no
    /// height of its own to fill: it takes the rows a full page gets,
    /// `(viewport_h - pads) / grid_lines` each, centred. Never below the printed
    /// spacing: a page that cannot fit at it reports a `content_h` taller than
    /// the viewport.
    pub fill_height: bool,
    /// The line grid to lay the page out inside; 0 means the page's own grid
    /// ([`Page::grid`], 15 lines for this mushaf). Pass the page's line count to
    /// make a short page fill the viewport on its own.
    pub grid_lines: u32,
    /// Printed side margins to cut, in page units (0 = keep the print's margins). The
    /// ink then spans the padded viewport width instead of the full page width.
    pub crop_left: f32,
    pub crop_right: f32,
    /// Upper bound on the content width as a multiple of the page's aspect ratio applied
    /// to the viewport height: the page is never wider than
    /// `viewport_h · page_w / page_h · max_aspect_slack`, so a landscape screen does not
    /// stretch the lines to its full width. 0 turns the bound off.
    pub max_aspect_slack: f32,
    /// Break the words onto other rows instead of drawing the printed lines. The page keeps
    /// its width and grows taller; the host scrolls it. `None` lays the page out as printed.
    pub reflow: Option<ReflowSpec>,
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
            fill_height: false,
            grid_lines: 0,
            crop_left: 0.0,
            crop_right: 0.0,
            max_aspect_slack: 0.0,
            reflow: None,
        }
    }
}

/// The line grid a page is designed on: how many lines a full page has and the printed
/// spacing between them, in page units.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grid {
    pub lines: u32,
    pub line_spacing: f32,
}
/// Result of [`Page::layout`]: page units → viewport px is
/// `view_x = offset_x + x*scale`, `view_y = offset_y + (y + line_dy[line])*scale`.
#[derive(Clone, Debug, PartialEq)]
pub struct Layout {
    pub scale: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    /// Per-line vertical shift in page units.
    pub line_dy: Vec<f32>,
    /// Laid-out vertical extent of each line in viewport px: (top, bottom).
    /// Boundaries sit halfway between neighbouring lines' laid-out centres —
    /// except beside a header line (surah name, basmalah), where a boundary
    /// stops half a line spacing from the centre. Slots never overlap.
    pub line_slots: Vec<(f32, f32)>,
    /// Total content height in viewport px including padding.
    pub content_h: f32,
    /// Content width in viewport px (the padded page width).
    pub content_w: f32,
    /// The line spacing the layout produced, in page units (printed spacing + the added delta).
    pub line_spacing: f32,
    /// The view transform that shows the whole laid-out content inside the viewport:
    /// shrink by `fit_scale` when the content is taller than the viewport (never enlarge),
    /// then offset by `fit_x`, `fit_y` so the content is centred. A host draws at
    /// `fit_x + fit_scale · vx`, `fit_y + fit_scale · vy`, and applies its own pan and
    /// zoom on top.
    pub fit_scale: f32,
    pub fit_x: f32,
    pub fit_y: f32,
    /// Where each group of paths is placed. Without reflow there is one group per printed
    /// line, holding that line's `line_dy`. With reflow the groups are the page's words and
    /// decorations: `path_group` says which group a path belongs to.
    pub groups: Vec<Placement>,
    /// Group of every path. Empty without reflow, where a path's group is its printed line.
    pub path_group: Vec<u32>,
    /// Paths this layout does not draw: the sheet's furniture on a reflowed page (running
    /// head, page number), which the print puts outside the page box. Sorted.
    pub omitted_paths: Vec<u32>,
    /// The rows the words were broken onto, when this layout reflowed the page.
    pub reflow: Option<Reflowed>,
}

impl Layout {
    /// Placement of a path's group.
    pub fn placement(&self, path: u32, line: u32) -> Placement {
        let g = if self.path_group.is_empty() { line } else { self.path_group[path as usize] };
        self.groups.get(g as usize).copied().unwrap_or(Placement::IDENTITY)
    }
    /// Placement of a word: where reflow put it, or its printed line's shift.
    pub fn word_placement(&self, word: u32, line: usize) -> Placement {
        match &self.reflow {
            Some(r) => r.word_place[word as usize],
            None => Placement::shifted(self.line_dy.get(line).copied().unwrap_or(0.0)),
        }
    }
    /// True when this layout broke the words onto rows of its own.
    pub fn is_reflowed(&self) -> bool {
        self.reflow.is_some()
    }
}

/// Leading (page units, between consecutive lines) that makes a page fill a
/// viewport when fitted to width: `needed = pageW·viewH/viewW`,
/// `gap = (needed − pageH)/(lines − 1)`, clamped at 0 and `max`.
fn gap_to_fill(page_w: f32, page_h: f32, lines: u32, view_w: f32, view_h: f32, max: f32) -> f32 {
    if lines < 2 || view_w <= 0.0 {
        return 0.0;
    }
    let needed = page_w * view_h / view_w;
    ((needed - page_h) / (lines as f32 - 1.0)).clamp(0.0, max)
}

/// Fraction of the viewport left empty when the page is fitted to width.
fn wasted_fraction(page_w: f32, page_h: f32, view_w: f32, view_h: f32) -> f32 {
    if view_w <= 0.0 || view_h <= 0.0 {
        return 0.0;
    }
    let shown_h = page_h * view_w / page_w;
    ((view_h - shown_h) / view_h).clamp(0.0, 1.0)
}

impl Page {
    /// The printed spacing between this page's lines, in page units.
    pub fn line_spacing(&self) -> f32 {
        self.line_spacing
    }
    /// The grid this page is laid out inside: the mushaf's line count (or this page's, when
    /// it has more) and the printed line spacing.
    pub fn grid(&self) -> Grid {
        Grid { lines: crate::defaults::GRID_LINES.max(self.data.lines.len() as u32), line_spacing: self.line_spacing }
    }
    fn grid_lines(&self, spec: &LayoutSpec) -> u32 {
        if spec.grid_lines > 0 {
            spec.grid_lines
        } else {
            self.grid().lines
        }
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
    /// The `line_spacing` multiplier that makes this page fill the padded viewport of `spec`
    /// when fitted to width; `max` bounds the multiplier (`INFINITY` for none). 1 when the
    /// page already fills it.
    pub fn line_spacing_to_fill(&self, spec: &LayoutSpec, max: f32) -> f32 {
        let view_w = spec.viewport_w - spec.pad_left - spec.pad_right;
        let view_h = spec.viewport_h - spec.pad_top - spec.pad_bottom;
        let lines = self.grid_lines(spec).max(self.data.lines.len() as u32);
        let max_gap = if max.is_finite() && max > 1.0 { (max - 1.0) * self.line_spacing } else { f32::INFINITY };
        let gap = gap_to_fill(self.width(), self.height(), lines, view_w, view_h, max_gap);
        if self.line_spacing > 0.0 {
            1.0 + gap / self.line_spacing
        } else {
            1.0
        }
    }
    /// The share of the padded viewport of `spec` left empty when the page is fitted to width.
    pub fn wasted_fraction(&self, spec: &LayoutSpec) -> f32 {
        let view_w = spec.viewport_w - spec.pad_left - spec.pad_right;
        let view_h = spec.viewport_h - spec.pad_top - spec.pad_bottom;
        wasted_fraction(self.width(), self.height(), view_w, view_h)
    }

    pub fn layout(&mut self, spec: &LayoutSpec) -> &Layout {
        // At the printed size the page is the printed page: the words are where the print has
        // them, and every layout knob — leading, fill height, the grid a short page sits on —
        // works exactly as it does without reflow. Rows only appear once the reader zooms in.
        if let Some(r) = spec.reflow.filter(|r| r.zoom > 1.0 + 1e-4) {
            let l = self.layout_reflow(spec, &r);
            self.layout = Some(l);
            return self.layout.as_ref().unwrap();
        }
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
        let nominal = self.grid_lines(spec).max(n as u32).max(2) as f32;
        let natural = self.line_spacing;
        let avail_h = (spec.viewport_h - spec.pad_top - spec.pad_bottom).max(1.0);
        // a short page has no height of its own to fill: it takes the rows a
        // full page fills the viewport with
        let on_grid = spec.fill_height && (n as f32) < nominal;
        // leading only opens up: the printed line spacing is the floor
        let delta = if on_grid {
            (avail_h / (nominal * scale) - natural).max(0.0)
        } else if spec.fill_height {
            ((avail_h / scale - ph) / (nominal - 1.0)).max(0.0)
        } else {
            (natural * (spec.line_spacing - 1.0)).max(0.0)
        };
        let line_spacing = natural + delta;
        // laid-out height in page units: the grid's rows, or the printed page
        // with the leading added
        let block_h = if on_grid { nominal * line_spacing } else { ph + (nominal - 1.0) * delta };
        // short pages: the printed page is already centred; centre the added
        // leading the same way so the page stays in the middle of the grid
        let slot0 = (nominal - n as f32) / 2.0;
        let top_units = spec.pad_top / scale + (block_h - ph - (nominal - 1.0) * delta) / 2.0;
        let mut line_dy = Vec::with_capacity(n);
        for l in self.data.lines.iter() {
            let k = slot0 + (l.line_number.max(1) as f32 - 1.0).min(n as f32 - 1.0);
            line_dy.push(top_units + k * delta);
        }
        // laid-out centres in page units; slot boundaries halfway between neighbours — except
        // beside a header line, where the printed gap (the opening pages' banner sits pitches
        // above the text) is not the line's to claim: there a boundary stops half a line spacing out
        let centres: Vec<f32> = (0..n).map(|i| self.line_centre[i] + line_dy[i]).collect();
        let headers: Vec<bool> = (0..n).map(|i| self.line_is_header(i)).collect();
        let half = line_spacing / 2.0;
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
            offset_x: spec.pad_left - crop.0 * scale,
            offset_y: 0.0,
            line_dy,
            line_slots,
            content_h,
            content_w,
            fit_scale: 0.0,
            fit_x: 0.0,
            fit_y: 0.0,
            line_spacing,
            groups: vec![],
            path_group: vec![],
            omitted_paths: vec![],
            reflow: None,
        };
        layout.groups = layout.line_dy.iter().map(|&d| Placement::shifted(d)).collect();
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

    /// Lay the page out by breaking its words onto rows of the padded viewport width.
    ///
    /// The ink is `zoom` times the size it has at fit-to-width, so a row holds
    /// `page_w / zoom` page units and the page grows taller than the viewport. `fit_scale`
    /// stays 1: the content is meant to be scrolled, never shrunk back to the screen.
    fn layout_reflow(&mut self, spec: &LayoutSpec, r: &ReflowSpec) -> Layout {
        let pw = self.width();
        let ph = self.height();
        let content_w = if spec.max_aspect_slack > 0.0 {
            spec.viewport_w.min(spec.viewport_h * pw / ph * spec.max_aspect_slack)
        } else {
            spec.viewport_w
        };
        let crop = (spec.crop_left.max(0.0), spec.crop_right.max(0.0));
        let avail_w = (content_w - spec.pad_left - spec.pad_right).max(1.0);
        let zoom = r.zoom.max(0.01);
        // A row holds the page's text block, not the whole sheet: the print keeps a margin on
        // each side, and a row that ignored it would put the words somewhere the print never
        // does. At zoom 1 this reproduces the printed lines where they are.
        let block = self.text_block();
        let page_w = (pw - crop.0 - crop.1).max(1.0);
        let (margin_l, margin_r) = ((block.0 - crop.0).max(0.0), ((pw - crop.1) - block.1).max(0.0));
        let row_w = (page_w - margin_l - margin_r).max(1.0) / zoom;
        let scale = avail_w / (page_w / zoom);
        let pitch = self.line_spacing * spec.line_spacing.max(1.0);
        let top = spec.pad_top / scale + pitch / 2.0;
        // the reader asked for no more leading than the print has, so a page whose rows come
        // out as the printed lines can keep the printed line positions too
        let printed_spacing = spec.line_spacing <= 1.0 + 1e-6 && !spec.fill_height;
        let flow = self.reflow(
            r,
            &crate::reflow::RowSpec {
                block,
                margins: (margin_l / zoom, margin_r / zoom),
                row_w,
                pitch,
                top,
                printed_spacing,
            },
        );
        // the content is the ink that was laid out; a page that came out as printed keeps the
        // printed page's own height, so zoom 1 gives the printed layout back whole
        let content_h = if flow.as_printed {
            spec.pad_top + ph * scale + spec.pad_bottom
        } else {
            spec.pad_top + (flow.y1 - flow.y0).max(flow.row_band.len() as f32 * pitch) * scale + spec.pad_bottom
        };
        let line_slots: Vec<(f32, f32)> = flow.row_band.iter().map(|(t, b)| (t * scale, b * scale)).collect();
        // one group per word, then one per decoration, then one per decoration's sajdah line:
        // a path's group follows the job it does, not only the record it belongs to
        let n_words = self.data.words.len();
        let n_decos = self.data.decorations.len();
        let mut groups: Vec<Placement> = Vec::with_capacity(n_words + n_decos * 2);
        groups.extend_from_slice(&flow.word_place);
        groups.extend_from_slice(&flow.deco_place);
        groups.extend_from_slice(&flow.sajdah_place);
        let stroke = self.sajdah_line_paths();
        let path_group: Vec<u32> = (0..self.geom.table.len())
            .map(|i| {
                let g = &self.geom.table[i];
                if g.word != u32::MAX {
                    g.word
                } else if self.path_deco[i] != u32::MAX {
                    let di = self.path_deco[i];
                    if stroke[i] {
                        (n_words + n_decos) as u32 + di
                    } else {
                        n_words as u32 + di
                    }
                } else {
                    0
                }
            })
            .collect();
        let mut omitted_paths: Vec<u32> = Vec::new();
        for &di in &flow.omitted {
            let deco = &self.data.decorations[di as usize];
            omitted_paths.extend(deco.first_path..deco.first_path + deco.n_paths as u32);
        }
        omitted_paths.sort_unstable();
        let mut layout = Layout {
            scale,
            // the block's own left margin, kept at the reader's size
            offset_x: spec.pad_left + margin_l / zoom * scale,
            offset_y: 0.0,
            line_dy: vec![0.0; self.data.lines.len()],
            line_slots,
            content_h,
            content_w,
            line_spacing: pitch,
            fit_scale: 1.0,
            fit_x: 0.0,
            fit_y: 0.0,
            groups,
            path_group,
            omitted_paths,
            reflow: Some(flow),
        };
        layout.fit_x = ((spec.viewport_w - layout.content_w) / 2.0).max(0.0);
        layout
    }

    pub fn current_layout(&self) -> Option<&Layout> {
        self.layout.as_ref()
    }

    /// Word ink box in viewport px through the current layout (for scroll-into-view etc.).
    pub fn word_bounds_view(&self, wi: u32) -> (f32, f32, f32, f32) {
        let q = self.quant();
        let w = &self.data.words[wi as usize];
        let (x0, y0, x1, y1) = (w.bbox.x0 as f32 / q, w.bbox.y0 as f32 / q, w.bbox.x1 as f32 / q, w.bbox.y1 as f32 / q);
        match &self.layout {
            Some(l) => {
                let p = l.word_placement(wi, w.line_index as usize);
                let (ax0, ay0) = p.apply(x0, y0);
                let (ax1, ay1) = p.apply(x1, y1);
                (
                    l.offset_x + ax0 * l.scale,
                    l.offset_y + ay0 * l.scale,
                    l.offset_x + ax1 * l.scale,
                    l.offset_y + ay1 * l.scale,
                )
            }
            None => (x0, y0, x1, y1),
        }
    }
}
