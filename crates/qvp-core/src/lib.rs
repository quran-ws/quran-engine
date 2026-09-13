//! QVP engine core: page loader, flattened geometry, hit-testing, layout,
//! layered styles with transitions, highlights, selection, memorisation, text,
//! search, metadata, crop export and the cross-page atlas.
//! Renderer-agnostic — a host canvas implements [`Renderer`] or consumes the C ABI.
#![forbid(unsafe_code)]

pub mod crop;
pub mod defaults;
pub mod highlight;
pub mod hit;
pub mod layout;
pub mod memorize;
pub mod meta;
pub mod selection;
pub mod style;
pub mod target;
pub mod text;

pub use crop::CropBox;
pub use highlight::{BandBox, BandHeight, HighlightMode, HighlightStyle, ViewBox};
pub use hit::{HitBox, HitEx, HitOptions, LineBand};
pub use layout::{gap_to_fill, wasted_fraction, Layout, LayoutSpec};
pub use memorize::{MaskMode, MaskState, Reveal};
pub use meta::{Division, MarkerInfo, Rosette, SurahInfo};
pub use qvp_format;
pub use qvp_format::atlas::Atlas;
pub use selection::Selection;
pub use style::{
    Handle, Paint, Selector, StyleEngine, Theme, LAYER_BASE, LAYER_HIGHLIGHT, LAYER_SELECTION, LAYER_THEME, LAYER_TOP,
};
pub use target::Target;
pub use text::{
    fold, is_mark, loose_key, normalize_query, parse_words_sidecar, search_key, search_variants, strip_marks, Form,
    Match, SearchMode, SearchOptions, WordForms,
};

use qvp_format::*;
use std::collections::HashMap;
use style::{PathAnim, PathCtx};

/// Colour as 0xRRGGBBAA. Alpha 0 hides the path.
pub type Rgba = u32;
pub const DEFAULT_INK: Rgba = defaults::INK;
pub const NONE: u32 = u32::MAX;

/// Per-path geometry range into [`Geometry::ops`] / [`Geometry::pts`].
#[derive(Clone, Copy, Debug, Default)]
#[repr(C)]
pub struct PathGeom {
    pub op_start: u32,
    pub op_count: u32,
    pub pt_start: u32,
    pub pt_count: u32,
    /// kind | mark<<8 | family<<16 | (evenodd?1:0)<<24 | (instance?2:0)<<24
    pub flags: u32,
    /// Owning word index, or NONE for decoration paths.
    pub word: u32,
    /// Line index the path is laid out with (always valid).
    pub line: u32,
    /// category | nth_in_word<<8 | nth_mark<<16
    pub extra: u32,
}

/// Flattened page geometry in page units (f32), ready for any canvas.
#[derive(Debug, Default)]
pub struct Geometry {
    pub ops: Vec<u8>,
    pub pts: Vec<f32>,
    pub table: Vec<PathGeom>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hit {
    pub word: u32,
    pub path: u32,
    pub deco: u32,
}

pub struct Page {
    data: PageData,
    geom: Geometry,
    line_centre: Vec<f32>,
    natural_pitch: f32,
    layout: Option<Layout>,
    /// words of each line sorted by bbox.x0 ascending: (x0, word idx)
    line_words: Vec<Vec<(i32, u32)>>,
    path_deco: Vec<u32>,
    pub(crate) path_ctx: Vec<PathCtx>,
    word_index: HashMap<(u16, u16, u16), u32>,
    pub styles: StyleEngine,
    pub(crate) mask: MaskState,
    pub(crate) reveal: Option<Reveal>,
    pub(crate) highlights: Vec<highlight::Highlight>,
    pub selection: Selection,
    pub(crate) clock_ms: f64,
    pub(crate) anim: Vec<PathAnim>,
    pub(crate) state_dirty: bool,
    colors: Vec<Rgba>,
}

impl Page {
    pub fn load(bytes: &[u8]) -> Result<Page, Error> {
        Ok(Page::from_data(decode(bytes)?))
    }

