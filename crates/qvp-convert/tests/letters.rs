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
use std::collections::{HashMap, HashSet};
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
    // or the source of at least one piece.
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
    if !have_letters() {
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
// The gate renders the page and a copy in which every cut body path is replaced by its
// pieces, and compares the two with the metric and the thresholds of the identity gate.
//
// Run all 604:  QVP_REQUIRE_LETTERS=1 QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test letters ink
//
// Diff images go to target/letters/: one per failing page, and one for each of the three
// worst pages of a run. Red pixels differ, blue pixels are near a cut and do not differ,
// grey pixels are ink.

/// The constants of the identity gate (`tests/identity.rs`), unchanged.
const SCALE: f32 = 4.0;
/// A pixel counts as different when its coverage differs by more than this (0..255).
const PIX_TOL: i32 = 48;
/// Fraction of differing pixels allowed per page.
const MAX_DIFF_FRACTION: f64 = 0.0005;

fn render(svg: &[u8]) -> tiny_skia::Pixmap {
    let tree = usvg::Tree::from_data(svg, &usvg::Options::default()).expect("usvg parse");
    let s = tree.size();
    let (w, h) = ((s.width() * SCALE).round() as u32, (s.height() * SCALE).round() as u32);
    let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
    resvg::render(&tree, tiny_skia::Transform::from_scale(SCALE, SCALE), &mut pm.as_mut());
    pm
}

/// A copy of the page in which each source path gives way to its pieces. With `fuse`, the
/// pieces of one source make one path with all their subpaths; else each piece is its own
/// path. A piece takes the kind, the mark, the family and the flags of its source, so it
/// also takes the fill rule. Word and decoration path ranges follow the new path table.
fn swap(page: &PageData, pieces: &[PieceRec], fuse: bool) -> PageData {
    let n = page.paths.len();
    let mut by_source: Vec<Vec<&[Cmd]>> = vec![vec![]; n];
    for p in pieces {
        by_source[p.source as usize].push(&p.cmds);
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
        assert!(!page.is_instance(i), "page {}: path {i} is a glyph instance and a piece source", page.header.page);
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
    let range = |first: u32, len: u16| {
        let (a, b) = (start[first as usize], start[first as usize + len as usize]);
        (a, u16::try_from(b - a).expect("a word with more than 65535 paths"))
    };
    for w in &mut out.words {
        (w.first_path, w.n_paths) = range(w.first_path, w.n_paths);
    }
    for d in &mut out.decorations {
        (d.first_path, d.n_paths) = range(d.first_path, d.n_paths);
    }
    out.paths = paths;
    out.canonicalize_ops();
    out
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
    a: &tiny_skia::Pixmap,
    b: &tiny_skia::Pixmap,
    (cut, edge): (&tiny_skia::Pixmap, &tiny_skia::Pixmap),
    image: bool,
) -> (Diff, Option<tiny_skia::Pixmap>) {
    assert_eq!((a.width(), a.height()), (b.width(), b.height()), "size mismatch");
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
    (d, img)
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

/// The new segments of a page's pieces, in two sets. The cuts: every segment that two
/// pieces of one source share. The remade edges: every other piece segment that is on no
/// segment of its source, such as the two parts of a source curve that a cut divides.
fn new_segments(page: &PageData, pieces: &[PieceRec]) -> (Vec<Seg>, Vec<Seg>) {
    let (mut cuts, mut edges) = (vec![], vec![]);
    for run in pieces.chunk_by(|a, b| a.source == b.source) {
        let (c, e) = source_segments(page, run);
        cuts.extend(c);
        edges.extend(e);
    }
    (cuts, edges)
}

/// The cuts and the remade edges of the pieces of one source.
fn source_segments(page: &PageData, run: &[PieceRec]) -> (Vec<Seg>, Vec<Seg>) {
    let src: HashSet<Seg> = segments(&page.path_cmds(run[0].source as usize).unwrap()).into_iter().collect();
    let mut count: HashMap<Seg, usize> = HashMap::new();
    for p in run {
        for s in segments(&p.cmds).into_iter().collect::<HashSet<_>>() {
            *count.entry(s).or_default() += 1;
        }
    }
    let mut c: Vec<(Seg, usize)> = count.into_iter().filter(|(s, _)| !src.contains(s)).collect();
    c.sort();
    let (cuts, edges): (Vec<_>, Vec<_>) = c.into_iter().partition(|(_, n)| *n >= 2);
    (cuts.into_iter().map(|(s, _)| s).collect(), edges.into_iter().map(|(s, _)| s).collect())
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

/// Per page: how far the area of each source is from the sum of the areas of its pieces.
#[derive(Clone, Copy, Default)]
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
    /// The largest ratio, over sources, of the difference to the bound that upstream's
    /// edge limit gives (see `areas`). Not more than 1 when the limit explains the area.
    worst_of_bound: f64,
}

fn areas(page: &PageData, pieces: &[PieceRec]) -> Area {
    let q2 = (page.header.quant as f64).powi(2);
    let mut a = Area::default();
    for run in pieces.chunk_by(|a, b| a.source == b.source) {
        let src = area(&page.path_cmds(run[0].source as usize).unwrap()) / q2;
        let sum: f64 = run.iter().map(|p| area(&p.cmds) / q2).sum();
        let d = (src - sum).abs();
        a.sum += d;
        a.total += src.abs();
        a.signed += (sum - src) * src.signum();
        if d > a.worst {
            (a.worst, a.worst_of, a.worst_path) = (d, src.abs(), run[0].source);
        }
        if d / src.abs() > a.worst_fraction {
            (a.worst_fraction, a.worst_fraction_path) = (d / src.abs(), run[0].source);
        }
        // Upstream puts every edge of a piece within `EDGE` of the artwork. Only a segment
        // that is on no segment of the source and on no other piece can move ink, so the
        // area can change by at most `EDGE` times the length of those segments.
        let len = source_segments(page, run).1.iter().map(polygon_len).sum::<f64>() / q2.sqrt();
        a.worst_of_bound = a.worst_of_bound.max(if d == 0.0 { 0.0 } else { d / (EDGE * len) });
    }
    a
}

/// How far upstream says the edge of a cut piece can be from the artwork, in page units.
const EDGE: f64 = 0.0125;

/// The length of a segment's control polygon: never shorter than the segment.
fn polygon_len(s: &Seg) -> f64 {
    let pts: Vec<(i32, i32)> = match *s {
        Seg::Line(a, b) => vec![a, b],
        Seg::Quad(a, c, b) => vec![a, c, b],
        Seg::Cubic(a, c1, c2, b) => vec![a, c1, c2, b],
    };
    pts.windows(2).map(|w| ((w[1].0 - w[0].0) as f64).hypot((w[1].1 - w[0].1) as f64)).sum()
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

/// Render one page three ways and compare. With `images`, write the diff images.
fn ink_page(page: &str, images: bool) -> Result<Ink, String> {
    let lp = build(page)?;
    let d = &lp.data;
    let swapped = valid(page, "swapped", swap(&lp.page, &d.pieces, false))?;
    let fused = valid(page, "fused", swap(&lp.page, &d.pieces, true))?;
    let svg = |p: &PageData| qvp_convert::to_svg(p).map_err(|e| format!("page {page}: to_svg: {e}"));
    let base = render(svg(&lp.page)?.as_bytes());
    let (cuts, edges) = new_segments(&lp.page, &d.pieces);
    let (w, h) = (base.width(), base.height());
    let masks = (&mask(&lp.page, &cuts, w, h), &mask(&lp.page, &edges, w, h));
    let (sw, sw_img) = compare(&base, &render(svg(&swapped)?.as_bytes()), masks, true);
    let (fu, fu_img) = compare(&base, &render(svg(&fused)?.as_bytes()), masks, images);
    if images || sw.fails() {
        let dir = repo().join("target/letters");
        std::fs::create_dir_all(&dir).ok();
        sw_img.unwrap().save_png(dir.join(format!("{page}.png"))).ok();
        if let Some(img) = fu_img {
            img.save_png(dir.join(format!("{page}-fused.png"))).ok();
        }
    }
    Ok(Ink {
        page: page.to_owned(),
        swapped: sw,
        fused: fu,
        area: areas(&lp.page, &d.pieces),
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
    // worst page's fraction over the allowed fraction, and the largest difference.
    println!(
        "{:>5} | {:>7} {:>6} {:>5} {:>6} {:>5} | {:>7} {:>6} {:>5} {:>6} {:>5} | {:>5} {:>6} | {:>10} {:>10}",
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
        "worst |Δ|"
    );
    for r in &rows {
        let flag = if r.swapped.fails() { "  FAIL" } else { "" };
        let (s, f) = (&r.swapped, &r.fused);
        println!(
            "{:>5} | {:>7} {:>6} {:>5} {:>6.3} {:>5} | {:>7} {:>6} {:>5} {:>6.3} {:>5} | {:>5} {:>6} | {:>10.3e} {:>10.3e}{flag}",
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
            r.area.worst
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
    if let Some(w) = rows.iter().max_by(|a, b| a.area.worst.total_cmp(&b.area.worst)) {
        let wf = rows.iter().max_by(|a, b| a.area.worst_fraction.total_cmp(&b.area.worst_fraction)).unwrap();
        let wb = rows.iter().map(|r| r.area.worst_of_bound).fold(0.0, f64::max);
        println!(
            "area: Σ|Δ| {sum:.4e} units² (signed {signed:+.4e}) over {src:.4e} units² of sources ({:.3e}); worst {:.4e} units² on page {} path {} ({:.3e} of its {:.3e}); worst fraction {:.3e} on page {} path {}; worst against the edge bound {wb:.3}",
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
    }
    // The diff images of the three worst pages, for the record.
    for r in rows.iter().take(3) {
        ink_page(&r.page, true).ok();
    }
    assert!(failed.is_empty(), "the ink gate could not build pages {failed:?}");
    let over: Vec<&str> = rows.iter().filter(|r| r.swapped.fails()).map(|r| r.page.as_str()).collect();
    assert!(
        over.is_empty(),
        "a page drawn from its pieces differs from the page on pages {over:?}; see target/letters/"
    );
}
