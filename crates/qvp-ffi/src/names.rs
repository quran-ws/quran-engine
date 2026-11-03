//! Names: the name tables the engine owns, and the engine's own version.

use crate::abi::{guard, in_str, out_str, QvpStr};
use qvp_core::qvp_format::{Category, DecoKind, Family, Mark, PathKind};
use std::ffi::c_char;

#[no_mangle]
pub unsafe extern "C" fn qvp_mark_name(mark: u8, out: *mut QvpStr) {
    guard(|| {
        let s = Mark::from_u8(mark).as_str();
        *out = QvpStr { ptr: s.as_ptr(), len: s.len() as u32 };
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_family_name(f: u8, out: *mut QvpStr) {
    guard(|| {
        let s = Family::from_u8(f).as_str();
        *out = QvpStr { ptr: s.as_ptr(), len: s.len() as u32 };
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_kind_name(k: u8, out: *mut QvpStr) {
    guard(|| {
        let s = PathKind::from_u8(k).as_str();
        *out = QvpStr { ptr: s.as_ptr(), len: s.len() as u32 };
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_category_name(c: u8, out: *mut QvpStr) {
    guard(|| {
        let s = Category::from_u8(c).as_str();
        *out = QvpStr { ptr: s.as_ptr(), len: s.len() as u32 };
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_mark_from_name(s: *const u8, len: u32) -> u8 {
    guard(|| guard(|| Mark::from_svg(in_str(s, len)) as u8))
}
/// The name tables the engine owns. `qvp_name`, `qvp_name_id` and `qvp_name_count` cover
/// every one of them, so a wrapper never carries a table of its own.
pub const QVP_NAMES_MARK: u8 = 0;
pub const QVP_NAMES_KIND: u8 = 1;
pub const QVP_NAMES_FAMILY: u8 = 2;
pub const QVP_NAMES_CATEGORY: u8 = 3;
pub const QVP_NAMES_DECORATION: u8 = 4;
pub const QVP_NAMES_DIVISION: u8 = 5;
pub const QVP_NAMES_PLACE: u8 = 6;

const DIVISION_NAMES: [&str; 4] = ["juz", "hizb", "nisf", "rubu_al_hizb"];
const PLACE_NAMES: [&str; 2] = ["makkah", "madinah"];

/// How many ids a table has (ids run from 0; 255 is "unknown" everywhere).
fn name_count(table: u8) -> u32 {
    match table {
        QVP_NAMES_MARK => 36,
        QVP_NAMES_KIND => 8,
        QVP_NAMES_FAMILY => 8,
        QVP_NAMES_CATEGORY => 9,
        QVP_NAMES_DECORATION => 7,
        QVP_NAMES_DIVISION => 4,
        QVP_NAMES_PLACE => 2,
        _ => 0,
    }
}

fn name_of(table: u8, id: u8) -> &'static str {
    if id as u32 >= name_count(table) {
        return "";
    }
    match table {
        QVP_NAMES_MARK => Mark::from_u8(id).as_str(),
        QVP_NAMES_KIND => PathKind::from_u8(id).as_str(),
        QVP_NAMES_FAMILY => Family::from_u8(id).as_str(),
        QVP_NAMES_CATEGORY => Category::from_u8(id).as_str(),
        QVP_NAMES_DECORATION => DecoKind::from_u8(id).as_str(),
        QVP_NAMES_DIVISION => DIVISION_NAMES[id as usize],
        QVP_NAMES_PLACE => PLACE_NAMES[id as usize],
        _ => "",
    }
}

/// Number of ids in a name table (QVP_NAMES_*).
#[no_mangle]
pub extern "C" fn qvp_name_count(table: u8) -> u32 {
    guard(|| name_count(table))
}
/// The name of `id` in a table; empty for an id outside the table.
#[no_mangle]
pub unsafe extern "C" fn qvp_name(table: u8, id: u8, out: *mut QvpStr) {
    guard(|| {
        let s = name_of(table, id);
        *out = QvpStr { ptr: s.as_ptr(), len: s.len() as u32 };
    })
}
/// The id of a name in a table, or 255 when the table has no such name.
#[no_mangle]
pub unsafe extern "C" fn qvp_name_id(table: u8, s: *const u8, len: u32) -> u8 {
    guard(|| {
        let name = in_str(s, len);
        (0..name_count(table)).map(|i| i as u8).find(|&i| name_of(table, i) == name).unwrap_or(255)
    })
}
#[no_mangle]
pub extern "C" fn qvp_mark_category(mark: u8) -> u8 {
    guard(|| guard(|| Mark::from_u8(mark).category() as u8))
}
#[no_mangle]
pub unsafe extern "C" fn qvp_version(out: *mut QvpStr) {
    guard(|| out_str(out, env!("CARGO_PKG_VERSION")))
}
/// The page format version this engine reads (`docs/FORMAT.md`).
#[no_mangle]
pub extern "C" fn qvp_format_version() -> u32 {
    guard(|| guard(|| qvp_core::qvp_format::VERSION as u32))
}
/// Keeps the symbol referenced on platforms that need a C string somewhere.
#[no_mangle]
pub extern "C" fn qvp_engine_name() -> *const c_char {
    guard(|| guard(|| c"qvp".as_ptr()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn name_tables_round_trip() {
        fn name(table: u8, id: u8) -> String {
            let mut o = QvpStr { ptr: std::ptr::null(), len: 0 };
            unsafe { qvp_name(table, id, &mut o) };
            unsafe { String::from_utf8_lossy(std::slice::from_raw_parts(o.ptr, o.len as usize)).into_owned() }
        }
        for table in QVP_NAMES_MARK..=QVP_NAMES_PLACE {
            let n = qvp_name_count(table);
            assert!(n > 0);
            for id in 1..n as u8 {
                let s = name(table, id);
                assert!(!s.is_empty(), "table {table} id {id}");
                assert_eq!(unsafe { qvp_name_id(table, s.as_ptr(), s.len() as u32) }, id, "{s}");
            }
            assert_eq!(name(table, 200), "");
            assert_eq!(unsafe { qvp_name_id(table, b"nope".as_ptr(), 4) }, 255);
        }
        assert_eq!(name(QVP_NAMES_MARK, 7), "shaddah");
        assert_eq!(name(QVP_NAMES_DECORATION, 0), "ayah-mark");
        assert_eq!(name(QVP_NAMES_DIVISION, 3), "rubu_al_hizb");
        assert_eq!(name(QVP_NAMES_PLACE, 1), "madinah");
    }
}
