//! QVA1: the cross-page atlas. One small file for the whole mushaf answering
//! "which page is 2:255 on", surah first pages and names, and the 240 rubʿ
//! boundaries (juz/hizb/nisf derive from rubʿ numbers).
use crate::{read_varint, write_varint, Error};

pub const ATLAS_MAGIC: &[u8; 4] = b"QVA1";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtlasPage {
    pub page: u16,
    pub first: (u16, u16),
    pub last: (u16, u16),
    pub n_words: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AtlasSurah {
    pub n: u16,
    pub first_page: u16,
    pub ayah_count: u16,
    /// 0 = makkah, 1 = madinah, 255 = unknown
    pub place: u8,
    pub arabic: String,
    pub latin: String,
    pub english: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AtlasRub {
    pub rub: u16,
    pub sura: u16,
    pub ayah: u16,
    pub page: u16,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Atlas {
    /// sorted by page
    pub pages: Vec<AtlasPage>,
    /// sorted by n
    pub surahs: Vec<AtlasSurah>,
    /// sorted by rub
    pub rubs: Vec<AtlasRub>,
}

fn key(s: u16, a: u16) -> u32 {
    (s as u32) << 16 | a as u32
}

impl Atlas {
    pub fn encode(&self) -> Vec<u8> {
        let mut o = Vec::with_capacity(16 * 1024);
        o.extend_from_slice(ATLAS_MAGIC);
        write_varint(&mut o, self.pages.len() as u32);
        write_varint(&mut o, self.surahs.len() as u32);
        write_varint(&mut o, self.rubs.len() as u32);
        for p in &self.pages {
            for v in [p.page, p.first.0, p.first.1, p.last.0, p.last.1, p.n_words] {
                write_varint(&mut o, v as u32);
            }
        }
        let s = |o: &mut Vec<u8>, t: &str| {
            write_varint(o, t.len() as u32);
            o.extend_from_slice(t.as_bytes());
        };
        for su in &self.surahs {
            for v in [su.n, su.first_page, su.ayah_count, su.place as u16] {
                write_varint(&mut o, v as u32);
            }
            s(&mut o, &su.arabic);
            s(&mut o, &su.latin);
            s(&mut o, &su.english);
        }
        for r in &self.rubs {
            for v in [r.rub, r.sura, r.ayah, r.page] {
                write_varint(&mut o, v as u32);
            }
        }
        o
    }

    pub fn decode(b: &[u8]) -> Result<Atlas, Error> {
        if b.len() < 4 || &b[0..4] != ATLAS_MAGIC {
            return Err(Error::BadMagic);
        }
        let mut pos = 4;
        let v = |pos: &mut usize| -> Result<u16, Error> { Ok(read_varint(b, pos)? as u16) };
        let np = v(&mut pos)? as usize;
        let ns = v(&mut pos)? as usize;
        let nr = v(&mut pos)? as usize;
        let mut pages = Vec::with_capacity(np);
        for _ in 0..np {
            pages.push(AtlasPage { page: v(&mut pos)?, first: (v(&mut pos)?, v(&mut pos)?), last: (v(&mut pos)?, v(&mut pos)?), n_words: v(&mut pos)? });
        }
        let mut surahs = Vec::with_capacity(ns);
        for _ in 0..ns {
            let n = v(&mut pos)?;
            let first_page = v(&mut pos)?;
            let ayah_count = v(&mut pos)?;
            let place = v(&mut pos)? as u8;
            let st = |pos: &mut usize| -> Result<String, Error> {
                let len = read_varint(b, pos)? as usize;
                if *pos + len > b.len() {
                    return Err(Error::Truncated("atlas string"));
                }
                let s = std::str::from_utf8(&b[*pos..*pos + len]).map_err(|_| Error::Corrupt("utf8"))?.to_owned();
                *pos += len;
                Ok(s)
            };
            let arabic = st(&mut pos)?;
            let latin = st(&mut pos)?;
            let english = st(&mut pos)?;
            surahs.push(AtlasSurah { n, first_page, ayah_count, place, arabic, latin, english });
        }
        let mut rubs = Vec::with_capacity(nr);
        for _ in 0..nr {
            rubs.push(AtlasRub { rub: v(&mut pos)?, sura: v(&mut pos)?, ayah: v(&mut pos)?, page: v(&mut pos)? });
        }
        Ok(Atlas { pages, surahs, rubs })
    }

    /// Page holding ayah (sura, ayah). Binary search over first ayah per page —
    /// valid because no ayah spans two pages.
    pub fn page_of(&self, sura: u16, ayah: u16) -> Option<u16> {
        let k = key(sura, ayah);
        let i = self.pages.partition_point(|p| key(p.first.0, p.first.1) <= k);
        if i == 0 {
            return None;
        }
        let p = &self.pages[i - 1];
        (k <= key(p.last.0, p.last.1)).then_some(p.page)
    }
    pub fn page_range(&self, page: u16) -> Option<((u16, u16), (u16, u16))> {
        self.pages.iter().find(|p| p.page == page).map(|p| (p.first, p.last))
    }
    pub fn surah(&self, n: u16) -> Option<&AtlasSurah> {
        self.surahs.iter().find(|s| s.n == n)
    }
    pub fn page_of_surah(&self, n: u16) -> Option<u16> {
        self.surah(n).map(|s| s.first_page)
    }
    pub fn rub(&self, n: u16) -> Option<&AtlasRub> {
        self.rubs.iter().find(|r| r.rub == n)
    }
    pub fn juz(&self, n: u16) -> Option<&AtlasRub> {
        self.rub((n - 1) * 8 + 1)
    }
    pub fn hizb(&self, n: u16) -> Option<&AtlasRub> {
        self.rub((n - 1) * 4 + 1)
    }
    pub fn nisf(&self, n: u16) -> Option<&AtlasRub> {
        self.rub((n - 1) * 2 + 1)
    }
    /// Rubʿ containing the ayah (largest rub start ≤ ayah).
    pub fn rub_at(&self, sura: u16, ayah: u16) -> Option<&AtlasRub> {
        let k = key(sura, ayah);
        let i = self.rubs.partition_point(|r| key(r.sura, r.ayah) <= k);
        if i == 0 {
            None
        } else {
            Some(&self.rubs[i - 1])
        }
    }
    pub fn juz_at(&self, sura: u16, ayah: u16) -> Option<u16> {
        self.rub_at(sura, ayah).map(|r| (r.rub - 1) / 8 + 1)
    }
    pub fn hizb_at(&self, sura: u16, ayah: u16) -> Option<u16> {
        self.rub_at(sura, ayah).map(|r| (r.rub - 1) / 4 + 1)
    }
    pub fn nisf_at(&self, sura: u16, ayah: u16) -> Option<u16> {
        self.rub_at(sura, ayah).map(|r| (r.rub - 1) / 2 + 1)
    }
    /// Pages a juz spans (first..=last).
    pub fn pages_of_juz(&self, n: u16) -> Option<(u16, u16)> {
        let start = self.juz(n)?.page;
        let end = match self.juz(n + 1) {
            Some(next) => {
                // the next juz starts on `next.page`; if it starts at the top of that page the
                // previous juz ends on the page before
                let first_on_page = self.pages.iter().find(|p| p.page == next.page).map(|p| p.first);
                if first_on_page == Some((next.sura, next.ayah)) { next.page - 1 } else { next.page }
            }
            None => self.pages.last()?.page,
        };
        Some((start, end.max(start)))
    }
    /// Case-insensitive substring match on Arabic, Latin or English names.
    pub fn find_surah(&self, text: &str) -> Vec<&AtlasSurah> {
        let t = text.trim().to_lowercase();
        if t.is_empty() {
            return vec![];
        }
        self.surahs
            .iter()
            .filter(|s| s.arabic.contains(&t) || s.latin.to_lowercase().contains(&t) || s.english.to_lowercase().contains(&t) || s.n.to_string() == t)
            .collect()
    }
    pub fn to_json(&self) -> String {
        let esc = |s: &str| s.replace('\\', "\\\\").replace('"', "\\\"");
        let mut j = String::from("{\"pages\":[");
        for (i, p) in self.pages.iter().enumerate() {
            if i > 0 {
                j.push(',');
            }
            j.push_str(&format!("[{},\"{}:{}\",\"{}:{}\",{}]", p.page, p.first.0, p.first.1, p.last.0, p.last.1, p.n_words));
        }
        j.push_str("],\"surahs\":[");
        for (i, s) in self.surahs.iter().enumerate() {
            if i > 0 {
                j.push(',');
            }
            j.push_str(&format!(
                "{{\"n\":{},\"page\":{},\"ayahs\":{},\"place\":\"{}\",\"arabic\":\"{}\",\"latin\":\"{}\",\"english\":\"{}\"}}",
                s.n, s.first_page, s.ayah_count, match s.place { 0 => "makkah", 1 => "madinah", _ => "" }, esc(&s.arabic), esc(&s.latin), esc(&s.english)
            ));
        }
        j.push_str("],\"rubs\":[");
        for (i, r) in self.rubs.iter().enumerate() {
            if i > 0 {
                j.push(',');
            }
            j.push_str(&format!("[{},\"{}:{}\",{}]", r.rub, r.sura, r.ayah, r.page));
        }
        j.push_str("]}");
        j
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn roundtrip_and_lookup() {
        let a = Atlas {
            pages: vec![
                AtlasPage { page: 1, first: (1, 1), last: (1, 7), n_words: 29 },
                AtlasPage { page: 2, first: (2, 1), last: (2, 5), n_words: 36 },
                AtlasPage { page: 3, first: (2, 6), last: (2, 16), n_words: 100 },
            ],
            surahs: vec![
                AtlasSurah { n: 1, first_page: 1, ayah_count: 7, place: 0, arabic: "الفاتحة".into(), latin: "Al-Fatihah".into(), english: "The Opener".into() },
                AtlasSurah { n: 2, first_page: 2, ayah_count: 286, place: 1, arabic: "البقرة".into(), latin: "Al-Baqarah".into(), english: "The Cow".into() },
            ],
            rubs: vec![AtlasRub { rub: 1, sura: 1, ayah: 1, page: 1 }, AtlasRub { rub: 2, sura: 2, ayah: 26, page: 5 }],
        };
        let b = a.encode();
        let d = Atlas::decode(&b).unwrap();
        assert_eq!(a, d);
        assert_eq!(d.page_of(2, 3), Some(2));
        assert_eq!(d.page_of(2, 6), Some(3));
        assert_eq!(d.page_of(1, 7), Some(1));
        assert_eq!(d.page_of(3, 1), None);
        assert_eq!(d.page_of_surah(2), Some(2));
        assert_eq!(d.juz_at(2, 10), Some(1));
        assert_eq!(d.rub_at(2, 30).map(|r| r.rub), Some(2));
        assert_eq!(d.find_surah("cow")[0].n, 2);
        assert!(d.to_json().contains("\"latin\":\"Al-Baqarah\""));
    }
}
