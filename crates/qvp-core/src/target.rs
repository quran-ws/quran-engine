//! Targets: anything that resolves to a list of word indices in reading order.
use crate::Page;

#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Page,
    Word(u32),
    Words(Vec<u32>),
    /// surah:ayah — all fragments on this page
    Ayah(u16, u16),
    /// inclusive range of ayahs (surah, from, to) on this page
    AyahRange(u16, u16, u16),
    /// printed line number (1-based, as in the file)
    Line(u8),
    Surah(u16),
    /// words from index a to b inclusive, either order
    Range(u32, u32),
}

impl Page {
    pub fn target_words(&self, t: &Target) -> Vec<u32> {
        let d = self.data();
        let n = d.words.len() as u32;
        let mut v: Vec<u32> = match t {
            Target::Page => (0..n).collect(),
            Target::Word(w) => {
                if *w < n {
                    vec![*w]
                } else {
                    vec![]
                }
            }
            Target::Words(ws) => ws.iter().copied().filter(|w| *w < n).collect(),
            Target::Ayah(s, a) => {
                (0..n).filter(|&i| d.words[i as usize].surah == *s && d.words[i as usize].ayah == *a).collect()
            }
            Target::AyahRange(s, a, b) => (0..n)
                .filter(|&i| {
                    let w = &d.words[i as usize];
                    w.surah == *s && w.ayah >= *a && w.ayah <= *b
                })
                .collect(),
            Target::Line(l) => d
                .lines
                .iter()
                .filter(|x| x.line_number == *l)
                .flat_map(|x| x.first_word as u32..(x.first_word + x.n_words) as u32)
                .collect(),
            Target::Surah(s) => (0..n).filter(|&i| d.words[i as usize].surah == *s).collect(),
            Target::Range(a, b) => {
                let (lo, hi) = (a.min(b), a.max(b));
                (*lo..=(*hi).min(n.saturating_sub(1))).filter(|_| n > 0).collect()
            }
        };
        v.sort_unstable();
        v.dedup();
        v
    }

    /// Ayah keys present on the page in reading order, deduplicated.
    pub fn ayah_keys(&self) -> Vec<(u16, u16)> {
        let mut v = Vec::new();
        for a in &self.data().ayahs {
            if v.last() != Some(&(a.surah, a.ayah)) && !v.contains(&(a.surah, a.ayah)) {
                v.push((a.surah, a.ayah));
            }
        }
        v
    }

    /// Number of words of (surah, ayah) on this page, and whether the whole ayah is here.
    pub fn ayah_word_count(&self, surah: u16, ayah: u16) -> (u32, bool) {
        let d = self.data();
        let frags: Vec<_> = d.ayahs.iter().filter(|a| a.surah == surah && a.ayah == ayah).collect();
        let count: u32 = frags.iter().map(|a| a.n_words as u32).sum();
        let complete = frags.first().map(|a| frags.len() as u8 == a.fragments).unwrap_or(false);
        (count, complete)
    }

    /// Map recitation segments onto words: Some(words) when the segment count equals
    /// the ayah's word count and the whole ayah is on this page; None on mismatch
    /// (follow the ayah whole instead of drifting).
    pub fn recite_map(&self, surah: u16, ayah: u16, n_segments: u32) -> Option<Vec<u32>> {
        let (count, complete) = self.ayah_word_count(surah, ayah);
        if !complete || count != n_segments {
            return None;
        }
        Some(self.target_words(&Target::Ayah(surah, ayah)))
    }

    pub fn next_word(&self, wi: u32) -> Option<u32> {
        (wi + 1 < self.data().words.len() as u32).then_some(wi + 1)
    }
    pub fn prev_word(&self, wi: u32) -> Option<u32> {
        (wi > 0).then(|| wi - 1)
    }
}
