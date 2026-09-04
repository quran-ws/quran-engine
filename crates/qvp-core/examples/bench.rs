//! Real-page numbers: cargo run -p qvp-core --release --example bench -- dist/pages/036.qvp
use qvp_core::*;
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "dist/pages/036.qvp".into());
    let bytes = std::fs::read(&path).expect("read qvp");
    let t = Instant::now();
    let mut page = Page::load(&bytes).expect("load");
    let load = t.elapsed();
    let (first_sura, first_ayah) = (page.data().ayahs[0].sura, page.data().ayahs[0].ayah);
    let d = page.data();
    println!("{path}: {} bytes, {} words, {} paths, {} pts; load {:?}", bytes.len(), d.words.len(), d.paths.len(), page.geometry().pts.len() / 2, load);

    // hit-test every word's bbox centre plus a grid of misses
    let q = page.quant();
    let centres: Vec<(f32, f32)> = d.words.iter().map(|w| ((w.bbox.x0 + w.bbox.x1) as f32 / 2.0 / q, (w.bbox.y0 + w.bbox.y1) as f32 / 2.0 / q)).collect();
    let n_words = d.words.len();
    let t = Instant::now();
    let mut hits = 0;
    let mut exact = 0;
    for _ in 0..100 {
        for &(x, y) in &centres {
            if let Some(h) = page.hit_test(x, y) {
                hits += 1;
                if h.path != NONE {
                    exact += 1;
                }
            }
        }
    }
    let per = t.elapsed() / (100 * centres.len() as u32);
    println!("hit-test: {:?} per query; {}/{} word centres hit ({} exact outline hits)", per, hits / 100, n_words, exact / 100);

    let t = Instant::now();
    for _ in 0..100 {
        let _ = page.paint();
    }
    println!("paint (no styles): {:?} per full display list", t.elapsed() / 100);

    page.style.set(Selector::Family(qvp_format::Family::Diacritic), 0x1a73e8ff);
    page.style.set(Selector::Ayah(first_sura, first_ayah), 0x0a7d32ff);
    let t = Instant::now();
    for _ in 0..100 {
        let _ = page.paint();
    }
    println!("paint (2 selectors): {:?} per full display list; styled paths = {}", t.elapsed() / 100, page.styled().len());
    let t = Instant::now();
    for _ in 0..100 {
        let _ = page.styled();
    }
    println!("styled() overlay list: {:?}", t.elapsed() / 100);
}
