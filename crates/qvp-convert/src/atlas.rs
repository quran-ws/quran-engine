//! Builds the cross-page atlas from converted pages.
use qvp_format::atlas::*;
use qvp_format::{DecoKind, PageData, NONE_U16};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Builder {
    pages: Vec<AtlasPage>,
    surahs: BTreeMap<u16, AtlasSurah>,
    rubu_al_hizbs: BTreeMap<u16, AtlasRubuAlHizb>,
}

impl Builder {
    pub fn add_page(&mut self, p: &PageData) {
        let page = p.header.page;
        if let (Some(f), Some(l)) = (p.words.first(), p.words.last()) {
            self.pages.push(AtlasPage {
                page,
                first: (f.surah, f.ayah),
                last: (l.surah, l.ayah),
                n_words: p.words.len() as u16,
            });
        }
        for w in &p.words {
            self.surahs.entry(w.surah).or_insert_with(|| AtlasSurah {
                n: w.surah,
                first_page: page,
                ayah_count: 0,
                place: 255,
                arabic: String::new(),
                latin: String::new(),
                english: String::new(),
            });
            let s = self.surahs.get_mut(&w.surah).unwrap();
            if page < s.first_page {
                s.first_page = page;
            }
        }
        for d in p.decorations.iter().filter(|d| d.kind == DecoKind::SurahName && d.text != NONE_U16) {
            let fragments: Vec<&str> = p.strings[d.text as usize].split('|').collect();
            if fragments.len() < 5 {
                continue;
            }
            let s = self.surahs.entry(d.surah).or_insert_with(|| AtlasSurah {
                n: d.surah,
                first_page: page,
                ayah_count: 0,
                place: 255,
                arabic: String::new(),
                latin: String::new(),
                english: String::new(),
            });
            s.first_page = s.first_page.min(page);
            s.arabic = fragments[0].to_owned();
            s.latin = fragments[1].to_owned();
            s.english = fragments[2].to_owned();
            s.place = match fragments[3] {
                "makkah" => 0,
                "madinah" => 1,
                _ => 255,
            };
            s.ayah_count = fragments[4].parse().unwrap_or(0);
        }
        for a in p.ayahs.iter().filter(|a| a.rubu_al_hizb != 0) {
            self.rubu_al_hizbs.entry(a.rubu_al_hizb).or_insert(AtlasRubuAlHizb {
                rubu_al_hizb: a.rubu_al_hizb,
                surah: a.surah,
                ayah: a.ayah,
                page,
            });
        }
    }
    pub fn build(mut self) -> Atlas {
        self.pages.sort_by_key(|p| p.page);
        Atlas {
            pages: self.pages,
            surahs: self.surahs.into_values().collect(),
            rubu_al_hizbs: self.rubu_al_hizbs.into_values().collect(),
        }
    }
}