    pub fn from_data(data: PageData) -> Page {
        let q = data.header.quant as f32;
        let mut geom = Geometry::default();
        let mut path_word = vec![NONE; data.paths.len()];
        for (wi, w) in data.words.iter().enumerate() {
            for p in w.first_path..w.first_path + w.n_paths as u32 {
                path_word[p as usize] = wi as u32;
            }
        }
        let mut path_deco = vec![NONE; data.paths.len()];
        for (di, d) in data.decos.iter().enumerate() {
            for p in d.first_path..d.first_path + d.n_paths as u32 {
                path_deco[p as usize] = di as u32;
            }
        }
        // line reference centres from body ink (marks would wobble them)
        let mut line_centre = vec![0f32; data.lines.len()];
        for (li, l) in data.lines.iter().enumerate() {
            let mut bb = IBox::EMPTY;
            for wi in l.first_word..l.first_word + l.n_words {
                let w = &data.words[wi as usize];
                for p in w.first_path..w.first_path + w.n_paths as u32 {
                    let pr = &data.paths[p as usize];
                    if pr.kind == PathKind::Body {
                        bb.union(&pr.bbox);
                    }
                }
            }
            for d in data.decos.iter().filter(|d| d.line as usize == li) {
                bb.union(&d.bbox);
            }
            if bb.is_empty() {
                bb = l.bbox;
            }
            line_centre[li] = if bb.is_empty() { 0.0 } else { (bb.y0 + bb.y1) as f32 / 2.0 / q };
        }
        let natural_pitch = {
            let mut d: Vec<f32> = line_centre.windows(2).map(|w| (w[1] - w[0]).abs()).filter(|v| *v > 1.0).collect();
            d.sort_by(|a, b| a.partial_cmp(b).unwrap());
            if d.is_empty() {
                data.header.height / 15.0
            } else {
                d[d.len() / 2]
            }
        };
        let nearest_line = |y: f32| -> u32 {
            let mut best = 0usize;
            let mut bd = f32::MAX;
            for (i, c) in line_centre.iter().enumerate() {
                let dd = (c - y).abs();
                if dd < bd {
                    bd = dd;
                    best = i;
                }
            }
            best as u32
        };
        let mut path_line = vec![0u32; data.paths.len()];
        for i in 0..data.paths.len() {
            let wi = path_word[i];
            path_line[i] = if wi != NONE {
                data.words[wi as usize].line_idx as u32
            } else if path_deco[i] != NONE {
                let d = &data.decos[path_deco[i] as usize];
                if d.line != NONE_U16 && (d.line as usize) < data.lines.len() {
                    d.line as u32
                } else {
                    nearest_line((d.bbox.y0 + d.bbox.y1) as f32 / 2.0 / q)
                }
            } else {
                0
            };
        }
        // per-path context for the style matcher
        let mut path_ctx = Vec::with_capacity(data.paths.len());
        let mut nth_in_word = 0u16;
        let mut nth_mark = 0u16;
        let mut named: HashMap<Mark, u16> = HashMap::new();
        let mut cur_word = NONE;
        for (i, p) in data.paths.iter().enumerate() {
            let wi = path_word[i];
            if wi != cur_word {
                cur_word = wi;
                nth_in_word = 0;
                nth_mark = 0;
                named.clear();
            }
            let di = path_deco[i];
            let (surah, ayah, line_no) = if wi != NONE {
                let w = &data.words[wi as usize];
                (w.surah, w.ayah, data.lines[w.line_idx as usize].line_no)
            } else if di != NONE {
                let d = &data.decos[di as usize];
                (d.surah, d.ayah, data.lines[path_line[i] as usize].line_no)
            } else {
                (0, 0, 0)
            };
            let nm = *named.entry(p.mark).and_modify(|v| *v += 1).or_insert(0);
            path_ctx.push(PathCtx {
                word: wi,
                deco: di,
                line_no,
                surah,
                ayah,
                kind: p.kind,
                mark: p.mark,
                family: p.family,
                category: p.mark.category(),
                nth_in_word,
                nth_mark: if p.kind == PathKind::Mark { nth_mark } else { u16::MAX },
                nth_mark_named: nm,
                deco_kind: if di != NONE { data.decos[di as usize].kind } else { DecoKind::Other },
            });
            if wi != NONE {
                nth_in_word += 1;
                if p.kind == PathKind::Mark {
                    nth_mark += 1;
                }
            }
        }
        for (i, p) in data.paths.iter().enumerate() {
            let op_start = geom.ops.len() as u32;
            let pt_start = geom.pts.len() as u32;
            let mut push = |x: f32, y: f32| {
                geom.pts.push(x);
                geom.pts.push(y);
            };
            if let Some(inst) = data.path_inst(i) {
                let m = |x: i32, y: i32| -> (f32, f32) {
                    let (gx, gy) = (x as f64 / q as f64, y as f64 / q as f64);
                    (
                        (inst.a as f64 * gx + inst.c as f64 * gy + inst.e as f64) as f32,
                        (inst.b as f64 * gx + inst.d as f64 * gy + inst.f as f64) as f32,
                    )
                };
                for c in data.glyph_cmds(inst.glyph as usize).unwrap_or_default() {
                    emit(&mut geom.ops, &mut push, c, m);
                }
            } else {
                let m = |x: i32, y: i32| -> (f32, f32) { (x as f32 / q, y as f32 / q) };
                for c in data.path_cmds(i).unwrap_or_default() {
                    emit(&mut geom.ops, &mut push, c, m);
                }
            }
            let evenodd = (p.flags & PF_EVENODD != 0) as u32;
            let inst = (p.flags & PF_GLYPH != 0) as u32;
            let c = &path_ctx[i];
            geom.table.push(PathGeom {
                op_start,
                op_count: geom.ops.len() as u32 - op_start,
                pt_start,
                pt_count: geom.pts.len() as u32 - pt_start,
                flags: p.kind as u32 | (p.mark as u32) << 8 | (p.family as u32) << 16 | (evenodd | inst << 1) << 24,
                word: path_word[i],
                line: path_line[i],
                extra: c.category as u32
                    | (c.nth_in_word as u32 & 0xff) << 8
                    | ((if c.nth_mark == u16::MAX { 0xff } else { c.nth_mark as u32 }) & 0xff) << 16,
            });
        }
        let mut line_words = Vec::with_capacity(data.lines.len());
        for l in &data.lines {
            let mut v: Vec<(i32, u32)> = (l.first_word..l.first_word + l.n_words)
                .map(|wi| (data.words[wi as usize].bbox.x0, wi as u32))
                .collect();
            v.sort_unstable();
            line_words.push(v);
        }
        let word_index = data.words.iter().enumerate().map(|(i, w)| ((w.surah, w.ayah, w.word), i as u32)).collect();
        let n = data.paths.len();
        Page {
            data,
            geom,
            line_centre,
            natural_pitch,
            layout: None,
            line_words,
            path_deco,
            path_ctx,
            word_index,
            styles: StyleEngine::new(),
            mask: MaskState::default(),
            reveal: None,
            highlights: vec![],
            selection: Selection::default(),
            clock_ms: 0.0,
            anim: vec![PathAnim::default(); n],
            state_dirty: true,
            colors: vec![DEFAULT_INK; n],
        }
    }

