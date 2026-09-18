pub mod affine;
pub mod atlas;
pub mod qvp2svg;
pub mod svg2qvp;
pub mod zoom_levels;

pub use qvp2svg::to_svg;
pub use svg2qvp::{convert, Converted, WordText};

use std::path::{Path, PathBuf};

/// Where the quran-svg bundle keeps one JSON record per page: `index/by-page/NNN.json`.
/// Production pages carry only `data-rasm-uthmani`, so the four derived forms
/// (`rasm_imlai`, `qpc`, `rasm`, `search`) are read from there — see the bundle's
/// `schema/FORMAT.md` §6.1.
pub fn default_words_index(svg_dir: &Path) -> Option<PathBuf> {
    let d = svg_dir.parent()?.join("index").join("by-page");
    d.is_dir().then_some(d)
}

/// Merge one page's `index/by-page/NNN.json` into the words already read from the SVG.
/// Returns the number of words filled in, and pushes a warning for every word the
/// index and the page disagree about — the data is never repaired here.
pub fn merge_words_index(words: &mut [WordText], json: &[u8], warnings: &mut Vec<String>) -> usize {
    let Some(forms) = qvp_core::text::parse_words_sidecar(json) else {
        warnings.push("words index: not valid JSON, ignored".into());
        return 0;
    };
    let index: std::collections::HashMap<&str, &qvp_core::text::WordForms> =
        forms.iter().map(|(k, f)| (k.as_str(), f)).collect();
    let mut n = 0;
    for w in words.iter_mut() {
        let Some(f) = index.get(w.word_key.as_str()) else {
            warnings.push(format!("word {}: not in the words index", w.word_key));
            continue;
        };
        if let Some(u) = &f.rasm_uthmani {
            if !w.rasm_uthmani.is_empty() && u != &w.rasm_uthmani {
                warnings.push(format!("word {}: rasm_uthmani differs between page and words index", w.word_key));
            } else if w.rasm_uthmani.is_empty() {
                w.rasm_uthmani = u.clone();
            }
        }
        let fill = |slot: &mut String, v: &Option<String>| {
            if slot.is_empty() {
                if let Some(v) = v {
                    *slot = v.clone();
                }
            }
        };
        fill(&mut w.rasm_imlai, &f.rasm_imlai);
        fill(&mut w.qpc, &f.qpc);
        fill(&mut w.rasm, &f.rasm);
        fill(&mut w.search, &f.search);
        n += 1;
    }
    n
}

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
        s.push_str(&json_str(&w.word_key));
        s.push_str(":{\"rasm_uthmani\":");
        s.push_str(&json_str(&w.rasm_uthmani));
        s.push_str(",\"rasm_imlai\":");
        s.push_str(&json_str(&w.rasm_imlai));
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
