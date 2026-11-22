//! The letter layer of one page, built from the letters archive.
//!
//! The letters archive draws each page a second time, with every word divided into letter
//! groups (`schema/FORMAT.md` §6.9 of quran-svg-elements). A path that the division did
//! not cut has the same `d` as a path of the word in the words archive, so it maps to that
//! page path. A path that it cut (`data-cut="1"`) is a piece of one body outline of the
//! word: the geometry decides which. The pieces are new geometry, quantised as the page
//! is. Nothing is repaired: a path that does not fit is an error that names its word.

use crate::affine::Affine;
use crate::svg2qvp::parse_d;
use qvp_format::letters::{LetterData, LetterRec, MemberPath, MemberRec, PieceRec, WordMarkRec, NO_AT};
use qvp_format::{Cmd, PageData, PathKind, PF_EVENODD};
use roxmltree::{Document, Node};
use std::collections::HashMap;

/// A page's letter layer, and what did not fit but did not stop the page.
pub struct LettersConverted {
    pub data: LetterData,
    pub warnings: Vec<String>,
}

/// The text the letters index puts before a word that a hizb rosette stands before:
/// U+06DE and a no-break space (§9.6).
const ROSETTE_PREFIX: &str = "\u{06DE}\u{00A0}";
/// How far, in page units, a point of a piece can be from its body's outline and still
/// belong to it. Upstream proves every piece within 1/8 unit of its contour (§6.9).
const MARGIN: f64 = 0.125;
/// The largest distance, in page units, between a curve and the lines that replace it
/// for the geometric test. Well under `MARGIN`.
const FLATTEN_TOLERANCE: f64 = 0.001;
/// When two bodies take a piece, the nearer one must be this many times nearer.
const CLEAR_FACTOR: f64 = 4.0;

/// `letters_svg`: the page from the letters archive. `words_svg`: the same page from the words
/// archive. `page`: the `PageData` built from `words_svg`. `words_qpc`: the `qpc` text of each
/// page word, in page order, from the words archive. `letters_qpc`: the same from the letters
/// archive's index, used only to check the rosette prefix.
pub fn convert_letters(
    letters_svg: &str,
    words_svg: &str,
    page: &PageData,
    words_qpc: &[String],
    letters_qpc: &[String],
) -> Result<LettersConverted, String> {
    let pg = page.header.page;
    let ldoc = Document::parse(letters_svg).map_err(|e| format!("page {pg}: letters page: xml: {e}"))?;
    let wdoc = Document::parse(words_svg).map_err(|e| format!("page {pg}: words page: xml: {e}"))?;
    let (lroot, wroot) = (ldoc.root_element(), wdoc.root_element());
    let page_no = lroot.attribute("data-page").and_then(|s| s.parse::<u16>().ok());
    if page_no != Some(pg) {
        return Err(format!("page {pg}: the letters page says data-page {:?}", lroot.attribute("data-page")));
    }
    if lroot.attribute("viewBox") != wroot.attribute("viewBox") {
        return Err(format!("page {pg}: the two pages have different viewBox values"));
    }
    let lwords = word_nodes(&ldoc)?;
    let wwords = word_nodes(&wdoc)?;
    check_words(pg, page, &lwords, &wwords)?;
    if words_qpc.len() != page.words.len() || letters_qpc.len() != page.words.len() {
        return Err(format!(
            "page {pg}: {} words, but {} words qpc and {} letters qpc texts",
            page.words.len(),
            words_qpc.len(),
            letters_qpc.len()
        ));
    }

    let mut b =
        Builder { quant: page.header.quant as f64, page, out: LetterData::default(), pieces: vec![], warnings: vec![] };
    for (w, ((lnode, ltf), (wnode, wtf))) in lwords.iter().zip(&wwords).enumerate() {
        b.word(w, *lnode, *ltf, *wnode, *wtf, &words_qpc[w], &letters_qpc[w])?;
    }
    let Builder { mut out, pieces, warnings, .. } = b;

    // Sort the pieces by source. The sort is stable, so the pieces of one source keep the
    // order of the letters that hold them.
    let mut order: Vec<usize> = (0..pieces.len()).collect();
    order.sort_by_key(|&i| pieces[i].source);
    let mut new_index = vec![0u32; pieces.len()];
    for (new, &old) in order.iter().enumerate() {
        new_index[old] = new as u32;
    }
    for m in &mut out.members {
        if let MemberPath::Piece(i) = &mut m.path {
            *i = new_index[*i as usize];
        }
    }
    let mut pieces: Vec<Option<PieceRec>> = pieces.into_iter().map(Some).collect();
    out.pieces = order.iter().map(|&i| pieces[i].take().expect("each piece once")).collect();

    out.page = pg;
    out.quant = page.header.quant;
    out.n_page_paths = page.paths.len() as u32;
    out.n_page_words = page.words.len() as u16;
    out.check_against(page).map_err(|e| format!("page {pg}: the letter layer does not fit the page: {e}"))?;
    Ok(LettersConverted { data: out, warnings })
}

