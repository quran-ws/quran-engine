//! The engine over real letter layers. Each test builds the selected pages and their layers
//! in memory from `pages/` and `pages-letters/`, loads them into the engine, and checks what
//! the letter feature rests on: the attach, the letter selectors, the swap of a cut path for
//! its pieces, the letter hit test on ink, on seams and in counters, printed and reflowed,
//! and the offsets against the `qpc` text.
//!
//! No data release carries a layer, and `dist/pages` is built from another source release
//! than `pages-letters/`. So the test never pairs `dist/` files: it builds both files here.
//!
//! Run a subset:  QVP_TEST_PAGES=001,002,036 cargo test -p qvp-convert --release --test letters_engine
//! Run all 604:   QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test letters_engine -- --nocapture
//!
//! `QVP_LETTERS_DIR` points the test at another copy of the letters archive, relative to the
//! repository. Without the archive the test skips, unless `QVP_REQUIRE_LETTERS` is set.
//!
//! A failure prints the page, the word key, the letter index and the path, and every test
//! prints its counts. Over all 604 pages the tests assert the counts, so a change that checks
//! fewer things fails too.

use qvp_convert::letter_page::{letter_page, LetterPage};
use qvp_convert::{convert, merge_words_index, words_json};
use qvp_core::{
    Draw, HitOptions, LayoutSpec, LetterHit, LetterMember, LetterPart, Page, Paint, ReflowSpec, Selector, NONE,
};
use qvp_format::letters::{encode_letters, NO_AT};
use rayon::prelude::*;
use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// The totals over all 604 pages of the Hafs data. The first two are the structure gate's.
const ALL_PAGES: usize = 604;
const ALL_LETTERS: usize = 333_220;
const ALL_PIECES: usize = 261_953;
/// Letters whose offset passes the text check, so `WordChars` can address them.
const ALL_CHECKED_LETTERS: usize = 333_220;
/// Marks addressed one by one with `LetterMark`.
const ALL_LETTER_MARKS: usize = 430_453;
/// Cut paths that draw their pieces when one letter of each word takes a colour.
const ALL_SWAPPED: usize = 73_793;
/// Cut paths swapped one at a time, at most 20 on a page.
const ALL_SWAPPED_ONE_BY_ONE: usize = 12_067;
/// Of those first 20 cut paths of each page, the ones whose pieces are all one letter's, so
/// a rule on that letter takes the path whole: each is a cut path with one piece.
const ALL_TAKEN_WHOLE: usize = 13;
/// Body members hit at a point inside them, and those with no clear point at the scan's resolution.
const ALL_BODY_HITS: usize = 335_548;
const ALL_BODY_SKIPS: usize = 0;
/// Pieces whose box centre is inside them, hit at that centre.
const ALL_PIECE_CENTRES: usize = 86_814;
/// Pieces among the body members hit inside.
const ALL_PIECE_HITS: usize = 261_953;
/// Grid points on word ink, printed, and the seam hits among them: a cut path holds the
/// point and no piece does.
const ALL_INK_POINTS: usize = 39_674_994;
const ALL_SEAM_HITS: usize = 23_379;
/// Grid points on word ink under the reflow at `REFLOW_ZOOM`, through the view call, the
/// seam hits among them, and the points the round trip through the view moves off the ink.
const ALL_REFLOW_POINTS: usize = 39_674_298;
const ALL_REFLOW_SEAM_HITS: usize = 23_333;
const ALL_REFLOW_LOST: usize = 696;
/// Counters (a nested subpath of a body member that the member does not fill) hit at a
/// point inside them, and those with no point clear of all ink at the scan's resolution.
const ALL_COUNTERS: usize = 104_835;
const ALL_COUNTER_SKIPS: usize = 0;

/// The grid step of the ink tests, in page units.
const INK_STEP: f32 = 0.5;
/// The zoom of the reflow the ink test lays out.
const REFLOW_ZOOM: f32 = 2.0;

/// The phone layout of the draw lists the tests compare.
const PHONE: LayoutSpec = LayoutSpec {
    viewport_w: 390.0,
    viewport_h: 844.0,
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
    banner_zoom: 0.0,
    surah_frames: true,
};

const RED: u32 = 0xd0_20_20_ff;

/// The failures to print per page and test. A line gives the count of the rest.
const MAX_PRINTED: usize = 25;

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
        "skipped  letter engine test (no {}/); run QVP_SYNC=letters scripts/sync-test-data.sh to enable it",
        letters_dir().display()
    );
    false
}

/// The bytes the engine attaches. One place, so a change of the layer's encoding is one line.
fn layer_bytes(lp: &LetterPage) -> Vec<u8> {
    encode_letters(&lp.data)
}

/// What the tests need of one page, built once for all of them.
struct Built {
    qvp: Vec<u8>,
    layer: Vec<u8>,
    /// The words sidecar, as `batch` writes `NNN.words.json`.
    words: Vec<u8>,
    /// The `qpc` text of each word key.
    qpc: HashMap<String, String>,
    n_letters: usize,
    /// Per piece: the page path it is cut from.
    piece_source: Vec<u32>,
    /// Per letter, in the layer's order: the offsets the layer stores for its members, its
    /// own offset and its `more_at` included, leaving out `NO_AT`.
    stored_at: Vec<usize>,
    /// The word marks: (path, the offset the layer stores).
    word_marks: Vec<(u32, u16)>,
}

fn build(page: &str) -> Result<Built, String> {
    let ld = letters_dir();
    let words_svg = repo().join("pages").join(format!("{page}.svg"));
    let words_index = repo().join("index/by-page").join(format!("{page}.json"));
    let lp = letter_page(
        &ld.join(format!("{page}.svg")),
        &words_svg,
        &words_index,
        &ld.join("index/by-page").join(format!("{page}.json")),
    )
    .map_err(|e| format!("page {page}: {e}"))?;

    // the sidecar as `batch` writes it: the page's own forms, filled from the words index
    let svg = std::fs::read_to_string(&words_svg).map_err(|e| format!("page {page}: {e}"))?;
    let mut c = convert(&svg).map_err(|e| format!("page {page}: {e}"))?;
    let index = std::fs::read(&words_index).map_err(|e| format!("page {page}: {e}"))?;
    let mut warnings = vec![];
    merge_words_index(&mut c.words_text, &index, &mut warnings);
    if !warnings.is_empty() {
        return Err(format!("page {page}: words index: {warnings:?}"));
    }
    let qpc = c.words_text.iter().map(|w| (w.word_key.clone(), w.qpc.clone())).collect();

    let d = &lp.data;
    let stored_at = d
        .letters
        .iter()
        .map(|l| {
            let members = &d.members[l.first_member as usize..l.first_member as usize + l.n_members as usize];
            members.iter().filter(|m| m.at != NO_AT).count() + l.more_at.iter().filter(|&&a| a != NO_AT).count()
        })
        .collect();
    Ok(Built {
        qvp: qvp_format::encode(&lp.page),
        layer: layer_bytes(&lp),
        words: words_json(&c.words_text).into_bytes(),
        qpc,
        n_letters: d.letters.len(),
        piece_source: d.pieces.iter().map(|p| p.source).collect(),
        stored_at,
        word_marks: d.word_marks.iter().map(|w| (w.path, w.at)).collect(),
    })
}

