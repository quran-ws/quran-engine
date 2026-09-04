//! QVP engine core: page loader, flattened geometry, hit-testing, style state
//! and display list. Renderer-agnostic — a host canvas (or a bundled
//! rasteriser) implements [`Renderer`].
#![forbid(unsafe_code)]

use qvp_format::*;
use std::collections::HashMap;

pub use qvp_format;

/// Colour as 0xRRGGBBAA. Alpha 0 hides the path.
pub type Rgba = u32;
pub const DEFAULT_INK: Rgba = 0x231f20ff;
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
}

/// Flattened page geometry in page units (f32), ready for any canvas.
/// `ops` uses the same opcodes as the file (OP_MOVE..OP_CLOSE); `pts` holds
/// x,y pairs consumed in order (1 for move/line, 2 for quad, 3 for cubic).
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Selector {
    Path(u32),
    Word(u16, u16, u16),
    Ayah(u16, u16),
    Line(u8),
    Mark(Mark),
    Family(Family),
    Kind(PathKind),
    Deco(DecoKind),
}

/// Style state. Resolution precedence:
/// path > word > ayah > line > mark > family > kind > deco kind > default.
#[derive(Debug, Clone)]
pub struct StyleState {
    pub default_ink: Rgba,
    map: HashMap<Selector, Rgba>,
}

impl Default for StyleState {
    fn default() -> Self {
        StyleState { default_ink: DEFAULT_INK, map: HashMap::new() }
    }
}

