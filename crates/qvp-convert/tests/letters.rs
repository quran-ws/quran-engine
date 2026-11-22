//! Gates for the letter layer. The structure gate builds each page's layer from the
//! letters archive, then checks that it fits its page and covers every path of every word
//! once. The ink gate draws each page from its pieces and compares it with the page.
//!
//! Run a subset:  QVP_TEST_PAGES=001,002,036 cargo test -p qvp-convert --release --test letters
//! Run all 604:   QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test letters
//!
//! The letters archive is `pages-letters/` (`QVP_SYNC=letters scripts/sync-test-data.sh`).
//! `QVP_LETTERS_DIR` points the gate at another copy, relative to the repository. Without the archive the gate skips,
//! unless `QVP_REQUIRE_LETTERS` is set.
//!
//! A page with a warning fails the gate, because the converter emits none on good data.

use qvp_convert::letter_page::{letter_page, write_letters, LetterDirs, LetterPage};
use qvp_format::letters::{decode_letters, encode_letters, MemberPath, PieceRec};
use qvp_format::{encode_cmds, Cmd, PageData, PathRec};
use rayon::prelude::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// The totals over all 604 pages of the Hafs data.
const ALL_PAGES: usize = 604;
const ALL_LETTERS: usize = 333_220;
const ALL_PIECES: usize = 261_953;
const ALL_SOURCES: usize = 89_853;

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

/// `QVP_LETTERS_DIR`, or `pages-letters/`. A relative path is relative to the repository,
/// because cargo runs the test in the crate directory.
fn letters_dir() -> PathBuf {
    repo().join(std::env::var_os("QVP_LETTERS_DIR").map(PathBuf::from).unwrap_or_else(|| "pages-letters".into()))
}

fn selected_pages() -> Vec<String> {
    if std::env::var("QVP_TEST_ALL").is_ok() {
        let mut v: Vec<String> = std::fs::read_dir(letters_dir())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let p = e.path();
                (p.extension()? == "svg").then(|| p.file_stem().unwrap().to_string_lossy().to_string())
            })
            .filter(|n| !n.starts_with('.'))
            .collect();
        v.sort();
        v
    } else if let Ok(s) = std::env::var("QVP_TEST_PAGES") {
        s.split(',').map(|x| x.trim().to_owned()).filter(|x| !x.is_empty()).collect()
    } else {
        ["001", "002", "017", "036", "144", "302", "485", "604"].iter().map(|s| s.to_string()).collect()
    }
}

struct Counts {
    letters: usize,
    pieces: usize,
    sources: usize,
}

/// Build one page and its layer from the two archives. An error names the page.
fn build(page: &str) -> Result<LetterPage, String> {
    let ld = letters_dir();
    letter_page(
        &ld.join(format!("{page}.svg")),
        &repo().join("pages").join(format!("{page}.svg")),
        &repo().join("index/by-page").join(format!("{page}.json")),
        &ld.join("index/by-page").join(format!("{page}.json")),
    )
    .map_err(|e| format!("page {page}: {e}"))
}

/// Build one page's layer and check it. An error names the page and, where it can, the word.
fn check(page: &str) -> Result<Counts, String> {
    let lp = build(page)?;
    let d = &lp.data;
    if !lp.warnings.is_empty() {
        return Err(format!("page {page}: warnings: {:?}", lp.warnings));
    }
    if decode_letters(&encode_letters(d)).as_ref() != Ok(d) {
        return Err(format!("page {page}: the layer does not round-trip"));
    }
    d.check_against(&lp.page).map_err(|e| format!("page {page}: check_against: {e:?}"))?;

    // Every path of every word has exactly one role: a member of one letter, a word mark,
    // or the source of at least one piece. `check_against` now checks this too; this
    // check stays so that a failure names the word and the path.
    let n = lp.page.paths.len();
    let (mut member, mut mark, mut source) = (vec![0u32; n], vec![0u32; n], vec![false; n]);
    for m in &d.members {
        if let MemberPath::Page(p) = m.path {
            member[p as usize] += 1;
        }
    }
    for w in &d.word_marks {
        mark[w.path as usize] += 1;
    }
    for p in &d.pieces {
        source[p.source as usize] = true;
    }
    for w in &lp.page.words {
        let range = w.first_path as usize..w.first_path as usize + w.n_paths as usize;
        if range.end > n {
            return Err(format!(
                "page {page}: word {}:{}:{}: paths {range:?} run past the {n} paths of the page",
                w.surah, w.ayah, w.word
            ));
        }
        for p in range {
            let roles = (member[p] > 0) as u32 + (mark[p] > 0) as u32 + source[p] as u32;
            if roles != 1 || member[p] > 1 || mark[p] > 1 {
                return Err(format!(
                    "page {page}: word {}:{}:{}: path {p} is a member {} times, a word mark {} times, a source: {}",
                    w.surah, w.ayah, w.word, member[p], mark[p], source[p]
                ));
            }
        }
    }

    if lp.index_letters != Some(d.letters.len() as u64) {
        return Err(format!("page {page}: {} letters, the letters index says {:?}", d.letters.len(), lp.index_letters));
    }
    let sources = source.iter().filter(|s| **s).count();
    Ok(Counts { letters: d.letters.len(), pieces: d.pieces.len(), sources })
}

/// False, after a skip line, when the letters archive is missing and not required.
fn have_letters() -> bool {
    if letters_dir().join("001.svg").exists() {
        return true;
    }
    if std::env::var("QVP_REQUIRE_LETTERS").is_ok() {
        panic!(
                "{} (the letters archive) is missing and QVP_REQUIRE_LETTERS is set; run QVP_SYNC=letters scripts/sync-test-data.sh",
                letters_dir().display()
            );
    }
    eprintln!(
        "skip: {} is missing; run QVP_SYNC=letters scripts/sync-test-data.sh to enable the letter gate",
        letters_dir().display()
    );
    false
}