fn class<'a>(n: Node<'a, '_>) -> &'a str {
    n.attribute("class").unwrap_or("")
}

fn key<'a>(n: Node<'a, '_>) -> &'a str {
    n.attribute("data-word-key").unwrap_or("")
}

/// `parent` then the node's own `transform`, as `svg2qvp` composes them.
fn node_tf(n: Node, parent: Affine) -> Result<Affine, String> {
    match n.attribute("transform") {
        Some(t) => Ok(parent.then(&Affine::parse(t)?)),
        None => Ok(parent),
    }
}

/// Every `g.word` of a page in document order, with the transform from its own space to
/// page space. As in `svg2qvp`, the chain starts at the viewBox origin and skips the root.
fn word_nodes<'a, 'i>(doc: &'a Document<'i>) -> Result<Vec<(Node<'a, 'i>, Affine)>, String> {
    let root = doc.root_element();
    let vb: Vec<f64> = root
        .attribute("viewBox")
        .ok_or("missing viewBox")?
        .split_whitespace()
        .map(|t| t.parse::<f64>().map_err(|e| e.to_string()))
        .collect::<Result<_, _>>()?;
    if vb.len() != 4 {
        return Err("viewBox must have 4 numbers".into());
    }
    let page_tf = Affine::translate(-vb[0], -vb[1]);
    let mut out = Vec::new();
    for n in root.descendants().filter(|n| n.has_tag_name("g") && class(*n) == "word") {
        let mut chain: Vec<Node> = n.ancestors().take_while(|a| *a != root).collect();
        chain.reverse();
        let mut tf = page_tf;
        for a in chain {
            tf = node_tf(a, tf)?;
        }
        out.push((n, tf));
    }
    Ok(out)
}

/// Rule 1: the two pages and `page` have the same words in the same order.
fn check_words(pg: u16, page: &PageData, lwords: &[(Node, Affine)], wwords: &[(Node, Affine)]) -> Result<(), String> {
    let n = lwords.len().max(wwords.len()).max(page.words.len());
    for i in 0..n {
        let l = lwords.get(i).map(|(n, _)| key(*n));
        let w = wwords.get(i).map(|(n, _)| key(*n));
        let p = page.words.get(i).map(|r| format!("{}:{}:{}", r.surah, r.ayah, r.word));
        if l != w || w != p.as_deref() {
            return Err(format!(
                "page {pg}: word {i} differs: letters page {:?}, words page {:?}, page data {:?}",
                l.unwrap_or("none"),
                w.unwrap_or("none"),
                p.as_deref().unwrap_or("none")
            ));
        }
    }
    Ok(())
}

struct Builder<'p> {
    quant: f64,
    page: &'p PageData,
    out: LetterData,
    /// In letter order. `MemberPath::Piece` indexes this list until the sort.
    pieces: Vec<PieceRec>,
    warnings: Vec<String>,
}

/// One cut piece of a word, waiting for its body.
struct Cut {
    cmds: Vec<Cmd>,
    member: usize,
    what: String,
}