impl StyleState {
    pub fn set(&mut self, s: Selector, c: Rgba) {
        self.map.insert(s, c);
    }
    pub fn unset(&mut self, s: Selector) {
        self.map.remove(&s);
    }
    pub fn clear(&mut self) {
        self.map.clear();
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

pub struct Page {
    data: PageData,
    geom: Geometry,
    /// words of each line sorted by bbox.x0 ascending: (x0, word idx)
    line_words: Vec<Vec<(i32, u32)>>,
    /// word index → deco owner mapping for paths
    path_deco: Vec<u32>,
    word_index: HashMap<(u16, u16, u16), u32>,
    pub style: StyleState,
    /// scratch: per-path colours from the last paint()
    colors: Vec<Rgba>,
}

impl Page {
    pub fn load(bytes: &[u8]) -> Result<Page, Error> {
        let data = decode(bytes)?;
        Ok(Page::from_data(data))
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
        for (i, p) in data.paths.iter().enumerate() {
            let op_start = geom.ops.len() as u32;
            let pt_start = geom.pts.len() as u32;
            let mut push = |x: f32, y: f32| {
                geom.pts.push(x);
                geom.pts.push(y);
            };
            if let Some(inst) = data.path_inst(i) {
                // exact: transform glyph units in f64, no re-quantisation
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
            geom.table.push(PathGeom {
                op_start,
                op_count: geom.ops.len() as u32 - op_start,
                pt_start,
                pt_count: geom.pts.len() as u32 - pt_start,
                flags: p.kind as u32 | (p.mark as u32) << 8 | (p.family as u32) << 16 | (evenodd | inst << 1) << 24,
                word: path_word[i],
            });
        }
        let mut line_words = Vec::with_capacity(data.lines.len());
        for l in &data.lines {
            let mut v: Vec<(i32, u32)> =
                (l.first_word..l.first_word + l.n_words).map(|wi| (data.words[wi as usize].bbox.x0, wi as u32)).collect();
            v.sort_unstable();
            line_words.push(v);
        }
        let word_index = data.words.iter().enumerate().map(|(i, w)| ((w.sura, w.ayah, w.word), i as u32)).collect();
        let n = data.paths.len();
        Page { data, geom, line_words, path_deco, word_index, style: StyleState::default(), colors: vec![DEFAULT_INK; n] }
    }

    pub fn data(&self) -> &PageData {
        &self.data
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
    pub fn word_text(&self, wi: u32) -> &str {
        let w = &self.data.words[wi as usize];
        if w.text == NONE_U16 {
            ""
        } else {
            &self.data.strings[w.text as usize]
        }
    }
    pub fn deco_text(&self, di: u32) -> &str {
        let d = &self.data.decos[di as usize];
        if d.text == NONE_U16 {
            ""
        } else {
            &self.data.strings[d.text as usize]
        }
    }
    pub fn find_word(&self, sura: u16, ayah: u16, word: u16) -> Option<u32> {
        self.word_index.get(&(sura, ayah, word)).copied()
    }
    /// All word indices of an ayah on this page, in reading order.
    pub fn ayah_words(&self, sura: u16, ayah: u16) -> Vec<u32> {
        let mut v = Vec::new();
        for a in &self.data.ayahs {
            if a.sura == sura && a.ayah == ayah {
                v.extend(a.first_word as u32..(a.first_word + a.n_words) as u32);
            }
        }
        v
    }

    // ───────────── hit testing ─────────────

    /// Hit-test a point in page units. Words first (bbox → exact outline),
    /// then decorations (bbox).
    pub fn hit_test(&self, x: f32, y: f32) -> Option<Hit> {
        let q = self.quant();
        let (qx, qy) = ((x * q).round() as i32, (y * q).round() as i32);
        let mut bbox_only: Option<u32> = None;
        for (li, l) in self.data.lines.iter().enumerate() {
            if !l.bbox.contains(qx, qy) {
                continue;
            }
            let ws = &self.line_words[li];
            // last word with x0 <= qx, then check neighbours (overhanging marks overlap)
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

    fn exact_path_hit(&self, first: u32, n: u32, x: f32, y: f32) -> Option<u32> {
        let q = self.quant();
        let (qx, qy) = ((x * q).round() as i32, (y * q).round() as i32);
        // bodies first so a diacritic hovering over a letter reports the letter's word/path sensibly
        for pass in 0..2 {
            for pi in first..first + n {
                let p = &self.data.paths[pi as usize];
                let is_body = matches!(p.kind, PathKind::Body | PathKind::HeaderInk | PathKind::AyahOrnament | PathKind::AyahNumber);
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

    // ───────────── style / display list ─────────────

    pub fn color_of(&self, pi: u32) -> Rgba {
        let p = &self.data.paths[pi as usize];
        let st = &self.style;
        if st.is_empty() {
            return st.default_ink;
        }
        let m = &st.map;
        if let Some(&c) = m.get(&Selector::Path(pi)) {
            return c;
        }
        let wi = self.geom.table[pi as usize].word;
        if wi != NONE {
            let w = &self.data.words[wi as usize];
            if let Some(&c) = m.get(&Selector::Word(w.sura, w.ayah, w.word)) {
                return c;
            }
            if let Some(&c) = m.get(&Selector::Ayah(w.sura, w.ayah)) {
                return c;
            }
            if let Some(&c) = m.get(&Selector::Line(self.data.lines[w.line_idx as usize].line_no)) {
                return c;
            }
        } else {
            let di = self.path_deco[pi as usize];
            if di != NONE {
                let d = &self.data.decos[di as usize];
                if d.ayah != 0 {
                    if let Some(&c) = m.get(&Selector::Ayah(d.sura, d.ayah)) {
                        return c;
                    }
                }
            }
        }
        if p.mark != Mark::None {
            if let Some(&c) = m.get(&Selector::Mark(p.mark)) {
                return c;
            }
        }
        if p.family != Family::None {
            if let Some(&c) = m.get(&Selector::Family(p.family)) {
                return c;
            }
        }
        if let Some(&c) = m.get(&Selector::Kind(p.kind)) {
            return c;
        }
        let di = self.path_deco[pi as usize];
        if di != NONE {
            if let Some(&c) = m.get(&Selector::Deco(self.data.decos[di as usize].kind)) {
                return c;
            }
        }
        st.default_ink
    }

    /// Resolve every path's colour (full repaint display list).
    pub fn paint(&mut self) -> &[Rgba] {
        for i in 0..self.data.paths.len() {
            self.colors[i] = self.color_of(i as u32);
        }
        &self.colors
    }

    /// Paths whose colour differs from the default ink (overlay repaint).
    pub fn styled(&self) -> Vec<(u32, Rgba)> {
        if self.style.is_empty() {
            return Vec::new();
        }
        (0..self.data.paths.len() as u32)
            .filter_map(|i| {
                let c = self.color_of(i);
                (c != self.style.default_ink).then_some((i, c))
            })
            .collect()
    }

    /// Drive a renderer over the whole page.
    pub fn render<R: Renderer>(&mut self, r: &mut R) {
        r.begin(self.width(), self.height());
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
    fn end(&mut self);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<Cmd> {
        vec![Cmd::MoveTo(x0, y0), Cmd::LineTo(x1, y0), Cmd::LineTo(x1, y1), Cmd::LineTo(x0, y1), Cmd::Close]
    }

    /// Two words on one line: A = [10,20]×[10,20] units, B = [30,40]×[10,20]; a mark above B.
    fn page() -> Page {
        let mut ops = Vec::new();
        let mut paths = Vec::new();
        let mut add = |cmds: &[Cmd], kind: PathKind, mark: Mark, fam: Family| {
            let bb = cmds_bbox(cmds);
            let off = ops.len() as u32;
            encode_cmds(cmds, bb.x0, bb.y0, &mut ops);
            paths.push(PathRec { kind, mark, family: fam, flags: PF_EVENODD, ox: bb.x0, oy: bb.y0, op_off: off, op_len: ops.len() as u32 - off, bbox: bb });
            bb
        };
        let a = add(&square(1000, 1000, 2000, 2000), PathKind::Body, Mark::None, Family::None);
        let mut b = add(&square(3000, 1000, 4000, 2000), PathKind::Body, Mark::None, Family::None);
        b.union(&add(&square(3200, 500, 3400, 700), PathKind::Mark, Mark::Fatha, Family::Diacritic));
        let mut lb = a;
        lb.union(&b);
        let data = PageData {
            header: Header { version: VERSION, quant: 100, page: 1, flags: 0, width: 100.0, height: 100.0 },
            lines: vec![LineRec { line_no: 1, first_word: 0, n_words: 2, bbox: lb }],
            ayahs: vec![AyahRec { sura: 1, ayah: 1, part: 1, parts: 1, flags: 0, first_word: 0, n_words: 2, marker_deco: NONE_U16, bbox: lb }],
            words: vec![
                WordRec { sura: 1, ayah: 1, word: 1, line_idx: 0, ayah_idx: 0, text: 0, first_path: 0, n_paths: 1, bbox: a },
                WordRec { sura: 1, ayah: 1, word: 2, line_idx: 0, ayah_idx: 0, text: 1, first_path: 1, n_paths: 2, bbox: b },
            ],
            paths,
            decos: vec![],
            glyphs: vec![],
            insts: vec![],
            ops,
            strings: vec!["أ".into(), "ب".into()],
        };
        Page::load(&encode(&data)).unwrap()
    }

    #[test]
    fn hit_testing() {
        let p = page();
        assert_eq!(p.hit_test(15.0, 15.0), Some(Hit { word: 0, path: 0, deco: NONE }));
        assert_eq!(p.hit_test(35.0, 15.0), Some(Hit { word: 1, path: 1, deco: NONE }));
        // on the mark above B
        assert_eq!(p.hit_test(33.0, 6.0), Some(Hit { word: 1, path: 2, deco: NONE }));
        // inside B's bbox but in the gap between mark and body → bbox-only hit
        assert_eq!(p.hit_test(33.0, 8.5), Some(Hit { word: 1, path: NONE, deco: NONE }));
        assert_eq!(p.hit_test(25.0, 15.0), None);
        assert_eq!(p.hit_test(50.0, 50.0), None);
        assert_eq!(p.word_text(1), "ب");
        assert_eq!(p.find_word(1, 1, 2), Some(1));
    }

    #[test]
    fn style_precedence_and_overlay() {
        let mut p = page();
        assert!(p.styled().is_empty());
        p.style.set(Selector::Ayah(1, 1), 0x0000ffff);
        p.style.set(Selector::Word(1, 1, 2), 0x00ff00ff);
        p.style.set(Selector::Family(Family::Diacritic), 0xff0000ff);
        assert_eq!(p.color_of(0), 0x0000ffff, "ayah applies to word 1");
        assert_eq!(p.color_of(1), 0x00ff00ff, "word beats ayah");
        assert_eq!(p.color_of(2), 0x00ff00ff, "word beats family for its mark");
        p.style.unset(Selector::Word(1, 1, 2));
        assert_eq!(p.color_of(2), 0x0000ffff, "ayah beats family");
        p.style.unset(Selector::Ayah(1, 1));
        assert_eq!(p.color_of(2), 0xff0000ff, "family applies to the mark only");
        assert_eq!(p.styled(), vec![(2, 0xff0000ff)]);
        p.style.set(Selector::Path(2), 0);
        assert_eq!(p.color_of(2) & 0xff, 0, "alpha 0 hides");
        struct Count(u32, u32);
        impl Renderer for Count {
            fn begin(&mut self, _: f32, _: f32) {}
            fn fill_path(&mut self, _: u32, _: &[u8], pts: &[f32], _: Rgba, _: bool) {
                self.0 += 1;
                self.1 += pts.len() as u32;
            }
            fn end(&mut self) {}
        }
        let mut c = Count(0, 0);
        p.render(&mut c);
        assert_eq!((c.0, c.1), (2, 16), "hidden path skipped; 4 points × 2 coords per square");
    }
}