/// False, after a skip line, when a file of the words archive or its index is missing,
/// as `have_letters` does for the letters archive.
fn have_words(pages: &[&str]) -> bool {
    let missing: Vec<PathBuf> = pages
        .iter()
        .flat_map(|p| [repo().join(format!("pages/{p}.svg")), repo().join(format!("index/by-page/{p}.json"))])
        .filter(|f| !f.exists())
        .collect();
    let Some(first) = missing.first() else { return true };
    if std::env::var("QVP_REQUIRE_LETTERS").is_ok() {
        panic!(
            "{} (the words archive) is missing and QVP_REQUIRE_LETTERS is set; run scripts/sync-test-data.sh",
            first.display()
        );
    }
    eprintln!("skip: {} is missing; run scripts/sync-test-data.sh to enable this test", first.display());
    false
}

#[test]
fn structure() {
    if !have_letters() {
        return;
    }
    let pages = selected_pages();
    let results: Vec<(String, Result<Counts, String>)> = pages.par_iter().map(|p| (p.clone(), check(p))).collect();
    let (mut letters, mut pieces, mut sources) = (0, 0, 0);
    let mut failed = Vec::new();
    for (page, r) in &results {
        match r {
            Ok(c) => {
                letters += c.letters;
                pieces += c.pieces;
                sources += c.sources;
            }
            Err(e) => {
                eprintln!("FAIL {e}");
                failed.push(page.clone());
            }
        }
    }
    println!(
        "{} pages: {letters} letters, {pieces} pieces, {sources} distinct sources, {} failed",
        pages.len(),
        failed.len()
    );
    assert!(failed.is_empty(), "the letter layer failed for pages {failed:?}");
    if std::env::var("QVP_TEST_ALL").is_ok() {
        assert_eq!(pages.len(), ALL_PAGES, "page count");
        assert_eq!(letters, ALL_LETTERS, "letters over all pages");
        assert_eq!(pieces, ALL_PIECES, "pieces over all pages");
        assert_eq!(sources, ALL_SOURCES, "distinct piece sources over all pages");
    }
}

/// A run of `write_letters` over one good page and one bad page: the good page has its file,
/// and neither the bad page's old file nor the file of a page outside the run is left.
#[test]
fn a_run_leaves_no_stale_file() {
    if !have_letters() || !have_words(&["001", "002"]) {
        return;
    }
    let tmp = Path::new(env!("CARGO_TARGET_TMPDIR")).join("letters-run");
    let _ = std::fs::remove_dir_all(&tmp);
    let dirs = LetterDirs {
        letters_svg: tmp.join("letters"),
        words_svg: tmp.join("words"),
        words_index: tmp.join("words-index"),
        letters_index: tmp.join("letters-index"),
    };
    let out = tmp.join("out");
    for d in [&dirs.letters_svg, &dirs.words_svg, &dirs.words_index, &dirs.letters_index, &out] {
        std::fs::create_dir_all(d).unwrap();
    }
    let copy = |from: PathBuf, to: PathBuf| std::fs::copy(from, to).unwrap();
    for page in ["001", "002"] {
        copy(repo().join(format!("pages/{page}.svg")), dirs.words_svg.join(format!("{page}.svg")));
        copy(repo().join(format!("index/by-page/{page}.json")), dirs.words_index.join(format!("{page}.json")));
        copy(letters_dir().join(format!("index/by-page/{page}.json")), dirs.letters_index.join(format!("{page}.json")));
    }
    copy(letters_dir().join("001.svg"), dirs.letters_svg.join("001.svg"));
    // Page 002 fails: its letters page is not a page.
    std::fs::write(dirs.letters_svg.join("002.svg"), "not a page").unwrap();
    for f in ["001.qvl", "002.qvl", "700.qvl"] {
        std::fs::write(out.join(f), "old").unwrap();
    }

    let run = write_letters(&dirs, &out).unwrap();
    assert_eq!(run.removed, 1, "700.qvl is the one page outside the run");
    let ok: Vec<(&str, bool)> = run.pages.iter().map(|(p, r)| (p.as_str(), r.is_ok())).collect();
    assert_eq!(ok, [("001", true), ("002", false)]);
    let mut left: Vec<String> =
        std::fs::read_dir(&out).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    left.sort();
    assert_eq!(left, ["001.qvl"]);
    let lp = letter_page(
        &dirs.letters_svg.join("001.svg"),
        &dirs.words_svg.join("001.svg"),
        &dirs.words_index.join("001.json"),
        &dirs.letters_index.join("001.json"),
    )
    .unwrap();
    assert_eq!(std::fs::read(out.join("001.qvl")).unwrap(), lp.bytes);
    std::fs::remove_dir_all(&tmp).ok();
}

// ───────────────────────────── the ink gate ─────────────────────────────
//
// Does a page drawn from its letters hold the same ink as the page drawn from its words?
// The gate asserts two checks.
//
// 1. Pixels. The gate renders the page and a copy in which every cut body path is replaced
//    by its pieces (`swapped`). It compares the two with the metric and the thresholds of
//    the identity gate. It also reports `fused`, in which the pieces of one source are one
//    path, so that no antialiasing seam is drawn along a cut.
// 2. Area, for each source. A lost or an extra piece can stay under the pixel threshold of
//    a page, so the gate also compares each source with its pieces, from the geometry. See
//    `EDGE` and `MAX_AREA_RATIO`.
//
// Run all 604:  QVP_REQUIRE_LETTERS=1 QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test letters ink
//
// A page that fails the pixel check writes its diff images to target/letters/.
// `QVP_LETTERS_DIFFS=1` also writes them for the three worst pages of a run. Red pixels
// differ, blue pixels are near a cut and do not differ, grey pixels are ink.

/// The constants of the identity gate (`tests/identity.rs`), unchanged.
const SCALE: f32 = 4.0;
/// A pixel counts as different when its coverage differs by more than this (0..255).
const PIX_TOL: i32 = 48;
/// Fraction of differing pixels allowed per page.
const MAX_DIFF_FRACTION: f64 = 0.0005;

