//! The work that the `letters` subcommand and the letter-layer gate share: build the
//! letter layer of one page from the two archives and their indexes, and write the layers
//! of a set of pages to a directory.

use crate::convert;
use crate::json::Json;
use crate::letters::convert_letters;
use qvp_format::letters::{decode_letters, encode_letters, LetterData};
use qvp_format::PageData;
use rayon::prelude::*;
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};

/// One page and its letter layer.
pub struct LetterPage {
    /// The page, converted in memory from the words archive.
    pub page: PageData,
    pub data: LetterData,
    /// The encoded layer: the bytes of `NNN.qvl`.
    pub bytes: Vec<u8>,
    /// The top-level `letters` count of the letters index, when the index has one.
    pub index_letters: Option<u64>,
    pub warnings: Vec<String>,
}

fn read(p: &Path) -> Result<Vec<u8>, String> {
    fs::read(p).map_err(|e| format!("{}: {e}", p.display()))
}

/// One page's index file: the `qpc` text of each word key, and the top-level `letters`
/// count. A duplicate key, a key or `qpc` that is not a string, and a count that is not a
/// whole number of zero or more are errors.
fn read_index(path: &Path) -> Result<(HashMap<String, String>, Option<u64>), String> {
    let at = path.display();
    let ix = Json::parse(&read(path)?).map_err(|e| format!("{at}: {e}"))?;
    let Some(Json::Arr(words)) = ix.get("words") else {
        return Err(format!("{at}: no `words` list"));
    };
    let mut qpc = HashMap::with_capacity(words.len());
    for w in words {
        let key = w.get("word_key").and_then(Json::str).ok_or_else(|| format!("{at}: a word_key is not a string"))?;
        let text = w.get("qpc").and_then(Json::str).ok_or_else(|| format!("{at}: word {key}: qpc is not a string"))?;
        if qpc.insert(key.to_owned(), text.to_owned()).is_some() {
            return Err(format!("{at}: word {key} is in the index twice"));
        }
    }
    let letters = match ix.get("letters") {
        None => None,
        Some(Json::Num(n)) if *n >= 0.0 && n.fract() == 0.0 && *n <= u32::MAX as f64 => Some(*n as u64),
        Some(v) => return Err(format!("{at}: letters is {v:?}, not a whole number of zero or more")),
    };
    Ok((qpc, letters))
}

/// The `qpc` text of each page word, in page word order. A word that the index does not
/// have, or has with empty text, is an error.
fn qpc_in_page_order<'a>(
    keys: impl Iterator<Item = (&'a str, &'a str)>,
    index: &HashMap<String, String>,
    path: &Path,
) -> Result<Vec<String>, String> {
    keys.map(|(key, from_svg)| {
        let q = index.get(key).ok_or_else(|| format!("word {key}: not in {}", path.display()))?;
        // As `batch` publishes it: the page's own text when it has one, else the index's.
        let q = if from_svg.is_empty() { q.as_str() } else { from_svg };
        if q.is_empty() {
            return Err(format!("word {key}: empty qpc text in {}", path.display()));
        }
        Ok(q.to_owned())
    })
    .collect()
}

/// Build the letter layer of one page. `words_svg` and `words_index` come from the words
/// archive, `letters_svg` and `letters_index` from the letters archive. The `qpc` texts go
/// to `convert_letters` in page word order. `words_qpc` is the text that `batch` publishes.
/// The layer must decode to the same data that it encodes from. An error does not name
/// the page: the caller does.
pub fn letter_page(
    letters_svg: &Path,
    words_svg: &Path,
    words_index: &Path,
    letters_index: &Path,
) -> Result<LetterPage, String> {
    let lsvg = String::from_utf8(read(letters_svg)?).map_err(|e| format!("{}: {e}", letters_svg.display()))?;
    let wsvg = String::from_utf8(read(words_svg)?).map_err(|e| format!("{}: {e}", words_svg.display()))?;
    let c = convert(&wsvg).map_err(|e| format!("{}: {e}", words_svg.display()))?;
    let pg = c.page.header.page;

    let (wix, _) = read_index(words_index)?;
    let words_qpc =
        qpc_in_page_order(c.words_text.iter().map(|w| (w.word_key.as_str(), w.qpc.as_str())), &wix, words_index)?;
    let (lix, index_letters) = read_index(letters_index)?;
    let letters_qpc = qpc_in_page_order(c.words_text.iter().map(|w| (w.word_key.as_str(), "")), &lix, letters_index)?;

    // `convert_letters` starts most errors with the page; the caller names it once.
    let prefix = format!("page {pg}: ");
    let lc = convert_letters(&lsvg, &wsvg, &c.page, &words_qpc, &letters_qpc)
        .map_err(|e| e.strip_prefix(&prefix).map(str::to_owned).unwrap_or(e))?;
    let mut warnings = c.report.warnings;
    warnings.extend(lc.warnings);
    let bytes = encode_letters(&lc.data);
    if decode_letters(&bytes).as_ref() != Ok(&lc.data) {
        return Err("the letter layer does not decode to the data it encodes".into());
    }
    Ok(LetterPage { page: c.page, data: lc.data, bytes, index_letters, warnings })
}

