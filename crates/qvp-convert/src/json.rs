//! A small JSON reader, enough for the ornament catalogue. The engine crates
//! carry no dependencies and this is a build-time tool, so it stays hand-rolled
//! rather than pulling a parser into the workspace.
use std::collections::BTreeMap;

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(BTreeMap<String, Json>),
}

impl Json {
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(m) => m.get(key),
            _ => None,
        }
    }
    /// `a.b.c` in one step; any missing link is None.
    pub fn path(&self, dotted: &str) -> Option<&Json> {
        let mut cur = self;
        for k in dotted.split('.') {
            cur = cur.get(k)?;
        }
        Some(cur)
    }
    pub fn str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn num(&self) -> Option<f64> {
        match self {
            Json::Num(n) => Some(*n),
            _ => None,
        }
    }
    pub fn bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }
    pub fn arr(&self) -> &[Json] {
        match self {
            Json::Arr(v) => v,
            _ => &[],
        }
    }
    pub fn obj(&self) -> Option<&BTreeMap<String, Json>> {
        match self {
            Json::Obj(m) => Some(m),
            _ => None,
        }
    }
    /// `"a.b"` as a string, or "".
    pub fn s(&self, dotted: &str) -> &str {
        self.path(dotted).and_then(|v| v.str()).unwrap_or("")
    }
    pub fn f(&self, dotted: &str) -> Option<f64> {
        self.path(dotted).and_then(|v| v.num())
    }

    pub fn parse(b: &[u8]) -> Result<Json, String> {
        let mut p = P { b, i: 0 };
        let v = p.value()?;
        Ok(v)
    }
}

struct P<'a> {
    b: &'a [u8],
    i: usize,
}

impl<'a> P<'a> {
    fn ws(&mut self) {
        while self.i < self.b.len() && matches!(self.b[self.i], b' ' | b'\t' | b'\n' | b'\r') {
            self.i += 1;
        }
    }
    fn peek(&mut self) -> Result<u8, String> {
        self.ws();
        self.b.get(self.i).copied().ok_or_else(|| "unexpected end of JSON".to_string())
    }
    fn lit(&mut self, s: &str) -> bool {
        if self.b[self.i..].starts_with(s.as_bytes()) {
            self.i += s.len();
            true
        } else {
            false
        }
    }
    fn value(&mut self) -> Result<Json, String> {
        match self.peek()? {
            b'{' => {
                self.i += 1;
                let mut m = BTreeMap::new();
                if self.peek()? == b'}' {
                    self.i += 1;
                    return Ok(Json::Obj(m));
                }
                loop {
                    let k = self.string()?;
                    if self.peek()? != b':' {
                        return Err(format!("expected ':' after {k:?}"));
                    }
                    self.i += 1;
                    m.insert(k, self.value()?);
                    match self.peek()? {
                        b',' => self.i += 1,
                        b'}' => {
                            self.i += 1;
                            return Ok(Json::Obj(m));
                        }
                        c => return Err(format!("expected ',' or '}}', got {:?}", c as char)),
                    }
                }
            }
            b'[' => {
                self.i += 1;
                let mut v = Vec::new();
                if self.peek()? == b']' {
                    self.i += 1;
                    return Ok(Json::Arr(v));
                }
                loop {
                    v.push(self.value()?);
                    match self.peek()? {
                        b',' => self.i += 1,
                        b']' => {
                            self.i += 1;
                            return Ok(Json::Arr(v));
                        }
                        c => return Err(format!("expected ',' or ']', got {:?}", c as char)),
                    }
                }
            }
            b'"' => Ok(Json::Str(self.string()?)),
            b't' if self.lit("true") => Ok(Json::Bool(true)),
            b'f' if self.lit("false") => Ok(Json::Bool(false)),
            b'n' if self.lit("null") => Ok(Json::Null),
            _ => {
                let start = self.i;
                while self.i < self.b.len() && matches!(self.b[self.i], b'0'..=b'9' | b'-' | b'+' | b'.' | b'e' | b'E') {
                    self.i += 1;
                }
                std::str::from_utf8(&self.b[start..self.i])
                    .ok()
                    .and_then(|s| s.parse::<f64>().ok())
                    .map(Json::Num)
                    .ok_or_else(|| "bad JSON number".to_string())
            }
        }
    }
    fn string(&mut self) -> Result<String, String> {
        if self.peek()? != b'"' {
            return Err("expected a JSON string".into());
        }
        self.i += 1;
        let mut out = String::new();
        loop {
            let c = *self.b.get(self.i).ok_or("unterminated JSON string")?;
            self.i += 1;
            match c {
                b'"' => return Ok(out),
                b'\\' => {
                    let e = *self.b.get(self.i).ok_or("unterminated escape")?;
                    self.i += 1;
                    match e {
                        b'n' => out.push('\n'),
                        b't' => out.push('\t'),
                        b'r' => out.push('\r'),
                        b'b' => out.push('\u{8}'),
                        b'f' => out.push('\u{c}'),
                        b'u' => {
                            let h = std::str::from_utf8(self.b.get(self.i..self.i + 4).ok_or("short \\u")?).map_err(|_| "bad \\u")?;
                            self.i += 4;
                            let mut cp = u32::from_str_radix(h, 16).map_err(|_| "bad \\u")?;
                            if (0xD800..0xDC00).contains(&cp) && self.b.get(self.i..self.i + 2) == Some(b"\\u") {
                                let h2 = std::str::from_utf8(self.b.get(self.i + 2..self.i + 6).ok_or("short surrogate")?).map_err(|_| "bad \\u")?;
                                let lo = u32::from_str_radix(h2, 16).map_err(|_| "bad \\u")?;
                                self.i += 6;
                                cp = 0x10000 + ((cp - 0xD800) << 10) + (lo - 0xDC00);
                            }
                            out.push(char::from_u32(cp).ok_or("bad code point")?);
                        }
                        other => out.push(other as char),
                    }
                }
                _ => {
                    let start = self.i - 1;
                    let mut end = self.i;
                    while end < self.b.len() && self.b[end] != b'"' && self.b[end] != b'\\' {
                        end += 1;
                    }
                    out.push_str(std::str::from_utf8(&self.b[start..end]).map_err(|_| "bad utf8 in JSON string")?);
                    self.i = end;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reads_the_shapes_a_catalogue_uses() {
        let v = Json::parse(br#"{"assets":[{"type":"page-frames","slices":{"corner":{"w":9.4,"h":6.5}},"license":{"redistributable":false}}],"n":3}"#).unwrap();
        assert_eq!(v.f("n"), Some(3.0));
        let a = &v.get("assets").unwrap().arr()[0];
        assert_eq!(a.s("type"), "page-frames");
        assert_eq!(a.f("slices.corner.w"), Some(9.4));
        assert_eq!(a.path("license.redistributable").unwrap().bool(), Some(false));
        assert_eq!(a.f("slices.nope.w"), None);
    }
    #[test]
    fn reads_escapes() {
        assert_eq!(Json::parse(r#""aق\nb""#.as_bytes()).unwrap().str(), Some("aق\nb"));
    }
}
