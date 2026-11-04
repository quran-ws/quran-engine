//! The words sidecar: the JSON file of derived text forms a page attaches to its words.
use crate::Page;

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
                                // A high surrogate needs a low one after it; anything else is
                                // not a character.
                                if !(0xDC00..0xE000).contains(&lo) {
                                    return None;
                                }
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
    /// Skip any value. `depth` counts the containers open above it: a sidecar nests two
    /// deep, and a run of brackets deeper than the limit would otherwise recurse until
    /// the stack ran out.
    fn skip(&mut self, depth: u32) -> Option<()> {
        const MAX_DEPTH: u32 = 64;
        if depth > MAX_DEPTH {
            return None;
        }
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
                    self.skip(depth + 1)?;
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
                    self.skip(depth + 1)?;
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
                self.skip(2)?;
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
                "rasm_uthmani" | "uthmani" => f.rasm_uthmani = Some(v), // terminology: ignore, pre-1.0 sidecar key
                "rasm_imlai" | "imlaei" => f.rasm_imlai = Some(v),      // terminology: ignore, pre-1.0 sidecar key
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
            j.skip(1)?;
        }
        j.eat(b',');
    }
    Some(out)
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn a_broken_sidecar_is_none_not_a_panic() {
        // a high surrogate followed by something that is not a low one
        let lone = br#"{"2:255:3": {"search": "\uD83DA"}}"#;
        assert_eq!(parse_words_sidecar(lone), None);
        // brackets nested past any sidecar's depth
        let mut deep = b"{\"x\": ".to_vec();
        deep.extend(vec![b'['; 100_000]);
        assert_eq!(parse_words_sidecar(&deep), None);
        for n in 0..lone.len() {
            let _ = parse_words_sidecar(&lone[..n]);
        }
    }
}
