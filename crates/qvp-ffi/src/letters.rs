//! Letters: the optional letter layer of a page, and the letters, members and offsets it
//! holds. The hit tests are in `hit.rs`, the query rule in `style.rs` and the draw revision in
//! `layout.rs`, under their own sections of the header.
//!
//! Every call takes a NULL page as a page with no layer, and every out-pointer may be NULL.

use crate::abi::{fill_with, guard};
use crate::convert::letter_query;
use crate::types::*;
use qvp_core::*;

/// Attach a letter layer (`NNN.qvl`). 0 on success. -1 for a NULL page or NULL bytes, a file
/// that does not decode, a layer of another page, or a file over the size limit: the page
/// then keeps the layer it had, and nothing it answered before changes.
#[no_mangle]
pub unsafe extern "C" fn qvp_attach_letters(page: *mut Page, bytes: *const u8, len: u32) -> i32 {
    guard(|| {
        if page.is_null() || bytes.is_null() {
            return -1;
        }
        match (*page).attach_letters(std::slice::from_raw_parts(bytes, len as usize)) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    })
}

/// 1 when a letter layer is attached.
#[no_mangle]
pub unsafe extern "C" fn qvp_has_letters(page: *const Page) -> u8 {
    guard(|| (!page.is_null() && (*page).has_letters()) as u8)
}

/// How many letters a word has: 0 with no layer, or for a word past the page.
#[no_mangle]
pub unsafe extern "C" fn qvp_letter_count(page: *const Page, word: u32) -> u32 {
    guard(|| if page.is_null() { 0 } else { (*page).letter_count(word) })
}

/// One letter of a word. 0 when there is no such letter or no layer; `out` is then left as
/// it was.
#[no_mangle]
pub unsafe extern "C" fn qvp_letter_info(page: *const Page, word: u32, letter: u32, out: *mut QvpLetterInfo) -> i32 {
    guard(|| {
        if page.is_null() {
            return 0;
        }
        let Some(l) = (*page).letter_info(word, letter) else { return 0 };
        if !out.is_null() {
            *out = QvpLetterInfo {
                word: l.word,
                index: l.index,
                at: l.at,
                text: l.text as u32,
                n_members: l.n_members,
                n_marks: l.n_marks,
                more_at: l.more_at.first().copied().unwrap_or(NONE),
                n_more_at: l.more_at.len() as u32,
                x0: l.x0,
                y0: l.y0,
                x1: l.x1,
                y1: l.y1,
                has_ink: l.has_ink as u8,
                _pad: [0; 3],
            };
        }
        1
    })
}

/// The paths of one letter: the bodies first, then the marks in text order. Returns the
/// count, 0 with no layer or for no such letter.
#[no_mangle]
pub unsafe extern "C" fn qvp_letter_members(
    page: *const Page,
    word: u32,
    letter: u32,
    out: *mut QvpLetterMember,
    cap: u32,
) -> u32 {
    guard(|| {
        if page.is_null() {
            return 0;
        }
        let members = (*page).letter_members(word, letter);
        fill_with(
            out,
            cap,
            members.iter().map(|m| QvpLetterMember {
                path: m.path,
                at: m.at,
                mark: m.mark as u8,
                is_body: m.is_body as u8,
                is_cut: m.is_cut as u8,
                _pad: 0,
            }),
        )
    })
}

/// The letter a path draws. 1 for a letter's member (a page path or a piece) and for a
/// word-level mark, whose letter is `QVP_NONE`. 0 for a cut path, a path with no letter role,
/// a path past the table, or no layer; the outs are then left as they were.
#[no_mangle]
pub unsafe extern "C" fn qvp_path_letter(page: *const Page, path: u32, out_letter: *mut u32, out_at: *mut u32) -> i32 {
    guard(|| {
        if page.is_null() {
            return 0;
        }
        let Some((letter, at)) = (*page).path_letter(path) else { return 0 };
        if !out_letter.is_null() {
            *out_letter = letter;
        }
        if !out_at.is_null() {
            *out_at = at;
        }
        1
    })
}

/// The letter of a word that draws the character at offset `at`, or holds the mark at it.
/// 0 for a word-level mark's offset, an offset no letter has, or no layer.
#[no_mangle]
pub unsafe extern "C" fn qvp_letter_at_offset(page: *const Page, word: u32, at: u32, out_letter: *mut u32) -> i32 {
    guard(|| {
        if page.is_null() {
            return 0;
        }
        let Some(letter) = (*page).letter_at_offset(word, at) else { return 0 };
        if !out_letter.is_null() {
            *out_letter = letter;
        }
        1
    })
}

/// The letters a query finds, as `{word, letter}` pairs in word order, then letter order.
/// `cap` counts pairs. 0 for a NULL or bad query, or no layer.
#[no_mangle]
pub unsafe extern "C" fn qvp_find_letters(
    page: *const Page,
    query: *const QvpLetterQuery,
    out: *mut u32,
    cap: u32,
) -> u32 {
    guard(|| {
        if page.is_null() {
            return 0;
        }
        let Some(q) = letter_query(query) else { return 0 };
        let found = (*page).find_letters(&q);
        fill_with(out.cast::<[u32; 2]>(), cap, found.iter().map(|&(w, l)| [w, l]))
    })
}