/// Every selected page, built once and shared by the tests of this file.
fn built() -> &'static Vec<(String, Result<Built, String>)> {
    static BUILT: OnceLock<Vec<(String, Result<Built, String>)>> = OnceLock::new();
    BUILT.get_or_init(|| {
        let t0 = std::time::Instant::now();
        let v: Vec<_> = selected_pages().par_iter().map(|p| (p.clone(), build(p))).collect();
        println!("built {} pages and their layers in {:.1} s", v.len(), t0.elapsed().as_secs_f64());
        v
    })
}

/// The page with its words sidecar, and with its layer when `layer` is true.
fn load(b: &Built, layer: bool) -> Result<Page, String> {
    let mut p = Page::load(&b.qvp).map_err(|e| format!("load: {e:?}"))?;
    p.attach_words_json(&b.words).ok_or("the words sidecar does not parse")?;
    if layer {
        p.attach_letters(&b.layer).map_err(|e| format!("attach_letters: {e:?}"))?;
    }
    Ok(p)
}

/// The failures of one page, with a cap on what is kept for printing.
#[derive(Default)]
struct Fails {
    n: usize,
    lines: Vec<String>,
}

impl Fails {
    fn push(&mut self, s: String) {
        self.n += 1;
        if self.lines.len() < MAX_PRINTED {
            self.lines.push(s);
        }
    }
}

/// The counts one page gives a test, or why it could not be checked.
type Counts<const N: usize> = Result<[usize; N], String>;

/// Run `check` over every selected page in parallel, print each page's failures, sum the
/// counts and fail when a page failed. Returns the summed counts and the page count.
fn run<const N: usize>(
    name: &str,
    check: impl Fn(&Built, &mut Fails) -> Result<[usize; N], String> + Sync,
) -> ([usize; N], usize) {
    let t0 = std::time::Instant::now();
    let pages = built();
    let results: Vec<(&str, Counts<N>, Fails)> = pages
        .par_iter()
        .map(|(page, b)| {
            let mut f = Fails::default();
            let r = match b {
                Ok(b) => check(b, &mut f),
                Err(e) => Err(e.clone()),
            };
            (page.as_str(), r, f)
        })
        .collect();
    let mut total = [0usize; N];
    let mut failed = vec![];
    for (page, r, f) in &results {
        match r {
            Ok(c) => total.iter_mut().zip(c).for_each(|(t, c)| *t += c),
            Err(e) => {
                eprintln!("FAIL {name}: page {page}: {e}");
                failed.push(*page);
            }
        }
        for l in &f.lines {
            eprintln!("FAIL {name}: page {page}: {l}");
        }
        if f.n > f.lines.len() {
            eprintln!("FAIL {name}: page {page}: {} more failures", f.n - f.lines.len());
        }
        if f.n > 0 && r.is_ok() {
            failed.push(*page);
        }
    }
    println!("{name}: {} pages in {:.1} s, {} failed", results.len(), t0.elapsed().as_secs_f64(), failed.len());
    assert!(failed.is_empty(), "{name} failed on pages {failed:?}; see the FAIL lines");
    (total, results.len())
}

fn all_pages() -> bool {
    std::env::var("QVP_TEST_ALL").is_ok()
}

fn key(p: &Page, w: u32) -> String {
    let d = &p.data().words[w as usize];
    format!("{}:{}:{}", d.surah, d.ayah, d.word)
}

/// The letter facts of a page with its layer, read through the engine's API.
struct Index {
    /// The page's own path count.
    n: u32,
    /// Per piece: its source.
    source: Vec<u32>,
    /// Per cut path: its pieces, in order.
    pieces_of: HashMap<u32, Vec<u32>>,
    /// The cut paths, ascending.
    cut: Vec<u32>,
    /// Per piece: (word, letter index in the word).
    piece_letter: Vec<Option<(u32, u32)>>,
}

fn index(p: &Page, b: &Built) -> Index {
    let n = p.data().paths.len() as u32;
    let mut pieces_of: HashMap<u32, Vec<u32>> = HashMap::new();
    for (i, &s) in b.piece_source.iter().enumerate() {
        pieces_of.entry(s).or_default().push(n + i as u32);
    }
    let mut cut: Vec<u32> = pieces_of.keys().copied().collect();
    cut.sort_unstable();
    let mut piece_letter = vec![None; b.piece_source.len()];
    for w in 0..p.data().words.len() as u32 {
        for l in 0..p.letter_count(w) {
            for m in p.letter_members(w, l) {
                if m.is_cut && m.path >= n && ((m.path - n) as usize) < piece_letter.len() {
                    piece_letter[(m.path - n) as usize] = Some((w, l));
                }
            }
        }
    }
    Index { n, source: b.piece_source.clone(), pieces_of, cut, piece_letter }
}

/// The paths that take a rule: every path the rule paints red. A cut path whose pieces all
/// take it takes the paint too, so it is in the set.
fn selected(p: &mut Page, sel: Selector) -> BTreeSet<u32> {
    let h = p.style(sel, Paint::new(RED));
    let set = p.colors().iter().enumerate().filter(|(_, &c)| c == RED).map(|(i, _)| i as u32).collect();
    p.remove_style(h);
    set
}

fn draws(p: &mut Page) -> Vec<Draw> {
    p.refresh_display();
    p.layout_draw_list()
}

/// The first entry where two draw lists differ, for a failure line.
fn first_difference(a: &[Draw], b: &[Draw]) -> String {
    let i = a.iter().zip(b).position(|(x, y)| x != y).unwrap_or(a.len().min(b.len()));
    format!("lengths {} and {}; first difference at entry {i}: {:?} and {:?}", a.len(), b.len(), a.get(i), b.get(i))
}