    pub fn data(&self) -> &PageData {
        &self.data
    }
    pub(crate) fn data_mut(&mut self) -> &mut PageData {
        &mut self.data
    }
    /// Intern a string into the page's string table.
    pub(crate) fn intern(&mut self, s: &str) -> u16 {
        if let Some(i) = self.data.strings.iter().position(|x| x == s) {
            return i as u16;
        }
        self.data.strings.push(s.to_owned());
        (self.data.strings.len() - 1) as u16
    }
    pub fn geometry(&self) -> &Geometry {
        &self.geom
    }
    pub fn width(&self) -> f32 {
        self.data.header.width
    }
    pub fn height(&self) -> f32 {
        self.data.header.height
    }
    pub fn quant(&self) -> f32 {
        self.data.header.quant as f32
    }
    pub fn page_number(&self) -> u16 {
        self.data.header.page
    }
    pub fn word_text(&self, wi: u32) -> &str {
        self.word_form(wi, Form::RasmUthmani)
    }
    pub fn deco_text(&self, di: u32) -> &str {
        let d = &self.data.decos[di as usize];
        if d.text == NONE_U16 {
            ""
        } else {
            &self.data.strings[d.text as usize]
        }
    }
    pub fn find_word(&self, surah: u16, ayah: u16, word: u16) -> Option<u32> {
        self.word_index.get(&(surah, ayah, word)).copied()
    }
    /// All word indices of an ayah on this page, in reading order.
    pub fn ayah_words(&self, surah: u16, ayah: u16) -> Vec<u32> {
        self.resolve(&Target::Ayah(surah, ayah))
    }
    pub fn path_deco(&self, pi: u32) -> u32 {
        self.path_deco[pi as usize]
    }
    pub fn clock(&self) -> f64 {
        self.clock_ms
    }

