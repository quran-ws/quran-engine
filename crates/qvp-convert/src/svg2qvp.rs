//! SVG → QVP. Walks the exporter's structure
//! (`line > ayah > word > path`, plus `ayah_markers`, `surah-name`, `basmalah`,
//! `hizb-mark`, `sajdah-mark`), flattens every transform into page space,
//! quantises and builds the tables. No data repair: quirks are reported.

use crate::affine::Affine;
use qvp_format::*;
use roxmltree::Node;
use std::collections::HashMap;

#[derive(Default)]
pub struct Report {
    pub warnings: Vec<String>,
}

/// Side text index: wid → other text forms.
#[derive(Debug, Default)]
pub struct WordText {
    pub wid: String,
    pub uthmani: String,
    pub imlaei: String,
    pub qpc: String,
    pub rasm: String,
    pub search: String,
}

pub struct Converted {
    pub page: PageData,
    pub words_text: Vec<WordText>,
    pub report: Report,
}

struct Ctx<'a> {
    quant: f64,
    page_tf: Affine, // maps SVG user space → page space (origin at viewBox min)
    page: PageData,
    words_text: Vec<WordText>,
    report: Report,
    marker_ids: HashMap<String, u16>,
    strings: HashMap<String, u16>,
    /// `d` strings that occur more than once on the page → glyph index (lazily assigned).
    shared_d: HashMap<&'a str, Option<u16>>,
    _p: std::marker::PhantomData<&'a ()>,
}

/// Parsed but not yet encoded path.
struct RawPath {
    kind: PathKind,
    mark: Mark,
    family: Family,
    flags: u8,
    /// Inline geometry in page space, or a glyph instance.
    geom: Geom,
}

enum Geom {
    Inline(Vec<Cmd>),
    Instance { inst: InstRec, cmds_page: Vec<Cmd> },
}

pub fn convert(svg: &str) -> Result<Converted, String> {
    let doc = roxmltree::Document::parse(svg).map_err(|e| format!("xml: {e}"))?;
    let root = doc.root_element();
    // pre-scan: outlines used more than once become shared glyphs
    let mut d_count: HashMap<&str, u32> = HashMap::new();
    for n in root.descendants().filter(|n| n.has_tag_name("path")) {
        if let Some(d) = n.attribute("d") {
            *d_count.entry(d).or_insert(0) += 1;
        }
    }
    let shared_d: HashMap<&str, Option<u16>> = d_count.into_iter().filter(|(_, c)| *c > 1).map(|(d, _)| (d, None)).collect();
    let vb = root.attribute("viewBox").ok_or("missing viewBox")?;
    let vb: Vec<f64> = vb
        .split_whitespace()
        .map(|t| t.parse::<f64>().map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    if vb.len() != 4 {
        return Err("viewBox must have 4 numbers".into());
    }
    let page_no: u16 = root.attribute("data-page").and_then(|s| s.parse().ok()).unwrap_or(0);
    let quant = DEFAULT_QUANT;
    let mut cx = Ctx {
        quant: quant as f64,
        page_tf: Affine::translate(-vb[0], -vb[1]),
        page: PageData {
            header: Header { version: VERSION, quant, page: page_no, flags: 0, width: vb[2] as f32, height: vb[3] as f32 },
            lines: vec![],
            ayahs: vec![],
            words: vec![],
            paths: vec![],
            decos: vec![],
            glyphs: vec![],
            insts: vec![],
            ops: vec![],
            strings: vec![],
        },
        words_text: vec![],
        report: Report::default(),
        marker_ids: HashMap::new(),
        strings: HashMap::new(),
        shared_d,
        _p: Default::default(),
    };
    let tf = cx.page_tf;
    cx.walk(root, tf)?;
    cx.link_markers();
    Ok(Converted { page: cx.page, words_text: cx.words_text, report: cx.report })
}

fn class<'a, 'i>(n: Node<'a, 'i>) -> &'a str {
    n.attribute("class").unwrap_or("")
}

fn parse_aid(s: &str) -> Option<(u16, u16)> {
    let (a, b) = s.split_once(':')?;
    Some((a.parse().ok()?, b.parse().ok()?))
}

