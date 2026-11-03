//! Page and metadata: load a page and read its words, ayahs, lines, decorations and surahs.

use crate::abi::{fill, fill_with, guard, out_str, QvpStr, STR_BUF};
use crate::convert::{ayah_mark, header, target};
use crate::types::*;
use qvp_core::*;

#[no_mangle]
pub unsafe extern "C" fn qvp_page_load(bytes: *const u8, len: usize) -> *mut Page {
    guard(|| {
        if bytes.is_null() {
            return std::ptr::null_mut();
        }
        match Page::load(std::slice::from_raw_parts(bytes, len)) {
            Ok(p) => Box::into_raw(Box::new(p)),
            Err(_) => std::ptr::null_mut(),
        }
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_page_free(page: *mut Page) {
    guard(|| {
        if !page.is_null() {
            drop(Box::from_raw(page));
        }
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_page_info(page: *const Page, out: *mut QvpPageInfo) {
    guard(|| {
        let p = &*page;
        let d = p.data();
        *out = QvpPageInfo {
            width: p.width(),
            height: p.height(),
            page: d.header.page as u32,
            n_lines: d.lines.len() as u32,
            n_ayahs: d.ayahs.len() as u32,
            n_words: d.words.len() as u32,
            n_paths: d.paths.len() as u32,
            n_decorations: d.decorations.len() as u32,
        };
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_geometry(page: *const Page, out: *mut QvpGeometry) {
    guard(|| {
        let g = (*page).geometry();
        *out = QvpGeometry {
            ops: g.ops.as_ptr(),
            ops_len: g.ops.len() as u32,
            pts: g.pts.as_ptr(),
            pts_len: g.pts.len() as u32,
            table: g.table.as_ptr() as *const u32,
            n_paths: g.table.len() as u32,
        };
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_word_info(page: *const Page, index: u32, out: *mut QvpWordInfo) -> i32 {
    guard(|| {
        let p = &*page;
        let d = p.data();
        let Some(w) = d.words.get(index as usize) else { return 0 };
        let q = p.quant();
        let t = p.word_text(index);
        *out = QvpWordInfo {
            surah: w.surah,
            ayah: w.ayah,
            word: w.word,
            line_number: d.lines[w.line_index as usize].line_number as u16,
            ayah_index: w.ayah_index as u32,
            line_index: w.line_index as u32,
            x0: w.bbox.x0 as f32 / q,
            y0: w.bbox.y0 as f32 / q,
            x1: w.bbox.x1 as f32 / q,
            y1: w.bbox.y1 as f32 / q,
            text: QvpStr { ptr: t.as_ptr(), len: t.len() as u32 },
            first_path: w.first_path,
            n_paths: w.n_paths as u32,
        };
        1
    })
}
/// form: 0 rasm_uthmani, 1 rasm_imlai, 2 qpc, 3 rasm, 4 search
#[no_mangle]
pub unsafe extern "C" fn qvp_word_form(page: *const Page, index: u32, form: u8, out: *mut QvpStr) -> i32 {
    guard(|| {
        let p = &*page;
        if index as usize >= p.data().words.len() {
            return 0;
        }
        let t = p.word_form(index, Form::from_u8(form));
        *out = QvpStr { ptr: t.as_ptr(), len: t.len() as u32 };
        1
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_ayah_info(page: *const Page, index: u32, out: *mut QvpAyahInfo) -> i32 {
    guard(|| {
        let p = &*page;
        let Some(a) = p.data().ayahs.get(index as usize) else { return 0 };
        let q = p.quant();
        *out = QvpAyahInfo {
            surah: a.surah,
            ayah: a.ayah,
            fragment: a.fragment,
            fragments: a.fragments,
            flags: a.flags,
            _pad: 0,
            rubu_al_hizb: a.rubu_al_hizb,
            first_word: a.first_word as u32,
            n_words: a.n_words as u32,
            ayah_mark_decoration: if a.ayah_mark_decoration == u16::MAX { NONE } else { a.ayah_mark_decoration as u32 },
            x0: a.bbox.x0 as f32 / q,
            y0: a.bbox.y0 as f32 / q,
            x1: a.bbox.x1 as f32 / q,
            y1: a.bbox.y1 as f32 / q,
        };
        1
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_line_info(page: *const Page, index: u32, out: *mut QvpLineInfo) -> i32 {
    guard(|| {
        let p = &*page;
        let Some(l) = p.data().lines.get(index as usize) else { return 0 };
        let q = p.quant();
        let spacing = p.line_spacing();
        let c = p.line_centre(index as usize);
        *out = QvpLineInfo {
            line_number: l.line_number,
            is_header: p.line_is_header(index as usize) as u8,
            first_word: l.first_word as u32,
            n_words: l.n_words as u32,
            x0: l.bbox.x0 as f32 / q,
            y0: l.bbox.y0 as f32 / q,
            x1: l.bbox.x1 as f32 / q,
            y1: l.bbox.y1 as f32 / q,
            band_y0: c - spacing / 2.0,
            band_y1: c + spacing / 2.0,
            centre: c,
        };
        1
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_decoration_info(page: *const Page, index: u32, out: *mut QvpDecorationInfo) -> i32 {
    guard(|| {
        let p = &*page;
        let Some(d) = p.data().decorations.get(index as usize) else { return 0 };
        let q = p.quant();
        let t = p.deco_text(index);
        *out = QvpDecorationInfo {
            decoration: d.kind as u8,
            _pad: 0,
            surah: d.surah,
            ayah: d.ayah,
            _pad2: 0,
            line: p.geometry().table[d.first_path as usize].line,
            x0: d.bbox.x0 as f32 / q,
            y0: d.bbox.y0 as f32 / q,
            x1: d.bbox.x1 as f32 / q,
            y1: d.bbox.y1 as f32 / q,
            text: QvpStr { ptr: t.as_ptr(), len: t.len() as u32 },
            first_path: d.first_path,
            n_paths: d.n_paths as u32,
        };
        1
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_find_word(page: *const Page, surah: u16, ayah: u16, word: u16) -> i32 {
    guard(|| (*page).find_word(surah, ayah, word).map(|i| i as i32).unwrap_or(-1))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_target_words(page: *const Page, t: *const QvpTarget, out: *mut u32, cap: u32) -> u32 {
    guard(|| {
        let v = (*page).target_words(&target(t));
        fill(out, cap, &v)
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_page_line_spacing(page: *const Page) -> f32 {
    guard(|| (*page).line_spacing())
}
/// The grid the page is laid out inside: the mushaf's line count and the printed line spacing.
#[no_mangle]
pub unsafe extern "C" fn qvp_page_grid(page: *const Page, out: *mut QvpGrid) {
    guard(|| {
        let g = (*page).grid();
        *out = QvpGrid { lines: g.lines, line_spacing: g.line_spacing };
    })
}

// ───────────── metadata ─────────────

#[no_mangle]
pub unsafe extern "C" fn qvp_surah_count(page: *const Page) -> u32 {
    guard(|| (*page).surahs().len() as u32)
}
/// Strings point into a thread-local buffer valid until the next string-returning call.
#[no_mangle]
pub unsafe extern "C" fn qvp_surah_at(page: *const Page, i: u32, out: *mut QvpSurah) -> i32 {
    guard(|| {
        let v = (*page).surahs();
        let Some(s) = v.get(i as usize) else { return 0 };
        let joined = format!("{}\u{0}{}\u{0}{}", s.arabic, s.latin, s.english);
        STR_BUF.with(|b| {
            let mut b = b.borrow_mut();
            b.clear();
            b.extend_from_slice(joined.as_bytes());
            let base = b.as_ptr();
            let (a, l, e) = (s.arabic.len(), s.latin.len(), s.english.len());
            *out = QvpSurah {
                number: s.number,
                ayah_count: s.ayah_count,
                has_banner: s.has_banner as u8,
                has_basmalah: s.has_basmalah as u8,
                place: match s.revelation_place.as_str() {
                    "makkah" => 0,
                    "madinah" => 1,
                    _ => 255,
                },
                _pad: 0,
                banner_decoration: s.banner_decoration,
                arabic: QvpStr { ptr: base, len: a as u32 },
                latin: QvpStr { ptr: base.add(a + 1), len: l as u32 },
                english: QvpStr { ptr: base.add(a + l + 2), len: e as u32 },
            };
        });
        1
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_divisions(page: *const Page, out: *mut QvpDivision, cap: u32) -> u32 {
    guard(|| {
        fill_with(
            out,
            cap,
            (*page).divisions().iter().map(|d| QvpDivision {
                division: d.kind,
                line: d.line,
                number: d.n,
                surah: d.surah,
                ayah: d.ayah,
                ayah_index: d.ayah_index,
            }),
        )
    })
}
/// The surah headings on the page, in page units: the box a frame fills and the title ink.
#[no_mangle]
pub unsafe extern "C" fn qvp_surah_headers(page: *const Page, out: *mut QvpSurahHeader, cap: u32) -> u32 {
    guard(|| fill_with(out, cap, (*page).surah_headers().iter().map(header)))
}
/// The surah headings in viewport px through the current layout: where each is drawn.
#[no_mangle]
pub unsafe extern "C" fn qvp_surah_headers_view(page: *const Page, out: *mut QvpSurahHeader, cap: u32) -> u32 {
    guard(|| fill_with(out, cap, (*page).surah_headers_view().iter().map(header)))
}
/// The ayah medallions on the page, in page units.
#[no_mangle]
pub unsafe extern "C" fn qvp_ayah_marks(page: *const Page, out: *mut QvpAyahMark, cap: u32) -> u32 {
    guard(|| fill_with(out, cap, (*page).ayah_marks().iter().map(ayah_mark)))
}
/// The ayah medallions in viewport px through the current layout: where each is drawn.
#[no_mangle]
pub unsafe extern "C" fn qvp_ayah_marks_view(page: *const Page, out: *mut QvpAyahMark, cap: u32) -> u32 {
    guard(|| fill_with(out, cap, (*page).ayah_marks_view().iter().map(ayah_mark)))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_rosettes(page: *const Page, out: *mut QvpRosette, cap: u32) -> u32 {
    guard(|| {
        fill_with(
            out,
            cap,
            (*page).rosettes().iter().map(|r| QvpRosette {
                decoration: r.decoration,
                surah: r.surah,
                ayah: r.ayah,
                juz: r.juz,
                hizb: r.hizb,
                nisf: r.nisf,
                rubu_al_hizb: r.rubu_al_hizb,
                rubu_al_hizb_in_hizb: r.rubu_al_hizb_in_hizb,
                _pad: 0,
            }),
        )
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_sajdahs(page: *const Page, out: *mut QvpSajdah, cap: u32) -> u32 {
    guard(|| {
        fill_with(
            out,
            cap,
            (*page).sajdahs().iter().map(|&(d, s, a, p)| QvpSajdah { decoration: d, surah: s, ayah: a, sign_path: p }),
        )
    })
}
/// pairs of (surah, ayah) as u32 = surah<<16 | ayah
#[no_mangle]
pub unsafe extern "C" fn qvp_ayah_keys(page: *const Page, out: *mut u32, cap: u32) -> u32 {
    guard(|| fill_with(out, cap, (*page).ayah_keys().iter().map(|(s, a)| (*s as u32) << 16 | *a as u32)))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_ayah_word_count(page: *const Page, surah: u16, ayah: u16, is_complete: *mut u8) -> u32 {
    guard(|| {
        let (n, c) = (*page).ayah_word_count(surah, ayah);
        if !is_complete.is_null() {
            *is_complete = c as u8;
        }
        n
    })
}
/// Returns the number of words written, or -1 when the segment count does not match
/// (follow the ayah whole instead of drifting).
#[no_mangle]
pub unsafe extern "C" fn qvp_recite_map(
    page: *const Page,
    surah: u16,
    ayah: u16,
    n_segments: u32,
    out: *mut u32,
    cap: u32,
) -> i32 {
    guard(|| match (*page).recite_map(surah, ayah, n_segments) {
        Some(v) => fill(out, cap, &v) as i32,
        None => -1,
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_word_label(page: *const Page, word_index: u32, out: *mut QvpStr) {
    guard(|| {
        out_str(out, &(*page).word_label(word_index));
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_ayah_label(page: *const Page, ayah_index: u32, out: *mut QvpStr) {
    guard(|| {
        out_str(out, &(*page).ayah_label(ayah_index));
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garbage_bytes_load_as_null() {
        let p = unsafe { qvp_page_load(b"not a page".as_ptr(), 10) };
        assert!(p.is_null());
    }
}