impl Builder<'_> {
    #[allow(clippy::too_many_arguments)]
    fn word(
        &mut self,
        w: usize,
        lword: Node,
        ltf: Affine,
        wword: Node,
        wtf: Affine,
        qpc: &str,
        letters_qpc: &str,
    ) -> Result<(), String> {
        let (pg, wk) = (self.page.header.page, key(wword));
        let err = |s: String| format!("page {pg} word {wk}: {s}");
        let rec = &self.page.words[w];

        // Rule 2: the i-th path of the word in the words page is page path first_path + i.
        let wpaths: Vec<Node> = wword.children().filter(|c| c.has_tag_name("path")).collect();
        if wpaths.len() != rec.n_paths as usize {
            return Err(err(format!("{} paths in the words page, {} in the page data", wpaths.len(), rec.n_paths)));
        }
        let mut by_d: HashMap<&str, usize> = HashMap::new();
        for (i, p) in wpaths.iter().enumerate() {
            let d = p.attribute("d").ok_or_else(|| err(format!("path {i} of the words page has no d")))?;
            if by_d.insert(d, i).is_some() {
                return Err(err(format!("two paths of the words page have the same d (path {i})")));
            }
        }
        let mut used = vec![false; wpaths.len()];
        let mut take = |d: &str, what: &str| -> Result<u32, String> {
            let i = *by_d.get(d).ok_or_else(|| err(format!("{what}: no path of the words page has this d")))?;
            if std::mem::replace(&mut used[i], true) {
                return Err(err(format!("{what}: the letters page uses this path twice")));
            }
            Ok(rec.first_path + i as u32)
        };

        // Rule 5: a rosette is the first child of its word.
        let children: Vec<Node> = lword.children().filter(|c| c.is_element()).collect();
        let is_rosette = |c: &Node| {
            (c.has_tag_name("g") && class(*c) == "division-mark")
                || (c.has_tag_name("path") && c.attribute("data-mark") == Some("hizb"))
        };
        let rosette = children.first().is_some_and(is_rosette);
        if children.iter().skip(1).any(is_rosette) {
            return Err(err("a rosette that is not the first child of its word".into()));
        }
        let prefixed = letters_qpc.strip_prefix(ROSETTE_PREFIX);
        let shift: u16 = match (rosette, prefixed) {
            (true, Some(rest)) if rest == qpc => 2,
            (true, _) => {
                return Err(err(format!(
                    "a rosette, but the letters qpc {letters_qpc:?} is not U+06DE U+00A0 then {qpc:?}"
                )))
            }
            (false, Some(_)) => return Err(err("the letters qpc starts with U+06DE U+00A0, but no rosette".into())),
            (false, None) if letters_qpc != qpc => {
                return Err(err(format!("the letters qpc {letters_qpc:?} differs from the words qpc {qpc:?}")))
            }
            (false, None) => 0,
        };
        let n_chars = qpc.chars().count();
        let at_of = |s: &str, what: &str| -> Result<u16, String> {
            let v: u16 = s.parse().map_err(|_| err(format!("{what}: data-at {s:?} is not a number")))?;
            v.checked_sub(shift).ok_or_else(|| err(format!("{what}: data-at {v} points into the rosette prefix")))
        };

        let mut cuts: Vec<Cut> = Vec::new();
        for c in children.iter().skip(rosette as usize) {
            let ctf = node_tf(*c, ltf)?;
            match (c.tag_name().name(), class(*c)) {
                ("g", "letter") => {
                    let idx = c.attribute("data-index").unwrap_or("?");
                    let lw = |s: String| format!("letter {idx}: {s}");
                    if c.has_attribute("data-unsplit") {
                        return Err(err(lw("data-unsplit is set".into())));
                    }
                    let text = c.attribute("data-text").and_then(|t| t.chars().next());
                    let text = text.ok_or_else(|| err(lw("no data-text".into())))?;
                    let ats = c.attribute("data-at").ok_or_else(|| err(lw("no data-at".into())))?;
                    let ats: Vec<u16> =
                        ats.split_whitespace().map(|a| at_of(a, &format!("letter {idx}"))).collect::<Result<_, _>>()?;
                    let (&at, more_at) = ats.split_first().ok_or_else(|| err(lw("an empty data-at".into())))?;
                    // Rule 6: the letter's character is at its offset in the words qpc.
                    if qpc.chars().nth(at as usize) != Some(text) {
                        return Err(err(lw(format!("{text:?} is not at offset {at} of {qpc:?}"))));
                    }
                    if let Some(a) = more_at.iter().find(|a| **a as usize >= n_chars) {
                        return Err(err(lw(format!("offset {a} is past the end of {qpc:?}"))));
                    }
                    let first_member = self.out.members.len() as u32;
                    let mut n_body = 0;
                    for (j, p) in c.children().filter(|p| p.is_element()).enumerate() {
                        let what = lw(format!("path {j}"));
                        if !p.has_tag_name("path") {
                            return Err(err(format!("{what}: <{}> inside a letter", p.tag_name().name())));
                        }
                        let d = p.attribute("d").ok_or_else(|| err(format!("{what}: no d")))?;
                        let cut = cut_flag(p).map_err(|s| err(format!("{what}: {s}")))?;
                        // Rule 7: the letters page's kind decides body or mark.
                        let (is_body, m_at) = match p.attribute("data-kind") {
                            Some("body") => (true, at),
                            Some("mark") => match p.attribute("data-at") {
                                Some(a) => (false, at_of(a, &what)?),
                                None => (false, NO_AT),
                            },
                            k => return Err(err(format!("{what}: data-kind {k:?} inside a letter"))),
                        };
                        if m_at != NO_AT && m_at as usize >= n_chars {
                            self.warnings.push(err(format!("{what}: data-at {m_at} is past the end of {qpc:?}")));
                        }
                        n_body += is_body as u32;
                        let member = self.out.members.len();
                        if cut {
                            if !is_body {
                                return Err(err(format!("{what}: a cut mark")));
                            }
                            // Rule 4: the same parser, transform and quantisation as a word path.
                            let cmds =
                                parse_d(d, &node_tf(p, ctf)?, self.quant).map_err(|e| err(format!("{what}: {e}")))?;
                            cuts.push(Cut { cmds, member, what: what.clone() });
                            self.out.members.push(MemberRec { path: MemberPath::Piece(u32::MAX), at: m_at, is_body });
                        } else {
                            let path = take(d, &what)?;
                            self.out.members.push(MemberRec { path: MemberPath::Page(path), at: m_at, is_body });
                        }
                    }
                    if n_body == 0 {
                        return Err(err(lw("no body path".into())));
                    }
                    let n_members = self.out.members.len() as u32 - first_member;
                    let n_members = u16::try_from(n_members).map_err(|_| err(lw("too many paths".into())))?;
                    self.out.letters.push(LetterRec {
                        word: w as u16,
                        at,
                        text,
                        more_at: more_at.to_vec(),
                        first_member,
                        n_members,
                    });
                }
                ("path", _) => {
                    // Rule 8: a path outside every letter is a mark of the word.
                    let what = format!("word-level path {:?}", c.attribute("data-mark").unwrap_or(""));
                    if c.attribute("data-kind") != Some("mark") {
                        return Err(err(format!("{what}: data-kind {:?} outside a letter", c.attribute("data-kind"))));
                    }
                    if cut_flag(*c).map_err(|s| err(format!("{what}: {s}")))? {
                        return Err(err(format!("{what}: a cut path outside a letter")));
                    }
                    let d = c.attribute("d").ok_or_else(|| err(format!("{what}: no d")))?;
                    let path = take(d, &what)?;
                    let at = match c.attribute("data-at") {
                        Some(a) => at_of(a, &what)?,
                        None => NO_AT,
                    };
                    if at != NO_AT && at as usize >= n_chars {
                        self.warnings.push(err(format!("{what}: data-at {at} is past the end of {qpc:?}")));
                    }
                    self.out.word_marks.push(WordMarkRec { word: w as u16, path, at });
                }
                (t, cl) => return Err(err(format!("unexpected <{t} class={cl:?}> inside the word"))),
            }
        }

        // Rule 3: the paths that no uncut path took are the cut bodies.
        let mut bodies = Vec::new();
        for (i, p) in wpaths.iter().enumerate().filter(|(i, _)| !used[*i]) {
            let path = rec.first_path + i as u32;
            if self.page.paths[path as usize].kind != PathKind::Body {
                return Err(err(format!("words page path {i} is in no letter and is not a body")));
            }
            let tf = node_tf(*p, wtf)?;
            let cmds = parse_d(p.attribute("d").unwrap_or(""), &tf, self.quant).map_err(&err)?;
            let evenodd = self.page.paths[path as usize].flags & PF_EVENODD != 0;
            bodies.push((i, path, Outline::new(&cmds, evenodd, self.quant)));
        }
        let mut n_pieces = vec![0usize; bodies.len()];
        let margin = MARGIN * self.quant;
        for cut in cuts {
            let points = on_curve_points(&cut.cmds);
            let mut fits: Vec<(usize, f64)> = Vec::new();
            for (bi, (_, _, outline)) in bodies.iter().enumerate() {
                if let Some(sum) = outline.fit(&points, margin) {
                    fits.push((bi, sum));
                }
            }
            fits.sort_by(|a, b| a.1.total_cmp(&b.1));
            let bi = match fits.as_slice() {
                [] => return Err(err(format!("{}: a cut piece that fits no body", cut.what))),
                [(bi, _)] => *bi,
                [(bi, best), (_, next), ..] if best * CLEAR_FACTOR < *next => *bi,
                [(a, best), (b, next), ..] => {
                    return Err(err(format!(
                        "{}: a cut piece that fits two bodies, words page paths {} and {}, at summed distances {:.2} and {:.2}",
                        cut.what,
                        bodies[*a].0,
                        bodies[*b].0,
                        best / self.quant,
                        next / self.quant
                    )))
                }
            };
            n_pieces[bi] += 1;
            self.out.members[cut.member].path = MemberPath::Piece(self.pieces.len() as u32);
            self.pieces.push(PieceRec { source: bodies[bi].1, cmds: cut.cmds });
        }
        if let Some(bi) = n_pieces.iter().position(|n| *n == 0) {
            return Err(err(format!("words page path {} is a cut body with no piece", bodies[bi].0)));
        }
        Ok(())
    }
}