/// The four inputs of a run of `write_letters`.
pub struct LetterDirs {
    pub letters_svg: PathBuf,
    pub words_svg: PathBuf,
    pub words_index: PathBuf,
    pub letters_index: PathBuf,
}

/// What a run wrote for one page.
pub struct Written {
    pub letters: usize,
    pub pieces: usize,
    pub bytes: usize,
    pub warnings: Vec<String>,
}

/// What a run of `write_letters` did.
pub struct LettersRun {
    /// The stale files that the run removed before it started.
    pub removed: usize,
    /// Each page, in page order, with what it wrote or its error.
    pub pages: Vec<(String, Result<Written, String>)>,
}

/// The stems of the `NNN.svg` files in `dir`. A name that starts with a dot is not a page:
/// macOS leaves `._<file>` sidecars with the same extension.
pub fn page_stems(dir: &Path) -> Result<BTreeSet<String>, String> {
    Ok(fs::read_dir(dir)
        .map_err(|e| format!("{}: {e}", dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "svg"))
        .filter_map(|p| p.file_stem().and_then(|n| n.to_str()).map(str::to_owned))
        .filter(|n| !n.starts_with('.'))
        .collect())
}

/// Remove `path`. A file that is not there is not an error.
fn remove(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("{}: {e}", path.display())),
        _ => Ok(()),
    }
}