/// How far, in page units, the edge of a cut piece can be from the artwork. This is the
/// limit that upstream states for its cut pieces (quran-svg-elements, `schema/FORMAT.md`
/// §10.10). It is not a measurement of this gate.
const EDGE: f64 = 0.0125;

/// The largest allowed ratio of the area difference of a source to `EDGE` × L, where L is
/// the length of the remade edges of its pieces (see `split`). It is 1 by derivation:
/// the pieces of a source cover it, and a cut adds on one side the area it removes on the
/// other. So ink can move only between a remade edge and the part of the artwork that it
/// replaces. Every point of that edge is within `EDGE` of the artwork, so the area between
/// them is at most `EDGE` times the length of the edge, to first order. L is measured on
/// the control polygon, which is never shorter than the curve.
const MAX_AREA_RATIO: f64 = 1.0;

/// Sources whose pieces leave out a part of the source: the page, the source path, and
/// the area left out, in quantum squares (1/quant² of a page unit squared). The area check
/// expects exactly this difference on these sources and fails on any other.
///
/// - Page 001, path 47 (word 1:2:1): the source ends with a closed subpath of three
///   points, 0.05 by 0.01 units. Its one piece does not draw it. Upstream leaves it out.
const KNOWN_LEFT_OUT: &[(&str, u32, f64)] = &[("001", 47, 2.5)];

/// An area difference, in page units squared, that is only the rounding of the f64 sums.
/// The areas come from integer points, and the rounding is about 1e-12. One quantum square
/// is 1e-4. Without this, a source cut only at its own points, with no remade edge, would
/// fail on rounding.
const ROUNDING: f64 = 1e-9;

fn render(page: &str, svg: &[u8]) -> Result<tiny_skia::Pixmap, String> {
    let tree =
        usvg::Tree::from_data(svg, &usvg::Options::default()).map_err(|e| format!("page {page}: usvg parse: {e}"))?;
    let s = tree.size();
    let (w, h) = ((s.width() * SCALE).round() as u32, (s.height() * SCALE).round() as u32);
    let mut pm = tiny_skia::Pixmap::new(w, h).ok_or_else(|| format!("page {page}: no {w}×{h} pixmap"))?;
    resvg::render(&tree, tiny_skia::Transform::from_scale(SCALE, SCALE), &mut pm.as_mut());
    Ok(pm)
}

/// A copy of the page in which each source path gives way to its pieces. With `fuse`, the
/// pieces of one source make one path with all their subpaths; else each piece is its own
/// path. A piece takes the kind, the mark, the family and the flags of its source, so it
/// also takes the fill rule. Word and decoration path ranges follow the new path table.
fn swap(name: &str, page: &PageData, pieces: &[PieceRec], fuse: bool) -> Result<PageData, String> {
    let n = page.paths.len();
    let mut by_source: Vec<Vec<&[Cmd]>> = vec![vec![]; n];
    for p in pieces {
        let at = by_source
            .get_mut(p.source as usize)
            .ok_or_else(|| format!("page {name}: a piece names path {}, past the {n} paths", p.source))?;
        at.push(&p.cmds);
    }
    let mut out = page.clone();
    let mut paths = Vec::with_capacity(n + pieces.len());
    // `start[i]`: the new index of the first path that old path `i` becomes.
    let mut start = Vec::with_capacity(n + 1);
    for (i, src) in page.paths.iter().enumerate() {
        start.push(paths.len() as u32);
        if by_source[i].is_empty() {
            paths.push(*src);
            continue;
        }
        if page.is_instance(i) {
            return Err(format!("page {name}: path {i} is a glyph instance and a piece source"));
        }
        let groups: Vec<Vec<Cmd>> =
            if fuse { vec![by_source[i].concat()] } else { by_source[i].iter().map(|c| c.to_vec()).collect() };
        for cmds in groups {
            let op_off = out.ops.len() as u32;
            let bbox = encode_cmds(&cmds, src.ox, src.oy, &mut out.ops);
            let op_len = out.ops.len() as u32 - op_off;
            paths.push(PathRec { op_off, op_len, bbox, ..*src });
        }
    }
    start.push(paths.len() as u32);
    let range = |what: &str, first: u32, len: u16| -> Result<(u32, u16), String> {
        let end = first as usize + len as usize;
        if end > n {
            return Err(format!("page {name}: {what} paths {first}..{end} run past the {n} paths"));
        }
        let (a, b) = (start[first as usize], start[end]);
        let len = u16::try_from(b - a).map_err(|_| format!("page {name}: {what} has more than 65535 paths"))?;
        Ok((a, len))
    };
    for w in &mut out.words {
        (w.first_path, w.n_paths) = range("a word", w.first_path, w.n_paths)?;
    }
    for d in &mut out.decorations {
        (d.first_path, d.n_paths) = range("a decoration", d.first_path, d.n_paths)?;
    }
    out.paths = paths;
    out.canonicalize_ops();
    Ok(out)
}

/// One comparison of two renders under the identity gate's metric.
#[derive(Clone, Copy, Default)]
struct Diff {
    n_diff: usize,
    max_diff: i32,
    /// Differing pixels near a cut (see `mask`).
    on_cut: usize,
    /// Differing pixels near a remade edge and not near a cut.
    on_edge: usize,
    total: usize,
}

impl Diff {
    fn fraction(&self) -> f64 {
        self.n_diff as f64 / self.total as f64
    }
    fn fails(&self) -> bool {
        self.fraction() > MAX_DIFF_FRACTION
    }
}

