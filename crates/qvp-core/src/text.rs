//! Arabic text tools, text extraction and search.
use crate::{Page, NONE};
use qvp_format::NONE_U16;

/// Text forms carried per word.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Form {
    RasmUthmani = 0,
    RasmImlai = 1,
    Qpc = 2,
    Rasm = 3,
    Search = 4,
}

impl Form {
    pub fn from_u8(v: u8) -> Form {
        match v {
            1 => Form::RasmImlai,
            2 => Form::Qpc,
            3 => Form::Rasm,
            4 => Form::Search,
            _ => Form::RasmUthmani,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Form::RasmUthmani => "rasm_uthmani",
            Form::RasmImlai => "rasm_imlai",
            Form::Qpc => "qpc",
            Form::Rasm => "rasm",
            Form::Search => "search",
        }
    }
}

/// Harakah, tanwin (incl. the open forms U+08F0–08F2), omitted alif, waqf/dabt
/// block, tatweel, the `rubu_al_hizb` sign, small letters and Quranic annotation signs.
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

/// Additionally drops bare alif and hamzah so a typed الرحمان finds the printed الرحمن.
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
            Form::RasmUthmani => w.text,
            Form::RasmImlai => w.rasm_imlai,
            Form::Qpc => w.qpc,
            Form::Rasm => w.rasm,
            Form::Search => w.search,
        };
        if r == NONE_U16 {
            // fall back to rasm_uthmani, then search form
            if form != Form::RasmUthmani && w.text != NONE_U16 {
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

// ───────────── sidecar text forms ─────────────

/// Minimal JSON reader for the words sidecar:
/// `{"2:255:3": {"rasm_imlai": "...", "qpc": "...", "rasm": "...", "search": "..."}, ...}`
/// (also accepts `{"words": [{"word_key": "...", ...}, ...]}`). Unknown keys are ignored.
struct Json<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> Json<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\n' | b'\r' | b'\t') {
            self.i += 1;
        }
    }
    fn eat(&mut self, c: u8) -> bool {
        self.ws();
        if self.i < self.b.len() && self.b[self.i] == c {
            self.i += 1;
            true
        } else {
            false
        }
    }
    fn string(&mut self) -> Option<String> {
        self.ws();
        if !self.eat(b'"') {
            return None;
        }
        let mut out = String::new();
        while self.i < self.b.len() {
            let c = self.b[self.i];
            self.i += 1;
            match c {
                b'"' => return Some(out),
                b'\\' => {
                    let e = *self.b.get(self.i)?;
                    self.i += 1;
                    match e {
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'u' => {
                            let h = std::str::from_utf8(self.b.get(self.i..self.i + 4)?).ok()?;
                            self.i += 4;
                            let mut cp = u32::from_str_radix(h, 16).ok()?;
                            if (0xD800..0xDC00).contains(&cp) && self.b.get(self.i..self.i + 2) == Some(b"\\u") {
                                let h2 = std::str::from_utf8(self.b.get(self.i + 2..self.i + 6)?).ok()?;
                                let lo = u32::from_str_radix(h2, 16).ok()?;
                                self.i += 6;
                                cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                            }
                            out.push(char::from_u32(cp)?);
                        }
                        other => out.push(other as char),
                    }
                }
                _ => {
                    // copy raw UTF-8 bytes
                    let start = self.i - 1;
                    let mut end = self.i;
                    while end < self.b.len() && self.b[end] != b'"' && self.b[end] != b'\\' {
                        end += 1;
                    }
                    out.push_str(std::str::from_utf8(&self.b[start..end]).ok()?);
                    self.i = end;
                }
            }
        }
        None
    }
    /// skip any value
    fn skip(&mut self) -> Option<()> {
        self.ws();
        match *self.b.get(self.i)? {
            b'"' => {
                self.string()?;
            }
            b'{' => {
                self.i += 1;
                loop {
                    self.ws();
                    if self.eat(b'}') {
                        break;
                    }
                    self.string()?;
                    self.eat(b':');
                    self.skip()?;
                    self.eat(b',');
                }
            }
            b'[' => {
                self.i += 1;
                loop {
                    self.ws();
                    if self.eat(b']') {
                        break;
                    }
                    self.skip()?;
                    self.eat(b',');
                }
            }
            _ => {
                while self.i < self.b.len() && !matches!(self.b[self.i], b',' | b'}' | b']') {
                    self.i += 1;
                }
            }
        }
        Some(())
    }
    /// read an object of string values into (key, value) pairs, ignoring non-strings
    fn string_object(&mut self) -> Option<Vec<(String, String)>> {
        if !self.eat(b'{') {
            return None;
        }
        let mut out = Vec::new();
        loop {
            self.ws();
            if self.eat(b'}') {
                return Some(out);
            }
            let k = self.string()?;
            self.eat(b':');
            self.ws();
            if self.b.get(self.i) == Some(&b'"') {
                out.push((k, self.string()?));
            } else {
                self.skip()?;
            }
            self.eat(b',');
        }
    }
}

