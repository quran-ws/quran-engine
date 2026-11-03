//! Text and search: the text of a target, search, Arabic folding, citations and word sidecars.

use crate::abi::{fill, guard, in_str, out_str, QvpStr};
use crate::convert::target;
use crate::types::*;
use qvp_core::*;

#[no_mangle]
pub unsafe extern "C" fn qvp_text(
    page: *const Page,
    t: *const QvpTarget,
    form: u8,
    word_sep: *const u8,
    word_sep_len: u32,
    line_sep: *const u8,
    line_sep_len: u32,
    out: *mut QvpStr,
) {
    guard(|| {
        let ws = if t.is_null() { (*page).target_words(&Target::Page) } else { (*page).target_words(&target(t)) };
        let s =
            (*page).text_of(&ws, Form::from_u8(form), in_str(word_sep, word_sep_len), in_str(line_sep, line_sep_len));
        out_str(out, &s);
    })
}
/// mode: 0 includes, 1 exact, 2 prefix
#[no_mangle]
pub unsafe extern "C" fn qvp_search(
    page: *const Page,
    query: *const u8,
    query_len: u32,
    form: u8,
    mode: u8,
    normalize: u8,
    loose_match: u8,
    limit: u32,
    out: *mut QvpMatch,
    cap: u32,
) -> u32 {
    guard(|| {
        let opt = SearchOptions {
            form: Form::from_u8(form),
            mode: match mode {
                1 => SearchMode::Exact,
                2 => SearchMode::Prefix,
                _ => SearchMode::Includes,
            },
            normalize: normalize != 0,
            loose: loose_match != 0,
            limit: if limit == 0 { usize::MAX } else { limit as usize },
        };
        let v: Vec<QvpMatch> = (*page)
            .search(in_str(query, query_len), &opt)
            .iter()
            .map(|m| QvpMatch { word: m.word, index: m.index as u32, is_loose_match: m.loose as u8 })
            .collect();
        fill(out, cap, &v)
    })
}
/// op (`QVP_ARABIC_*`): 0 strip marks, 1 fold, 2 normalize query (match fold), 3 loose key,
/// 4 search key. See docs/SEARCH-FOLD.md; 4 is additive, the rest are unchanged.
#[no_mangle]
pub unsafe extern "C" fn qvp_arabic(op: u8, s: *const u8, len: u32, out: *mut QvpStr) {
    guard(|| {
        let i = in_str(s, len);
        let r = match op {
            0 => strip_marks(i),
            1 => fold(i),
            3 => loose_key(i),
            4 => search_key(i),
            _ => normalize_query(i),
        };
        out_str(out, &r);
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_citation(page: *const Page, words: *const u32, n: u32, out: *mut QvpStr) {
    guard(|| {
        let ws = std::slice::from_raw_parts(words, n as usize);
        out_str(out, &(*page).citation(ws));
    })
}

/// Attach a JSON sidecar of derived text forms. Returns words updated, or -1 on a parse error.
#[no_mangle]
pub unsafe extern "C" fn qvp_attach_words(page: *mut Page, json: *const u8, len: u32) -> i32 {
    guard(|| {
        guard(|| match (*page).attach_words_json(std::slice::from_raw_parts(json, len as usize)) {
            Some(n) => n as i32,
            None => -1,
        })
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_has_form(page: *const Page, form: u8) -> u8 {
    guard(|| guard(|| (*page).has_form(Form::from_u8(form)) as u8))
}
