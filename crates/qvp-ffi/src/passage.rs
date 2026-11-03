//! Passage: a range of complete ayahs laid out on rows of a given width.

use crate::abi::{fill_with, guard, out_str, QvpStr};
use crate::convert::placement;
use qvp_core::*;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpPassageSpec {
    pub width: f32,
    pub scale: f32,
    pub line_spacing: f32,
    pub padding: f32,
    pub align: u8,
    pub keep_ayah_mark: u8,
    pub _pad: [u8; 2],
    pub max_rows: u32,
    pub ellipsis_width: f32,
    pub max_stretch: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct QvpPassageLayout {
    pub width: f32,
    pub height: f32,
    pub scale: f32,
    pub line_spacing: f32,
    pub n_rows: u32,
    pub n_words: u32,
    pub n_ayahs: u32,
    pub n_draws: u32,
    pub is_truncated: u8,
    pub _pad: [u8; 3],
    pub ellipsis_x0: f32,
    pub ellipsis_y0: f32,
    pub ellipsis_x1: f32,
    pub ellipsis_y1: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpPassageRow {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub baseline: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpPassageWord {
    pub surah: u16,
    pub ayah: u16,
    pub word: u16,
    pub page: u16,
    pub row: u32,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub word_x0: f32,
    pub word_y0: f32,
    pub word_x1: f32,
    pub word_y1: f32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct QvpPassageAyah {
    pub surah: u16,
    pub ayah: u16,
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
}

#[no_mangle]
pub unsafe extern "C" fn qvp_passage_load(
    pages: *const *const Page,
    n_pages: u32,
    surah: u16,
    from: u16,
    to: u16,
) -> *mut Passage {
    guard(|| {
        let Some(refs) = passage_pages(pages, n_pages) else { return std::ptr::null_mut() };
        match Passage::load(&refs, surah, from, to) {
            Ok(passage) => Box::into_raw(Box::new(passage)),
            Err(_) => std::ptr::null_mut(),
        }
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_passage_load_basmalah(
    pages: *const *const Page,
    n_pages: u32,
    surah: u16,
) -> *mut Passage {
    guard(|| {
        let Some(refs) = passage_pages(pages, n_pages) else { return std::ptr::null_mut() };
        match Passage::load_basmalah(&refs, surah) {
            Ok(passage) => Box::into_raw(Box::new(passage)),
            Err(_) => std::ptr::null_mut(),
        }
    })
}
/// The pages a passage loads from; `None` when the list is empty or holds a null page.
unsafe fn passage_pages<'a>(pages: *const *const Page, n_pages: u32) -> Option<Vec<&'a Page>> {
    if pages.is_null() || n_pages == 0 {
        return None;
    }
    let mut refs = Vec::with_capacity(n_pages as usize);
    for i in 0..n_pages as usize {
        let page = *pages.add(i);
        if page.is_null() {
            return None;
        }
        refs.push(&*page);
    }
    Some(refs)
}
#[no_mangle]
pub unsafe extern "C" fn qvp_passage_free(passage: *mut Passage) {
    guard(|| {
        if !passage.is_null() {
            drop(Box::from_raw(passage));
        }
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_passage_ayah_count(passage: *const Passage) -> u32 {
    guard(|| (*passage).ayahs().count() as u32)
}
#[no_mangle]
pub unsafe extern "C" fn qvp_passage_ayah_text(passage: *const Passage, index: u32, out: *mut QvpStr) {
    guard(|| {
        let text = (*passage).ayahs().nth(index as usize).map_or("", |(_, _, t)| t);
        out_str(out, text);
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_passage_line_spacing(passage: *const Passage) -> f32 {
    guard(|| (*passage).line_spacing())
}
#[no_mangle]
pub unsafe extern "C" fn qvp_passage_layout(
    passage: *mut Passage,
    spec: *const QvpPassageSpec,
    out: *mut QvpPassageLayout,
) -> i32 {
    guard(|| {
        let s = &*spec;
        let align = match s.align {
            0 => Align::Right,
            1 => Align::Center,
            2 => Align::Justified,
            _ => return 0,
        };
        let spec = PassageSpec {
            width: s.width,
            scale: s.scale,
            line_spacing: s.line_spacing,
            padding: s.padding,
            align,
            max_rows: s.max_rows,
            keep_ayah_mark: s.keep_ayah_mark != 0,
            ellipsis_width: s.ellipsis_width,
            // 0 is the default, as for the page's reflow; a negative value reaches the core as it
            // is, which reads any value at or below 0 as no cap.
            max_stretch: if s.max_stretch == 0.0 { qvp_core::defaults::REFLOW_MAX_STRETCH } else { s.max_stretch },
        };
        let Some(l) = (*passage).layout(&spec) else { return 0 };
        if !out.is_null() {
            let [ellipsis_x0, ellipsis_y0, ellipsis_x1, ellipsis_y1] = l.ellipsis.unwrap_or_default();
            *out = QvpPassageLayout {
                width: l.width,
                height: l.height,
                scale: l.scale,
                line_spacing: l.line_spacing,
                n_rows: l.rows.len() as u32,
                n_words: l.words.len() as u32,
                n_ayahs: l.ayahs.len() as u32,
                n_draws: l.draws.len() as u32,
                is_truncated: l.is_truncated as u8,
                _pad: [0; 3],
                ellipsis_x0,
                ellipsis_y0,
                ellipsis_x1,
                ellipsis_y1,
            };
        }
        1
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_passage_rows(passage: *const Passage, out: *mut QvpPassageRow, cap: u32) -> u32 {
    guard(|| {
        let Some(l) = (*passage).current_layout() else { return 0 };
        fill_with(
            out,
            cap,
            l.rows.iter().map(|r| QvpPassageRow { x0: r.x0, y0: r.y0, x1: r.x1, y1: r.y1, baseline: r.baseline }),
        )
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_passage_words(passage: *const Passage, out: *mut QvpPassageWord, cap: u32) -> u32 {
    guard(|| {
        let Some(l) = (*passage).current_layout() else { return 0 };
        fill_with(
            out,
            cap,
            l.words.iter().map(|w| QvpPassageWord {
                surah: w.surah,
                ayah: w.ayah,
                word: w.word,
                page: w.page,
                row: w.row,
                x0: w.x0,
                y0: w.y0,
                x1: w.x1,
                y1: w.y1,
                word_x0: w.word_x0,
                word_y0: w.word_y0,
                word_x1: w.word_x1,
                word_y1: w.word_y1,
            }),
        )
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_passage_ayahs(passage: *const Passage, out: *mut QvpPassageAyah, cap: u32) -> u32 {
    guard(|| {
        let Some(l) = (*passage).current_layout() else { return 0 };
        fill_with(
            out,
            cap,
            l.ayahs.iter().map(|a| QvpPassageAyah {
                surah: a.surah,
                ayah: a.ayah,
                x0: a.x0,
                y0: a.y0,
                x1: a.x1,
                y1: a.y1,
            }),
        )
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_passage_draw_list(passage: *const Passage, out: *mut u32, cap: u32) -> u32 {
    guard(|| {
        let Some(l) = (*passage).current_layout() else { return 0 };
        fill_with(out.cast(), cap, l.draws.iter().map(|d| [d.page as u32, d.path, d.placement]))
    })
}
#[no_mangle]
pub unsafe extern "C" fn qvp_passage_placements(passage: *const Passage, out: *mut f32, cap: u32) -> u32 {
    guard(|| {
        let Some(l) = (*passage).current_layout() else { return 0 };
        fill_with(out.cast(), cap, l.placements.iter().map(placement))
    })
}