/// True for `data-cut="1"`, false when the attribute is absent.
fn cut_flag(n: Node) -> Result<bool, String> {
    match n.attribute("data-cut") {
        None => Ok(false),
        Some("1") => Ok(true),
        Some(v) => Err(format!("data-cut {v:?}")),
    }
}

/// The end point of every drawing command.
fn on_curve_points(cmds: &[Cmd]) -> Vec<(f64, f64)> {
    cmds.iter()
        .filter_map(|c| match *c {
            Cmd::MoveTo(x, y) | Cmd::LineTo(x, y) | Cmd::QuadTo(_, _, x, y) | Cmd::CubicTo(_, _, _, _, x, y) => {
                Some((x as f64, y as f64))
            }
            Cmd::Close => None,
        })
        .collect()
}

/// A body outline flattened to closed polygons, in quantised units.
struct Outline {
    rings: Vec<Vec<(f64, f64)>>,
    evenodd: bool,
    /// x0, y0, x1, y1 of every polygon point.
    bbox: [f64; 4],
}

impl Outline {
    fn new(cmds: &[Cmd], evenodd: bool, quant: f64) -> Outline {
        use kurbo::{PathEl, Point};
        let p = |x: i32, y: i32| Point::new(x as f64, y as f64);
        let els = cmds.iter().map(|c| match *c {
            Cmd::MoveTo(x, y) => PathEl::MoveTo(p(x, y)),
            Cmd::LineTo(x, y) => PathEl::LineTo(p(x, y)),
            Cmd::QuadTo(x1, y1, x, y) => PathEl::QuadTo(p(x1, y1), p(x, y)),
            Cmd::CubicTo(x1, y1, x2, y2, x, y) => PathEl::CurveTo(p(x1, y1), p(x2, y2), p(x, y)),
            Cmd::Close => PathEl::ClosePath,
        });
        let mut rings: Vec<Vec<(f64, f64)>> = Vec::new();
        kurbo::flatten(els, FLATTEN_TOLERANCE * quant, |el| match el {
            PathEl::MoveTo(q) => rings.push(vec![(q.x, q.y)]),
            PathEl::LineTo(q) => match rings.last_mut() {
                Some(r) => r.push((q.x, q.y)),
                None => rings.push(vec![(q.x, q.y)]),
            },
            _ => {}
        });
        let mut bbox = [f64::INFINITY, f64::INFINITY, f64::NEG_INFINITY, f64::NEG_INFINITY];
        for &(x, y) in rings.iter().flatten() {
            bbox = [bbox[0].min(x), bbox[1].min(y), bbox[2].max(x), bbox[3].max(y)];
        }
        Outline { rings, evenodd, bbox }
    }