/// The draw list `plain` with each cut path in `swapped` replaced by its pieces, in place
/// and under the same placement.
fn with_pieces(plain: &[Draw], swapped: &HashSet<u32>, ix: &Index) -> Vec<Draw> {
    let mut out = Vec::with_capacity(plain.len());
    for d in plain {
        match ix.pieces_of.get(&d.path) {
            Some(pieces) if swapped.contains(&d.path) => {
                out.extend(pieces.iter().map(|&path| Draw { path, placement: d.placement }))
            }
            _ => out.push(*d),
        }
    }
    out
}

/// The cut paths that draw their pieces when exactly the letters in `coloured` take a
/// colour: those with a piece of such a letter and a piece of another letter.
fn expected_swaps(coloured: &HashSet<(u32, u32)>, ix: &Index) -> HashSet<u32> {
    ix.cut
        .iter()
        .copied()
        .filter(|s| {
            let pieces = &ix.pieces_of[s];
            let of = |p: &u32| ix.piece_letter[(p - ix.n) as usize].is_some_and(|l| coloured.contains(&l));
            pieces.iter().any(of) && !pieces.iter().all(of)
        })
        .collect()
}

/// Attach the layer to a laid-out page. Nothing that is drawn changes.
///
/// (a) The attach keeps every geometry row, op and point of the page's own paths, adds one
/// row per piece, and indexes as many letters as the layer holds. (b) With no rule, the
/// draw list is the one before the attach and the one of the page with no layer, entry for
/// entry. The colours of the page's paths are the same, and no path is styled.
#[test]
fn attaching_changes_no_drawing() {
    if !have_letters() {
        return;
    }
    let ([letters, pieces], n_pages) = run("attaching_changes_no_drawing", |b, f| {
        let mut plain = load(b, false)?;
        plain.layout(&PHONE);
        let plain_list = draws(&mut plain);
        let mut p = load(b, false)?;
        p.layout(&PHONE);
        let before_list = draws(&mut p);
        let (rows, ops, pts) = {
            let g = p.geometry();
            (g.table.clone(), g.ops.clone(), g.pts.clone())
        };
        p.attach_letters(&b.layer).map_err(|e| format!("attach_letters: {e:?}"))?;
        if !p.has_letters() {
            f.push("has_letters is false after the attach".into());
        }
        let n = p.data().paths.len();
        let g = p.geometry();
        if g.table.len() != rows.len() + b.piece_source.len() {
            f.push(format!(
                "path count {} after the attach, {} before and {} pieces",
                g.table.len(),
                rows.len(),
                b.piece_source.len()
            ));
        }
        if rows.len() != n {
            f.push(format!("{} geometry rows before the attach, {n} page paths", rows.len()));
        }
        for (i, (a, z)) in rows.iter().zip(&g.table).enumerate() {
            let same = (a.op_start, a.op_count, a.pt_start, a.pt_count, a.flags, a.word, a.line, a.extra)
                == (z.op_start, z.op_count, z.pt_start, z.pt_count, z.flags, z.word, z.line, z.extra);
            if !same {
                f.push(format!("path {i}: the geometry row changed: {a:?} to {z:?}"));
            }
        }
        if g.ops.get(..ops.len()) != Some(&ops[..]) {
            f.push(format!("the ops of the page's paths changed ({} ops before)", ops.len()));
        }
        let same_pts = g.pts.len() >= pts.len() && g.pts.iter().zip(&pts).all(|(a, z)| a.to_bits() == z.to_bits());
        if !same_pts {
            f.push(format!("the points of the page's paths changed ({} values before)", pts.len()));
        }
        let engine_letters: u32 = (0..p.data().words.len() as u32).map(|w| p.letter_count(w)).sum();
        if engine_letters as usize != b.n_letters {
            f.push(format!("the engine indexes {engine_letters} letters, the layer holds {}", b.n_letters));
        }

        let after_list = draws(&mut p);
        if after_list != before_list {
            f.push(format!("the draw list changed at the attach: {}", first_difference(&before_list, &after_list)));
        }
        if after_list != plain_list {
            f.push(format!(
                "the draw list differs from the page with no layer: {}",
                first_difference(&plain_list, &after_list)
            ));
        }
        let colours = p.colors()[..n].to_vec();
        let plain_colours = plain.colors().to_vec();
        if colours != plain_colours {
            let i = colours.iter().zip(&plain_colours).position(|(a, z)| a != z).unwrap_or(0);
            f.push(format!("path {i}: colour {:08x}, the page with no layer has {:08x}", colours[i], plain_colours[i]));
        }
        let styled = p.styled_paths();
        if !styled.is_empty() {
            f.push(format!("{} styled paths with no rule, the first {:?}", styled.len(), styled[0]));
        }
        Ok([b.n_letters, b.piece_source.len()])
    });
    println!("totals: {n_pages} pages, {letters} letters, {pieces} pieces");
    if all_pages() {
        assert_eq!(n_pages, ALL_PAGES, "page count");
        assert_eq!(letters, ALL_LETTERS, "letters over all pages");
        assert_eq!(pieces, ALL_PIECES, "pieces over all pages");
    }
}

/// With the words sidecar, every offset the layer stores passes the text check: each
/// letter's own offset, its `more_at`, every member's and every word mark's. The character
/// at a letter's offset is the letter's own.
#[test]
fn every_offset_fits_its_text() {
    if !have_letters() {
        return;
    }
    let ([letters], _) = run("every_offset_fits_its_text", |b, f| {
        let p = load(b, true)?;
        let mut g = 0usize;
        for w in 0..p.data().words.len() as u32 {
            let k = key(&p, w);
            let text: Vec<char> = b.qpc.get(&k).map(|t| t.chars().collect()).unwrap_or_default();
            if text.is_empty() {
                f.push(format!("word {k}: the sidecar has no qpc text"));
            }
            for l in 0..p.letter_count(w) {
                let Some(info) = p.letter_info(w, l) else {
                    f.push(format!("word {k} letter {l}: no letter_info"));
                    continue;
                };
                if info.at == NONE {
                    f.push(format!("word {k} letter {l}: its offset fails the text check"));
                } else if text.get(info.at as usize) != Some(&info.text) {
                    f.push(format!(
                        "word {k} letter {l}: the letter is {:?}, the qpc text has {:?} at {}",
                        info.text,
                        text.get(info.at as usize),
                        info.at
                    ));
                }
                for &a in &info.more_at {
                    if a == NONE {
                        f.push(format!("word {k} letter {l}: a more_at offset fails the text check"));
                    }
                }
                let members = p.letter_members(w, l);
                let checked = members.iter().filter(|m| m.at != NONE).count()
                    + info.more_at.iter().filter(|&&a| a != NONE).count();
                match b.stored_at.get(g) {
                    Some(&stored) if stored == checked => {}
                    stored => f.push(format!(
                        "word {k} letter {l}: the layer stores {stored:?} offsets, {checked} pass the text check"
                    )),
                }
                g += 1;
            }
        }
        if g != b.n_letters {
            f.push(format!("{g} letters read, the layer holds {}", b.n_letters));
        }
        for &(path, at) in &b.word_marks {
            match p.path_letter(path) {
                Some((letter, checked)) if letter == NONE && (at == NO_AT) == (checked == NONE) => {}
                r => f.push(format!("word mark path {path} (stored offset {at}): path_letter gives {r:?}")),
            }
        }
        Ok([g])
    });
    println!("totals: {letters} letters read against their qpc text");
    if all_pages() {
        assert_eq!(letters, ALL_LETTERS, "letters over all pages");
    }
}

