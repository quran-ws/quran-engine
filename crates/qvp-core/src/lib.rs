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
pub mod reflow;
pub mod selection;
pub mod style;
pub mod target;
pub mod text;
pub mod view;
pub mod zoom;
mod zoom_table;

pub use crop::CropBounds;
pub use highlight::{BandBox, BandHeight, HighlightMode, HighlightStyle, ViewBox};
pub use hit::{Hit, HitArea, HitOptions, LineBand};
pub use layout::{Draw, Grid, Layout, LayoutSpec};
pub use memorize::{MaskMode, MaskState, Reveal};
pub use meta::{Division, MarkerInfo, Rosette, SurahHeader, SurahInfo};
pub use qvp_format;
pub use qvp_format::atlas::Atlas;
pub use reflow::{Breaks, Fill, GapMode, Placement, ReflowSpec, Reflowed};
pub use selection::Selection;
pub use style::{
    Handle, Paint, Selector, StyleEngine, Theme, LAYER_BASE, LAYER_HIGHLIGHT, LAYER_SELECTION, LAYER_THEME, LAYER_TOP,
};
pub use target::Target;
pub use text::{
    fold, is_mark, loose_key, normalize_query, parse_words_sidecar, search_key, search_variants, strip_marks, Form,
    Match, SearchMode, SearchOptions, WordForms,
};
pub use view::{swipe_direction, swipe_pages, View};
pub use zoom::{Sideways, Zoom, ZoomChange, ZoomMode};

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
pub struct HitExact {
    pub word: u32,
    pub path: u32,
    pub decoration: u32,
}

/// The traced silhouettes of a page's words and decorations. See [`Page::silhouettes`].
struct Silhouettes {
    words: Vec<Vec<(i16, f32, f32)>>,
    decorations: Vec<Vec<(i16, f32, f32)>>,
}