/// Compare alpha per pixel, as the identity gate does. With `image`, also draw the diff.
fn compare(
    page: &str,
    a: &tiny_skia::Pixmap,
    b: &tiny_skia::Pixmap,
    (cut, edge): (&tiny_skia::Pixmap, &tiny_skia::Pixmap),
    image: bool,
) -> Result<(Diff, Option<tiny_skia::Pixmap>), String> {
    if (a.width(), a.height()) != (b.width(), b.height()) {
        return Err(format!(
            "page {page}: the renders are {}×{} and {}×{}",
            a.width(),
            a.height(),
            b.width(),
            b.height()
        ));
    }
    let (pa, pb, pc, pe) = (a.pixels(), b.pixels(), cut.pixels(), edge.pixels());
    let mut d = Diff { total: pa.len(), ..Diff::default() };
    let mut img = image.then(|| tiny_skia::Pixmap::new(a.width(), a.height()).unwrap());
    for i in 0..pa.len() {
        let delta = (pa[i].alpha() as i32 - pb[i].alpha() as i32).abs();
        d.max_diff = d.max_diff.max(delta);
        let near_cut = pc[i].alpha() > 0;
        let colour = if delta > PIX_TOL {
            d.n_diff += 1;
            d.on_cut += near_cut as usize;
            d.on_edge += (!near_cut && pe[i].alpha() > 0) as usize;
            Some((255, 0, 0))
        } else if near_cut {
            Some((0, 0, 255))
        } else if pa[i].alpha() > 0 {
            Some((200, 200, 200))
        } else {
            None
        };
        if let (Some(img), Some((r, g, b))) = (img.as_mut(), colour) {
            img.pixels_mut()[i] = tiny_skia::PremultipliedColorU8::from_rgba(r, g, b, 255).unwrap();
        }
    }
    Ok((d, img))
}

/// One segment of an outline, the same whichever way it runs.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Seg {
    Line((i32, i32), (i32, i32)),
    Quad((i32, i32), (i32, i32), (i32, i32)),
    Cubic((i32, i32), (i32, i32), (i32, i32), (i32, i32)),
}

/// The segments of a path. A subpath that does not return to its start gets the closing
/// line that the fill draws. A segment of zero length is left out.
fn segments(cmds: &[Cmd]) -> Vec<Seg> {
    let mut out = vec![];
    let (mut cur, mut first) = ((0, 0), (0, 0));
    let close = |out: &mut Vec<Seg>, cur: (i32, i32), first: (i32, i32)| {
        if cur != first {
            out.push(Seg::Line(cur.min(first), cur.max(first)));
        }
    };
    for c in cmds {
        match *c {
            Cmd::MoveTo(x, y) => {
                close(&mut out, cur, first);
                (cur, first) = ((x, y), (x, y));
            }
            Cmd::LineTo(x, y) => {
                if cur != (x, y) {
                    out.push(Seg::Line(cur.min((x, y)), cur.max((x, y))));
                }
                cur = (x, y);
            }
            Cmd::QuadTo(x1, y1, x, y) => {
                let (p0, p1) = (cur, (x, y));
                out.push(if p0 <= p1 { Seg::Quad(p0, (x1, y1), p1) } else { Seg::Quad(p1, (x1, y1), p0) });
                cur = p1;
            }
            Cmd::CubicTo(x1, y1, x2, y2, x, y) => {
                let (p0, p1) = (cur, (x, y));
                out.push(if p0 <= p1 {
                    Seg::Cubic(p0, (x1, y1), (x2, y2), p1)
                } else {
                    Seg::Cubic(p1, (x2, y2), (x1, y1), p0)
                });
                cur = p1;
            }
            Cmd::Close => {
                close(&mut out, cur, first);
                cur = first;
            }
        }
    }
    close(&mut out, cur, first);
    out
}

/// How the segments of the pieces of one source relate to the source.
struct Split {
    /// Segments that the pieces draw twice and the source does not draw: two pieces, or two
    /// subpaths of one piece, meet there.
    cuts: Vec<Seg>,
    /// Segments that the pieces draw once and the source does not draw, such as one part of
    /// a source curve that a cut divides.
    edges: Vec<Seg>,
    /// The first segment that the pieces draw more often than a cover of the source can: a
    /// source segment more often than the source draws it, or a new segment more than twice.
    extra: Option<String>,
}

/// Split the segments of the pieces of one source. A segment counts once for each time a
/// path draws it.
fn split(src: &[Cmd], run: &[PieceRec]) -> Split {
    let mut in_src: HashMap<Seg, usize> = HashMap::new();
    for s in segments(src) {
        *in_src.entry(s).or_default() += 1;
    }
    let mut count: HashMap<Seg, usize> = HashMap::new();
    for p in run {
        for s in segments(&p.cmds) {
            *count.entry(s).or_default() += 1;
        }
    }
    let mut all: Vec<(Seg, usize)> = count.into_iter().collect();
    all.sort();
    let mut out = Split { cuts: vec![], edges: vec![], extra: None };
    for (s, n) in all {
        let limit = in_src.get(&s).copied().unwrap_or(2);
        if n > limit && out.extra.is_none() {
            out.extra =
                Some(format!("the pieces draw a segment {n} times, a cover of the source draws it {limit} times"));
        }
        match (in_src.contains_key(&s), n) {
            (true, _) => {}
            (false, 1) => out.edges.push(s),
            (false, _) => out.cuts.push(s),
        }
    }
    out
}

/// The commands of page path `i`, or an error that names the page.
fn cmds_of(name: &str, page: &PageData, i: u32) -> Result<Vec<Cmd>, String> {
    page.path_cmds(i as usize).map_err(|e| format!("page {name}: path {i}: {e}"))
}

/// The cuts and the remade edges of all pieces of a page, for the masks.
fn new_segments(name: &str, page: &PageData, pieces: &[PieceRec]) -> Result<(Vec<Seg>, Vec<Seg>), String> {
    let (mut cuts, mut edges) = (vec![], vec![]);
    for run in pieces.chunk_by(|a, b| a.source == b.source) {
        let s = split(&cmds_of(name, page, run[0].source)?, run);
        cuts.extend(s.cuts);
        edges.extend(s.edges);
    }
    Ok((cuts, edges))
}