/// One word's derived forms from the sidecar.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct WordForms {
    pub rasm_uthmani: Option<String>,
    pub rasm_imlai: Option<String>,
    pub qpc: Option<String>,
    pub rasm: Option<String>,
    pub search: Option<String>,
}

impl Page {
    /// Attach text forms (rasm_uthmani / rasm_imlai / qpc / rasm / search) for words of this page.
    /// Keys are word ids "s:a:w". Returns the number of words updated. Inline forms
    /// already in the file are kept unless the sidecar provides a value.
    pub fn attach_forms(&mut self, forms: &[(String, WordForms)]) -> usize {
        let mut n = 0;
        for (word_key, f) in forms {
            let mut it = word_key.split(':').map(|x| x.parse::<u16>().unwrap_or(0));
            let (s, a, w) = (it.next().unwrap_or(0), it.next().unwrap_or(0), it.next().unwrap_or(0));
            let Some(wi) = self.find_word(s, a, w) else { continue };
            let set = |v: &Option<String>, slot: fn(&mut qvp_format::WordRec) -> &mut u16, page: &mut Page| {
                if let Some(v) = v {
                    let idx = page.intern(v);
                    *slot(&mut page.data_mut().words[wi as usize]) = idx;
                }
            };
            set(&f.rasm_uthmani, |w| &mut w.text, self);
            set(&f.rasm_imlai, |w| &mut w.rasm_imlai, self);
            set(&f.qpc, |w| &mut w.qpc, self);
            set(&f.rasm, |w| &mut w.rasm, self);
            set(&f.search, |w| &mut w.search, self);
            n += 1;
        }
        n
    }

    /// Attach a JSON sidecar (see [`parse_words_sidecar`]). Returns words updated, or None on a parse error.
    pub fn attach_words_json(&mut self, json: &[u8]) -> Option<usize> {
        let forms = parse_words_sidecar(json)?;
        Some(self.attach_forms(&forms))
    }

    pub fn has_form(&self, form: Form) -> bool {
        match form {
            Form::RasmUthmani => true,
            Form::RasmImlai => self.data().words.iter().any(|w| w.rasm_imlai != NONE_U16),
            Form::Qpc => self.data().words.iter().any(|w| w.qpc != NONE_U16),
            Form::Rasm => self.data().words.iter().any(|w| w.rasm != NONE_U16),
            Form::Search => self.data().words.iter().any(|w| w.search != NONE_U16),
        }
    }
}

/// Parse `{"s:a:w": {form: text, ...}, ...}` or `{"words": [{"word_key": "...", form: text}, ...]}`.
pub fn parse_words_sidecar(json: &[u8]) -> Option<Vec<(String, WordForms)>> {
    let mut j = Json { b: json, i: 0 };
    let mut out = Vec::new();
    if !j.eat(b'{') {
        return None;
    }
    let to_forms = |kv: Vec<(String, String)>| {
        let mut f = WordForms::default();
        let mut word_key = None;
        for (k, v) in kv {
            match k.as_str() {
                "rasm_uthmani" | "uthmani" => f.rasm_uthmani = Some(v),
                "rasm_imlai" | "imlaei" => f.rasm_imlai = Some(v),
                "qpc" => f.qpc = Some(v),
                "rasm" => f.rasm = Some(v),
                "search" => f.search = Some(v),
                "word_key" => word_key = Some(v),
                _ => {}
            }
        }
        (word_key, f)
    };
    loop {
        j.ws();
        if j.eat(b'}') {
            break;
        }
        let key = j.string()?;
        j.eat(b':');
        j.ws();
        if key == "words" && j.b.get(j.i) == Some(&b'[') {
            j.i += 1;
            loop {
                j.ws();
                if j.eat(b']') {
                    break;
                }
                let (word_key, f) = to_forms(j.string_object()?);
                if let Some(w) = word_key {
                    out.push((w, f));
                }
                j.eat(b',');
            }
        } else if j.b.get(j.i) == Some(&b'{') {
            let (_, f) = to_forms(j.string_object()?);
            out.push((key, f));
        } else {
            j.skip()?;
        }
        j.eat(b',');
    }
    Some(out)
}

#[cfg(test)]
mod sidecar_tests {
    use super::*;
    #[test]
    fn parses_both_shapes() {
        let a = r#"{"2:255:3": {"rasm_imlai": "الْحَيُّ", "search": "الحي", "x": 1}, "2:255:4": {"qpc": "ٱلۡقَيُّومُ"}}"#;
        let v = parse_words_sidecar(a.as_bytes()).unwrap();
        assert_eq!(v.len(), 2);
        assert_eq!(v[0].0, "2:255:3");
        assert_eq!(v[0].1.search.as_deref(), Some("الحي"));
        let b = r#"{"page": 42, "words": [{"word_key": "2:253:1", "rasm": "تلك", "search": "تلك"}]}"#;
        let v = parse_words_sidecar(b.as_bytes()).unwrap();
        assert_eq!(v[0].1.search.as_deref(), Some("تلك"));
    }
}