/// The letter selectors take what the members say. For every letter:
///
/// - `Letter(All)` takes its members, and a cut path whose pieces are all its members.
/// - `BodyAndDots` and `Marks` split `All` in two.
/// - `BodyAndDots` takes what `WordChars(at, at)` takes.
/// - `LetterMark(n)` takes the nth mark of `letter_members`. The marks come after the body,
///   in offset order.
/// - `path_letter` of each member names the letter.
#[test]
fn letters_address_their_paths() {
    if !have_letters() {
        return;
    }
    let ([checked, marks], _) = run("letters_address_their_paths", |b, f| {
        let mut p = load(b, true)?;
        let ix = index(&p, b);
        let (mut checked, mut n_marks) = (0, 0);
        for w in 0..p.data().words.len() as u32 {
            let k = key(&p, w);
            for l in 0..p.letter_count(w) {
                let at = format!("word {k} letter {l}");
                let Some(info) = p.letter_info(w, l) else {
                    f.push(format!("{at}: no letter_info"));
                    continue;
                };
                let members = p.letter_members(w, l);
                let lw = l as u16;
                let all = selected(&mut p, Selector::Letter(w, lw, LetterPart::All));
                let body = selected(&mut p, Selector::Letter(w, lw, LetterPart::BodyAndDots));
                let rest = selected(&mut p, Selector::Letter(w, lw, LetterPart::Marks));

                let paths: BTreeSet<u32> = members.iter().map(|m| m.path).collect();
                let mut want = paths.clone();
                for s in &ix.cut {
                    if ix.pieces_of[s].iter().all(|q| paths.contains(q)) {
                        want.insert(*s);
                    }
                }
                if all != want {
                    f.push(format!("{at}: Letter(All) takes {all:?}, the members are {want:?}"));
                }
                if !body.is_disjoint(&rest) {
                    f.push(format!("{at}: BodyAndDots {body:?} and Marks {rest:?} share paths"));
                }
                if body.union(&rest).copied().collect::<BTreeSet<u32>>() != all {
                    f.push(format!("{at}: BodyAndDots {body:?} and Marks {rest:?} do not make All {all:?}"));
                }
                if info.at != NONE {
                    let a = info.at as u16;
                    let chars = selected(&mut p, Selector::WordChars(w, a, a));
                    if chars != body {
                        f.push(format!("{at}: WordChars({a}, {a}) takes {chars:?}, BodyAndDots {body:?}"));
                    }
                    checked += 1;
                }

                if members.len() != info.n_members as usize {
                    f.push(format!("{at}: {} members, letter_info says {}", members.len(), info.n_members));
                }
                let first_mark = members.iter().position(|m| !m.is_body).unwrap_or(members.len());
                if members[first_mark..].iter().any(|m| m.is_body) {
                    f.push(format!("{at}: a body member comes after a mark: {members:?}"));
                }
                let marks: Vec<&LetterMember> = members.iter().filter(|m| !m.is_body).collect();
                if marks.len() != info.n_marks as usize {
                    f.push(format!("{at}: {} marks, letter_info says {}", marks.len(), info.n_marks));
                }
                // NONE is the largest offset, so the order also puts a mark with no offset last
                if marks.windows(2).any(|m| m[0].at > m[1].at) {
                    f.push(format!("{at}: the marks are not in offset order: {marks:?}"));
                }
                for (n, m) in marks.iter().enumerate() {
                    let one = selected(&mut p, Selector::LetterMark(w, lw, n as u16));
                    if one.len() != 1 || !one.contains(&m.path) {
                        f.push(format!("{at}: LetterMark({n}) takes {one:?}, the nth mark is path {}", m.path));
                    }
                    n_marks += 1;
                }
                let past = selected(&mut p, Selector::LetterMark(w, lw, marks.len() as u16));
                if !past.is_empty() {
                    f.push(format!("{at}: LetterMark({}) past the last mark takes {past:?}", marks.len()));
                }
                for m in &members {
                    if p.path_letter(m.path) != Some((l, m.at)) {
                        f.push(format!(
                            "{at}: path {} (offset {}): path_letter gives {:?}",
                            m.path,
                            m.at,
                            p.path_letter(m.path)
                        ));
                    }
                    if p.geometry().table.get(m.path as usize).map(|r| r.word) != Some(w) {
                        f.push(format!("{at}: path {} is not a path of the word", m.path));
                    }
                }
            }
        }
        for &s in &ix.cut {
            if p.path_letter(s).is_some() {
                f.push(format!("cut path {s}: path_letter gives {:?}, not None", p.path_letter(s)));
            }
        }
        if ix.piece_letter.iter().any(Option::is_none) {
            let i = ix.piece_letter.iter().position(Option::is_none).unwrap_or(0);
            f.push(format!("piece {} (path {}) is no letter's member", i, ix.n as usize + i));
        }
        Ok([checked, n_marks])
    });
    println!("totals: {checked} letters with WordChars(at, at) = Letter(BodyAndDots), {marks} marks by LetterMark");
    if all_pages() {
        assert_eq!(checked, ALL_CHECKED_LETTERS, "letters checked against WordChars over all pages");
        assert_eq!(marks, ALL_LETTER_MARKS, "marks checked by LetterMark over all pages");
    }
}