/// Write `<out_dir>/NNN.qvl` for every page in either SVG directory, in parallel. A page in
/// one directory only is an error. When the run ends, `<out_dir>` has a `.qvl` file for each
/// page that converted, and no other `.qvl` file:
/// - before the pages, the run removes each `.qvl` file whose page is not in the set, and
///   each temporary file that an earlier run left;
/// - a page writes to a temporary name, then renames the file into place;
/// - a page with an error removes its old file.
pub fn write_letters(dirs: &LetterDirs, out_dir: &Path) -> Result<LettersRun, String> {
    let mut pages = page_stems(&dirs.letters_svg)?;
    pages.extend(page_stems(&dirs.words_svg)?);
    fs::create_dir_all(out_dir).map_err(|e| format!("{}: {e}", out_dir.display()))?;

    let mut removed = 0;
    for e in fs::read_dir(out_dir).map_err(|e| format!("{}: {e}", out_dir.display()))? {
        let path = e.map_err(|e| format!("{}: {e}", out_dir.display()))?.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
        let stale = match name.strip_suffix(".qvl") {
            Some(stem) => !pages.contains(stem),
            None => name.starts_with('.') && name.ends_with(".qvl.tmp"),
        };
        if stale {
            remove(&path)?;
            removed += 1;
        }
    }

    let pages: Vec<String> = pages.into_iter().collect();
    let pages = pages
        .into_par_iter()
        .map(|stem| {
            let out = out_dir.join(format!("{stem}.qvl"));
            let r = letter_page(
                &dirs.letters_svg.join(format!("{stem}.svg")),
                &dirs.words_svg.join(format!("{stem}.svg")),
                &dirs.words_index.join(format!("{stem}.json")),
                &dirs.letters_index.join(format!("{stem}.json")),
            )
            .and_then(|lp| {
                let tmp = out_dir.join(format!(".{stem}.qvl.tmp"));
                fs::write(&tmp, &lp.bytes).map_err(|e| format!("{}: {e}", tmp.display()))?;
                fs::rename(&tmp, &out).map_err(|e| format!("{}: {e}", out.display()))?;
                Ok(Written {
                    letters: lp.data.letters.len(),
                    pieces: lp.data.pieces.len(),
                    bytes: lp.bytes.len(),
                    warnings: lp.warnings,
                })
            });
            let r = r.map_err(|e| match remove(&out) {
                Ok(()) => e,
                Err(r) => format!("{e}; and the old file stays: {r}"),
            });
            (stem, r)
        })
        .collect();
    Ok(LettersRun { removed, pages })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("qvp-letter-page-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    fn write_index(dir: &Path, body: &str) -> PathBuf {
        let p = dir.join("ix.json");
        fs::write(&p, body).unwrap();
        p
    }

    #[test]
    fn read_index_takes_each_word_once() {
        let d = temp("index");
        let p = write_index(&d, r#"{"letters": 3, "words": [{"word_key": "1:1:1", "qpc": "بِسۡمِ"}]}"#);
        let (q, n) = read_index(&p).unwrap();
        assert_eq!(n, Some(3));
        assert_eq!(q["1:1:1"], "بِسۡمِ");
        let refused = |body: &str| read_index(&write_index(&d, body)).unwrap_err();
        assert!(refused(r#"{"words": [{"word_key": "1:1:1", "qpc": "a"}, {"word_key": "1:1:1", "qpc": "b"}]}"#)
            .contains("1:1:1 is in the index twice"));
        assert!(refused(r#"{"words": [{"word_key": 7, "qpc": "a"}]}"#).contains("word_key is not a string"));
        assert!(refused(r#"{"words": [{"word_key": "1:1:1", "qpc": null}]}"#).contains("qpc is not a string"));
        assert!(refused(r#"{"letters": -1, "words": []}"#).contains("not a whole number"));
        assert!(refused(r#"{"letters": 2.5, "words": []}"#).contains("not a whole number"));
        assert!(refused(r#"{"letters": "2", "words": []}"#).contains("not a whole number"));
        assert!(refused(r#"{"count": 0}"#).contains("no `words` list"));
        fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn a_word_missing_from_an_index_is_an_error() {
        let ix: HashMap<String, String> = [("1:1:1".to_owned(), "a".to_owned())].into();
        let p = Path::new("ix.json");
        let got = qpc_in_page_order([("1:1:1", ""), ("1:1:2", "")].into_iter(), &ix, p);
        assert_eq!(got, Err("word 1:1:2: not in ix.json".to_owned()));
        // The page's own text wins over the index, as in `batch`, but the index must have the word.
        assert_eq!(qpc_in_page_order([("1:1:1", "b")].into_iter(), &ix, p), Ok(vec!["b".to_owned()]));
        assert!(qpc_in_page_order([("1:1:2", "b")].into_iter(), &ix, p).is_err());
    }

    /// A run removes the file of a page that fails and the file of a page that is not in
    /// the set. The data gate (`tests/letters.rs`) also checks that a good page's file is
    /// there; this test needs no data.
    #[test]
    fn a_run_leaves_no_stale_file() {
        let d = temp("run");
        let dirs = LetterDirs {
            letters_svg: d.join("letters"),
            words_svg: d.join("words"),
            words_index: d.join("words-index"),
            letters_index: d.join("letters-index"),
        };
        for p in [&dirs.letters_svg, &dirs.words_svg] {
            fs::create_dir_all(p).unwrap();
            fs::write(p.join("005.svg"), "not a page").unwrap();
        }
        let out = d.join("out");
        fs::create_dir_all(&out).unwrap();
        for f in ["005.qvl", "700.qvl", ".006.qvl.tmp", "notes.txt"] {
            fs::write(out.join(f), "old").unwrap();
        }
        let run = write_letters(&dirs, &out).unwrap();
        assert_eq!(run.removed, 2);
        assert_eq!(run.pages.len(), 1);
        assert!(run.pages[0].1.is_err());
        let mut left: Vec<String> =
            fs::read_dir(&out).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
        left.sort();
        assert_eq!(left, ["notes.txt"]);
        fs::remove_dir_all(&d).ok();
    }
}
