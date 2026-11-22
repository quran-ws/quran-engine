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
use qvp_format::{decode, encode, encode_cmds, Cmd, PageData, PathRec};
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
    // The codec gives every piece back bit for bit: it codes the pieces against the page.
    // And the file is in its one coding: encoding the decoded layer again gives its bytes.
    let decoded = decode_letters(&lp.bytes, &lp.page).map_err(|e| format!("page {page}: decode: {e:?}"))?;
    if &decoded != d {
        return Err(format!("page {page}: the layer does not round-trip"));
    }
    let again = encode_letters(&decoded, &lp.page).map_err(|e| format!("page {page}: encode: {e:?}"))?;
    if again != lp.bytes {
        return Err(format!("page {page}: encoding the decoded layer gives other bytes"));
    }
    // The layer pairs with the page file that `batch` writes: decoded from its bytes, that
    // page decodes the layer to the same data, so its page digest is the layer's.
    let file = decode(&encode(&lp.page)).map_err(|e| format!("page {page}: the page file: {e:?}"))?;
    if decode_letters(&lp.bytes, &file).as_ref() != Ok(d) {
        return Err(format!("page {page}: the layer does not decode against its page file"));
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
//    `EDGE` and `MAX_AREA_RATIO`. The allowance counts only the remade outline edges, the
//    new segments near the source outline. Every other new segment that a piece draws once
//    is a cut side, and it must have a partner: a side of another piece along it (see
//    `partner_limit`). A new segment that the pieces draw twice is a cut where two pieces
//    meet, and they must run it once in each direction (see `Split::same_way`): a copy of
//    a piece runs it twice the same way. The gate also asserts what a sum of signed areas
//    rests on: a subpath nested inside another winds against it (see `nesting`).
//
// Run all 604:  QVP_REQUIRE_LETTERS=1 QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test letters ink
// The mutation sweep of the area check runs only with QVP_LETTERS_SWEEP=1.
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
/// the control-polygon length of the remade outline edges of its pieces (see `Split`).
/// The pieces of a source cover it, and a cut adds on one side the area that it removes on
/// the other. So ink can move only between a remade edge and the part of the artwork that
/// it replaces. Every point of that edge is within `near_limit` of the source outline, so,
/// to first order, the area between them is at most (`EDGE` + `rounding`) × L. The control
/// polygon is never shorter than the curve. The gate asserts `EDGE` × L, which is
/// stricter. Real data stays under it: the worst ratio on 604 pages is 0.491, page 381
/// path 810.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
enum Seg {
    Line((i32, i32), (i32, i32)),
    Quad((i32, i32), (i32, i32), (i32, i32)),
    Cubic((i32, i32), (i32, i32), (i32, i32), (i32, i32)),
}

/// The segments of a path. A subpath that does not return to its start gets the closing
/// line that the fill draws. A segment of zero length is left out.
fn segments(cmds: &[Cmd]) -> Vec<Seg> {
    directed(cmds).into_iter().map(|(s, _)| s).collect()
}

/// The segments of a path, each with the way the path runs it: true when it runs from the
/// first end of the `Seg` to the second, false when it runs back. A segment whose two ends
/// are one point reads as true.
fn directed(cmds: &[Cmd]) -> Vec<(Seg, bool)> {
    let mut out = vec![];
    let (mut cur, mut first) = ((0, 0), (0, 0));
    let close = |out: &mut Vec<(Seg, bool)>, cur: (i32, i32), first: (i32, i32)| {
        if cur != first {
            out.push((Seg::Line(cur.min(first), cur.max(first)), cur < first));
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
                    out.push((Seg::Line(cur.min((x, y)), cur.max((x, y))), cur < (x, y)));
                }
                cur = (x, y);
            }
            Cmd::QuadTo(x1, y1, x, y) => {
                let (p0, p1) = (cur, (x, y));
                out.push(if p0 <= p1 {
                    (Seg::Quad(p0, (x1, y1), p1), true)
                } else {
                    (Seg::Quad(p1, (x1, y1), p0), false)
                });
                cur = p1;
            }
            Cmd::CubicTo(x1, y1, x2, y2, x, y) => {
                let (p0, p1) = (cur, (x, y));
                out.push(if p0 <= p1 {
                    (Seg::Cubic(p0, (x1, y1), (x2, y2), p1), true)
                } else {
                    (Seg::Cubic(p1, (x2, y2), (x1, y1), p0), false)
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
    /// Segments that a piece draws once, that the source does not draw, and that lie within
    /// `near_limit` of the source outline: remade outline edges, such as one part of a
    /// source curve that a cut divides. Only these count in L.
    edges: Vec<Seg>,
    /// Every other segment that a piece draws once: a cut side. It must have a partner (see
    /// `partner_limit`).
    sides: Vec<Seg>,
    /// The cut sides with no partner, with the piece that draws each.
    alone: Vec<(Seg, usize)>,
    /// The first segment that the pieces draw more often than a cover of the source can: a
    /// source segment more often than the source draws it, or a new segment more than twice.
    extra: Option<String>,
    /// The cuts that the pieces draw twice in the same direction, with the two pieces that
    /// draw them. Two pieces that meet along a cut each hold their ink on the same side of
    /// their own outline, so they run the cut in opposite directions. A copy of a piece
    /// runs it in the same direction.
    same_way: Vec<(Seg, usize, usize)>,
    /// The cuts that the pieces draw twice, once in each direction.
    opposite: usize,
    /// The source segments that the pieces draw as often as the source does, and of those,
    /// how many the pieces run another way than the source.
    kept: usize,
    kept_turned: usize,
}

/// One source and what the split of its pieces needs of it: how often it draws each
/// segment, and its outline flattened into a grid.
struct Source {
    quant: u16,
    /// Per segment: how often the source draws it, and how many of those run forward (see
    /// `directed`).
    in_src: HashMap<Seg, (usize, usize)>,
    outline: Grid,
    /// Whether a segment lies within `near_limit` of the outline. It does not depend on the
    /// pieces, so the mutation test asks it once per segment.
    near: std::cell::RefCell<HashMap<Seg, bool>>,
}

impl Source {
    fn new(src: &[Cmd], quant: u16) -> Self {
        let mut in_src: HashMap<Seg, (usize, usize)> = HashMap::new();
        let mut outline = Grid::default();
        for (s, fwd) in directed(src) {
            let e = in_src.entry(s).or_default();
            *e = (e.0 + 1, e.1 + fwd as usize);
            outline.add(&flatten(&s, quant), 0);
        }
        Source { quant, in_src, outline, near: Default::default() }
    }

    fn near(&self, s: &Seg) -> bool {
        if let Some(n) = self.near.borrow().get(s) {
            return *n;
        }
        let limit = near_limit(self.quant);
        let n = within(&flatten(s, self.quant), limit, |p| self.outline.dist(p, None));
        self.near.borrow_mut().insert(*s, n);
        n
    }

    /// Split the segments of the pieces. A segment counts once for each time a path draws
    /// it.
    fn split(&self, run: &[PieceRec]) -> Split {
        /// Per segment: how often the pieces draw it, how many of those run forward, and the
        /// first and the last piece that draw it.
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
        struct Drawn {
            n: usize,
            fwd: usize,
            first: usize,
            last: usize,
        }
        let mut count: HashMap<Seg, Drawn> = HashMap::new();
        for (k, p) in run.iter().enumerate() {
            for (s, fwd) in directed(&p.cmds) {
                let e = count.entry(s).or_insert(Drawn { n: 0, fwd: 0, first: k, last: k });
                (e.n, e.fwd, e.last) = (e.n + 1, e.fwd + fwd as usize, k);
            }
        }
        let mut all: Vec<(Seg, Drawn)> = count.into_iter().collect();
        all.sort();
        let mut out = Split {
            cuts: vec![],
            edges: vec![],
            sides: vec![],
            alone: vec![],
            extra: None,
            same_way: vec![],
            opposite: 0,
            kept: 0,
            kept_turned: 0,
        };
        let mut side_of = vec![];
        for (s, d) in all {
            let in_src = self.in_src.get(&s).copied();
            let limit = in_src.map_or(2, |c| c.0);
            if d.n > limit && out.extra.is_none() {
                out.extra = Some(format!(
                    "the pieces draw a segment {} times, a cover of the source draws it {limit} times",
                    d.n
                ));
            }
            match (in_src, d.n) {
                (Some((n, fwd)), _) => {
                    if d.n == n {
                        out.kept += 1;
                        out.kept_turned += (d.fwd != fwd) as usize;
                    }
                }
                (None, 1) if self.near(&s) => out.edges.push(s),
                (None, 1) => {
                    out.sides.push(s);
                    side_of.push(d.last);
                }
                (None, n) => {
                    if n == 2 && d.fwd == 1 {
                        out.opposite += 1;
                    } else if n == 2 {
                        out.same_way.push((s, d.first, d.last));
                    }
                    out.cuts.push(s)
                }
            }
        }
        if !out.sides.is_empty() {
            let pieces = pieces_grid(run, self.quant);
            for (s, &k) in out.sides.iter().zip(&side_of) {
                if !within(&flatten(s, self.quant), partner_limit(self.quant), |p| pieces.dist(p, Some(k as u32))) {
                    out.alone.push((*s, k));
                }
            }
        }
        out
    }
}

/// Every segment of every piece of a run, in one grid, each tagged with its piece.
fn pieces_grid(run: &[PieceRec], quant: u16) -> Grid {
    let mut g = Grid::default();
    for (k, p) in run.iter().enumerate() {
        for s in segments(&p.cmds) {
            g.add(&flatten(&s, quant), k as u32);
        }
    }
    g
}

/// The largest distance from a cut side to the segments of the other pieces of its run,
/// measured at points `STEP` apart. A distance of `CELL` or more reads as `CELL`.
fn partner_distance(s: &Seg, k: usize, pieces: &Grid, quant: u16) -> f64 {
    let pts = flatten(s, quant);
    let mut worst: f64 = 0.0;
    for w in pts.windows(2) {
        let n = (dist(w[0], w[1]) / STEP).ceil().max(1.0) as usize;
        for i in 0..=n {
            let t = i as f64 / n as f64;
            worst = worst.max(pieces.dist(lerp(w[0], w[1], t), Some(k as u32)));
        }
    }
    worst
}

// ───────────────────────────── distances ─────────────────────────────
//
// A remade edge and a cut side differ in one thing: how far the segment is from the
// source outline. A cut side needs a partner: a side of another piece of its source along
// it. Both tests measure the largest distance from a segment to a set of other segments.

/// What rounding to `1/quant` can do to two outlines. Rounding moves a coordinate by at
/// most `0.5/quant`, and so a point by at most `√2/(2·quant)`. A Bézier curve is a convex
/// combination of its control points, so when each control point moves by at most that,
/// each point of the curve moves by at most that too. The page rounds the source and the
/// piece each, so two curves that upstream draws `d` apart are at most `d + √2/quant`
/// apart in the page: 0.0141 units at `quant` 100.
fn rounding(quant: u16) -> f64 {
    std::f64::consts::SQRT_2 / quant as f64
}

/// The flattening tolerance of a curve, in page units.
const FLAT: f64 = 0.0002;
/// The longest step between two points at which a distance is measured, in page units.
const STEP: f64 = 0.002;
/// How much the measure can miss: the flattening of each of the two curves, and half a step
/// between two measured points (a distance changes by at most the length moved along the
/// segment).
const MEASURE: f64 = 2.0 * FLAT + STEP / 2.0;

/// The largest distance from the source outline at which a segment that a piece draws
/// once is a remade outline edge: upstream's limit, the rounding of the source and of the
/// piece, and the measure's own error. 0.0280 units at `quant` 100. It is derived, not
/// tuned.
fn near_limit(quant: u16) -> f64 {
    EDGE + rounding(quant) + MEASURE
}

/// The largest distance from a cut side to the sides of the other pieces of its source:
/// 0.0280 units at `quant` 100. Upstream's polygon check (quran-svg-elements
/// `schema/FORMAT.md` §6.9) finds any strip 0.0125 units wide that is lost, added, or held
/// by two letters. So, where two pieces meet, the gap or the overlap between their sides is
/// less than `EDGE` wide, and the two sides are less than `EDGE` apart. The page rounds
/// both sides (`rounding`), and the measure adds its error (`MEASURE`).
///
/// That check passes a strip under 0.002 units² of any width, so this distance is not a
/// promise of upstream. It is the distance that a gap of the width that upstream checks
/// can give. On all 604 pages, every cut side is within it of a side of another piece:
/// 116 cut sides in 32 sources, the farthest 0.0200 units.
fn partner_limit(quant: u16) -> f64 {
    EDGE + rounding(quant) + MEASURE
}

/// The size of a grid cell, in page units. A query finds every segment within one cell of
/// its point, so every distance that a test compares is less than this.
const CELL: f64 = 0.1;

fn dist(a: (f64, f64), b: (f64, f64)) -> f64 {
    (b.0 - a.0).hypot(b.1 - a.1)
}

fn lerp(a: (f64, f64), b: (f64, f64), t: f64) -> (f64, f64) {
    (a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)
}

/// The distance from `p` to the line segment from `a` to `b`.
fn to_line(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let (dx, dy) = (b.0 - a.0, b.1 - a.1);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 { 0.0 } else { (((p.0 - a.0) * dx + (p.1 - a.1) * dy) / l2).clamp(0.0, 1.0) };
    dist(p, (a.0 + t * dx, a.1 + t * dy))
}

/// A segment as a polyline in page units, within `FLAT` of the curve.
fn flatten(s: &Seg, quant: u16) -> Vec<(f64, f64)> {
    let q = quant as f64;
    let p = |(x, y): (i32, i32)| kurbo::Point::new(x as f64 / q, y as f64 / q);
    let mut bp = kurbo::BezPath::new();
    match *s {
        Seg::Line(a, b) => {
            bp.move_to(p(a));
            bp.line_to(p(b));
        }
        Seg::Quad(a, c, b) => {
            bp.move_to(p(a));
            bp.quad_to(p(c), p(b));
        }
        Seg::Cubic(a, c1, c2, b) => {
            bp.move_to(p(a));
            bp.curve_to(p(c1), p(c2), p(b));
        }
    }
    let mut out = vec![];
    kurbo::flatten(&bp, FLAT, |el| {
        if let kurbo::PathEl::MoveTo(p) | kurbo::PathEl::LineTo(p) = el {
            out.push((p.x, p.y));
        }
    });
    out
}

/// True when every point of the polyline `pts` is within `limit` of what `d` measures. The
/// walk steps by `limit - d(p)`, and by at least `STEP`: no point between two steps can be
/// farther than `limit`, except by half a step when the step is `STEP`.
fn within(pts: &[(f64, f64)], limit: f64, d: impl Fn((f64, f64)) -> f64) -> bool {
    for w in pts.windows(2) {
        let len = dist(w[0], w[1]);
        let mut at = 0.0;
        loop {
            let p = lerp(w[0], w[1], if len > 0.0 { (at / len).min(1.0) } else { 1.0 });
            let here = d(p);
            if here > limit {
                return false;
            }
            if at >= len {
                break;
            }
            at += (limit - here).max(STEP);
        }
    }
    pts.len() != 1 || d(pts[0]) <= limit
}

/// A point in page units.
type Pt = (f64, f64);

/// Line segments in square cells of side `CELL`. A segment longer than a cell is cut into
/// parts no longer than a cell, so each part touches at most four cells.
#[derive(Default)]
struct Grid {
    cells: HashMap<(i64, i64), Vec<u32>>,
    lines: Vec<(Pt, Pt, u32)>,
}

fn cell(p: (f64, f64)) -> (i64, i64) {
    ((p.0 / CELL).floor() as i64, (p.1 / CELL).floor() as i64)
}

impl Grid {
    /// Add a polyline, with a tag.
    fn add(&mut self, pts: &[(f64, f64)], tag: u32) {
        for w in pts.windows(2) {
            let n = (dist(w[0], w[1]) / CELL).ceil().max(1.0) as usize;
            for i in 0..n {
                let (a, b) = (lerp(w[0], w[1], i as f64 / n as f64), lerp(w[0], w[1], (i + 1) as f64 / n as f64));
                let id = self.lines.len() as u32;
                self.lines.push((a, b, tag));
                let (c0, c1) = (cell((a.0.min(b.0), a.1.min(b.1))), cell((a.0.max(b.0), a.1.max(b.1))));
                for cx in c0.0..=c1.0 {
                    for cy in c0.1..=c1.1 {
                        self.cells.entry((cx, cy)).or_default().push(id);
                    }
                }
            }
        }
    }

    /// The distance from `p` to the nearest line whose tag is not `skip`, or `CELL` when
    /// none is nearer than that.
    fn dist(&self, p: (f64, f64), skip: Option<u32>) -> f64 {
        let (cx, cy) = cell(p);
        let mut best = CELL;
        for x in cx - 1..=cx + 1 {
            for y in cy - 1..=cy + 1 {
                for &id in self.cells.get(&(x, y)).map(Vec::as_slice).unwrap_or(&[]) {
                    let (a, b, tag) = self.lines[id as usize];
                    if Some(tag) != skip {
                        best = best.min(to_line(p, a, b));
                    }
                }
            }
        }
        best
    }
}

/// The commands of page path `i`, or an error that names the page.
fn cmds_of(name: &str, page: &PageData, i: u32) -> Result<Vec<Cmd>, String> {
    page.path_cmds(i as usize).map_err(|e| format!("page {name}: path {i}: {e}"))
}

/// The cuts (with the cut sides) and the remade edges of all pieces of a page, for the
/// masks.
fn new_segments(name: &str, page: &PageData, pieces: &[PieceRec]) -> Result<(Vec<Seg>, Vec<Seg>), String> {
    let (mut cuts, mut edges) = (vec![], vec![]);
    for run in pieces.chunk_by(|a, b| a.source == b.source) {
        let s = Source::new(&cmds_of(name, page, run[0].source)?, page.header.quant).split(run);
        cuts.extend(s.cuts);
        cuts.extend(s.sides);
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

/// The subpaths of a path, each with its signed area, its box and its outline flattened, in
/// quantised units.
struct Subpath {
    area: f64,
    lo: (f64, f64),
    hi: (f64, f64),
    poly: Vec<(f64, f64)>,
}

fn subpaths(cmds: &[Cmd]) -> Vec<Subpath> {
    let mut runs: Vec<Vec<Cmd>> = vec![];
    for c in cmds {
        match c {
            Cmd::MoveTo(..) => runs.push(vec![*c]),
            _ => {
                if let Some(r) = runs.last_mut() {
                    r.push(*c);
                }
            }
        }
    }
    runs.iter()
        .map(|r| {
            let mut poly = vec![];
            for s in segments(r) {
                // `flatten` works in page units; quant 1 keeps quantised units.
                let pts = flatten(&s, 1);
                poly.extend_from_slice(&pts[..pts.len() - 1]);
                poly.push(pts[pts.len() - 1]);
            }
            let lo = poly.iter().fold((f64::MAX, f64::MAX), |m, p| (m.0.min(p.0), m.1.min(p.1)));
            let hi = poly.iter().fold((f64::MIN, f64::MIN), |m, p| (m.0.max(p.0), m.1.max(p.1)));
            let first = match r[0] {
                Cmd::MoveTo(x, y) => (x as f64, y as f64),
                _ => unreachable!("a run starts with a move"),
            };
            if poly.is_empty() {
                poly.push(first);
            }
            Subpath { area: area(r), lo, hi, poly }
        })
        .collect()
}

/// Whether subpath `s` lies inside the outline `poly`, from five points of `s`: the
/// centroid of its outline points and four of its points, a quarter of the way apart. A
/// point within one quantised unit of `poly` is left out: two subpaths that touch share
/// points, and for such a point the even-odd test can go either way. The answer is the
/// majority of the points left, or `None` when no majority exists: every point is left
/// out, or the points left split evenly.
fn held_by(s: &[(f64, f64)], poly: &[(f64, f64)]) -> Option<bool> {
    let n = s.len();
    let c = s.iter().fold((0.0, 0.0), |m, p| (m.0 + p.0 / n as f64, m.1 + p.1 / n as f64));
    let (mut yes, mut no) = (0, 0);
    for p in [c, s[0], s[n / 4], s[n / 2], s[3 * n / 4]] {
        let mut odd = false;
        let mut near = f64::MAX;
        for i in 0..poly.len() {
            let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
            if (a.1 > p.1) != (b.1 > p.1) && p.0 < a.0 + (p.1 - a.1) * (b.0 - a.0) / (b.1 - a.1) {
                odd = !odd;
            }
            near = near.min(to_line(p, a, b));
        }
        if near < 1.0 {
            continue;
        }
        if odd {
            yes += 1;
        } else {
            no += 1;
        }
    }
    (yes != no).then_some(yes > no)
}

/// For each subpath nested inside another: whether it winds against the smallest subpath
/// that holds it. The sum of signed areas counts ink only when it does. A full test of
/// containment is costly, so the test is a cheap one: the box of the inner subpath lies in
/// the box of the outer, and most of a few points of the inner lie inside the outer's
/// flattened outline (`held_by`). Returns the count that winds against, the count that
/// winds with, and the count of undecided subpaths. A subpath is undecided when `held_by`
/// gives no answer for a possible holder that is smaller than the holder found, or when
/// it finds no holder. Such a subpath can be counted against the wrong holder, or not at
/// all.
fn nesting(cmds: &[Cmd]) -> (usize, usize, usize) {
    let subs = subpaths(cmds);
    let (mut against, mut with, mut undecided) = (0, 0, 0);
    for (i, s) in subs.iter().enumerate() {
        if s.area == 0.0 {
            continue;
        }
        // The holders that the boxes allow, each with the answer of `held_by`.
        let mut parent: Option<&Subpath> = None;
        let mut open: Option<f64> = None;
        for (j, o) in subs.iter().enumerate() {
            if j == i
                || o.area.abs() <= s.area.abs()
                || o.lo.0 > s.lo.0
                || o.lo.1 > s.lo.1
                || o.hi.0 < s.hi.0
                || o.hi.1 < s.hi.1
            {
                continue;
            }
            match held_by(&s.poly, &o.poly) {
                Some(true) => {
                    if parent.is_none_or(|p| o.area.abs() < p.area.abs()) {
                        parent = Some(o);
                    }
                }
                Some(false) => {}
                None => open = Some(open.map_or(o.area.abs(), |a: f64| a.min(o.area.abs()))),
            }
        }
        if open.is_some_and(|a| parent.is_none_or(|p| a < p.area.abs())) {
            undecided += 1;
        }
        match parent {
            Some(o) if o.area.signum() == s.area.signum() => with += 1,
            Some(_) => against += 1,
            None => {}
        }
    }
    (against, with, undecided)
}

/// The area check of one source, in page units.
struct SourceArea {
    /// Signed areas: of the source, and the sum over its pieces.
    src: f64,
    sum: f64,
    /// The sum over the pieces of the size of each piece's signed area. The signed area of
    /// a whole path already takes its holes out.
    sum_abs: f64,
    /// L: the control-polygon length of the remade outline edges.
    len: f64,
    /// The segments of the pieces, split (see `Split`).
    split: Split,
}

impl SourceArea {
    fn new(source: &Source, src_area: f64, run: &[PieceRec]) -> Self {
        let q = source.quant as f64;
        let areas: Vec<f64> = run.iter().map(|p| area(&p.cmds) / (q * q)).collect();
        let s = source.split(run);
        SourceArea {
            src: src_area / (q * q),
            sum: areas.iter().sum(),
            sum_abs: areas.iter().map(|a| a.abs()).sum(),
            len: s.edges.iter().map(polygon_len).sum::<f64>() / q,
            split: s,
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
    /// Why the area fails, or `None`.
    fn area_failure(&self) -> Option<String> {
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
        why.extend(self.split.extra.clone());
        (!why.is_empty()).then(|| why.join("; "))
    }
    /// Why the cuts of the source break the direction rule, or `None`.
    fn direction_failure(&self) -> Option<String> {
        let (s, x, y) = self.split.same_way.first()?;
        Some(format!(
            "{} cuts drawn twice the same way, the first by pieces {x} and {y}: {s:?}",
            self.split.same_way.len()
        ))
    }
    /// Why the source fails the area check, the partner rule or the direction rule, or
    /// `None`.
    fn failure(&self) -> Option<String> {
        let mut why: Vec<String> = self.area_failure().into_iter().collect();
        if let Some((s, k)) = self.split.alone.first() {
            why.push(format!("{} cut sides with no partner, the first of piece {k}: {s:?}", self.split.alone.len()));
        }
        why.extend(self.direction_failure());
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

/// A cut side with no partner: its source, its word, its piece, its partner distance.
type Alone = (u32, String, usize, f64);

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
    /// For each entry of `KNOWN_LEFT_OUT`, how many times it matched on this page.
    known: Vec<usize>,
    /// The cut sides, and the sources that have at least one.
    sides: usize,
    sources_with_sides: usize,
    /// The largest partner distance of a cut side that has a partner, and its source.
    worst_partner: f64,
    worst_partner_path: u32,
    /// Each cut side with no partner: its source, its word, its piece, its distance.
    alone: Vec<Alone>,
    /// Cuts drawn twice: once in each direction, and twice the same way.
    opposite: usize,
    same_way: usize,
    /// Source segments that the pieces draw as often as the source, the count of those
    /// that the pieces run another way than the source, and one line for each source that
    /// has such a segment. The gate reports these and does not assert them.
    kept: usize,
    kept_turned_n: usize,
    kept_turned: Vec<String>,
    /// Nested subpaths, in sources and pieces, that wind against their holder and with it.
    nested_against: usize,
    /// Nested subpaths for which the check gives no answer (see `nesting`).
    nested_undecided: usize,
    nested_with: Vec<String>,
    /// One line for each source that fails.
    failures: Vec<String>,
}

/// The area check of every source of a page. `letter_of` gives the letter of each piece.
fn areas(name: &str, page: &PageData, pieces: &[PieceRec], letter_of: &[u32]) -> Result<Area, String> {
    let mut a = Area { known: vec![0; KNOWN_LEFT_OUT.len()], ..Area::default() };
    let quant = page.header.quant;
    let q2 = (quant as f64).powi(2);
    let mut first = 0;
    for run in pieces.chunk_by(|a, b| a.source == b.source) {
        let off = first;
        first += run.len();
        let path = run[0].source;
        let src_cmds = cmds_of(name, page, path)?;
        let source = Source::new(&src_cmds, quant);
        let s = SourceArea::new(&source, area(&src_cmds), run);
        let at = format!("page {name}: word {}: path {path}", word_of(page, path));
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
        for (what, cmds) in std::iter::once(("the source".to_owned(), &src_cmds))
            .chain(run.iter().enumerate().map(|(k, p)| (format!("piece {k}"), &p.cmds)))
        {
            let (against, with, undecided) = nesting(cmds);
            a.nested_against += against;
            a.nested_undecided += undecided;
            if with > 0 {
                a.nested_with.push(format!("{at}: {what}: {with} nested subpaths wind with their holder"));
            }
        }

        // The partner rule: measure every cut side, for the report.
        let split = &s.split;
        a.opposite += split.opposite;
        // The direction rule: one line for each cut drawn twice the same way.
        a.same_way += split.same_way.len();
        for (seg, x, y) in &split.same_way {
            let how = if x == y {
                "one piece".to_owned()
            } else if letter_of[off + x] == letter_of[off + y] {
                format!("pieces {x} and {y} of one letter")
            } else {
                format!("pieces {x} and {y} of letters {} and {}", letter_of[off + x], letter_of[off + y])
            };
            a.failures.push(format!("{at}: {how} draw a cut twice the same way: {seg:?}"));
        }
        a.kept += split.kept;
        a.kept_turned_n += split.kept_turned;
        if split.kept_turned > 0 {
            a.kept_turned.push(format!("{at}: {} source segments run another way", split.kept_turned));
        }
        a.sides += split.sides.len();
        a.sources_with_sides += !split.sides.is_empty() as usize;
        if !split.sides.is_empty() {
            let grid = pieces_grid(run, quant);
            let mut side_piece: HashMap<Seg, usize> = HashMap::new();
            for (k, p) in run.iter().enumerate() {
                for seg in segments(&p.cmds) {
                    side_piece.insert(seg, k);
                }
            }
            for seg in &split.sides {
                let k = side_piece[seg];
                let pd = partner_distance(seg, k, &grid, quant);
                if split.alone.iter().any(|(x, _)| x == seg) {
                    a.alone.push((path, word_of(page, path), k, pd));
                } else if pd > a.worst_partner {
                    (a.worst_partner, a.worst_partner_path) = (pd, path);
                }
            }
        }

        let known = KNOWN_LEFT_OUT.iter().position(|k| k.0 == name && k.1 == path);
        // The worst ratios are of the sources that the check holds to the bound.
        if known.is_none() && s.ratio_a() > a.worst_a {
            (a.worst_a, a.worst_a_path) = (s.ratio_a(), path);
        }
        if known.is_none() && s.ratio_b() > a.worst_b {
            (a.worst_b, a.worst_b_path) = (s.ratio_b(), path);
        }
        if let Some((seg, k)) = split.alone.first() {
            a.failures.push(format!(
                "{at}: {} cut sides with no partner, the first of piece {k}: {seg:?}",
                split.alone.len()
            ));
        }
        match (s.area_failure(), known) {
            (None, None) => {}
            (Some(why), None) => a.failures.push(format!("{at}: {why}")),
            (None, Some(_)) => a.failures.push(format!("{at}: passes; remove it from KNOWN_LEFT_OUT")),
            (Some(why), Some(i)) => {
                let left_out = KNOWN_LEFT_OUT[i].2;
                if split.extra.is_some() || (d * q2 - left_out).abs() > ROUNDING * q2 {
                    a.failures.push(format!("{at}: {why}; KNOWN_LEFT_OUT expects {left_out} quantum squares"));
                } else {
                    a.known[i] += 1;
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
    let mut letter_of = vec![u32::MAX; d.pieces.len()];
    for (i, l) in d.letters.iter().enumerate() {
        for m in &d.members[l.first_member as usize..l.first_member as usize + l.n_members as usize] {
            if let MemberPath::Piece(p) = m.path {
                letter_of[p as usize] = i as u32;
            }
        }
    }
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
        area: areas(page, &lp.page, &d.pieces, &letter_of)?,
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
    // How many times each entry of `KNOWN_LEFT_OUT` matched over the run.
    let known: Vec<usize> = (0..KNOWN_LEFT_OUT.len()).map(|i| rows.iter().map(|r| r.area.known[i]).sum()).collect();
    let n_known: usize = known.iter().sum();
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
            "area check, allowed ratio {MAX_AREA_RATIO}: (a) worst {:.3} on page {} path {}; (b) worst {:.3} on page {} path {}; {turned} pieces turn against their source; {n_known} known sources leave out a part",
            wa.area.worst_a, wa.page, wa.area.worst_a_path, wb.area.worst_b, wb.page, wb.area.worst_b_path
        );
    }
    // The partner rule.
    let sides: usize = rows.iter().map(|r| r.area.sides).sum();
    let with_sides: usize = rows.iter().map(|r| r.area.sources_with_sides).sum();
    let alone: Vec<(&str, &Alone)> =
        rows.iter().flat_map(|r| r.area.alone.iter().map(move |x| (r.page.as_str(), x))).collect();
    if let Some(w) = worst_by(|a| a.worst_partner) {
        println!(
            "partner: {sides} cut sides in {with_sides} sources, limit {:.4} units; worst side with a partner {:.4} on page {} path {}; {} sides with no partner",
            partner_limit(100),
            w.area.worst_partner,
            w.page,
            w.area.worst_partner_path,
            alone.len()
        );
    }
    for (page, (path, word, k, d)) in &alone {
        let d = if *d >= CELL { format!(">= {CELL}") } else { format!("{d:.4}") };
        println!("  no partner: page {page} path {path} word {word} piece {k}: {d} units");
    }
    let against: usize = rows.iter().map(|r| r.area.nested_against).sum();
    let with: Vec<&String> = rows.iter().flat_map(|r| &r.area.nested_with).collect();
    let undecided: usize = rows.iter().map(|r| r.area.nested_undecided).sum();
    println!(
        "nesting: {against} nested subpaths wind against their holder, {} with it, {undecided} undecided",
        with.len()
    );
    let opposite: usize = rows.iter().map(|r| r.area.opposite).sum();
    let same_way: usize = rows.iter().map(|r| r.area.same_way).sum();
    let kept: usize = rows.iter().map(|r| r.area.kept).sum();
    let kept_turned: Vec<&String> = rows.iter().flat_map(|r| &r.area.kept_turned).collect();
    let kept_turned_n: usize = rows.iter().map(|r| r.area.kept_turned_n).sum();
    println!(
        "direction: {opposite} cuts drawn once each way, {same_way} drawn twice the same way; {kept} source segments drawn as often as the source, {kept_turned_n} of them run another way, in {} sources",
        kept_turned.len()
    );
    for w in kept_turned.iter().take(20) {
        println!("  turned: {w}");
    }

    let mut area_failures: Vec<String> = rows.iter().flat_map(|r| r.area.failures.iter().cloned()).collect();
    area_failures.extend(with.iter().map(|w| w.to_string()));
    // An entry of `KNOWN_LEFT_OUT` whose page is in the run must match once.
    for (k, n) in KNOWN_LEFT_OUT.iter().zip(&known) {
        if rows.iter().any(|r| r.page == k.0) && *n != 1 {
            area_failures.push(format!("page {}: path {}: KNOWN_LEFT_OUT matched {n} times, not once", k.0, k.1));
        }
    }
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
/// With `QVP_LETTERS_SWEEP=1`, every piece of every source then gets each change, and the
/// test prints how many pass. A change that moves less ink than the allowance of its
/// source (`EDGE` × L) can pass unless the segment count, the partner rule or the
/// direction rule sees it, so the test does not assert that sweep. The sweep takes most of
/// the test's time, so it runs only when asked.
#[test]
fn the_area_check_catches_a_lost_or_extra_piece() {
    if !have_letters() {
        return;
    }
    const NAMES: [&str; 3] = ["lost", "doubled", "replaced"];
    let sweep = std::env::var("QVP_LETTERS_SWEEP").is_ok();
    /// Per change in the sweep: made, caught by (a), by (b), by the segment count, by the
    /// partner rule, by the direction rule, and caught by none, with the most ink, in page units squared, that
    /// such a change moves.
    #[derive(Default, Clone, Copy)]
    struct Row {
        made: usize,
        a: usize,
        b: usize,
        segments: usize,
        partner: usize,
        direction: usize,
        none: usize,
        largest_missed: f64,
    }
    let one = |page: &String| -> Result<[Row; 3], String> {
        let lp = build(page)?;
        lp.data.check_against(&lp.page).map_err(|e| format!("page {page}: check_against: {e:?}"))?;
        let quant = lp.page.header.quant;
        let pieces = &lp.data.pieces;
        let fails = |src: u32, run: &[PieceRec]| -> Result<bool, String> {
            let cmds = cmds_of(page, &lp.page, src)?;
            Ok(SourceArea::new(&Source::new(&cmds, quant), area(&cmds), run).failure().is_some())
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
        if !sweep {
            return Ok(t);
        }
        for run in pieces.chunk_by(|a, b| a.source == b.source) {
            let src = cmds_of(page, &lp.page, run[0].source)?;
            let source = Source::new(&src, quant);
            let src_area = area(&src);
            let before = SourceArea::new(&source, src_area, run);
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
                    let s = SourceArea::new(&source, src_area, &v);
                    let row = &mut t[c];
                    row.made += 1;
                    row.a += (s.ratio_a() > MAX_AREA_RATIO) as usize;
                    row.b += (s.ratio_b() > MAX_AREA_RATIO) as usize;
                    row.segments += s.split.extra.is_some() as usize;
                    row.partner += !s.split.alone.is_empty() as usize;
                    row.direction += s.direction_failure().is_some() as usize;
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
                    sum.partner += add.partner;
                    sum.direction += add.direction;
                    sum.none += add.none;
                    sum.largest_missed = sum.largest_missed.max(add.largest_missed);
                }
            }
            Err(e) => errors.push(e),
        }
    }
    if sweep {
        for (name, r) in NAMES.iter().zip(total) {
            println!(
                "{name:>8}: {} changes; (a) catches {}, (b) {}, the segment count {}, the partner rule {}, the direction rule {}; {} pass, moving at most {:.4} units²",
                r.made, r.a, r.b, r.segments, r.partner, r.direction, r.none, r.largest_missed
            );
        }
    }
    assert!(errors.is_empty(), "{errors:#?}");
}