/// Paint the letters `coloured` red, check the draw list against `plain` with the expected
/// cut paths swapped, then remove the rules and check that the list is `plain` again.
/// Returns how many cut paths swapped.
fn swap_and_back(
    p: &mut Page,
    plain: &[Draw],
    ix: &Index,
    what: &str,
    coloured: &HashSet<(u32, u32)>,
    f: &mut Fails,
) -> usize {
    let rev = p.draw_revision();
    let handles: Vec<u32> = coloured
        .iter()
        .map(|&(w, l)| p.style(Selector::Letter(w, l as u16, LetterPart::All), Paint::new(RED)))
        .collect();
    let list = draws(p);
    let swapped = expected_swaps(coloured, ix);
    let want = with_pieces(plain, &swapped, ix);
    if list != want {
        let held: HashSet<u32> = list.iter().map(|d| d.path).collect();
        let mut whole: Vec<u32> = swapped.iter().copied().filter(|s| held.contains(s)).collect();
        whole.sort_unstable();
        let mut extra: Vec<u32> = ix
            .cut
            .iter()
            .copied()
            .filter(|s| !swapped.contains(s) && ix.pieces_of[s].iter().any(|q| held.contains(q)))
            .collect();
        extra.sort_unstable();
        f.push(format!(
                "{what}: the draw list is not the plain one with {} paths swapped ({}); whole that should swap {whole:?}, swapped that should not {extra:?}",
                swapped.len(),
                first_difference(&want, &list)
            ));
    }
    if !swapped.is_empty() && p.draw_revision() == rev {
        f.push(format!("{what}: {} paths swapped and the draw revision stayed {rev}", swapped.len()));
    }
    let held: HashSet<u32> = list.iter().map(|d| d.path).collect();
    let styled = p.styled_paths();
    if let Some(s) = styled.iter().find(|(path, _)| !held.contains(path)) {
        f.push(format!("{what}: styled path {} is not in the draw list", s.0));
    }
    // a cut path whose pieces all take the colour is drawn whole in it
    let colours = p.colors();
    for s in &ix.cut {
        let all =
            ix.pieces_of[s].iter().all(|q| ix.piece_letter[(q - ix.n) as usize].is_some_and(|l| coloured.contains(&l)));
        if all && colours[*s as usize] != RED {
            f.push(format!("{what}: cut path {s} has every piece coloured and is {:08x}", colours[*s as usize]));
        }
    }
    for h in handles {
        p.remove_style(h);
    }
    let back = draws(p);
    if back != plain {
        f.push(format!("{what}: with the rules gone the list differs: {}", first_difference(plain, &back)));
    }
    swapped.len()
}

/// Swap cut paths and back. First the first 20 cut paths of the page, one at a time: a rule
/// on the letter of the path's first piece. Then one letter of every word that has a cut
/// path of two letters or more, all at once. Each time, the draw list holds the pieces of
/// every swapped path in its place and under its placement, never the path with a piece of
/// it, and every other entry as before. The draw revision changes, and the styled paths are
/// all in the draw list. With the rules gone, the list is the plain one again.
#[test]
fn one_letter_colour_swaps_one_path_and_back() {
    if !have_letters() {
        return;
    }
    let ([one_by_one, together, whole], _) = run("one_letter_colour_swaps_one_path_and_back", |b, f| {
        let mut p = load(b, true)?;
        p.layout(&PHONE);
        let ix = index(&p, b);
        let plain = draws(&mut p);

        let swap = |p: &mut Page, what: &str, coloured: &HashSet<(u32, u32)>, f: &mut Fails| {
            swap_and_back(p, &plain, &ix, what, coloured, f)
        };

        let (mut one_by_one, mut whole) = (0, 0);
        for &s in ix.cut.iter().take(20) {
            let first = ix.pieces_of[&s][0];
            let Some((w, l)) = ix.piece_letter[(first - ix.n) as usize] else {
                f.push(format!("cut path {s}: its first piece {first} is no letter's member"));
                continue;
            };
            let what = format!("cut path {s} (word {} letter {l})", key(&p, w));
            let coloured = HashSet::from([(w, l)]);
            let swapped = expected_swaps(&coloured, &ix);
            if swapped.contains(&s) {
                one_by_one += 1;
            } else {
                // every piece of the path is this letter's: the rule takes the path whole
                whole += 1;
            }
            swap(&mut p, &what, &coloured, f);
        }

        // the first letter of each word that has a piece of a cut path of two letters or more
        let mut coloured = HashSet::new();
        for w in 0..p.data().words.len() as u32 {
            let first = (0..p.letter_count(w)).find(|&l| {
                p.letter_members(w, l).iter().any(|m| {
                    m.is_cut && {
                        let s = ix.source[(m.path - ix.n) as usize];
                        let letters: HashSet<_> =
                            ix.pieces_of[&s].iter().map(|q| ix.piece_letter[(q - ix.n) as usize]).collect();
                        letters.len() > 1
                    }
                })
            });
            if let Some(l) = first {
                coloured.insert((w, l));
            }
        }
        let together = swap(&mut p, "one letter of each word", &coloured, f);
        if !coloured.is_empty() && together == 0 {
            f.push(format!("{} letters coloured and no cut path swapped", coloured.len()));
        }
        Ok([one_by_one, together, whole])
    });
    println!(
        "totals: {one_by_one} cut paths swapped one at a time ({whole} more taken whole by a rule on their one letter), {together} swapped by one letter of each word"
    );
    if all_pages() {
        assert_eq!(one_by_one, ALL_SWAPPED_ONE_BY_ONE, "cut paths swapped one at a time over all pages");
        assert_eq!(whole, ALL_TAKEN_WHOLE, "cut paths taken whole by a rule on their one letter over all pages");
        assert_eq!(together, ALL_SWAPPED, "cut paths swapped together over all pages");
    }
}

/// Boxes of the drawn ink of a page, in page units, in a grid of cells, to find the paths
/// that hold a point.
struct Ink {
    cell: f32,
    cols: usize,
    cells: Vec<Vec<u32>>,
    boxes: Vec<[f32; 4]>,
}

impl Ink {
    /// Every path but the cut paths, whose pieces stand for them, and every piece.
    fn new(p: &Page, ix: &Index) -> Ink {
        let g = p.geometry();
        let boxes: Vec<[f32; 4]> = g
            .table
            .iter()
            .map(|r| {
                let pts = &g.pts[r.pt_start as usize..(r.pt_start + r.pt_count) as usize];
                pts.chunks(2).fold([f32::MAX, f32::MAX, f32::MIN, f32::MIN], |b, c| {
                    [b[0].min(c[0]), b[1].min(c[1]), b[2].max(c[0]), b[3].max(c[1])]
                })
            })
            .collect();
        let cell = 8.0;
        let cols = (p.width() / cell) as usize + 1;
        let rows = (p.height() / cell) as usize + 1;
        let mut cells = vec![vec![]; cols * rows];
        let clamp = |v: f32, n: usize| ((v / cell).max(0.0) as usize).min(n - 1);
        for (i, b) in boxes.iter().enumerate() {
            if b[0] > b[2] || ix.pieces_of.contains_key(&(i as u32)) {
                continue;
            }
            for y in clamp(b[1], rows)..=clamp(b[3], rows) {
                for x in clamp(b[0], cols)..=clamp(b[2], cols) {
                    cells[y * cols + x].push(i as u32);
                }
            }
        }
        Ink { cell, cols, cells, boxes }
    }

