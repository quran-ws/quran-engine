pub mod affine;
pub mod qvp2svg;
pub mod svg2qvp;

pub use qvp2svg::to_svg;
pub use svg2qvp::{convert, Converted, WordText};

/// Escape for JSON string values.
pub fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => {
                o.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

pub fn words_json(words: &[WordText]) -> String {
    let mut s = String::from("{");
    for (i, w) in words.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push_str(&json_str(&w.wid));
        s.push_str(":{\"uthmani\":");
        s.push_str(&json_str(&w.uthmani));
        s.push_str(",\"imlaei\":");
        s.push_str(&json_str(&w.imlaei));
        s.push_str(",\"qpc\":");
        s.push_str(&json_str(&w.qpc));
        s.push_str(",\"rasm\":");
        s.push_str(&json_str(&w.rasm));
        s.push_str(",\"search\":");
        s.push_str(&json_str(&w.search));
        s.push('}');
    }
    s.push('}');
    s
}
