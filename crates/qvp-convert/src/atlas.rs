//! Builds the cross-page atlas from converted pages.
use qvp_format::atlas::*;
use qvp_format::{DecoKind, PageData, NONE_U16};
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Builder {
    pages: Vec<AtlasPage>,
    surahs: BTreeMap<u16, AtlasSurah>,
    rubs: BTreeMap<u16, AtlasRub>,
}

impl Builder {
    pub fn add_page(&mut self, p: &PageData) {
        let page = p.header.page;
        if let (Some(f), Some(l)) = (p.words.first(), p.words.last()) {
            self.pages.push(AtlasPage { page, first: (f.sura, f.ayah), last: (l.sura, l.ayah), n_words: p.words.len() as u16 });
        }
        for w in &p.words {
            self.surahs.entry(w.sura).or_insert_with(|| AtlasSurah { n: w.sura, first_page: page, ayah_count: 0, place: 255, arabic: String::new(), latin: String::new(), english: String::new() });
            let s = self.surahs.get_mut(&w.sura).unwrap();
            if page < s.first_page {
                s.first_page = page;
            }
        }
        for d in p.decos.iter().filter(|d| d.kind == DecoKind::SurahName && d.text != NONE_U16) {
            let parts: Vec<&str> = p.strings[d.text as usize].split('|').collect();
            if parts.len() < 5 {
                continue;
            }
            let s = self.surahs.entry(d.sura).or_insert_with(|| AtlasSurah { n: d.sura, first_page: page, ayah_count: 0, place: 255, arabic: String::new(), latin: String::new(), english: String::new() });
            s.first_page = s.first_page.min(page);
            s.arabic = parts[0].to_owned();
            s.latin = parts[1].to_owned();
            s.english = parts[2].to_owned();
            s.place = match parts[3] { "makkah" => 0, "madinah" => 1, _ => 255 };
            s.ayah_count = parts[4].parse().unwrap_or(0);
        }
        for a in p.ayahs.iter().filter(|a| a.rub != 0) {
            self.rubs.entry(a.rub).or_insert(AtlasRub { rub: a.rub, sura: a.sura, ayah: a.ayah, page });
        }
    }
    pub fn build(mut self) -> Atlas {
        self.pages.sort_by_key(|p| p.page);
        Atlas { pages: self.pages, surahs: self.surahs.into_values().collect(), rubs: self.rubs.into_values().collect() }
    }
}