/// The pixels whose centre lies within 1.5 px of a segment: the pixels the segment
/// crosses and one more on each side, at the render scale.
fn mask(page: &PageData, segs: &[Seg], w: u32, h: u32) -> tiny_skia::Pixmap {
    let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
    let k = SCALE / page.header.quant as f32;
    let p = |(x, y): (i32, i32)| (x as f32 * k, y as f32 * k);
    let mut pb = tiny_skia::PathBuilder::new();
    for s in segs {
        match *s {
            Seg::Line(a, b) => {
                pb.move_to(p(a).0, p(a).1);
                pb.line_to(p(b).0, p(b).1);
            }
            Seg::Quad(a, c, b) => {
                pb.move_to(p(a).0, p(a).1);
                pb.quad_to(p(c).0, p(c).1, p(b).0, p(b).1);
            }
            Seg::Cubic(a, c1, c2, b) => {
                pb.move_to(p(a).0, p(a).1);
                pb.cubic_to(p(c1).0, p(c1).1, p(c2).0, p(c2).1, p(b).0, p(b).1);
            }
        }
    }
    if let Some(path) = pb.finish() {
        let mut paint = tiny_skia::Paint::default();
        paint.set_color_rgba8(0, 0, 255, 255);
        paint.anti_alias = false;
        let stroke = tiny_skia::Stroke { width: 3.0, line_cap: tiny_skia::LineCap::Round, ..Default::default() };
        pm.stroke_path(&path, &paint, &stroke, tiny_skia::Transform::identity(), None);
    }
    pm
}

/// The signed area of a path in quantised units squared, exact for its curves (Green's
/// theorem, as `kurbo` computes it). Each subpath is closed as the fill closes it.
fn area(cmds: &[Cmd]) -> f64 {
    use kurbo::Shape;
    let mut bp = kurbo::BezPath::new();
    let mut open = false;
    let f = |v: i32| v as f64;
    for c in cmds {
        match *c {
            Cmd::MoveTo(x, y) => {
                if open {
                    bp.close_path();
                }
                bp.move_to((f(x), f(y)));
                open = true;
            }
            Cmd::LineTo(x, y) => bp.line_to((f(x), f(y))),
            Cmd::QuadTo(x1, y1, x, y) => bp.quad_to((f(x1), f(y1)), (f(x), f(y))),
            Cmd::CubicTo(x1, y1, x2, y2, x, y) => bp.curve_to((f(x1), f(y1)), (f(x2), f(y2)), (f(x), f(y))),
            Cmd::Close => {
                bp.close_path();
                open = false;
            }
        }
    }
    if open {
        bp.close_path();
    }
    bp.area()
}

/// The area check of one source, in page units.
struct SourceArea {
    /// Signed areas: of the source, and the sum over its pieces.
    src: f64,
    sum: f64,
    /// The sum over the pieces of the size of each piece's signed area. The signed area of
    /// a whole path already takes its holes out.
    sum_abs: f64,
    /// L: the control-polygon length of the remade edges.
    len: f64,
    extra: Option<String>,
}

impl SourceArea {
    fn new(src: &[Cmd], run: &[PieceRec], quant: u16) -> Self {
        let q = quant as f64;
        let areas: Vec<f64> = run.iter().map(|p| area(&p.cmds) / (q * q)).collect();
        let s = split(src, run);
        SourceArea {
            src: area(src) / (q * q),
            sum: areas.iter().sum(),
            sum_abs: areas.iter().map(|a| a.abs()).sum(),
            len: s.edges.iter().map(polygon_len).sum::<f64>() / q,
            extra: s.extra,
        }
    }
    /// A difference in units of `EDGE` × L. A difference with no remade edge to explain it
    /// is infinite.
    fn ratio(&self, d: f64) -> f64 {
        if d <= ROUNDING {
            0.0
        } else if self.len > 0.0 {
            d / (EDGE * self.len)
        } else {
            f64::INFINITY
        }
    }
    /// Check (a): the signed area of the pieces against the signed area of the source.
    fn ratio_a(&self) -> f64 {
        self.ratio((self.sum - self.src).abs())
    }
    /// Check (b): the sizes of the pieces' areas against the size of the source's area. A
    /// loss and an equal gain of opposite sign cancel in (a) but add up here.
    fn ratio_b(&self) -> f64 {
        self.ratio((self.sum_abs - self.src.abs()).abs())
    }
    /// Why the source fails, or `None`.
    fn failure(&self) -> Option<String> {
        let mut why = vec![];
        if self.ratio_a() > MAX_AREA_RATIO {
            why.push(format!(
                "pieces {:.4} units², source {:.4}: {:.3} × EDGE × L (L {:.4})",
                self.sum,
                self.src,
                self.ratio_a(),
                self.len
            ));
        }
        if self.ratio_b() > MAX_AREA_RATIO {
            why.push(format!(
                "sizes: pieces {:.4} units², source {:.4}: {:.3} × EDGE × L",
                self.sum_abs,
                self.src.abs(),
                self.ratio_b()
            ));
        }
        why.extend(self.extra.clone());
        (!why.is_empty()).then(|| why.join("; "))
    }
}

/// The length of a segment's control polygon: never shorter than the segment.
fn polygon_len(s: &Seg) -> f64 {
    let pts: Vec<(i32, i32)> = match *s {
        Seg::Line(a, b) => vec![a, b],
        Seg::Quad(a, c, b) => vec![a, c, b],
        Seg::Cubic(a, c1, c2, b) => vec![a, c1, c2, b],
    };
    pts.windows(2).map(|w| ((w[1].0 - w[0].0) as f64).hypot((w[1].1 - w[0].1) as f64)).sum()
}

/// The key of the word that holds page path `i`.
fn word_of(page: &PageData, i: u32) -> String {
    page.words
        .iter()
        .find(|w| (w.first_path..w.first_path + w.n_paths as u32).contains(&i))
        .map(|w| format!("{}:{}:{}", w.surah, w.ayah, w.word))
        .unwrap_or_else(|| "no word".into())
}

