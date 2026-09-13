//! Real-page numbers: cargo run -p qvp-core --release --example bench -- dist/pages/036.qvp
use qvp_core::*;
use std::time::Instant;

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "dist/pages/036.qvp".into());
    let bytes = std::fs::read(&path).expect("read qvp");
    let t = Instant::now();
    let mut page = Page::load(&bytes).expect("load");
    let load = t.elapsed();
    let (first_surah, first_ayah) = (page.data().ayahs[0].surah, page.data().ayahs[0].ayah);
    let d = page.data();
    println!(
        "{path}: {} bytes, {} words, {} paths, {} pts; load {:?}",
        bytes.len(),
        d.words.len(),
        d.paths.len(),
        page.geometry().pts.len() / 2,
        load
    );

    // hit-test every word's bbox centre plus a grid of misses
    let q = page.quant();
    let centres: Vec<(f32, f32)> = d
        .words
        .iter()
        .map(|w| ((w.bbox.x0 + w.bbox.x1) as f32 / 2.0 / q, (w.bbox.y0 + w.bbox.y1) as f32 / 2.0 / q))
        .collect();
    let n_words = d.words.len();
    let t = Instant::now();
    let mut hits = 0;
    let mut exact = 0;
    for _ in 0..100 {
        for &(x, y) in &centres {
            if let Some(h) = page.hit_test_exact(x, y) {
                hits += 1;
                if h.path != NONE {
                    exact += 1;
                }
            }
        }
    }
    let per = t.elapsed() / (100 * centres.len() as u32);
    println!(
        "hit-test: {:?} per query; {}/{} word centres hit ({} exact outline hits)",
        per,
        hits / 100,
        n_words,
        exact / 100
    );

    let t = Instant::now();
    for _ in 0..100 {
        let _ = page.paint();
    }
    println!("paint (no styles): {:?} per full display list", t.elapsed() / 100);

    page.style(Selector::Family(qvp_format::Family::Diacritic), Paint::new(0x1a73e8ff));
    page.style(Selector::Ayah(first_surah, first_ayah), Paint::new(0x0a7d32ff));
    let t = Instant::now();
    for _ in 0..100 {
        let _ = page.paint();
    }
    println!(
        "paint (2 selectors): {:?} per full display list; styled paths = {}",
        t.elapsed() / 100,
        page.styled().len()
    );
    let t = Instant::now();
    for _ in 0..100 {
        let _ = page.styled();
    }
    println!("styled() overlay list: {:?}", t.elapsed() / 100);
    let t = Instant::now();
    let m = page.search("الله", &SearchOptions::default());
    println!("search 'الله': {} matches in {:?}", m.len(), t.elapsed());
    let t = Instant::now();
    let hb = page.hit_areas(0.6);
    println!("hit boxes: {} in {:?}", hb.len(), t.elapsed());
    let t = Instant::now();
    for _ in 0..100 {
        let _ = page.hit_test(100.0, 300.0, &HitOptions::default());
    }
    println!("gap-aware hit-test: {:?}", t.elapsed() / 100);
    let surah = page.data().words[0].surah;
    let ayah = page.data().words[0].ayah;
    let t = Instant::now();
    let h = page.highlight(
        &Target::Ayah(surah, ayah),
        HighlightStyle { mode: HighlightMode::Both, transition_ms: 200, ..Default::default() },
    );
    page.tick(100.0);
    let b = page.highlight_boxes_view();
    println!("highlight ayah {surah}:{ayah}: {} band boxes, tick+boxes {:?}", b.len(), t.elapsed());
    page.unhighlight(h);
    println!(
        "surahs: {:?}",
        page.surahs().iter().map(|s| (s.number, s.latin.clone(), s.has_banner)).collect::<Vec<_>>()
    );
    println!("divisions: {:?}", page.divisions().iter().map(|d| (d.kind, d.n, d.surah, d.ayah)).collect::<Vec<_>>());
    println!("ayah_marks: {}", page.ayah_marks().len());
}