pub struct Page {
    data: PageData,
    geom: Geometry,
    line_centre: Vec<f32>,
    line_spacing: f32,
    layout: Option<Layout>,
    /// words of each line sorted by bbox.x0 ascending: (x0, word idx)
    line_words: Vec<Vec<(i32, u32)>>,
    /// horizontal extent of each word's letters, page units, without the marks drawn over and
    /// under them
    word_body: Vec<(f32, f32)>,
    /// the baseline of each printed line: the height most of its words sit on
    line_baseline: Vec<f32>,
    /// Each word's letters, and each decoration, traced into horizontal bands as
    /// (band, leftmost x, rightmost x). Bands are counted from the line's baseline, so two
    /// words off different lines can still be measured against each other once a row puts them
    /// on one baseline.
    ///
    /// Tracing every outline costs more than the rest of loading a page, and only a reflow
    /// needs it, so it waits until something asks.
    silhouettes: std::cell::OnceCell<Silhouettes>,
    /// The searched zoom steps of a page the shipped table does not cover, kept so the search
    /// runs at most once a page. See [`Page::zoom_steps`].
    pub(crate) searched_steps: Option<Vec<f32>>,
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
        for (di, d) in data.decorations.iter().enumerate() {
            for p in d.first_path..d.first_path + d.n_paths as u32 {
                path_deco[p as usize] = di as u32;
            }
        }
        // Line reference centres come from body ink. Diacritics would wobble them. A sajdah
        // sign or a rosette on the line is taller than the ink and would pull the centre off
        // the text. A header line (surah name, basmalah) has no words, so it uses its
        // decoration.
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
            if bb.is_empty() {
                for d in data
                    .decorations
                    .iter()
                    .filter(|d| d.line as usize == li && matches!(d.kind, DecoKind::SurahName | DecoKind::Basmalah))
                {
                    for pi in d.first_path..d.first_path + d.n_paths as u32 {
                        let path = &data.paths[pi as usize];
                        if path.kind == PathKind::HeaderInk {
                            bb.union(&path.bbox);
                        }
                    }
                }
                // Old page data can carry an unclassified header path. Keep its established
                // centre rather than turning the line into an empty one.
                if bb.is_empty() {
                    for d in data
                        .decorations
                        .iter()
                        .filter(|d| d.line as usize == li && matches!(d.kind, DecoKind::SurahName | DecoKind::Basmalah))
                    {
                        bb.union(&d.bbox);
                    }
                }
            }
            if bb.is_empty() {
                bb = l.bbox;
            }
            line_centre[li] = if bb.is_empty() { 0.0 } else { (bb.y0 + bb.y1) as f32 / 2.0 / q };
        }
        let line_spacing = {
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
        // An ayah mark is stored without a line: the exporter keeps the marks outside the
        // line groups. It takes the line of the last word of the ayah it closes, so it
        // moves with that line under layout. Anything else without a line snaps to the
        // nearest line centre.
        let mut deco_line = vec![NONE; data.decorations.len()];
        for a in data.ayahs.iter().filter(|a| a.fragment == a.fragments && a.n_words > 0) {
            let di = a.ayah_mark_decoration as usize;
            if di < data.decorations.len() {
                deco_line[di] = data.words[(a.first_word + a.n_words - 1) as usize].line_index as u32;
            }
        }
        // The sajdah line is drawn over the sajdah word, which can sit lines above the
        // sign that closes the ayah: it takes the line of the word directly below it.
        let line_under = |pb: &IBox| -> Option<u32> {
            let tol = (2.0 * q) as i32;
            data.words
                .iter()
                .filter(|w| w.bbox.x0 < pb.x1 && w.bbox.x1 > pb.x0 && w.bbox.y0 >= pb.y0 - tol)
                .min_by_key(|w| w.bbox.y0 - pb.y1)
                .map(|w| w.line_index as u32)
        };
        let mut path_line = vec![0u32; data.paths.len()];
        for i in 0..data.paths.len() {
            let wi = path_word[i];
            path_line[i] = if wi != NONE {
                data.words[wi as usize].line_index as u32
            } else if path_deco[i] != NONE {
                let d = &data.decorations[path_deco[i] as usize];
                if data.paths[i].mark == Mark::SajdahLine && line_under(&data.paths[i].bbox).is_some() {
                    line_under(&data.paths[i].bbox).unwrap()
                } else if d.line != NONE_U16 && (d.line as usize) < data.lines.len() {
                    d.line as u32
                } else if deco_line[path_deco[i] as usize] != NONE {
                    deco_line[path_deco[i] as usize]
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
            let (surah, ayah, line_number) = if wi != NONE {
                let w = &data.words[wi as usize];
                (w.surah, w.ayah, data.lines[w.line_index as usize].line_number)
            } else if di != NONE {
                let d = &data.decorations[di as usize];
                (d.surah, d.ayah, data.lines[path_line[i] as usize].line_number)
            } else {
                (0, 0, 0)
            };
            let nm = *named.entry(p.mark).and_modify(|v| *v += 1).or_insert(0);
            path_ctx.push(PathCtx {
                word: wi,
                decoration: di,
                line_number,
                surah,
                ayah,
                kind: p.kind,
                mark: p.mark,
                family: p.family,
                category: p.mark.category(),
                nth_in_word,
                nth_mark: if p.kind == PathKind::Mark { nth_mark } else { u16::MAX },
                nth_mark_named: nm,
                deco_kind: if di != NONE { data.decorations[di as usize].kind } else { DecoKind::Other },
            });
            if wi != NONE {
                nth_in_word += 1;
                if p.kind == PathKind::Mark {
                    nth_mark += 1;
                }
            }
        }
        // A page's viewBox is what the page is. The artwork may carry ink beyond it — p17 of the
        // Hafs KFGQPC mushaf draws its page number and its running head outside the box, and it is
        // the only page that does — and the format keeps that ink rather than drop it, so a crop
        // or an SVG export still has every stroke the artwork drew. Drawing is the other question:
        // an SVG viewport clips to the viewBox, and ink outside it is not part of the page, so the
        // geometry gives that ink no outline to draw. Its record stays, and so does its box, its
        // kind and its place in the path numbering every other call is indexed by.
        let on_page = |bb: &IBox| -> bool {
            let (w, h) = (data.header.width, data.header.height);
            (bb.x1 as f32 / q) > 0.0 && (bb.y1 as f32 / q) > 0.0 && (bb.x0 as f32 / q) < w && (bb.y0 as f32 / q) < h
        };
        for (i, p) in data.paths.iter().enumerate() {
            let op_start = geom.ops.len() as u32;
            let pt_start = geom.pts.len() as u32;
            let mut push = |x: f32, y: f32| {
                geom.pts.push(x);
                geom.pts.push(y);
            };
            if !on_page(&p.bbox) {
                // no outline: every host draws it as nothing, without knowing to ask
            } else if let Some(inst) = data.path_inst(i) {
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
        // a word's letters, without its marks: 2 words in 3 carry ink outside them
        let word_body: Vec<(f32, f32)> = data
            .words
            .iter()
            .map(|w| {
                let (mut x0, mut x1) = (f32::INFINITY, f32::NEG_INFINITY);
                for pi in w.first_path..w.first_path + w.n_paths as u32 {
                    if data.paths[pi as usize].kind != PathKind::Body {
                        continue;
                    }
                    let b = &data.paths[pi as usize].bbox;
                    x0 = x0.min(b.x0 as f32 / q);
                    x1 = x1.max(b.x1 as f32 / q);
                }
                if x0 < x1 {
                    (x0, x1)
                } else {
                    (w.bbox.x0 as f32 / q, w.bbox.x1 as f32 / q)
                }
            })
            .collect();
        // the baseline of a line is the height most of its words end on
        let line_baseline: Vec<f32> = (0..data.lines.len())
            .map(|li| {
                let mut bottoms: Vec<f32> =
                    line_words[li].iter().map(|&(_, wi)| data.words[wi as usize].bbox.y1 as f32 / q).collect();
                if bottoms.is_empty() {
                    return line_centre[li];
                }
                bottoms.sort_by(|a, b| a.partial_cmp(b).unwrap());
                bottoms[bottoms.len() / 2]
            })
            .collect();
        // Two words are spaced by the air between their letters, not by the distance between
        // their boxes: the calligraphy interlocks one word's stroke with the next one's without
        // the ink ever meeting. Slicing each word into bands and recording where its ink starts
        // and ends in each is what makes that air measurable.
        let word_index = data.words.iter().enumerate().map(|(i, w)| ((w.surah, w.ayah, w.word), i as u32)).collect();
        let n = data.paths.len();
        Page {
            data,
            geom,
            line_centre,
            line_spacing,
            layout: None,
            line_words,
            word_body,
            line_baseline,
            silhouettes: std::cell::OnceCell::new(),
            searched_steps: None,
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
    /// The horizontal extent of a word's letters in page units, without the marks drawn over
    /// and under them: what the eye reads as the distance between two words.
    pub fn word_body(&self, wi: u32) -> (f32, f32) {
        self.word_body[wi as usize]
    }

    /// Trace a set of paths into bands: where the ink starts and ends at each height.
    // The silhouette of a set of paths, band by band: where its ink starts and ends at
    // each height. Walking the outline and not its points is the whole of it — a curve's
    // control points are far apart, so a band between two of them would read as empty and
    // two words would be measured against ink that is not facing them.
    fn trace(&self, first_path: u32, n_paths: u32, base: f32, body_only: bool) -> Vec<(i16, f32, f32)> {
        let (data, geom) = (&self.data, &self.geom);
        let band = self.line_spacing / defaults::SLICES_PER_LINE as f32;
        let mut rows: std::collections::BTreeMap<i16, (f32, f32)> = std::collections::BTreeMap::new();
        let mark = |y: f32, x: f32, rows: &mut std::collections::BTreeMap<i16, (f32, f32)>| {
            let r = (((y - base) / band).floor() as i32).clamp(i16::MIN as i32, i16::MAX as i32) as i16;
            let e = rows.entry(r).or_insert((f32::INFINITY, f32::NEG_INFINITY));
            e.0 = e.0.min(x);
            e.1 = e.1.max(x);
        };
        // a straight run of the outline, marked into every band it crosses
        let segment = |a: (f32, f32), b: (f32, f32), rows: &mut std::collections::BTreeMap<i16, (f32, f32)>| {
            let steps = (((b.1 - a.1).abs() / band).ceil() as u32).clamp(1, 512);
            for i in 0..=steps {
                let t = i as f32 / steps as f32;
                mark(a.1 + (b.1 - a.1) * t, a.0 + (b.0 - a.0) * t, rows);
            }
        };
        for pi in first_path..first_path + n_paths {
            if body_only && data.paths[pi as usize].kind != PathKind::Body {
                continue;
            }
            let g = &geom.table[pi as usize];
            let (ops, pts) = (&geom.ops, &geom.pts);
            let (mut k, mut here, mut start) = (g.pt_start as usize, (0.0f32, 0.0f32), (0.0f32, 0.0f32));
            let at = |k: usize| (pts[k], pts[k + 1]);
            for oi in g.op_start..g.op_start + g.op_count {
                match ops[oi as usize] {
                    OP_MOVE => {
                        here = at(k);
                        start = here;
                        mark(here.1, here.0, &mut rows);
                        k += 2;
                    }
                    OP_LINE => {
                        let to = at(k);
                        segment(here, to, &mut rows);
                        here = to;
                        k += 2;
                    }
                    // a curve is walked as a run of short straight pieces: close enough to
                    // its outline for a band to know where the ink is
                    OP_QUAD => {
                        let (c, to) = (at(k), at(k + 2));
                        let mut prev = here;
                        for i in 1..=defaults::CURVE_STEPS {
                            let t = i as f32 / defaults::CURVE_STEPS as f32;
                            let (u, tt) = (1.0 - t, t);
                            let p = (
                                u * u * here.0 + 2.0 * u * tt * c.0 + tt * tt * to.0,
                                u * u * here.1 + 2.0 * u * tt * c.1 + tt * tt * to.1,
                            );
                            segment(prev, p, &mut rows);
                            prev = p;
                        }
                        here = to;
                        k += 4;
                    }
                    OP_CUBIC => {
                        let (c1, c2, to) = (at(k), at(k + 2), at(k + 4));
                        let mut prev = here;
                        for i in 1..=defaults::CURVE_STEPS {
                            let t = i as f32 / defaults::CURVE_STEPS as f32;
                            let u = 1.0 - t;
                            let p = (
                                u * u * u * here.0 + 3.0 * u * u * t * c1.0 + 3.0 * u * t * t * c2.0 + t * t * t * to.0,
                                u * u * u * here.1 + 3.0 * u * u * t * c1.1 + 3.0 * u * t * t * c2.1 + t * t * t * to.1,
                            );
                            segment(prev, p, &mut rows);
                            prev = p;
                        }
                        here = to;
                        k += 6;
                    }
                    OP_CLOSE => {
                        segment(here, start, &mut rows);
                        here = start;
                    }
                    _ => {}
                }
            }
        }
        rows.into_iter().map(|(r, (x0, x1))| (r, x0, x1)).collect()
    }

    /// The page's traced outlines, built the first time something measures the air between two
    /// words and kept from then on.
    fn silhouettes(&self) -> &Silhouettes {
        self.silhouettes.get_or_init(|| Silhouettes {
            words: self
                .data
                .words
                .iter()
                // marks included: a mark reaching past its letters still has to clear the next
                // word, and a band knows the height it reaches at
                .map(|w| self.trace(w.first_path, w.n_paths as u32, self.line_baseline[w.line_index as usize], false))
                .collect(),
            decorations: self
                .data
                .decorations
                .iter()
                .map(|deco| {
                    let li = self.geom.table[deco.first_path as usize].line as usize;
                    let base = self.line_baseline.get(li).copied().unwrap_or(0.0);
                    self.trace(deco.first_path, deco.n_paths as u32, base, false)
                })
                .collect(),
        })
    }

    /// The air between the letters of two words placed side by side, in page units: the
    /// narrowest distance between them over the bands they share, with `b` shifted by `dx`.
    ///
    /// This is what the eye reads as the space between two words, and what the calligraphy
    /// keeps even. Their boxes say something else: an initial `ك` reaches its arm back over a
    /// preceding `ر` so the two boxes overlap by 12 page units while the strokes stay 5 apart.
    /// `None` when the two share no band, which is the case for a mark set above the line.
    pub fn words_clearance(&self, a: u32, b: u32, dx: f32) -> Option<f32> {
        let s = self.silhouettes();
        Self::slice_clearance(&s.words[a as usize], &s.words[b as usize], dx)
    }

    /// The bands of a word's own letters and marks.
    pub(crate) fn word_slices(&self, word: u32) -> &[(i16, f32, f32)] {
        &self.silhouettes().words[word as usize]
    }
    /// The bands of a decoration's ink.
    pub(crate) fn deco_slices(&self, deco: u32) -> &[(i16, f32, f32)] {
        &self.silhouettes().decorations[deco as usize]
    }

    /// The bands of a word together with the marks set inline with it, so a medallion standing
    /// between two words is measured with the word it closes.
    pub(crate) fn slices_with(&self, word: u32, decos: &[u32]) -> Vec<(i16, f32, f32)> {
        let s = self.silhouettes();
        let mut out = s.words[word as usize].clone();
        for &di in decos {
            for &(r, x0, x1) in &s.decorations[di as usize] {
                match out.binary_search_by_key(&r, |e| e.0) {
                    Ok(i) => {
                        out[i].1 = out[i].1.min(x0);
                        out[i].2 = out[i].2.max(x1);
                    }
                    Err(i) => out.insert(i, (r, x0, x1)),
                }
            }
        }
        out
    }

    /// The air between two sets of bands, with the second shifted by `dx`.
    pub(crate) fn slice_clearance(sa: &[(i16, f32, f32)], sb: &[(i16, f32, f32)], dx: f32) -> Option<f32> {
        let mut air = f32::INFINITY;
        let (mut i, mut j) = (0usize, 0usize);
        while i < sa.len() && j < sb.len() {
            match sa[i].0.cmp(&sb[j].0) {
                std::cmp::Ordering::Less => i += 1,
                std::cmp::Ordering::Greater => j += 1,
                std::cmp::Ordering::Equal => {
                    // `a` is the right-hand word: the air is from its leftmost ink in this band
                    // to `b`'s rightmost
                    air = air.min(sa[i].1 - (sb[j].2 + dx));
                    i += 1;
                    j += 1;
                }
            }
        }
        air.is_finite().then_some(air)
    }

    /// The shift that leaves `air` between two words' letters, as an offset applied to `b`.
    pub fn shift_for_clearance(&self, a: u32, b: u32, air: f32) -> Option<f32> {
        self.words_clearance(a, b, 0.0).map(|now| now - air)
    }

    /// How deeply two words' strokes overlap, when the print draws them as one piece of
    /// calligraphy. `None` for every other pair.
    ///
    /// Words interlock all over the mushaf without their strokes ever meeting: an initial `ك`
    /// reaches its arm back over a preceding `ر` so the two boxes overlap by 12 page units
    /// while the strokes stay 5 apart. That is kerning, and it survives being respaced, which
    /// is what [`Page::words_clearance`] measures. What does not survive is one word drawn
    /// inside another, where `ٱلرَّحِيمِ` sits in the bowl of `ٱلرَّحْمَٰنِ`. Strokes meet in 10 of
    /// this mushaf's 68,612 neighbouring pairs, and those three overlap by 19 to 26 page units
    /// where the next deepest reaches 3.8, so the two are far apart in the data.
    pub fn words_interlock(&self, a: u32, b: u32) -> Option<f32> {
        let d = self.data();
        let (wa, wb) = (&d.words[a as usize], &d.words[b as usize]);
        if wa.line_index != wb.line_index || b != a + 1 {
            return None;
        }
        let overlap = -self.words_clearance(a, b, 0.0)?;
        (overlap >= self.line_spacing * defaults::INTERLOCK_DEPTH).then_some(overlap)
    }
    pub fn word_text(&self, wi: u32) -> &str {
        self.word_form(wi, Form::RasmUthmani)
    }
    pub fn deco_text(&self, di: u32) -> &str {
        let d = &self.data.decorations[di as usize];
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
        self.target_words(&Target::Ayah(surah, ayah))
    }
    pub fn path_deco(&self, pi: u32) -> u32 {
        self.path_deco[pi as usize]
    }
    pub fn clock(&self) -> f64 {
        self.clock_ms
    }

    // ───────────── exact hit testing ─────────────

    /// Exact hit-test in page units (bbox → outline). Words first, then decorations.
    pub fn hit_test_exact(&self, x: f32, y: f32) -> Option<HitExact> {
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
                    return Some(HitExact { word: wi, path: pi, decoration: NONE });
                }
            }
        }
        if let Some(wi) = bbox_only {
            return Some(HitExact { word: wi, path: NONE, decoration: NONE });
        }
        for di in 0..self.data.decorations.len() {
            if let Some(hit) = self.exact_decoration_hit(di, x, y, &[]) {
                return Some(hit);
            }
        }
        None
    }

    /// Exact hit-test in viewport px using the current layout.
    pub fn hit_test_exact_view(&self, vx: f32, vy: f32) -> Option<HitExact> {
        let Some(l) = &self.layout else { return self.hit_test_exact(vx, vy) };
        let x = (vx - l.offset_x) / l.scale;
        let y = (vy - l.offset_y) / l.scale;
        let q = self.quant();
        // reflowed: every word carries its own placement, so undo it word by word
        if let Some(flow) = &l.reflow {
            if let Some(r) = flow.row_band.iter().position(|(t, b)| y >= *t && y <= *b) {
                for &wi in &flow.row_words[r] {
                    let w = &self.data.words[wi as usize];
                    let (px, py) = flow.word_place[wi as usize].invert(x, y);
                    let (qx, qy) = ((px * q).round() as i32, (py * q).round() as i32);
                    if !w.bbox.contains(qx, qy) {
                        continue;
                    }
                    let pi = self.exact_path_hit(w.first_path, w.n_paths as u32, px, py).unwrap_or(NONE);
                    return Some(HitExact { word: wi, path: pi, decoration: NONE });
                }
            }
            for di in 0..self.data.decorations.len() {
                let (px, py) = flow.deco_place[di].invert(x, y);
                if let Some(hit) = self.exact_decoration_hit(di, px, py, &l.omitted_paths) {
                    return Some(hit);
                }
            }
            return None;
        }
        for (li, line) in self.data.lines.iter().enumerate() {
            let py = y - l.line_dy[li];
            let (y0, y1) = (line.bbox.y0 as f32 / q, line.bbox.y1 as f32 / q);
            if py < y0 - 0.5 || py > y1 + 0.5 {
                continue;
            }
            if let Some(h) = self.hit_test_in_line(li, x, py, &l.omitted_paths) {
                return Some(h);
            }
        }
        for (di, d) in self.data.decorations.iter().enumerate() {
            let li = self.geom.table[d.first_path as usize].line as usize;
            let py = y - l.line_dy[li];
            if let Some(hit) = self.exact_decoration_hit(di, x, py, &l.omitted_paths) {
                return Some(hit);
            }
        }
        None
    }

    fn hit_test_in_line(&self, li: usize, x: f32, y: f32, omitted: &[u32]) -> Option<HitExact> {
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
                return Some(HitExact { word: wi, path: pi, decoration: NONE });
            }
        }
        if let Some(wi) = bbox_only {
            return Some(HitExact { word: wi, path: NONE, decoration: NONE });
        }
        for (di, d) in self.data.decorations.iter().enumerate() {
            if self.geom.table[d.first_path as usize].line as usize != li {
                continue;
            }
            if let Some(hit) = self.exact_decoration_hit(di, x, y, omitted) {
                return Some(hit);
            }
        }
        None
    }

    fn exact_decoration_hit(&self, di: usize, x: f32, y: f32, omitted: &[u32]) -> Option<HitExact> {
        let q = self.quant();
        let (qx, qy) = ((x * q).round() as i32, (y * q).round() as i32);
        let decoration = &self.data.decorations[di];
        if !self.decoration_bbox_without(di, omitted).contains(qx, qy) {
            return None;
        }
        if let Some(path) = self.exact_path_hit_without(decoration.first_path, decoration.n_paths as u32, x, y, omitted)
        {
            return Some(HitExact { word: NONE, path, decoration: di as u32 });
        }
        if decoration.kind != DecoKind::SurahName {
            return Some(HitExact { word: NONE, path: NONE, decoration: di as u32 });
        }
        // A viewport transform can move a point by a few f32 ulps across a frame edge. Retry its
        // ornament on the exact format grid, but never use the large hollow frame bbox as a hit.
        let (grid_x, grid_y) = (qx as f32 / q, qy as f32 / q);
        let mut title_bbox = IBox::EMPTY;
        for path in decoration.first_path..decoration.first_path + decoration.n_paths as u32 {
            if omitted.binary_search(&path).is_ok() {
                continue;
            }
            let record = &self.data.paths[path as usize];
            if record.kind == PathKind::Ornament {
                if record.bbox.contains(qx, qy) && self.point_in_path(path, grid_x, grid_y) {
                    return Some(HitExact { word: NONE, path, decoration: di as u32 });
                }
            } else {
                title_bbox.union(&record.bbox);
            }
        }
        title_bbox.contains(qx, qy).then_some(HitExact { word: NONE, path: NONE, decoration: di as u32 })
    }

    fn exact_path_hit(&self, first: u32, n: u32, x: f32, y: f32) -> Option<u32> {
        self.exact_path_hit_without(first, n, x, y, &[])
    }

    fn exact_path_hit_without(&self, first: u32, n: u32, x: f32, y: f32, omitted: &[u32]) -> Option<u32> {
        let q = self.quant();
        let (qx, qy) = ((x * q).round() as i32, (y * q).round() as i32);
        for pass in 0..2 {
            for pi in first..first + n {
                if omitted.binary_search(&pi).is_ok() {
                    continue;
                }
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
        self.colors();
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
