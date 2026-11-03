//! Atlas: cross-page lookup from the atlas file.

use crate::abi::{fill_with, guard, in_str, out_str, QvpStr, STR_BUF};
use crate::types::*;
use qvp_core::*;

#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_load(bytes: *const u8, len: usize) -> *mut Atlas {
    guard(|| match Atlas::decode(std::slice::from_raw_parts(bytes, len)) {
        Ok(a) => Box::into_raw(Box::new(a)),
        Err(_) => std::ptr::null_mut(),
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_free(a: *mut Atlas) {
    guard(|| {
        if !a.is_null() {
            drop(Box::from_raw(a));
        }
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_page_of(a: *const Atlas, surah: u16, ayah: u16) -> i32 {
    guard(|| (*a).page_of(surah, ayah).map(|p| p as i32).unwrap_or(-1))
}
/// out: first_surah, first_ayah, last_surah, last_ayah
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_page_range(a: *const Atlas, page: u16, out: *mut u16) -> i32 {
    guard(|| match (*a).page_range(page) {
        Some((f, l)) => {
            *out = f.0;
            *out.add(1) = f.1;
            *out.add(2) = l.0;
            *out.add(3) = l.1;
            1
        }
        None => 0,
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_page_count(a: *const Atlas) -> u32 {
    guard(|| (*a).pages.len() as u32)
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_surah_count(a: *const Atlas) -> u32 {
    guard(|| (*a).surahs.len() as u32)
}
unsafe fn atlas_surah_out(s: &qvp_core::qvp_format::atlas::AtlasSurah, out: *mut QvpAtlasSurah) {
    let joined = format!("{}\u{0}{}\u{0}{}", s.arabic, s.latin, s.english);
    STR_BUF.with(|b| {
        let mut b = b.borrow_mut();
        b.clear();
        b.extend_from_slice(joined.as_bytes());
        let base = b.as_ptr();
        let (ar, l, e) = (s.arabic.len(), s.latin.len(), s.english.len());
        *out = QvpAtlasSurah {
            number: s.n,
            first_page: s.first_page,
            ayah_count: s.ayah_count,
            place: s.place,
            _pad: 0,
            arabic: QvpStr { ptr: base, len: ar as u32 },
            latin: QvpStr { ptr: base.add(ar + 1), len: l as u32 },
            english: QvpStr { ptr: base.add(ar + l + 2), len: e as u32 },
        };
    });
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_surah(a: *const Atlas, n: u16, out: *mut QvpAtlasSurah) -> i32 {
    guard(|| match (*a).surah(n) {
        Some(s) => {
            atlas_surah_out(s, out);
            1
        }
        None => 0,
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_surah_at(a: *const Atlas, i: u32, out: *mut QvpAtlasSurah) -> i32 {
    guard(|| {
        let atlas = &*a;
        match atlas.surahs.get(i as usize) {
            Some(s) => {
                atlas_surah_out(s, out);
                1
            }
            None => 0,
        }
    })
}
fn rubu_al_hizb_out(r: &qvp_core::qvp_format::atlas::AtlasRubuAlHizb) -> QvpAtlasRubuAlHizb {
    QvpAtlasRubuAlHizb { rubu_al_hizb: r.rubu_al_hizb, surah: r.surah, ayah: r.ayah, page: r.page }
}
/// kind: 0 juz, 1 hizb, 2 nisf, 3 rubu_al_hizb
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_division(a: *const Atlas, kind: u8, n: u16, out: *mut QvpAtlasRubuAlHizb) -> i32 {
    guard(|| {
        let r = match kind {
            0 => (*a).juz(n),
            1 => (*a).hizb(n),
            2 => (*a).nisf(n),
            _ => (*a).rubu_al_hizb(n),
        };
        match r {
            Some(r) => {
                *out = rubu_al_hizb_out(r);
                1
            }
            None => 0,
        }
    })
}
/// kind: 0 juz, 1 hizb, 2 nisf, 3 rubu_al_hizb → division number containing the ayah, or -1
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_division_of(a: *const Atlas, kind: u8, surah: u16, ayah: u16) -> i32 {
    guard(|| {
        let r = match kind {
            0 => (*a).juz_at(surah, ayah),
            1 => (*a).hizb_at(surah, ayah),
            2 => (*a).nisf_at(surah, ayah),
            _ => (*a).rubu_al_hizb_at(surah, ayah).map(|r| r.rubu_al_hizb),
        };
        r.map(|x| x as i32).unwrap_or(-1)
    })
}
/// out: first_page, last_page
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_pages_of_juz(a: *const Atlas, n: u16, out: *mut u16) -> i32 {
    guard(|| match (*a).pages_of_juz(n) {
        Some((f, l)) => {
            *out = f;
            *out.add(1) = l;
            1
        }
        None => 0,
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_search_surahs(
    a: *const Atlas,
    text: *const u8,
    len: u32,
    out: *mut u16,
    cap: u32,
) -> u32 {
    guard(|| fill_with(out, cap, (*a).search_surahs(in_str(text, len)).iter().map(|s| s.n)))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_atlas_json(a: *const Atlas, out: *mut QvpStr) {
    guard(|| {
        out_str(out, &(*a).to_json());
    })
}