    // ───────────── exact hit testing ─────────────

    /// Exact hit-test in page units (bbox → outline). Words first, then decorations.
    pub fn hit_test(&self, x: f32, y: f32) -> Option<Hit> {
        let q = self.quant();
        let (qx, qy) = ((x * q).round() as i32, (y * q).round() as i32);
        let mut bbox_only: Option<u32> = None;
        for (li, l) in self.data.lines.iter().enumerate() {
            if !l.bbox.contains(qx, qy) {
                continue;
            }
            let ws = &self.line_words[li];
            let k = ws.partition_point(|(x0, _)| *x0 <= qx);
            let lo = k.saturating_sub(2);
            let hi = (k + 1).min(ws.len());
            for &(_, wi) in &ws[lo..hi] {
                let w = &self.data.words[wi as usize];
                if !w.bbox.contains(qx, qy) {
                    continue;
                }
                if bbox_only.is_none() {
                    bbox_only = Some(wi);
                }
                if let Some(pi) = self.exact_path_hit(w.first_path, w.n_paths as u32, x, y) {
                    return Some(Hit { word: wi, path: pi, deco: NONE });
                }
            }
        }
        if let Some(wi) = bbox_only {
            return Some(Hit { word: wi, path: NONE, deco: NONE });
        }
        for (di, d) in self.data.decos.iter().enumerate() {
            if d.bbox.contains(qx, qy) {
                let pi = self.exact_path_hit(d.first_path, d.n_paths as u32, x, y).unwrap_or(NONE);
                return Some(Hit { word: NONE, path: pi, deco: di as u32 });
            }
        }
        None
    }

    /// Exact hit-test in viewport px using the current layout.
    pub fn hit_test_view(&self, vx: f32, vy: f32) -> Option<Hit> {
        let Some(l) = &self.layout else { return self.hit_test(vx, vy) };
        let x = (vx - l.ox) / l.scale;
        let y = (vy - l.oy) / l.scale;
        let q = self.quant();
        for (li, line) in self.data.lines.iter().enumerate() {
            let py = y - l.line_dy[li];
            let (y0, y1) = (line.bbox.y0 as f32 / q, line.bbox.y1 as f32 / q);
            if py < y0 - 0.5 || py > y1 + 0.5 {
                continue;
            }
            if let Some(h) = self.hit_test_in_line(li, x, py) {
                return Some(h);
            }
        }
        for (di, d) in self.data.decos.iter().enumerate() {
            let li = self.geom.table[d.first_path as usize].line as usize;
            let py = y - l.line_dy[li];
            let (qx, qy) = ((x * q).round() as i32, (py * q).round() as i32);
            if d.bbox.contains(qx, qy) {
                let pi = self.exact_path_hit(d.first_path, d.n_paths as u32, x, py).unwrap_or(NONE);
                return Some(Hit { word: NONE, path: pi, deco: di as u32 });
            }
        }
        None
    }