impl<'a> Ctx<'a> {
    fn warn(&mut self, s: String) {
        self.report.warnings.push(s);
    }

    fn intern(&mut self, s: &str) -> u16 {
        if let Some(&i) = self.strings.get(s) {
            return i;
        }
        let i = self.page.strings.len() as u16;
        self.page.strings.push(s.to_owned());
        self.strings.insert(s.to_owned(), i);
        i
    }

    fn q(&self, v: f64) -> i32 {
        (v * self.quant).round() as i32
    }

    fn node_tf(&self, n: Node<'a, '_>, parent: Affine) -> Result<Affine, String> {
        match n.attribute("transform") {
            Some(t) => Ok(parent.then(&Affine::parse(t)?)),
            None => Ok(parent),
        }
    }

    /// Generic walk for containers we don't model (root, wrapper `g`s).
    fn walk(&mut self, n: Node<'a, '_>, tf: Affine) -> Result<(), String> {
        for c in n.children().filter(|c| c.is_element()) {
            let ctf = self.node_tf(c, tf)?;
            match (c.tag_name().name(), class(c)) {
                ("g", "line") => self.line(c, ctf)?,
                ("g", "ayah-marker") => self.deco(c, ctf, DecoKind::AyahMarker)?,
                ("g", "surah-name") => self.deco(c, ctf, DecoKind::SurahName)?,
                ("g", "basmalah") => self.deco(c, ctf, DecoKind::Basmalah)?,
                ("g", "hizb-mark") => self.deco(c, ctf, DecoKind::HizbMark)?,
                ("g", "sajdah-mark") => self.deco(c, ctf, DecoKind::SajdahMark)?,
                ("g", _) => self.walk(c, ctf)?,
                ("path", _) => {
                    self.warn(format!("stray path outside any group (parent class {:?})", class(n)));
                    let raw = self.raw_path(c, ctf)?;
                    self.push_deco_from_raw(DecoKind::Other, 0, 0, NONE_U16, vec![raw]);
                }
                (t, _) => self.warn(format!("ignored element <{t}>")),
            }
        }
        Ok(())
    }

    fn line(&mut self, n: Node<'a, '_>, tf: Affine) -> Result<(), String> {
        let line_no: u8 = n.attribute("data-line").and_then(|s| s.parse().ok()).unwrap_or(0);
        let first_word = self.page.words.len() as u16;
        let line_idx = self.page.lines.len() as u16;
        self.page.lines.push(LineRec { line_no, first_word, n_words: 0, bbox: IBox::EMPTY });
        self.line_children(n, tf, line_idx, line_no)?;
        let l = &mut self.page.lines[line_idx as usize];
        l.n_words = self.page.words.len() as u16 - first_word;
        let mut bb = IBox::EMPTY;
        for w in &self.page.words[first_word as usize..] {
            bb.union(&w.bbox);
        }
        self.page.lines[line_idx as usize].bbox = bb;
        Ok(())
    }

    fn line_children(&mut self, n: Node<'a, '_>, tf: Affine, line_idx: u16, line_no: u8) -> Result<(), String> {
        for c in n.children().filter(|c| c.is_element()) {
            let ctf = self.node_tf(c, tf)?;
            match (c.tag_name().name(), class(c)) {
                ("g", "ayah") => self.ayah(c, ctf, line_idx)?,
                ("g", "") => self.line_children(c, ctf, line_idx, line_no)?,
                ("g", "surah-name") => self.deco(c, ctf, DecoKind::SurahName)?,
                ("g", "basmalah") => self.deco(c, ctf, DecoKind::Basmalah)?,
                ("g", "hizb-mark") => self.deco(c, ctf, DecoKind::HizbMark)?,
                ("g", "sajdah-mark") => self.deco(c, ctf, DecoKind::SajdahMark)?,
                ("g", "ayah-marker") => self.deco(c, ctf, DecoKind::AyahMarker)?,
                (t, cl) => self.warn(format!("line {line_no}: unexpected <{t} class={cl:?}> inside line")),
            }
        }
        Ok(())
    }

