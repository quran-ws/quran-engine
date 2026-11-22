//! Structure gate for the letter layer: build each page's layer from the letters archive,
//! then check that it fits its page and covers every path of every word once.
//!
//! Run a subset:  QVP_TEST_PAGES=001,002,036 cargo test -p qvp-convert --release --test letters
//! Run all 604:   QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test letters
//!
//! The letters archive is `pages-letters/` (`QVP_SYNC=letters scripts/sync-test-data.sh`).
//! `QVP_LETTERS_DIR` points the gate at another copy, relative to the repository. Without the archive the gate skips,
//! unless `QVP_REQUIRE_LETTERS` is set.
//!
//! A page with a warning fails the gate, because the converter emits none on good data.

use qvp_convert::letter_page::{letter_page, write_letters, LetterDirs};
use qvp_format::letters::{decode_letters, encode_letters, MemberPath};
use rayon::prelude::*;
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

/// Build one page's layer and check it. An error names the page and, where it can, the word.
fn check(page: &str) -> Result<Counts, String> {
    let ld = letters_dir();
    let lp = letter_page(
        &ld.join(format!("{page}.svg")),
        &repo().join("pages").join(format!("{page}.svg")),
        &repo().join("index/by-page").join(format!("{page}.json")),
        &ld.join("index/by-page").join(format!("{page}.json")),
    )
    .map_err(|e| format!("page {page}: {e}"))?;
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