    fn hit_test_in_line(&self, li: usize, x: f32, y: f32) -> Option<Hit> {
        let q = self.quant();
        let (qx, qy) = ((x * q).round() as i32, (y * q).round() as i32);
        let ws = &self.line_words[li];
        let k = ws.partition_point(|(x0, _)| *x0 <= qx);
        let lo = k.saturating_sub(2);
        let hi = (k + 1).min(ws.len());
        let mut bbox_only = None;
        for &(_, wi) in &ws[lo..hi] {
            let w = &self.data.words[wi as usize];
            if !w.bbox.contains(qx, qy) {
                continue;
            }
            if bbox_only.is_none() {
                bbox_only = Some(wi);
            }
            if let Some(pi) = self.exact_path_hit(w.first_path, w.n_paths as u32, x, y) {
                return Some(Hit { word: wi, path: pi, deco: NONE });
            }
        }
        if let Some(wi) = bbox_only {
            return Some(Hit { word: wi, path: NONE, deco: NONE });
        }
        for (di, d) in self.data.decos.iter().enumerate() {
            if self.geom.table[d.first_path as usize].line as usize != li || !d.bbox.contains(qx, qy) {
                continue;
            }
            let pi = self.exact_path_hit(d.first_path, d.n_paths as u32, x, y).unwrap_or(NONE);
            return Some(Hit { word: NONE, path: pi, deco: di as u32 });
        }
        None
    }

    fn exact_path_hit(&self, first: u32, n: u32, x: f32, y: f32) -> Option<u32> {
        let q = self.quant();
        let (qx, qy) = ((x * q).round() as i32, (y * q).round() as i32);
        for pass in 0..2 {
            for pi in first..first + n {
                let p = &self.data.paths[pi as usize];
                let is_body = matches!(
                    p.kind,
                    PathKind::Body | PathKind::HeaderInk | PathKind::AyahMarkOrnament | PathKind::AyahNumber
                );
                if (pass == 0) != is_body || !p.bbox.contains(qx, qy) {
                    continue;
                }
                if self.point_in_path(pi, x, y) {
                    return Some(pi);
                }
            }
        }
        None
    }

    /// Exact containment test against the flattened outline.
    pub fn point_in_path(&self, pi: u32, x: f32, y: f32) -> bool {
        let g = &self.geom.table[pi as usize];
        let ops = &self.geom.ops[g.op_start as usize..(g.op_start + g.op_count) as usize];
        let pts = &self.geom.pts[g.pt_start as usize..(g.pt_start + g.pt_count) as usize];
        let evenodd = g.flags >> 24 & 1 == 1;
        let mut wn: i32 = 0;
        let mut crossings = 0u32;
        let (mut cx, mut cy) = (0f32, 0f32);
        let (mut sx, mut sy) = (0f32, 0f32);
        let mut k = 0usize;
        let mut seg = |x0: f32, y0: f32, x1: f32, y1: f32| {
            if (y0 <= y) != (y1 <= y) {
                let t = (y - y0) / (y1 - y0);
                let xi = x0 + t * (x1 - x0);
                if xi > x {
                    crossings += 1;
                    wn += if y1 > y0 { 1 } else { -1 };
                }
            }
        };
        const N: usize = 8;
        for &op in ops {
            match op {
                OP_MOVE => {
                    if (cx, cy) != (sx, sy) {
                        seg(cx, cy, sx, sy);
                    }
                    cx = pts[k];
                    cy = pts[k + 1];
                    sx = cx;
                    sy = cy;
                    k += 2;
                }
                OP_LINE => {
                    let (nx, ny) = (pts[k], pts[k + 1]);
                    seg(cx, cy, nx, ny);
                    cx = nx;
                    cy = ny;
                    k += 2;
                }
                OP_QUAD => {
                    let (x1, y1, x2, y2) = (pts[k], pts[k + 1], pts[k + 2], pts[k + 3]);
                    let (mut px, mut py) = (cx, cy);
                    for i in 1..=N {
                        let t = i as f32 / N as f32;
                        let u = 1.0 - t;
                        let bx = u * u * cx + 2.0 * u * t * x1 + t * t * x2;
                        let by = u * u * cy + 2.0 * u * t * y1 + t * t * y2;
                        seg(px, py, bx, by);
                        px = bx;
                        py = by;
                    }
                    cx = x2;
                    cy = y2;
                    k += 4;
                }
                OP_CUBIC => {
                    let (x1, y1, x2, y2, x3, y3) = (pts[k], pts[k + 1], pts[k + 2], pts[k + 3], pts[k + 4], pts[k + 5]);
                    let (mut px, mut py) = (cx, cy);
                    for i in 1..=N {
                        let t = i as f32 / N as f32;
                        let u = 1.0 - t;
                        let bx = u * u * u * cx + 3.0 * u * u * t * x1 + 3.0 * u * t * t * x2 + t * t * t * x3;
                        let by = u * u * u * cy + 3.0 * u * u * t * y1 + 3.0 * u * t * t * y2 + t * t * t * y3;
                        seg(px, py, bx, by);
                        px = bx;
                        py = by;
                    }
                    cx = x3;
                    cy = y3;
                    k += 6;
                }
                OP_CLOSE => {
                    seg(cx, cy, sx, sy);
                    cx = sx;
                    cy = sy;
                }
                _ => {}
            }
        }
        if (cx, cy) != (sx, sy) {
            seg(cx, cy, sx, sy);
        }
        if evenodd {
            crossings % 2 == 1
        } else {
            wn != 0
        }
    }