/// Per page: the area check of every source.
#[derive(Default)]
struct Area {
    /// The sum over sources of |area(source) - sum of area(piece)|, in page units squared.
    sum: f64,
    /// The largest single difference, the area of its source, and the source's index.
    worst: f64,
    worst_of: f64,
    worst_path: u32,
    /// The largest difference as a fraction of its source's area, and that source.
    worst_fraction: f64,
    worst_fraction_path: u32,
    /// The sum of the source areas.
    total: f64,
    /// The sum over sources of sum of area(piece) - area(source), signed as the source
    /// runs: more than zero when the pieces hold more ink than their source.
    signed: f64,
    /// The largest ratios of checks (a) and (b), and the source of each.
    worst_a: f64,
    worst_a_path: u32,
    worst_b: f64,
    worst_b_path: u32,
    /// Pieces whose signed area has another sign than their source's.
    turned: usize,
    /// Sources in `KNOWN_LEFT_OUT` that differ by the expected area.
    known: usize,
    /// One line for each source that fails.
    failures: Vec<String>,
}

fn areas(name: &str, page: &PageData, pieces: &[PieceRec]) -> Result<Area, String> {
    let mut a = Area::default();
    let q2 = (page.header.quant as f64).powi(2);
    for run in pieces.chunk_by(|a, b| a.source == b.source) {
        let path = run[0].source;
        let src_cmds = cmds_of(name, page, path)?;
        let s = SourceArea::new(&src_cmds, run, page.header.quant);
        let d = (s.src - s.sum).abs();
        a.sum += d;
        a.total += s.src.abs();
        a.signed += (s.sum - s.src) * s.src.signum();
        if d > a.worst {
            (a.worst, a.worst_of, a.worst_path) = (d, s.src.abs(), path);
        }
        if d / s.src.abs() > a.worst_fraction {
            (a.worst_fraction, a.worst_fraction_path) = (d / s.src.abs(), path);
        }
        a.turned += run.iter().filter(|p| (area(&p.cmds) / q2).signum() != s.src.signum()).count();
        let known = KNOWN_LEFT_OUT.iter().find(|k| k.0 == name && k.1 == path);
        // The worst ratios are of the sources that the check holds to the bound.
        if known.is_none() && s.ratio_a() > a.worst_a {
            (a.worst_a, a.worst_a_path) = (s.ratio_a(), path);
        }
        if known.is_none() && s.ratio_b() > a.worst_b {
            (a.worst_b, a.worst_b_path) = (s.ratio_b(), path);
        }
        let at = format!("page {name}: word {}: path {path}", word_of(page, path));
        match (s.failure(), known) {
            (None, None) => {}
            (Some(why), None) => a.failures.push(format!("{at}: {why}")),
            (None, Some(_)) => a.failures.push(format!("{at}: passes; remove it from KNOWN_LEFT_OUT")),
            (Some(why), Some(&(_, _, left_out))) => {
                if s.extra.is_some() || (d * q2 - left_out).abs() > ROUNDING * q2 {
                    a.failures.push(format!("{at}: {why}; KNOWN_LEFT_OUT expects {left_out} quantum squares"));
                } else {
                    a.known += 1;
                }
            }
        }
    }
    Ok(a)
}

/// The page as the codec writes it, with the boxes and origins it derives. A piece can
/// reach past the box of its word, so the box and the origin of the word can change. The
/// geometry of every path must not.
fn valid(page: &str, name: &str, p: PageData) -> Result<PageData, String> {
    let back = qvp_format::decode(&qvp_format::encode(&p)).map_err(|e| format!("page {page}: {name}: decode: {e}"))?;
    for i in 0..p.paths.len() {
        if back.path_cmds(i) != p.path_cmds(i) {
            return Err(format!("page {page}: {name}: path {i} changes in the codec"));
        }
    }
    if qvp_format::decode(&qvp_format::encode(&back)).as_ref() != Ok(&back) {
        return Err(format!("page {page}: the {name} page does not round-trip"));
    }
    Ok(back)
}

struct Ink {
    page: String,
    swapped: Diff,
    fused: Diff,
    area: Area,
    n_cuts: usize,
    n_edges: usize,
}

/// Render one page three ways and compare, and check the area of each source. A page that
/// fails the pixel check writes its diff images, and so does every page with `images`.
fn ink_page(page: &str, images: bool) -> Result<Ink, String> {
    let lp = build(page)?;
    let d = &lp.data;
    d.check_against(&lp.page).map_err(|e| format!("page {page}: check_against: {e:?}"))?;
    let swapped = valid(page, "swapped", swap(page, &lp.page, &d.pieces, false)?)?;
    let fused = valid(page, "fused", swap(page, &lp.page, &d.pieces, true)?)?;
    let svg = |p: &PageData| qvp_convert::to_svg(p).map_err(|e| format!("page {page}: to_svg: {e}"));
    let base = render(page, svg(&lp.page)?.as_bytes())?;
    let (sw_pm, fu_pm) = (render(page, svg(&swapped)?.as_bytes())?, render(page, svg(&fused)?.as_bytes())?);
    let (cuts, edges) = new_segments(page, &lp.page, &d.pieces)?;
    let (w, h) = (base.width(), base.height());
    let masks = (&mask(&lp.page, &cuts, w, h), &mask(&lp.page, &edges, w, h));
    let (sw, _) = compare(page, &base, &sw_pm, masks, false)?;
    let (fu, _) = compare(page, &base, &fu_pm, masks, false)?;
    if images || sw.fails() {
        let dir = repo().join("target/letters");
        std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
        for (pm, file) in [(&sw_pm, format!("{page}.png")), (&fu_pm, format!("{page}-fused.png"))] {
            let img = compare(page, &base, pm, masks, true)?.1.unwrap();
            let at = dir.join(file);
            img.save_png(&at).map_err(|e| format!("{}: {e}", at.display()))?;
        }
    }
    Ok(Ink {
        page: page.to_owned(),
        swapped: sw,
        fused: fu,
        area: areas(page, &lp.page, &d.pieces)?,
        n_cuts: cuts.len(),
        n_edges: edges.len(),
    })
}

