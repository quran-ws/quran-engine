//! Structure gate for the letter layer: build each page's layer from the letters archive,
//! then check that it fits its page and covers every path of every word once.
//!
//! Run a subset:  QVP_TEST_PAGES=001,002,036 cargo test -p qvp-convert --release --test letters
//! Run all 604:   QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test letters
//!
//! The letters archive is `pages-letters/` (`QVP_SYNC=letters scripts/sync-test-data.sh`).
//! `QVP_LETTERS_DIR` points the gate at another copy. Without the archive the gate skips,
//! unless `QVP_REQUIRE_LETTERS` is set.

use qvp_convert::letter_page::letter_page;
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

fn letters_dir() -> PathBuf {
    std::env::var_os("QVP_LETTERS_DIR").map(PathBuf::from).unwrap_or_else(|| repo().join("pages-letters"))
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
        for p in w.first_path as usize..w.first_path as usize + w.n_paths as usize {
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

#[test]
fn structure() {
    if !letters_dir().join("001.svg").exists() {
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
