//! Metadata read off the page: surahs, divisions, markers, sajdahs, rosettes.
use crate::{Page, NONE};
use qvp_format::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SurahInfo {
    pub number: u16,
    pub arabic: String,
    pub latin: String,
    pub english: String,
    /// "makkah" | "madinah" | ""
    pub revelation_place: String,
    pub ayah_count: u16,
    pub has_banner: bool,
    pub has_basmalah: bool,
    /// deco index of the banner, or NONE
    pub banner_deco: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Division {
    /// 0 juz, 1 hizb, 2 nisf, 3 rub
    pub kind: u8,
    pub n: u16,
    pub sura: u16,
    pub ayah: u16,
    pub line: u8,
    pub ayah_idx: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MarkerInfo {
    pub deco: u32,
    pub sura: u16,
    pub ayah: u16,
    pub line: u32,
    /// medallion centre and radius in page units (from the ornament, or the whole group)
    pub cx: f32,
    pub cy: f32,
    pub r: f32,
    /// ornament and numeral path index ranges (NONE when absent)
    pub ornament_path: u32,
    pub numeral_path: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rosette {
    pub deco: u32,
    pub sura: u16,
    pub ayah: u16,
    pub juz: u16,
    pub hizb: u16,
    pub nisf: u16,
    pub rub: u16,
    pub rub_in_hizb: u16,
}

impl Page {
    pub fn surahs(&self) -> Vec<SurahInfo> {
        let d = self.data();
        let mut out: Vec<SurahInfo> = Vec::new();
        for w in &d.words {
            if !out.iter().any(|s| s.number == w.sura) {
                out.push(SurahInfo { number: w.sura, arabic: String::new(), latin: String::new(), english: String::new(), revelation_place: String::new(), ayah_count: 0, has_banner: false, has_basmalah: false, banner_deco: NONE });
            }
        }
        for (di, dc) in d.decos.iter().enumerate() {
            if dc.kind != DecoKind::SurahName && dc.kind != DecoKind::Basmalah {
                continue;
            }
            let s = match out.iter_mut().find(|s| s.number == dc.sura) {
                Some(s) => s,
                None => {
                    out.push(SurahInfo { number: dc.sura, arabic: String::new(), latin: String::new(), english: String::new(), revelation_place: String::new(), ayah_count: 0, has_banner: false, has_basmalah: false, banner_deco: NONE });
                    out.last_mut().unwrap()
                }
            };
            if dc.text != NONE_U16 {
                let p: Vec<&str> = d.strings[dc.text as usize].split('|').collect();
                if p.len() >= 5 {
                    s.arabic = p[0].into();
                    s.latin = p[1].into();
                    s.english = p[2].into();
                    s.revelation_place = p[3].into();
                    s.ayah_count = p[4].parse().unwrap_or(0);
                }
            }
            if dc.kind == DecoKind::SurahName {
                s.has_banner = true;
                s.banner_deco = di as u32;
            } else {
                s.has_basmalah = true;
            }
        }
        out.sort_by_key(|s| s.number);
        out
    }

    /// Divisions that start on this page, sorted by reading order.
    pub fn divisions(&self) -> Vec<Division> {
        let d = self.data();
        let mut out = Vec::new();
        for (ai, a) in d.ayahs.iter().enumerate() {
            if a.flags == 0 || a.rub == 0 || a.part != 1 {
                continue;
            }
            let line = d.lines[d.words.get(a.first_word as usize).map(|w| w.line_idx as usize).unwrap_or(0)].line_no;
            let push = |out: &mut Vec<Division>, kind: u8, n: u16| out.push(Division { kind, n, sura: a.sura, ayah: a.ayah, line, ayah_idx: ai as u32 });
            if a.flags & AF_JUZ_START != 0 {
                push(&mut out, 0, (a.rub - 1) / 8 + 1);
            }
            if a.flags & AF_HIZB_START != 0 {
                push(&mut out, 1, (a.rub - 1) / 4 + 1);
            }
            if a.flags & AF_NISF_START != 0 {
                push(&mut out, 2, (a.rub - 1) / 2 + 1);
            }
            if a.flags & AF_RUB_START != 0 {
                push(&mut out, 3, a.rub);
            }
        }
        out
    }

    /// Drawn hizb rosettes.
    pub fn rosettes(&self) -> Vec<Rosette> {
        let d = self.data();
        d.decos
            .iter()
            .enumerate()
            .filter(|(_, x)| x.kind == DecoKind::HizbMark)
            .map(|(i, x)| {
                let mut r = Rosette { deco: i as u32, sura: x.sura, ayah: x.ayah, juz: 0, hizb: 0, nisf: 0, rub: 0, rub_in_hizb: 0 };
                if x.text != NONE_U16 {
                    for kv in d.strings[x.text as usize].split(';') {
                        if let Some((k, v)) = kv.split_once('=') {
                            let n = v.parse().unwrap_or(0);
                            match k {
                                "juz" => r.juz = n,
                                "hizb" => r.hizb = n,
                                "nisf" => r.nisf = n,
                                "rub" => r.rub = n,
                                _ => {}
                            }
                        }
                    }
                    if r.hizb > 0 && r.rub > 0 {
                        r.rub_in_hizb = r.rub - (r.hizb - 1) * 4;
                    }
                }
                r
            })
            .collect()
    }

    /// Sajdah sites: (deco, sura, ayah, sign path index).
    pub fn sajdahs(&self) -> Vec<(u32, u16, u16, u32)> {
        let d = self.data();
        let mut out = Vec::new();
        for (di, dc) in d.decos.iter().enumerate() {
            for p in dc.first_path..dc.first_path + dc.n_paths as u32 {
                if d.paths[p as usize].mark == Mark::SajdahSign {
                    out.push((di as u32, dc.sura, dc.ayah, p));
                }
            }
        }
        out
    }

    /// Real ayah medallions (with an ayah id). Decorative rosettes without an id are excluded.
    pub fn markers(&self) -> Vec<MarkerInfo> {
        let d = self.data();
        let q = self.quant();
        d.decos
            .iter()
            .enumerate()
            .filter(|(_, x)| x.kind == DecoKind::AyahMarker && x.ayah != 0)
            .map(|(i, x)| {
                let mut orn = NONE;
                let mut num = NONE;
                let mut ob = IBox::EMPTY;
                for p in x.first_path..x.first_path + x.n_paths as u32 {
                    let pr = &d.paths[p as usize];
                    match pr.kind {
                        PathKind::AyahOrnament => {
                            if orn == NONE {
                                orn = p;
                            }
                            ob.union(&pr.bbox);
                        }
                        PathKind::AyahNumber => num = p,
                        _ => {}
                    }
                }
                let b = if ob.is_empty() { x.bbox } else { ob };
                MarkerInfo {
                    deco: i as u32,
                    sura: x.sura,
                    ayah: x.ayah,
                    line: self.geometry().table[x.first_path as usize].line,
                    cx: (b.x0 + b.x1) as f32 / 2.0 / q,
                    cy: (b.y0 + b.y1) as f32 / 2.0 / q,
                    r: ((b.x1 - b.x0).max(b.y1 - b.y0)) as f32 / 2.0 / q,
                    ornament_path: orn,
                    numeral_path: num,
                }
            })
            .collect()
    }

    /// The medallion that closes (sura, ayah), if drawn on this page.
    pub fn marker_of(&self, sura: u16, ayah: u16) -> Option<MarkerInfo> {
        self.markers().into_iter().find(|m| m.sura == sura && m.ayah == ayah)
    }

    /// True for banner lines (surah name / basmalah) that hold no words.
    pub fn line_is_header(&self, li: usize) -> bool {
        let d = self.data();
        d.lines[li].n_words == 0 && d.decos.iter().any(|x| x.line as usize == li && matches!(x.kind, DecoKind::SurahName | DecoKind::Basmalah))
    }

    /// Accessible label for a word: "text (s:a:w)".
    pub fn word_label(&self, wi: u32) -> String {
        let w = &self.data().words[wi as usize];
        format!("{} ({}:{}:{})", self.word_text(wi), w.sura, w.ayah, w.word)
    }
    /// Accessible label for an ayah fragment.
    pub fn ayah_label(&self, ai: u32) -> String {
        let a = &self.data().ayahs[ai as usize];
        let name = self.surahs().into_iter().find(|s| s.number == a.sura).map(|s| s.latin).filter(|s| !s.is_empty()).unwrap_or_else(|| format!("surah {}", a.sura));
        if a.parts > 1 {
            format!("Ayah {} of {}, part {} of {}", a.ayah, name, a.part, a.parts)
        } else {
            format!("Ayah {} of {}", a.ayah, name)
        }
    }
}