    /// Drive a renderer over the whole page (bands first, then ink).
    pub fn render<R: Renderer>(&mut self, r: &mut R) {
        r.begin(self.width(), self.height());
        for b in self.highlight_boxes_view() {
            r.fill_rect(b.x0, b.y0, b.x1, b.y1, b.radius, b.color);
        }
        self.paint();
        for (i, g) in self.geom.table.iter().enumerate() {
            let c = self.colors[i];
            if c & 0xff == 0 {
                continue;
            }
            let ops = &self.geom.ops[g.op_start as usize..(g.op_start + g.op_count) as usize];
            let pts = &self.geom.pts[g.pt_start as usize..(g.pt_start + g.pt_count) as usize];
            r.fill_path(i as u32, ops, pts, c, g.flags >> 24 & 1 == 1);
        }
        for b in self.mask_boxes_view() {
            r.fill_rect(b.x0, b.y0, b.x1, b.y1, b.radius, b.color);
        }
        r.end();
    }
}

fn emit(ops: &mut Vec<u8>, push: &mut impl FnMut(f32, f32), c: Cmd, m: impl Fn(i32, i32) -> (f32, f32)) {
    match c {
        Cmd::MoveTo(x, y) => {
            ops.push(OP_MOVE);
            let (x, y) = m(x, y);
            push(x, y);
        }
        Cmd::LineTo(x, y) => {
            ops.push(OP_LINE);
            let (x, y) = m(x, y);
            push(x, y);
        }
        Cmd::QuadTo(x1, y1, x, y) => {
            ops.push(OP_QUAD);
            let (a, b) = m(x1, y1);
            push(a, b);
            let (x, y) = m(x, y);
            push(x, y);
        }
        Cmd::CubicTo(x1, y1, x2, y2, x, y) => {
            ops.push(OP_CUBIC);
            let (a, b) = m(x1, y1);
            push(a, b);
            let (a, b) = m(x2, y2);
            push(a, b);
            let (x, y) = m(x, y);
            push(x, y);
        }
        Cmd::Close => ops.push(OP_CLOSE),
    }
}

/// Renderer backend contract. The host canvas wrapper implements this (or
/// consumes the C ABI equivalents); a bundled rasteriser can slot in later.
pub trait Renderer {
    fn begin(&mut self, width: f32, height: f32);
    fn fill_path(&mut self, index: u32, ops: &[u8], pts: &[f32], color: Rgba, evenodd: bool);
    fn fill_rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, radius: f32, color: Rgba);
    fn end(&mut self);
}

#[cfg(test)]
mod tests;
