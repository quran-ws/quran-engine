//! Whole-word range selection and citations.
use crate::Page;
use std::fmt::Write;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Selection {
    pub anchor: Option<u32>,
    pub focus: Option<u32>,
}

impl Selection {
    pub fn words(&self) -> Vec<u32> {
        match (self.anchor, self.focus) {
            (Some(a), Some(f)) => (a.min(f)..=a.max(f)).collect(),
            (Some(a), None) => vec![a],
            _ => vec![],
        }
    }
    pub fn is_empty(&self) -> bool {
        self.anchor.is_none()
    }
}

impl Page {
    /// "2:255", "2:255-257", "2:286, 3:1-2" for a word list in reading order.
    pub fn citation(&self, words: &[u32]) -> String {
        let d = self.data();
        let mut out = String::new();
        let mut i = 0;
        while i < words.len() {
            let w = &d.words[words[i] as usize];
            let (s, a0) = (w.surah, w.ayah);
            let mut a1 = a0;
            let mut j = i + 1;
            while j < words.len() {
                let x = &d.words[words[j] as usize];
                if x.surah != s || x.ayah > a1 + 1 {
                    break;
                }
                a1 = a1.max(x.ayah);
                j += 1;
            }
            if !out.is_empty() {
                out.push_str(", ");
            }
            if a1 == a0 {
                write!(out, "{s}:{a0}").unwrap();
            } else {
                write!(out, "{s}:{a0}-{a1}").unwrap();
            }
            i = j;
        }
        out
    }
}