#[test]
fn ink() {
    if !have_letters() {
        return;
    }
    let t0 = std::time::Instant::now();
    let pages = selected_pages();
    let results: Vec<(String, Result<Ink, String>)> =
        pages.par_iter().map(|p| (p.clone(), ink_page(p, false))).collect();
    let mut rows = vec![];
    let mut failed = vec![];
    for (page, r) in results {
        match r {
            Ok(i) => rows.push(i),
            Err(e) => {
                eprintln!("FAIL {e}");
                failed.push(page);
            }
        }
    }
    rows.sort_by(|a, b| b.swapped.n_diff.cmp(&a.swapped.n_diff).then(a.page.cmp(&b.page)));
    // Per measure: differing pixels, those near a cut, those near a remade edge, the
    // page's fraction over the allowed fraction, and the largest difference. Then the
    // area: the sum of the differences and the largest ratio of check (a).
    println!(
        "{:>5} | {:>7} {:>6} {:>5} {:>6} {:>5} | {:>7} {:>6} {:>5} {:>6} {:>5} | {:>5} {:>6} | {:>10} {:>10} {:>7}",
        "page",
        "swap px",
        "cut",
        "edge",
        "×allow",
        "max Δ",
        "fuse px",
        "cut",
        "edge",
        "×allow",
        "max Δ",
        "cuts",
        "edges",
        "area Σ|Δ|",
        "worst |Δ|",
        "ratio a"
    );
    for r in &rows {
        let flag = if r.swapped.fails() { "  FAIL" } else { "" };
        let (s, f) = (&r.swapped, &r.fused);
        println!(
            "{:>5} | {:>7} {:>6} {:>5} {:>6.3} {:>5} | {:>7} {:>6} {:>5} {:>6.3} {:>5} | {:>5} {:>6} | {:>10.3e} {:>10.3e} {:>7.3}{flag}",
            r.page,
            s.n_diff,
            s.on_cut,
            s.on_edge,
            s.fraction() / MAX_DIFF_FRACTION,
            s.max_diff,
            f.n_diff,
            f.on_cut,
            f.on_edge,
            f.fraction() / MAX_DIFF_FRACTION,
            f.max_diff,
            r.n_cuts,
            r.n_edges,
            r.area.sum,
            r.area.worst,
            r.area.worst_a
        );
    }
    let total = |f: fn(&Ink) -> &Diff| {
        let n: usize = rows.iter().map(|r| f(r).n_diff).sum();
        let on: usize = rows.iter().map(|r| f(r).on_cut).sum();
        let edge: usize = rows.iter().map(|r| f(r).on_edge).sum();
        let max = rows.iter().map(|r| f(r).max_diff).max().unwrap_or(0);
        let worst = rows.iter().map(|r| f(r).fraction()).fold(0.0, f64::max);
        let fails = rows.iter().filter(|r| f(r).fails()).count();
        format!(
            "{n} differing px, {on} near a cut, {edge} near a remade edge, max Δ {max}, worst fraction {worst:.3e} ({:.3} of the allowed), {fails} pages over",
            worst / MAX_DIFF_FRACTION
        )
    };
    println!("{} pages in {:.1} s", rows.len(), t0.elapsed().as_secs_f64());
    println!("swapped: {}", total(|r| &r.swapped));
    println!("fused:   {}", total(|r| &r.fused));
    let (sum, src): (f64, f64) = rows.iter().fold((0.0, 0.0), |(s, t), r| (s + r.area.sum, t + r.area.total));
    let signed: f64 = rows.iter().map(|r| r.area.signed).sum();
    let turned: usize = rows.iter().map(|r| r.area.turned).sum();
    let known: usize = rows.iter().map(|r| r.area.known).sum();
    let worst_by = |f: fn(&Area) -> f64| rows.iter().max_by(|a, b| f(&a.area).total_cmp(&f(&b.area)));
    if let (Some(w), Some(wf), Some(wa), Some(wb)) =
        (worst_by(|a| a.worst), worst_by(|a| a.worst_fraction), worst_by(|a| a.worst_a), worst_by(|a| a.worst_b))
    {
        println!(
            "area: Σ|Δ| {sum:.4e} units² (signed {signed:+.4e}) over {src:.4e} units² of sources ({:.3e}); worst {:.4e} units² on page {} path {} ({:.3e} of its {:.3e}); worst fraction {:.3e} on page {} path {}",
            sum / src,
            w.area.worst,
            w.page,
            w.area.worst_path,
            w.area.worst / w.area.worst_of,
            w.area.worst_of,
            wf.area.worst_fraction,
            wf.page,
            wf.area.worst_fraction_path
        );
        println!(
            "area check, allowed ratio {MAX_AREA_RATIO}: (a) worst {:.3} on page {} path {}; (b) worst {:.3} on page {} path {}; {turned} pieces turn against their source; {known} known sources leave out a part",
            wa.area.worst_a, wa.page, wa.area.worst_a_path, wb.area.worst_b, wb.page, wb.area.worst_b_path
        );
    }
    let area_failures: Vec<&String> = rows.iter().flat_map(|r| &r.area.failures).collect();
    for f in area_failures.iter().take(50) {
        eprintln!("FAIL {f}");
    }
    if std::env::var("QVP_LETTERS_DIFFS").is_ok() {
        for r in rows.iter().take(3) {
            ink_page(&r.page, true).unwrap_or_else(|e| panic!("{e}"));
        }
    }
    assert!(failed.is_empty(), "the ink gate could not check pages {failed:?}");
    let over: Vec<&str> = rows.iter().filter(|r| r.swapped.fails()).map(|r| r.page.as_str()).collect();
    assert!(
        over.is_empty(),
        "a page drawn from its pieces differs from the page on pages {over:?}; see target/letters/"
    );
    assert!(area_failures.is_empty(), "{} sources fail the area check; see the FAIL lines", area_failures.len());
}

