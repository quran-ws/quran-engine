//! Arabic text tools, text extraction and search.
use crate::{Page, NONE};
use qvp_format::NONE_U16;

/// Text forms carried per word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Form {
    Uthmani = 0,
    Imlaei = 1,
    Qpc = 2,
    Rasm = 3,
    Search = 4,
}

impl Form {
    pub fn from_u8(v: u8) -> Form {
        match v {
            1 => Form::Imlaei,
            2 => Form::Qpc,
            3 => Form::Rasm,
            4 => Form::Search,
            _ => Form::Uthmani,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Form::Uthmani => "uthmani",
            Form::Imlaei => "imlaei",
            Form::Qpc => "qpc",
            Form::Rasm => "rasm",
            Form::Search => "search",
        }
    }
}

/// Harakat, tanween (incl. the open forms U+08F0–08F2), dagger alef, waqf/dabt
/// block, tatweel, the rubʿ sign, small letters and Quranic annotation signs.
pub fn is_arabic_mark(c: char) -> bool {
    matches!(c as u32,
        0x064B..=0x0652 | 0x0653..=0x0655 | 0x0656..=0x065F | 0x0670 | 0x06D6..=0x06DC | 0x06DF..=0x06E4 |
        0x06E7..=0x06E8 | 0x06EA..=0x06ED | 0x08D3..=0x08E1 | 0x08E3..=0x08FF | 0x0640 | 0x06DD | 0x06DE | 0x06E9 | 0x0610..=0x061A)
}

pub fn strip_marks(s: &str) -> String {
    s.chars().filter(|c| !is_arabic_mark(*c)).collect()
}

/// أإآٱ→ا, ى→ي, ة→ه, ؤ→و, ئ→ي
pub fn fold(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'أ' | 'إ' | 'آ' | 'ٱ' => 'ا',
            'ى' => 'ي',
            'ة' => 'ه',
            'ؤ' => 'و',
            'ئ' => 'ي',
            c => c,
        })
        .collect()
}

/// strip + fold + collapse whitespace: the default search key.
pub fn normalize_query(s: &str) -> String {
    fold(&strip_marks(s)).split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Additionally drops bare alef and hamza so a typed الرحمان finds the printed الرحمن.
pub fn loose_key(s: &str) -> String {
    normalize_query(s).chars().filter(|c| !matches!(c, 'ا' | 'ء')).collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum SearchMode {
    Includes = 0,
    Exact = 1,
    Prefix = 2,
}

#[derive(Clone, Debug)]
pub struct SearchOptions {
    pub form: Form,
    pub mode: SearchMode,
    pub normalize: bool,
    /// Retry with `loose_key` when the strict pass found nothing.
    pub loose: bool,
    pub limit: usize,
}

impl Default for SearchOptions {
    fn default() -> Self {
        SearchOptions { form: Form::Search, mode: SearchMode::Includes, normalize: true, loose: true, limit: usize::MAX }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub word: u32,
    /// byte offset of the match in the (normalised) word value
    pub index: usize,
    pub loose: bool,
}

impl Page {
    pub fn word_form(&self, wi: u32, form: Form) -> &str {
        let w = &self.data().words[wi as usize];
        let r = match form {
            Form::Uthmani => w.text,
            Form::Imlaei => w.imlaei,
            Form::Qpc => w.qpc,
            Form::Rasm => w.rasm,
            Form::Search => w.search,
        };
        if r == NONE_U16 {
            // fall back to uthmani, then search form
            if form != Form::Uthmani && w.text != NONE_U16 {
                return &self.data().strings[w.text as usize];
            }
            ""
        } else {
            &self.data().strings[r as usize]
        }
    }

    /// Text of a word list in reading order, with the mushaf's own line breaks.
    pub fn text_of(&self, words: &[u32], form: Form, word_sep: &str, line_sep: &str) -> String {
        let mut out = String::new();
        let mut last_line = NONE;
        for &wi in words {
            let w = &self.data().words[wi as usize];
            let line = w.line_idx as u32;
            if !out.is_empty() {
                out.push_str(if last_line != NONE && line != last_line { line_sep } else { word_sep });
            }
            out.push_str(self.word_form(wi, form));
            last_line = line;
        }
        out
    }

    pub fn search(&self, query: &str, opt: &SearchOptions) -> Vec<Match> {
        let mut out = Vec::new();
        let strict_q = if opt.normalize { normalize_query(query) } else { query.to_owned() };
        if strict_q.is_empty() {
            return out;
        }
        let pass = |q: &str, loose: bool, out: &mut Vec<Match>| {
            for wi in 0..self.data().words.len() as u32 {
                let raw = self.word_form(wi, opt.form);
                let v: String = if loose { loose_key(raw) } else if opt.normalize { normalize_query(raw) } else { raw.to_owned() };
                let hit = match opt.mode {
                    SearchMode::Exact => (v == q).then_some(0),
                    SearchMode::Prefix => v.starts_with(q).then_some(0),
                    SearchMode::Includes => v.find(q),
                };
                if let Some(index) = hit {
                    out.push(Match { word: wi, index, loose });
                    if out.len() >= opt.limit {
                        return;
                    }
                }
            }
        };
        pass(&strict_q, false, &mut out);
        if out.is_empty() && opt.loose {
            let lq = loose_key(query);
            if !lq.is_empty() {
                pass(&lq, true, &mut out);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tools() {
        assert_eq!(strip_marks("ذَٰلِكَ"), "ذلك");
        assert_eq!(strip_marks("رَيْبَۛ"), "ريب");
        assert_eq!(fold("ٱلْكِتَٰبُ"), "الْكِتَٰبُ");
        assert_eq!(normalize_query("  ٱلرَّحْمَٰنِ  "), "الرحمن");
        assert_eq!(loose_key("الرحمان"), "لرحمن");
        assert_eq!(loose_key("الرحمن"), "لرحمن");
    }
}
