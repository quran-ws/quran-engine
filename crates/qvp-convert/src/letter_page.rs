//! The work that the `letters` subcommand and the letter-layer gate share for one page:
//! read the two archives and their indexes, build the page and its letter layer, and
//! encode the layer.

use crate::json::Json;
use crate::letters::convert_letters;
use crate::{convert, merge_words_index};
use qvp_format::letters::{decode_letters, encode_letters, LetterData};
use qvp_format::PageData;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

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

/// Build the letter layer of one page. `words_svg` and `words_index` come from the words
/// archive, `letters_svg` and `letters_index` from the letters archive. The `qpc` texts go
/// to `convert_letters` in page word order. `words_qpc` is the text that `batch` publishes:
/// the page merged with the words index. A word with no `qpc` text is an error. The layer
/// must decode to the same data that it encodes from.
pub fn letter_page(
    letters_svg: &Path,
    words_svg: &Path,
    words_index: &Path,
    letters_index: &Path,
) -> Result<LetterPage, String> {
    let lsvg = String::from_utf8(read(letters_svg)?).map_err(|e| format!("{}: {e}", letters_svg.display()))?;
    let wsvg = String::from_utf8(read(words_svg)?).map_err(|e| format!("{}: {e}", words_svg.display()))?;
    let mut c = convert(&wsvg).map_err(|e| format!("{}: {e}", words_svg.display()))?;
    let pg = c.page.header.page;
    let mut warnings = std::mem::take(&mut c.report.warnings);
    merge_words_index(&mut c.words_text, &read(words_index)?, &mut warnings);
    let mut words_qpc = Vec::with_capacity(c.words_text.len());
    for w in &c.words_text {
        if w.qpc.is_empty() {
            return Err(format!("page {pg}: word {}: no qpc text in {}", w.word_key, words_index.display()));
        }
        words_qpc.push(w.qpc.clone());
    }

    let ix = Json::parse(&read(letters_index)?).map_err(|e| format!("{}: {e}", letters_index.display()))?;
    let by_key: HashMap<&str, &str> =
        ix.get("words").map(|w| w.arr()).unwrap_or_default().iter().map(|w| (w.s("word_key"), w.s("qpc"))).collect();
    let mut letters_qpc = Vec::with_capacity(c.words_text.len());
    for w in &c.words_text {
        match by_key.get(w.word_key.as_str()) {
            Some(q) if !q.is_empty() => letters_qpc.push(q.to_string()),
            _ => return Err(format!("page {pg}: word {}: no qpc text in {}", w.word_key, letters_index.display())),
        }
    }
    let index_letters = ix.f("letters").map(|n| n as u64);

    let lc = convert_letters(&lsvg, &wsvg, &c.page, &words_qpc, &letters_qpc)?;
    warnings.extend(lc.warnings);
    let bytes = encode_letters(&lc.data);
    if decode_letters(&bytes).as_ref() != Ok(&lc.data) {
        return Err(format!("page {pg}: the letter layer does not decode to the data it encodes"));
    }
    Ok(LetterPage { page: c.page, data: lc.data, bytes, index_letters, warnings })
}
