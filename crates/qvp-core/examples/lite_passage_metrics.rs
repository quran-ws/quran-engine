//! Generate the ink measurements used to check the optional JavaScript passage layout.
use qvp_core::Page;
use std::{env, fs, path::Path};

fn main() {
    let directory = env::args().nth(1).expect("page data directory");
    let mut records = Vec::new();
    for number in [1, 42, 272, 604] {
        let bytes = fs::read(Path::new(&directory).join(format!("{number:03}.qvp"))).unwrap();
        let page = Page::load(&bytes).unwrap();
        let mut pairs = Vec::new();
        for a in 0..page.data().words.len().saturating_sub(1) as u32 {
            let b = a + 1;
            if a % 20 != 0 && page.words_interlock(a, b).is_none() {
                continue;
            }
            if let Some(air) = page.words_clearance(a, b, 0.0) {
                pairs.push(format!("{{\"a\":{a},\"b\":{b},\"air\":{air:.4}}}"));
            }
        }
        records.push(format!(
            "{{\"page\":{number},\"lineSpacing\":{:.4},\"pairs\":[{}]}}",
            page.line_spacing(),
            pairs.join(",")
        ));
    }
    println!("[\n{}\n]", records.join(",\n"));
}