/// The area check fails a source when one of its pieces is lost, doubled, or replaced by
/// the geometry of the next piece of the same source. The test changes the pieces that the
/// check takes, after the layer has passed `check_against`, so it tests the check itself
/// and not the earlier rules of the layer.
///
/// On each selected page, these changes must fail: lose the median piece, lose the piece
/// at the 10th percentile of area, double the median piece, replace the median piece by
/// the next piece of its source, and add the whole source outline as one more piece. The
/// median and the percentile are of the pieces whose source has two pieces or more.
///
/// Then every piece of every source gets each change, and the test prints how many pass.
/// A change that moves less ink than the allowance of its source (`EDGE` × L) can pass
/// unless the segment count sees it, so the test does not assert that sweep.
#[test]
fn the_area_check_catches_a_lost_or_extra_piece() {
    if !have_letters() {
        return;
    }
    const NAMES: [&str; 3] = ["lost", "doubled", "replaced"];
    /// Per change in the sweep: made, caught by (a), by (b), by the segment count, and
    /// caught by none, with the most ink, in page units squared, that such a change moves.
    #[derive(Default, Clone, Copy)]
    struct Row {
        made: usize,
        a: usize,
        b: usize,
        segments: usize,
        none: usize,
        largest_missed: f64,
    }
    let one = |page: &String| -> Result<[Row; 3], String> {
        let lp = build(page)?;
        lp.data.check_against(&lp.page).map_err(|e| format!("page {page}: check_against: {e:?}"))?;
        let quant = lp.page.header.quant;
        let pieces = &lp.data.pieces;
        let fails = |src: u32, run: &[PieceRec]| -> Result<bool, String> {
            Ok(SourceArea::new(&cmds_of(page, &lp.page, src)?, run, quant).failure().is_some())
        };
        let run_of = |k: usize| -> (usize, &[PieceRec]) {
            let s = pieces[k].source;
            let first = pieces.partition_point(|p| p.source < s);
            let end = pieces.partition_point(|p| p.source <= s);
            (k - first, &pieces[first..end])
        };

        // The named changes.
        let mut shared: Vec<usize> = (0..pieces.len()).filter(|&k| run_of(k).1.len() > 1).collect();
        shared.sort_by(|&x, &y| area(&pieces[x].cmds).abs().total_cmp(&area(&pieces[y].cmds).abs()).then(x.cmp(&y)));
        let (median, tenth) = (shared[shared.len() / 2], shared[shared.len() / 10]);
        let mut named: Vec<(&str, usize, Vec<PieceRec>)> = vec![];
        for (what, k) in [("lose the median piece", median), ("lose the 10th-percentile piece", tenth)] {
            let (i, run) = run_of(k);
            let mut v = run.to_vec();
            v.remove(i);
            named.push((what, k, v));
        }
        let (i, run) = run_of(median);
        let mut v = run.to_vec();
        v.insert(i, run[i].clone());
        named.push(("double the median piece", median, v));
        let mut v = run.to_vec();
        v[i].cmds = run[(i + 1) % run.len()].cmds.clone();
        named.push(("replace the median piece by the next piece", median, v));
        let mut v = run.to_vec();
        v.push(PieceRec { source: run[0].source, cmds: cmds_of(page, &lp.page, run[0].source)? });
        named.push(("add the source outline as a piece", median, v));
        for (what, k, v) in named {
            if !fails(pieces[k].source, &v)? {
                return Err(format!("page {page}: path {}: {what}: the area check passes", pieces[k].source));
            }
        }

        // The sweep.
        let mut t = [Row::default(); 3];
        for run in pieces.chunk_by(|a, b| a.source == b.source) {
            let src = cmds_of(page, &lp.page, run[0].source)?;
            let before = SourceArea::new(&src, run, quant);
            for k in 0..run.len() {
                let mut changes: Vec<(usize, Vec<PieceRec>)> = vec![];
                if run.len() > 1 {
                    let mut lost = run.to_vec();
                    lost.remove(k);
                    changes.push((0, lost));
                    let mut other = run.to_vec();
                    other[k].cmds = run[(k + 1) % run.len()].cmds.clone();
                    changes.push((2, other));
                }
                let mut doubled = run.to_vec();
                doubled.insert(k, run[k].clone());
                changes.push((1, doubled));
                for (c, v) in changes {
                    let s = SourceArea::new(&src, &v, quant);
                    let row = &mut t[c];
                    row.made += 1;
                    row.a += (s.ratio_a() > MAX_AREA_RATIO) as usize;
                    row.b += (s.ratio_b() > MAX_AREA_RATIO) as usize;
                    row.segments += s.extra.is_some() as usize;
                    if s.failure().is_none() {
                        row.none += 1;
                        row.largest_missed = row.largest_missed.max((s.sum - before.sum).abs());
                    }
                }
            }
        }
        Ok(t)
    };
    let pages = selected_pages();
    let results: Vec<Result<[Row; 3], String>> = pages.par_iter().map(one).collect();
    let mut total = [Row::default(); 3];
    let mut errors = vec![];
    for r in results {
        match r {
            Ok(t) => {
                for (sum, add) in total.iter_mut().zip(t) {
                    sum.made += add.made;
                    sum.a += add.a;
                    sum.b += add.b;
                    sum.segments += add.segments;
                    sum.none += add.none;
                    sum.largest_missed = sum.largest_missed.max(add.largest_missed);
                }
            }
            Err(e) => errors.push(e),
        }
    }
    for (name, r) in NAMES.iter().zip(total) {
        println!(
            "{name:>8}: {} changes; (a) catches {}, (b) {}, the segment count {}; {} pass, moving at most {:.4} units²",
            r.made, r.a, r.b, r.segments, r.none, r.largest_missed
        );
    }
    assert!(errors.is_empty(), "{errors:#?}");
}