    /// True when a path of the ink other than `path` holds the point.
    fn other_holds(&self, p: &Page, path: u32, x: f32, y: f32) -> bool {
        if x < 0.0 || y < 0.0 {
            return false;
        }
        let i = (y / self.cell) as usize * self.cols + (x / self.cell) as usize;
        self.cells.get(i).is_some_and(|c| {
            c.iter().any(|&q| {
                let b = self.boxes[q as usize];
                q != path && b[0] <= x && x <= b[2] && b[1] <= y && y <= b[3] && p.point_in_path(q, x, y)
            })
        })
    }

    /// The box centre, then the points of grids of 4, 8 and 16 cells over the box: the first
    /// that `path` holds and no other path of the ink holds. None when there is none.
    fn clear_point(&self, p: &Page, path: u32) -> Option<(f32, f32)> {
        let [x0, y0, x1, y1] = *self.boxes.get(path as usize)?;
        if x0 > x1 {
            return None;
        }
        let centre = [((x0 + x1) / 2.0, (y0 + y1) / 2.0)];
        let grid = [4u32, 8, 16].into_iter().flat_map(|n| {
            (1..n).flat_map(move |i| {
                (1..n).map(move |j| (x0 + (x1 - x0) * i as f32 / n as f32, y0 + (y1 - y0) * j as f32 / n as f32))
            })
        });
        centre.into_iter().chain(grid).find(|&(x, y)| p.point_in_path(path, x, y) && !self.other_holds(p, path, x, y))
    }
}

/// A point inside a letter's ink names that letter. (1) Each piece whose box centre is inside
/// it: the letter hit at that centre is exact, in the piece's word, and is the piece or a
/// mark of the same word. (2) Each body member of each letter, pieces included: at a point
/// inside it and inside no other ink, the letter hit is exact and names that letter and that
/// path, and its word is the word hit's. A member with no such point at the scan's
/// resolution counts as a skip.
#[test]
fn a_point_inside_a_piece_names_its_letter() {
    if !have_letters() {
        return;
    }
    let ([centres, hits, skips, piece_hits], _) = run("a_point_inside_a_piece_names_its_letter", |b, f| {
        let p = load(b, true)?;
        let ix = index(&p, b);
        let ink = Ink::new(&p, &ix);
        let opt = HitOptions::default();
        let (mut centres, mut hits, mut skips, mut piece_hits) = (0, 0, 0, 0);

        for (i, &(w, l)) in ix.piece_letter.iter().enumerate().filter_map(|(i, x)| x.as_ref().map(|x| (i, x))) {
            let path = ix.n + i as u32;
            let Some(&[x0, y0, x1, y1]) = ink.boxes.get(path as usize) else { continue };
            let (x, y) = ((x0 + x1) / 2.0, (y0 + y1) / 2.0);
            if x0 > x1 || !p.point_in_path(path, x, y) {
                continue;
            }
            centres += 1;
            let h = p.hit_test_letter(x, y, &opt);
            let ok = h.is_some_and(|h| {
                h.is_exact
                    && h.word == w
                    && (h.path == path
                        || (h.letter != NONE
                            && p.letter_members(w, h.letter).iter().any(|m| m.path == h.path && !m.is_body)))
            });
            if !ok {
                f.push(format!("piece path {path} (word {} letter {l}) at its centre ({x}, {y}): {h:?}", key(&p, w)));
            }
        }
        for w in 0..p.data().words.len() as u32 {
            for l in 0..p.letter_count(w) {
                for m in p.letter_members(w, l).iter().filter(|m| m.is_body) {
                    let Some((x, y)) = ink.clear_point(&p, m.path) else {
                        skips += 1;
                        continue;
                    };
                    let h = p.hit_test_letter(x, y, &opt);
                    let word = p.hit_test(x, y, &opt).map(|h| h.word);
                    let ok = h.is_some_and(|h| {
                        h.is_exact && h.word == w && h.letter == l && h.path == m.path && Some(h.word) == word
                    });
                    if ok {
                        hits += 1;
                        piece_hits += m.is_cut as usize;
                    } else {
                        f.push(format!(
                            "word {} letter {l} body path {} at ({x}, {y}): letter hit {h:?}, word hit {word:?}",
                            key(&p, w),
                            m.path
                        ));
                    }
                }
            }
        }
        Ok([centres, hits, skips, piece_hits])
    });
    println!(
        "totals: {centres} pieces hit at their centre, {hits} body members hit inside ({piece_hits} of them pieces), {skips} with no clear point"
    );
    if all_pages() {
        assert_eq!(centres, ALL_PIECE_CENTRES, "pieces hit at their centre over all pages");
        assert_eq!(hits, ALL_BODY_HITS, "body members hit over all pages");
        assert_eq!(skips, ALL_BODY_SKIPS, "body members with no clear point over all pages");
        assert_eq!(piece_hits, ALL_PIECE_HITS, "pieces hit inside over all pages");
    }
}

/// The ink of the page's words: every page path of a word, cut paths included, and every
/// piece, in a grid of cells, to find the words whose ink holds a point.
struct WordInk {
    cell: f32,
    cols: usize,
    rows: usize,
    /// (path, its word) per cell
    cells: Vec<Vec<(u32, u32)>>,
    boxes: Vec<[f32; 4]>,
}

impl WordInk {
    fn new(p: &Page) -> WordInk {
        let g = p.geometry();
        let boxes: Vec<[f32; 4]> = g
            .table
            .iter()
            .map(|r| {
                let pts = &g.pts[r.pt_start as usize..(r.pt_start + r.pt_count) as usize];
                pts.chunks(2).fold([f32::MAX, f32::MAX, f32::MIN, f32::MIN], |b, c| {
                    [b[0].min(c[0]), b[1].min(c[1]), b[2].max(c[0]), b[3].max(c[1])]
                })
            })
            .collect();
        let cell = 4.0;
        let cols = (p.width() / cell) as usize + 1;
        let rows = (p.height() / cell) as usize + 1;
        let mut cells = vec![vec![]; cols * rows];
        let clamp = |v: f32, n: usize| ((v / cell).max(0.0) as usize).min(n - 1);
        for (i, b) in boxes.iter().enumerate() {
            let word = g.table[i].word;
            if b[0] > b[2] || word == NONE {
                continue;
            }
            for y in clamp(b[1], rows)..=clamp(b[3], rows) {
                for x in clamp(b[0], cols)..=clamp(b[2], cols) {
                    cells[y * cols + x].push((i as u32, word));
                }
            }
        }
        WordInk { cell, cols, rows, cells, boxes }
    }