    fn ayah(&mut self, n: Node<'a, '_>, tf: Affine, line_idx: u16) -> Result<(), String> {
        let (sura, ayah) = n.attribute("data-aid").and_then(parse_aid).unwrap_or((0, 0));
        let part = n.attribute("data-part").and_then(|s| s.parse().ok()).unwrap_or(1);
        let parts = n.attribute("data-ayah-parts").and_then(|s| s.parse().ok()).unwrap_or(1);
        let mut flags = 0;
        if n.has_attribute("data-juz-start") {
            flags |= AF_JUZ_START;
        }
        if n.has_attribute("data-hizb-start") {
            flags |= AF_HIZB_START;
        }
        if n.has_attribute("data-rub-start") {
            flags |= AF_RUB_START;
        }
        if n.has_attribute("data-nisf-start") {
            flags |= AF_NISF_START;
        }
        let first_word = self.page.words.len() as u16;
        let ayah_idx = self.page.ayahs.len() as u16;
        // marker id is resolved after decos are all known
        let marker_deco = match n.attribute("data-marker") {
            Some(id) => self.intern(id) | 0x8000, // temp: string ref flagged
            None => NONE_U16,
        };
        self.page.ayahs.push(AyahRec { sura, ayah, part, parts, flags, first_word, n_words: 0, marker_deco, bbox: IBox::EMPTY });
        self.ayah_children(n, tf, line_idx, ayah_idx, sura, ayah)?;
        let n_words = self.page.words.len() as u16 - first_word;
        let mut bb = IBox::EMPTY;
        for w in &self.page.words[first_word as usize..] {
            bb.union(&w.bbox);
        }
        let a = &mut self.page.ayahs[ayah_idx as usize];
        a.n_words = n_words;
        a.bbox = bb;
        Ok(())
    }

    fn ayah_children(&mut self, n: Node<'a, '_>, tf: Affine, line_idx: u16, ayah_idx: u16, sura: u16, ayah: u16) -> Result<(), String> {
        for c in n.children().filter(|c| c.is_element()) {
            let ctf = self.node_tf(c, tf)?;
            match (c.tag_name().name(), class(c)) {
                ("g", "word") => self.word(c, ctf, line_idx, ayah_idx)?,
                ("g", "") => self.ayah_children(c, ctf, line_idx, ayah_idx, sura, ayah)?,
                (t, cl) => self.warn(format!("ayah {sura}:{ayah}: unexpected <{t} class={cl:?}> inside ayah")),
            }
        }
        Ok(())
    }