    /// The summed distance of `points` to the outline when every point is inside the fill
    /// or within `margin` of the outline. `None` when one is not.
    fn fit(&self, points: &[(f64, f64)], margin: f64) -> Option<f64> {
        let [x0, y0, x1, y1] = self.bbox;
        // A point outside the box grown by the margin can pass neither test.
        if points.iter().any(|&(x, y)| x < x0 - margin || x > x1 + margin || y < y0 - margin || y > y1 + margin) {
            return None;
        }
        let mut sum = 0.0;
        for &p in points {
            let d = self.distance(p);
            if d > margin && !self.inside(p) {
                return None;
            }
            sum += d;
        }
        Some(sum)
    }

    /// Each ring as closed segments: the fill closes a ring that its commands leave open.
    fn segments(&self) -> impl Iterator<Item = ((f64, f64), (f64, f64))> + '_ {
        self.rings.iter().flat_map(|r| (0..r.len()).map(move |i| (r[i], r[(i + 1) % r.len()])))
    }

    fn inside(&self, (px, py): (f64, f64)) -> bool {
        let mut winding = 0i32;
        for ((ax, ay), (bx, by)) in self.segments() {
            let side = (bx - ax) * (py - ay) - (px - ax) * (by - ay);
            if ay <= py && by > py && side > 0.0 {
                winding += 1;
            } else if ay > py && by <= py && side < 0.0 {
                winding -= 1;
            }
        }
        if self.evenodd {
            winding % 2 != 0
        } else {
            winding != 0
        }
    }

    fn distance(&self, (px, py): (f64, f64)) -> f64 {
        let mut best = f64::INFINITY;
        for ((ax, ay), (bx, by)) in self.segments() {
            let (dx, dy) = (bx - ax, by - ay);
            let len2 = dx * dx + dy * dy;
            let t = if len2 == 0.0 { 0.0 } else { (((px - ax) * dx + (py - ay) * dy) / len2).clamp(0.0, 1.0) };
            let (cx, cy) = (ax + t * dx - px, ay + t * dy - py);
            best = best.min(cx * cx + cy * cy);
        }
        best.sqrt()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qvp_format::letters::{MemberPath, MemberRec, NO_AT};
    use qvp_format::{Cmd, PathKind};

    const ROSETTE: &str = "\u{06DE}\u{00A0}";

    /// A page of one line and one ayah. `words` gives each word's key and its children.
    fn page_svg(words: &[(&str, &str)]) -> String {
        page_svg_with(words, "")
    }

    /// The same, with `before` written in the line before the ayah.
    fn page_svg_with(words: &[(&str, &str)], before: &str) -> String {
        let mut s = String::from(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" data-page="3"><g class="line" data-line="1">"#,
        );
        s.push_str(before);
        s.push_str(r#"<g class="ayah-fragment" data-ayah-key="2:6" data-fragment="1" data-ayah-fragments="1">"#);
        for (key, inner) in words {
            s.push_str(&format!(r#"<g class="word" data-word-key="{key}">{inner}</g>"#));
        }
        s.push_str("</g></g></svg>");
        s
    }

    fn body(d: &str) -> String {
        format!(r#"<path data-kind="body" d="{d}"/>"#)
    }
    fn piece(d: &str) -> String {
        format!(r#"<path data-kind="body" data-cut="1" d="{d}"/>"#)
    }
    fn mark(d: &str, at: Option<u16>) -> String {
        match at {
            Some(a) => format!(r#"<path data-kind="mark" data-mark="fathah" data-at="{a}" d="{d}"/>"#),
            None => format!(r#"<path data-kind="mark" data-mark="fathah" d="{d}"/>"#),
        }
    }
    fn letter(index: usize, text: &str, at: &str, inner: &str) -> String {
        format!(r#"<g class="letter" data-text="{text}" data-index="{index}" data-at="{at}">{inner}</g>"#)
    }

    /// Convert the words page, then build the layer. One word per entry of `qpc`.
    fn build(
        letters: &[(&str, &str)],
        words: &[(&str, &str)],
        qpc: &[&str],
        letters_qpc: &[&str],
    ) -> Result<LettersConverted, String> {
        build_svg(&page_svg(letters), &page_svg(words), qpc, letters_qpc)
    }

    fn build_svg(
        letters_svg: &str,
        words_svg: &str,
        qpc: &[&str],
        letters_qpc: &[&str],
    ) -> Result<LettersConverted, String> {
        let page = crate::convert(words_svg).expect("the words page converts").page;
        let own = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        convert_letters(letters_svg, words_svg, &page, &own(qpc), &own(letters_qpc))
    }

    fn rect(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<Cmd> {
        let q = |v: i32| v * 100;
        vec![
            Cmd::MoveTo(q(x0), q(y0)),
            Cmd::LineTo(q(x1), q(y0)),
            Cmd::LineTo(q(x1), q(y1)),
            Cmd::LineTo(q(x0), q(y1)),
            Cmd::Close,
        ]
    }

    fn page_member(p: u32, at: u16, is_body: bool) -> MemberRec {
        MemberRec { path: MemberPath::Page(p), at, is_body }
    }
    fn piece_member(i: u32, at: u16) -> MemberRec {
        MemberRec { path: MemberPath::Piece(i), at, is_body: true }
    }

    const BODY: &str = "M0 0H30V10H0Z";
    const CUTS: [&str; 3] = ["M0 0H10V10H0Z", "M10 0H20V10H10Z", "M20 0H30V10H20Z"];

    #[test]
    fn uncut_paths_map_to_page_paths() {
        let (m1, m2) = ("M0 12H5V14H0Z", "M10 12H15V14H10Z");
        let words = format!("{}{}{}", body(BODY), mark(m1, None), mark(m2, None));
        let letters = letter(0, "ب", "0", &format!("{}{}{}", mark(m2, Some(2)), body(BODY), mark(m1, Some(1))));
        let out = build(&[("2:6:1", &letters)], &[("2:6:1", &words)], &["بَّ"], &["بَّ"]).unwrap();
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
        let d = &out.data;
        assert_eq!((d.page, d.quant, d.n_page_paths, d.n_page_words), (3, 100, 3, 1));
        assert_eq!(d.letters.len(), 1);
        let l = &d.letters[0];
        assert_eq!((l.word, l.at, l.text, l.first_member, l.n_members), (0, 0, 'ب', 0, 3));
        assert!(l.more_at.is_empty());
        assert_eq!(d.members, vec![page_member(2, 2, false), page_member(0, 0, true), page_member(1, 1, false)]);
        assert!(d.pieces.is_empty() && d.word_marks.is_empty());
    }

    #[test]
    fn cut_pieces_map_to_their_body() {
        let other = "M40 0H70V10H40Z";
        let other_cuts = ["M40 0H50V10H40Z", "M50 0H60V10H50Z", "M60 0H70V10H60Z"];
        let words = format!("{}{}", body(BODY), body(other));
        // The letters take the pieces of the two bodies in turn.
        let mut letters = String::new();
        for i in 0..6 {
            let d = if i % 2 == 0 { CUTS[i / 2] } else { other_cuts[i / 2] };
            letters.push_str(&letter(i, &"ابتثجح".chars().nth(i).unwrap().to_string(), &i.to_string(), &piece(d)));
        }
        let out = build(&[("2:6:1", &letters)], &[("2:6:1", &words)], &["ابتثجح"], &["ابتثجح"]).unwrap();
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
        let d = &out.data;
        let sources: Vec<u32> = d.pieces.iter().map(|p| p.source).collect();
        assert_eq!(sources, vec![0, 0, 0, 1, 1, 1]);
        assert_eq!(d.pieces[0].cmds, rect(0, 0, 10, 10));
        assert_eq!(d.pieces[1].cmds, rect(10, 0, 20, 10));
        assert_eq!(d.pieces[3].cmds, rect(40, 0, 50, 10));
        assert_eq!(d.pieces[5].cmds, rect(60, 0, 70, 10));
        let members: Vec<MemberRec> =
            [0, 3, 1, 4, 2, 5].iter().zip(0u16..).map(|(&p, at)| piece_member(p, at)).collect();
        assert_eq!(d.members, members);
    }

    #[test]
    fn nested_boxes_resolve_by_geometry() {
        // A ring and a square in its hole. The pieces of the square lie inside the boxes
        // of both bodies, but inside the fill of the square only.
        let ring = r#"<path data-kind="body" fill-rule="evenodd" d="M0 0H40V40H0ZM10 10V30H30V10Z"/>"#;
        let square = "M12 12H28V28H12Z";
        let words = format!("{ring}{}", body(square));
        let ring_cuts = ["M0 0H40V10H0Z", "M0 10H10V30H0Z", "M0 30H40V40H0Z", "M30 10H40V30H30Z"];
        let square_cuts = ["M12 12H20V28H12Z", "M20 12H28V28H20Z"];
        let letters = [
            letter(0, "ا", "0", &format!("{}{}", piece(square_cuts[0]), piece(ring_cuts[0]))),
            letter(1, "ب", "1", &format!("{}{}", piece(ring_cuts[1]), piece(ring_cuts[2]))),
            letter(2, "ت", "2", &format!("{}{}", piece(ring_cuts[3]), piece(square_cuts[1]))),
        ]
        .concat();
        let out = build(&[("2:6:1", &letters)], &[("2:6:1", &words)], &["ابت"], &["ابت"]).unwrap();
        let d = &out.data;
        let sources: Vec<u32> = d.pieces.iter().map(|p| p.source).collect();
        assert_eq!(sources, vec![0, 0, 0, 0, 1, 1]);
        assert_eq!(d.pieces[4].cmds, rect(12, 12, 20, 28));
        assert_eq!(d.pieces[5].cmds, rect(20, 12, 28, 28));
        let paths: Vec<MemberPath> = d.members.iter().map(|m| m.path).collect();
        use MemberPath::Piece;
        assert_eq!(paths, vec![Piece(4), Piece(0), Piece(1), Piece(2), Piece(3), Piece(5)]);
    }

    #[test]
    fn a_piece_that_fits_two_bodies_alike_is_an_error() {
        // Two bodies that differ by a hair: a piece fits both, and neither is clearly nearer.
        let words = format!("{}{}", body(BODY), body("M0 0.01H30V10H0Z"));
        let letters = [
            letter(0, "ا", "0", &piece("M1 1H9V9H1Z")),
            letter(1, "ب", "1", &format!("{}{}", piece("M11 1H19V9H11Z"), piece("M21 1H29V9H21Z"))),
        ]
        .concat();
        let e = build(&[("2:6:1", &letters)], &[("2:6:1", &words)], &["اب"], &["اب"]).err().unwrap();
        assert!(e.contains("2:6:1") && e.contains("piece"), "{e}");
    }

    #[test]
    fn a_piece_with_no_body_is_an_error() {
        let letters = [
            letter(0, "ا", "0", &piece(CUTS[0])),
            letter(1, "ب", "1", &format!("{}{}", piece(CUTS[1]), piece(CUTS[2]))),
            letter(2, "ت", "2", &piece("M50 50H60V60H50Z")),
        ]
        .concat();
        let e = build(&[("2:6:1", &letters)], &[("2:6:1", &body(BODY))], &["ابت"], &["ابت"]).err().unwrap();
        assert!(e.contains("2:6:1") && e.contains("no body"), "{e}");
    }

    #[test]
    fn a_cut_body_with_no_piece_is_an_error() {
        let other = "M40 0H70V10H40Z";
        let words = format!("{}{}", body(BODY), body(other));
        let letters = letter(0, "ا", "0", &body(BODY));
        let e = build(&[("2:6:1", &letters)], &[("2:6:1", &words)], &["ا"], &["ا"]).err().unwrap();
        assert!(e.contains("2:6:1") && e.contains("no piece"), "{e}");
    }

    /// A word page with a rosette before its word, and the letters page that holds it.
    fn rosette_pages(letters_qpc: &str) -> (String, String, String) {
        let rosette = "M1 30H5V34H1Z";
        let deco = format!(
            r#"<g class="division-mark" data-mark="hizb" data-ayah-key="2:6" data-hizb="1"><path data-kind="mark" data-mark="hizb" data-standalone="1" d="{rosette}"/></g>"#
        );
        let words_svg = page_svg_with(&[("2:6:1", &format!("{}{}", body(BODY), mark("M0 12H5V14H0Z", None)))], &deco);
        let letters_word = format!(
            r#"<g class="division-mark" data-mark="hizb" data-ayah-key="2:6"><path data-at="0" data-kind="mark" data-mark="hizb" d="{rosette}"/></g>{}"#,
            letter(0, "و", "2", &format!("{}{}", body(BODY), mark("M0 12H5V14H0Z", Some(3))))
        );
        (page_svg(&[("2:6:1", &letters_word)]), words_svg, letters_qpc.to_string())
    }

    #[test]
    fn a_rosette_is_dropped_and_offsets_shift() {
        let (letters_svg, words_svg, lq) = rosette_pages(&format!("{ROSETTE}وَ"));
        let out = build_svg(&letters_svg, &words_svg, &["وَ"], &[&lq]).unwrap();
        assert!(out.warnings.is_empty(), "{:?}", out.warnings);
        let d = &out.data;
        // The page holds the rosette as a decoration, as path 0, before the word's two paths.
        assert_eq!(d.n_page_paths, 3);
        assert_eq!((d.letters[0].at, d.letters[0].text), (0, 'و'));
        assert_eq!(d.members, vec![page_member(1, 0, true), page_member(2, 1, false)]);
        assert!(d.word_marks.is_empty());
    }

    #[test]
    fn a_rosette_without_the_prefix_is_an_error() {
        let (letters_svg, words_svg, _) = rosette_pages("");
        let e = build_svg(&letters_svg, &words_svg, &["وَ"], &["وَ"]).err().unwrap();
        assert!(e.contains("2:6:1") && e.contains("rosette"), "{e}");
        // and the prefix without a rosette
        let letters = letter(0, "و", "2", &body(BODY));
        let e =
            build(&[("2:6:1", &letters)], &[("2:6:1", &body(BODY))], &["وَ"], &[&format!("{ROSETTE}وَ")]).err().unwrap();
        assert!(e.contains("2:6:1") && e.contains("rosette"), "{e}");
    }

    #[test]
    fn a_mark_that_is_a_letters_body_is_a_body_member() {
        let alif = "M5 12H6V20H5Z";
        let words = format!(r#"{}<path data-kind="mark" data-mark="omitted_alif" d="{alif}"/>"#, body(BODY));
        let letters = [
            letter(0, "ه", "0", &body(BODY)),
            letter(1, "ٰ", "1", &format!(r#"<path data-kind="body" data-mark="omitted_alif" d="{alif}"/>"#)),
        ]
        .concat();
        let out = build(&[("2:6:1", &letters)], &[("2:6:1", &words)], &["هٰ"], &["هٰ"]).unwrap();
        let d = &out.data;
        assert_eq!(d.members, vec![page_member(0, 0, true), page_member(1, 1, true)]);
        assert_eq!((d.letters[1].at, d.letters[1].text), (1, '\u{0670}'));
        // the page keeps the path's kind
        let page = crate::convert(&page_svg(&[("2:6:1", &words)])).unwrap().page;
        assert_eq!(page.paths[1].kind, PathKind::Mark);
    }

    #[test]
    fn a_letter_with_two_offsets_keeps_both() {
        let qpc = "لِّـج";
        let letters = [
            letter(0, "لـ", "0 3", &piece(CUTS[0])),
            letter(1, "ج", "4", &format!("{}{}", piece(CUTS[1]), piece(CUTS[2]))),
        ]
        .concat();
        let out = build(&[("2:6:1", &letters)], &[("2:6:1", &body(BODY))], &[qpc], &[qpc]).unwrap();
        let l = &out.data.letters;
        assert_eq!((l[0].at, l[0].text, l[0].more_at.clone()), (0, 'ل', vec![3]));
        assert_eq!((l[1].at, l[1].text, l[1].more_at.clone()), (4, 'ج', vec![]));
    }

    #[test]
    fn word_level_marks_are_listed() {
        let (m1, m2) = ("M0 12H5V14H0Z", "M10 12H15V14H10Z");
        let words = format!("{}{}{}", mark(m1, None), body(BODY), mark(m2, None));
        let letters = format!("{}{}{}", letter(0, "ب", "0", &body(BODY)), mark(m2, None), mark(m1, Some(1)));
        let out = build(&[("2:6:1", &letters)], &[("2:6:1", &words)], &["بَ"], &["بَ"]).unwrap();
        let w = &out.data.word_marks;
        assert_eq!(w.len(), 2);
        assert_eq!((w[0].word, w[0].path, w[0].at), (0, 2, NO_AT));
        assert_eq!((w[1].word, w[1].path, w[1].at), (0, 0, 1));
        assert_eq!(out.data.members, vec![page_member(1, 0, true)]);
    }

    #[test]
    fn a_letter_whose_text_is_not_at_its_offset_is_an_error() {
        let letters = letter(0, "ت", "0", &body(BODY));
        let e = build(&[("2:6:1", &letters)], &[("2:6:1", &body(BODY))], &["بَ"], &["بَ"]).err().unwrap();
        assert!(e.contains("2:6:1") && e.contains("offset"), "{e}");
        // an offset past the end of the text
        let letters = letter(0, "ب", "0 5", &body(BODY));
        let e = build(&[("2:6:1", &letters)], &[("2:6:1", &body(BODY))], &["بَ"], &["بَ"]).err().unwrap();
        assert!(e.contains("2:6:1") && e.contains("offset"), "{e}");
    }

    #[test]
    fn different_word_keys_are_an_error() {
        let letters = letter(0, "ب", "0", &body(BODY));
        let e = build(&[("2:6:2", &letters)], &[("2:6:1", &body(BODY))], &["بَ"], &["بَ"]).err().unwrap();
        assert!(e.contains("2:6:1") && e.contains("2:6:2"), "{e}");
    }

    #[test]
    fn a_word_with_two_paths_of_one_outline_is_an_error() {
        let words = format!("{}{}", body(BODY), mark(BODY, None));
        let letters = letter(0, "ب", "0", &format!("{}{}", body(BODY), mark(BODY, Some(1))));
        let e = build(&[("2:6:1", &letters)], &[("2:6:1", &words)], &["بَ"], &["بَ"]).err().unwrap();
        assert!(e.contains("2:6:1") && e.contains("same d"), "{e}");
    }
}