    /// The words whose ink holds the point, ascending, with no repeat.
    fn owners(&self, p: &Page, x: f32, y: f32) -> Vec<u32> {
        if !(x >= 0.0 && y >= 0.0) || x / self.cell >= self.cols as f32 || y / self.cell >= self.rows as f32 {
            return vec![];
        }
        let i = (y / self.cell) as usize * self.cols + (x / self.cell) as usize;
        let mut out: Vec<u32> = self.cells[i]
            .iter()
            .filter(|&&(q, _)| {
                let b = self.boxes[q as usize];
                b[0] <= x && x <= b[2] && b[1] <= y && y <= b[3] && p.point_in_path(q, x, y)
            })
            .map(|&(_, w)| w)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// Per path: true for a body member of a letter. Every other path is false.
fn body_roles(p: &Page) -> Vec<bool> {
    let mut role = vec![false; p.geometry().table.len()];
    for w in 0..p.data().words.len() as u32 {
        for l in 0..p.letter_count(w) {
            for m in p.letter_members(w, l) {
                role[m.path as usize] = m.is_body;
            }
        }
    }
    role
}

/// What is wrong with an exact letter hit at a page-unit point `(x, y)`, if anything: the
/// hit is exact, its path holds the point or is a piece of the cut path that holds it, and
/// `is_body` is the member's role. Returns whether it was a seam hit.
fn exact_problem(p: &Page, ix: &Index, role: &[bool], h: &LetterHit, x: f32, y: f32) -> Result<bool, String> {
    if !h.is_exact || h.distance != 0.0 {
        return Err("not exact".into());
    }
    if ix.pieces_of.contains_key(&h.path) {
        return Err("the hit names a cut path".into());
    }
    let seam = !p.point_in_path(h.path, x, y);
    if seam {
        let source = h.path.checked_sub(ix.n).and_then(|i| ix.source.get(i as usize));
        if !source.is_some_and(|&s| p.point_in_path(s, x, y)) {
            return Err("neither the path nor a cut path it is cut from holds the point".into());
        }
    }
    if h.is_body != role.get(h.path as usize).copied().unwrap_or(false) {
        return Err(format!("is_body {} for a path whose role is {}", h.is_body, !h.is_body));
    }
    Ok(seam)
}

/// The subpaths of a path's flattened outline, each a closed polygon. A curve is cut into
/// 8 segments, as the engine cuts it.
fn subpaths(p: &Page, path: u32) -> Vec<Vec<(f32, f32)>> {
    use qvp_format::{OP_CLOSE, OP_CUBIC, OP_LINE, OP_MOVE, OP_QUAD};
    let g = p.geometry();
    let r = g.table[path as usize];
    let ops = &g.ops[r.op_start as usize..(r.op_start + r.op_count) as usize];
    let pts = &g.pts[r.pt_start as usize..(r.pt_start + r.pt_count) as usize];
    let mut out: Vec<Vec<(f32, f32)>> = vec![];
    let mut cur: Vec<(f32, f32)> = vec![];
    let mut k = 0;
    for &op in ops {
        let at = |i: usize| (pts[k + i], pts[k + i + 1]);
        match op {
            OP_MOVE => {
                if cur.len() > 2 {
                    out.push(std::mem::take(&mut cur));
                }
                cur.clear();
                cur.push(at(0));
                k += 2;
            }
            OP_LINE => {
                cur.push(at(0));
                k += 2;
            }
            OP_QUAD | OP_CUBIC => {
                let n = if op == OP_QUAD { 2 } else { 3 };
                let p0 = *cur.last().unwrap_or(&(0.0, 0.0));
                let c: Vec<(f32, f32)> = (0..n).map(|i| at(2 * i)).collect();
                for i in 1..=8 {
                    let t = i as f32 / 8.0;
                    let u = 1.0 - t;
                    cur.push(if n == 2 {
                        (
                            u * u * p0.0 + 2.0 * u * t * c[0].0 + t * t * c[1].0,
                            u * u * p0.1 + 2.0 * u * t * c[0].1 + t * t * c[1].1,
                        )
                    } else {
                        (
                            u * u * u * p0.0 + 3.0 * u * u * t * c[0].0 + 3.0 * u * t * t * c[1].0 + t * t * t * c[2].0,
                            u * u * u * p0.1 + 3.0 * u * u * t * c[0].1 + 3.0 * u * t * t * c[1].1 + t * t * t * c[2].1,
                        )
                    });
                }
                k += 2 * n;
            }
            OP_CLOSE => {}
            _ => {}
        }
    }
    if cur.len() > 2 {
        out.push(cur);
    }
    out
}

/// Even-odd containment in one polygon.
fn in_polygon(poly: &[(f32, f32)], x: f32, y: f32) -> bool {
    let mut inside = false;
    for i in 0..poly.len() {
        let (a, b) = (poly[i], poly[(i + 1) % poly.len()]);
        if (a.1 <= y) != (b.1 <= y) && a.0 + (y - a.1) / (b.1 - a.1) * (b.0 - a.0) > x {
            inside = !inside;
        }
    }
    inside
}

/// A point in each counter of a body member. A counter is a subpath nested inside another
/// subpath of the same path, with points inside it that the path does not fill (two strokes
/// of one path that cross are no counter). The point is inside the counter and on no word's
/// ink. `None` for a counter with no such point on grids of 8, 16 and 32 cells over its box:
/// ink of another path covers it, or it is too small to sample.
fn counter_points(p: &Page, ink: &WordInk, path: u32) -> Vec<Option<(f32, f32)>> {
    let subs = subpaths(p, path);
    let mut out = vec![];
    for (j, hole) in subs.iter().enumerate() {
        let nested = subs.iter().enumerate().any(|(k, outer)| k != j && in_polygon(outer, hole[0].0, hole[0].1));
        if !nested {
            continue;
        }
        let (x0, y0, x1, y1) = hole.iter().fold((f32::MAX, f32::MAX, f32::MIN, f32::MIN), |b, &(x, y)| {
            (b.0.min(x), b.1.min(y), b.2.max(x), b.3.max(y))
        });
        let grid = || {
            [8u32, 16, 32].into_iter().flat_map(move |n| {
                (1..n).flat_map(move |i| {
                    (1..n).map(move |j| (x0 + (x1 - x0) * i as f32 / n as f32, y0 + (y1 - y0) * j as f32 / n as f32))
                })
            })
        };
        let open: Vec<(f32, f32)> =
            grid().filter(|&(x, y)| in_polygon(hole, x, y) && !p.point_in_path(path, x, y)).collect();
        if open.is_empty() {
            continue;
        }
        out.push(open.into_iter().find(|&(x, y)| ink.owners(p, x, y).is_empty()));
    }
    out
}

/// The letter hit prefers ink. (1) Printed: at every grid point on a word's ink, the letter
/// hit is exact, in a word whose ink holds the point, on a path that holds it or on a piece
/// of the cut path that holds it (a seam), and `is_body` is the member's role. (2) The same
/// under a reflow, through the view call: each ink point of one word only goes to the view
/// through that word's placement. (3) A point in a body member's counter, on no ink, gives a
/// near hit that names the member's letter.
#[test]
fn a_letter_hit_prefers_ink() {
    if !have_letters() {
        return;
    }
    let ([ink_points, seams, reflow_points, reflow_seams, lost, counters, counter_skips], _) = run(
        "a_letter_hit_prefers_ink",
        |b, f| {
            let mut p = load(b, true)?;
            let ix = index(&p, b);
            let ink = WordInk::new(&p);
            let role = body_roles(&p);
            let opt = HitOptions::default();
            let mut c = [0usize; 7];

            // (1) and the points of (2): every grid point on ink
            let mut single = vec![];
            let (w, h) = (p.width(), p.height());
            let mut y = 0.0;
            while y <= h {
                let mut x = 0.0;
                while x <= w {
                    let owners = ink.owners(&p, x, y);
                    if !owners.is_empty() {
                        c[0] += 1;
                        let hit = p.hit_test_letter(x, y, &opt);
                        let problem = match hit {
                            None => Err("no hit".to_string()),
                            Some(h) if !owners.contains(&h.word) => Err(format!("word {}", key(&p, h.word))),
                            Some(h) => exact_problem(&p, &ix, &role, &h, x, y),
                        };
                        match problem {
                            Ok(seam) => c[1] += seam as usize,
                            Err(e) => f.push(format!(
                                "printed ({x}, {y}) on the ink of {:?}: {e}: letter hit {hit:?}, exact word hit {:?}",
                                owners.iter().map(|&w| key(&p, w)).collect::<Vec<_>>(),
                                p.hit_test_exact(x, y)
                            )),
                        }
                        if owners.len() == 1 {
                            single.push((x, y, owners[0]));
                        }
                    }
                    x += INK_STEP;
                }
                y += INK_STEP;
            }

            // (2) the reflow
            let spec = LayoutSpec { reflow: Some(ReflowSpec { zoom: REFLOW_ZOOM, ..Default::default() }), ..PHONE };
            let l = p.layout(&spec).clone();
            for &(x, y, w) in &single {
                let line = p.data().words[w as usize].line_index as usize;
                let place = l.word_placement(w, line);
                let (lx, ly) = place.apply(x, y);
                let (vx, vy) = (l.offset_x + lx * l.scale, l.offset_y + ly * l.scale);
                // the point the engine works with, after the round trip through the view
                let (px, py) = place.invert((vx - l.offset_x) / l.scale, (vy - l.offset_y) / l.scale);
                if !ink.owners(&p, px, py).contains(&w) {
                    c[4] += 1;
                    continue;
                }
                c[2] += 1;
                let hit = p.hit_test_letter_view(vx, vy, &opt);
                let problem = match hit {
                    None => Err("no hit".to_string()),
                    Some(h) if h.word != w => {
                        // another word placed over this one: its own ink must hold the point
                        let line = p.data().words[h.word as usize].line_index as usize;
                        let (qx, qy) = l
                            .word_placement(h.word, line)
                            .invert((vx - l.offset_x) / l.scale, (vy - l.offset_y) / l.scale);
                        exact_problem(&p, &ix, &role, &h, qx, qy)
                            .map_err(|e| format!("word {} over word {}: {e}", key(&p, h.word), key(&p, w)))
                    }
                    Some(h) => exact_problem(&p, &ix, &role, &h, px, py),
                };
                match problem {
                    Ok(seam) => c[3] += seam as usize,
                    Err(e) => f.push(format!(
                        "reflow zoom {REFLOW_ZOOM}: ink of {} at ({x}, {y}), view ({vx}, {vy}): {e}: letter hit {hit:?}, exact view hit {:?}",
                        key(&p, w),
                        p.hit_test_exact_view(vx, vy)
                    )),
                }
            }

            // (3) the counters, in page units
            for w in 0..p.data().words.len() as u32 {
                for l in 0..p.letter_count(w) {
                    for m in p.letter_members(w, l).into_iter().filter(|m| m.is_body) {
                        for point in counter_points(&p, &ink, m.path) {
                            let Some((x, y)) = point else {
                                c[6] += 1;
                                continue;
                            };
                            c[5] += 1;
                            let hit = p.hit_test_letter(x, y, &opt);
                            if !hit.is_some_and(|h| !h.is_exact && !h.is_body && h.word == w && h.letter == l) {
                                f.push(format!(
                                    "counter of word {} letter {l} body path {} at ({x}, {y}): letter hit {hit:?}",
                                    key(&p, w),
                                    m.path
                                ));
                            }
                        }
                    }
                }
            }
            Ok(c)
        },
    );
    println!(
        "totals: {ink_points} printed ink points ({seams} seam hits), {reflow_points} reflowed ink points at zoom {REFLOW_ZOOM} ({reflow_seams} seam hits, {lost} moved off the ink by the view round trip), {counters} counters ({counter_skips} with no clear point)"
    );
    if all_pages() {
        assert_eq!(ink_points, ALL_INK_POINTS, "printed ink points over all pages");
        assert_eq!(seams, ALL_SEAM_HITS, "printed seam hits over all pages");
        assert_eq!(reflow_points, ALL_REFLOW_POINTS, "reflowed ink points over all pages");
        assert_eq!(reflow_seams, ALL_REFLOW_SEAM_HITS, "reflowed seam hits over all pages");
        assert_eq!(lost, ALL_REFLOW_LOST, "reflowed ink points moved off the ink over all pages");
        assert_eq!(counters, ALL_COUNTERS, "counters over all pages");
        assert_eq!(counter_skips, ALL_COUNTER_SKIPS, "counters with no clear point over all pages");
    }
}