    fn word(&mut self, n: Node<'a, '_>, tf: Affine, line_idx: u16, ayah_idx: u16) -> Result<(), String> {
        let wid = n.attribute("data-wid").unwrap_or("");
        let mut it = wid.split(':').map(|s| s.parse::<u16>().unwrap_or(0));
        let (sura, ayah, word) = (it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0));
        let uthmani = n.attribute("data-uthmani").unwrap_or("");
        self.words_text.push(WordText {
            wid: wid.to_owned(),
            uthmani: uthmani.to_owned(),
            imlaei: n.attribute("data-imlaei").unwrap_or("").to_owned(),
            qpc: n.attribute("data-qpc").unwrap_or("").to_owned(),
            rasm: n.attribute("data-rasm").unwrap_or("").to_owned(),
            search: n.attribute("data-search").unwrap_or("").to_owned(),
        });
        let text = if uthmani.is_empty() { NONE_U16 } else { self.intern(uthmani) };
        let mut raws = Vec::new();
        for c in n.children().filter(|c| c.is_element()) {
            let ctf = self.node_tf(c, tf)?;
            match c.tag_name().name() {
                "path" => raws.push(self.raw_path(c, ctf)?),
                t => self.warn(format!("word {wid}: unexpected <{t}> inside word")),
            }
        }
        let (first_path, n_paths, bbox) = self.push_paths(raws);
        self.page.words.push(WordRec { sura, ayah, word, line_idx, ayah_idx, text, first_path, n_paths, bbox });
        Ok(())
    }

    fn deco(&mut self, n: Node<'a, '_>, tf: Affine, kind: DecoKind) -> Result<(), String> {
        let (sura, ayah) = n
            .attribute("data-aid")
            .and_then(parse_aid)
            .or_else(|| n.attribute("data-sid").and_then(|s| s.parse().ok()).map(|s| (s, 0)))
            .unwrap_or((0, 0));
        let text = match kind {
            DecoKind::SurahName | DecoKind::Basmalah => n.attribute("data-surah-name-ar").map(|s| self.intern(s)),
            DecoKind::HizbMark => {
                let s = format!(
                    "juz={};hizb={};rub={};nisf={}",
                    n.attribute("data-juz").unwrap_or(""),
                    n.attribute("data-hizb").unwrap_or(""),
                    n.attribute("data-rub").unwrap_or(""),
                    n.attribute("data-nisf").unwrap_or("")
                );
                Some(self.intern(&s))
            }
            _ => None,
        }
        .unwrap_or(NONE_U16);
        let mut raws = Vec::new();
        self.collect_paths(n, tf, &mut raws)?;
        let idx = self.push_deco_from_raw(kind, sura, ayah, text, raws);
        if let Some(id) = n.attribute("id") {
            self.marker_ids.insert(id.to_owned(), idx);
        }
        Ok(())
    }

    fn collect_paths(&mut self, n: Node<'a, '_>, tf: Affine, out: &mut Vec<RawPath>) -> Result<(), String> {
        for c in n.children().filter(|c| c.is_element()) {
            let ctf = self.node_tf(c, tf)?;
            match c.tag_name().name() {
                "path" => out.push(self.raw_path(c, ctf)?),
                "g" => self.collect_paths(c, ctf, out)?,
                t => self.warn(format!("ignored <{t}> inside decoration")),
            }
        }
        Ok(())
    }

    fn push_deco_from_raw(&mut self, kind: DecoKind, sura: u16, ayah: u16, text: u16, raws: Vec<RawPath>) -> u16 {
        let (first_path, n_paths, bbox) = self.push_paths(raws);
        let idx = self.page.decos.len() as u16;
        self.page.decos.push(DecoRec { kind, sura, ayah, text, first_path, n_paths, bbox });
        idx
    }

    /// Encode a group's paths relative to the group's bbox origin.
    fn push_paths(&mut self, raws: Vec<RawPath>) -> (u32, u16, IBox) {
        let mut group = IBox::EMPTY;
        for r in &raws {
            match &r.geom {
                Geom::Inline(c) | Geom::Instance { cmds_page: c, .. } => group.union(&cmds_bbox(c)),
            }
        }
        let (ox, oy) = if group.is_empty() { (0, 0) } else { (group.x0, group.y0) };
        let first_path = self.page.paths.len() as u32;
        for r in raws {
            match r.geom {
                Geom::Inline(cmds) => {
                    let op_off = self.page.ops.len() as u32;
                    let bbox = encode_cmds(&cmds, ox, oy, &mut self.page.ops);
                    let op_len = self.page.ops.len() as u32 - op_off;
                    self.page.paths.push(PathRec { kind: r.kind, mark: r.mark, family: r.family, flags: r.flags, ox, oy, op_off, op_len, bbox });
                }
                Geom::Instance { inst, cmds_page } => {
                    let bbox = cmds_bbox(&cmds_page);
                    let ii = self.page.insts.len() as u32;
                    self.page.insts.push(inst);
                    self.page.paths.push(PathRec { kind: r.kind, mark: r.mark, family: r.family, flags: r.flags | PF_GLYPH, ox: 0, oy: 0, op_off: ii, op_len: 0, bbox });
                }
            }
        }
        let n = self.page.paths.len() as u32 - first_path;
        (first_path, n as u16, group)
    }

    fn raw_path(&mut self, n: Node<'a, '_>, tf: Affine) -> Result<RawPath, String> {
        let kind = match n.attribute("data-kind") {
            Some(k) => PathKind::from_svg(k),
            None => {
                let parent = n.parent().map(class).unwrap_or("");
                self.warn(format!("path without data-kind (parent class {parent:?}) → kind=other"));
                PathKind::Other
            }
        };
        let mark_s = n.attribute("data-mark").unwrap_or("");
        let mark = Mark::from_svg(mark_s);
        if mark == Mark::Unknown {
            self.warn(format!("unknown data-mark {mark_s:?}"));
        }
        let family = Family::from_svg(n.attribute("data-mark-family").unwrap_or(""));
        let mut flags = 0;
        if n.attribute("fill-rule") == Some("evenodd") {
            flags |= PF_EVENODD;
        }
        match n.attribute("data-form") {
            Some("stacked") => flags |= PF_STACKED,
            Some("staggered") => flags |= PF_STAGGERED,
            _ => {}
        }
        if n.has_attribute("data-standalone") {
            flags |= PF_STANDALONE;
        }
        let d = n.attribute("d").ok_or("path without d")?;
        let quant = self.quant;
        let parse = |tf: &Affine| -> Result<Vec<Cmd>, String> {
            let mut cmds = Vec::new();
            let q = |x: f64, y: f64| {
                let (px, py) = tf.apply(x, y);
                ((px * quant).round() as i32, (py * quant).round() as i32)
            };
            for seg in svgtypes::SimplifyingPathParser::from(d) {
                let seg = seg.map_err(|e| format!("path d: {e}"))?;
                use svgtypes::SimplePathSegment as S;
                cmds.push(match seg {
                    S::MoveTo { x, y } => {
                        let (x, y) = q(x, y);
                        Cmd::MoveTo(x, y)
                    }
                    S::LineTo { x, y } => {
                        let (x, y) = q(x, y);
                        Cmd::LineTo(x, y)
                    }
                    S::Quadratic { x1, y1, x, y } => {
                        let (x1, y1) = q(x1, y1);
                        let (x, y) = q(x, y);
                        Cmd::QuadTo(x1, y1, x, y)
                    }
                    S::CurveTo { x1, y1, x2, y2, x, y } => {
                        let (x1, y1) = q(x1, y1);
                        let (x2, y2) = q(x2, y2);
                        let (x, y) = q(x, y);
                        Cmd::CubicTo(x1, y1, x2, y2, x, y)
                    }
                    S::ClosePath => Cmd::Close,
                });
            }
            Ok(cmds)
        };
        let geom = match self.shared_d.get(d).copied() {
            Some(slot) => {
                let gi = match slot {
                    Some(gi) => gi,
                    None => {
                        // first sighting: store outline once in glyph space (identity transform)
                        let gcmds = parse(&Affine::IDENTITY)?;
                        let op_off = self.page.ops.len() as u32;
                        let bbox = encode_cmds(&gcmds, 0, 0, &mut self.page.ops);
                        let op_len = self.page.ops.len() as u32 - op_off;
                        let gi = self.page.glyphs.len() as u16;
                        self.page.glyphs.push(GlyphRec { op_off, op_len, bbox });
                        self.shared_d.insert(d, Some(gi));
                        gi
                    }
                };
                let inst = InstRec { glyph: gi, a: tf.a as f32, b: tf.b as f32, c: tf.c as f32, d: tf.d as f32, e: tf.e as f32, f: tf.f as f32 };
                Geom::Instance { inst, cmds_page: parse(&tf)? }
            }
            None => Geom::Inline(parse(&tf)?),
        };
        Ok(RawPath { kind, mark, family, flags, geom })
    }

    fn link_markers(&mut self) {
        let mut unresolved = 0;
        for a in &mut self.page.ayahs {
            if a.marker_deco != NONE_U16 && a.marker_deco & 0x8000 != 0 {
                let id = &self.page.strings[(a.marker_deco & 0x7fff) as usize];
                match self.marker_ids.get(id) {
                    Some(&i) => a.marker_deco = i,
                    None => {
                        unresolved += 1;
                        a.marker_deco = NONE_U16;
                    }
                }
            }
        }
        if unresolved > 0 {
            self.report.warnings.push(format!("{unresolved} ayah(s) reference a marker id that does not exist on the page"));
        }
    }
}
