//! Real-page numbers: cargo run -p qvp-core --profile dist --example bench -- dist/pages/036.qvp
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
        let _ = page.colors();
    }
    println!("paint (no styles): {:?} per full display list", t.elapsed() / 100);

    page.style(Selector::Family(qvp_format::Family::Diacritic), Paint::new(0x1a73e8ff));
    page.style(Selector::Ayah(first_surah, first_ayah), Paint::new(0x0a7d32ff));
    let t = Instant::now();
    for _ in 0..100 {
        let _ = page.colors();
    }
    println!(
        "paint (2 selectors): {:?} per full display list; styled paths = {}",
        t.elapsed() / 100,
        page.styled_paths().len()
    );
    let t = Instant::now();
    for _ in 0..100 {
        let _ = page.styled_paths();
    }
    println!("styled_paths() overlay list: {:?}", t.elapsed() / 100);
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
    page.remove_highlight(h);

    // The layouts a reader's gestures ask for: the printed page, a reflowed page, and a
    // continuous pinch that lays the page out again on every frame.
    let spec = LayoutSpec { viewport_w: 390.0, viewport_h: 844.0, ..Default::default() };
    let t = Instant::now();
    for _ in 0..100 {
        let _ = page.layout(&spec);
    }
    println!("printed layout: {:?} per layout", t.elapsed() / 100);
    let reflowed = LayoutSpec { reflow: Some(ReflowSpec { zoom: 1.6, ..Default::default() }), ..spec };
    let _ = page.layout(&reflowed);
    let t = Instant::now();
    for _ in 0..20 {
        let _ = page.layout(&reflowed);
    }
    println!("reflow layout (zoom 1.6): {:?} per layout", t.elapsed() / 20);
    let pinch = Zoom { mode: ZoomMode::Continuous, step: 0, zoom: 1.0 };
    let focal = (spec.viewport_w / 2.0, spec.viewport_h / 2.0);
    let _ = page.layout(&spec);
    let t = Instant::now();
    let mut relaid = 0;
    for frame in 1..=10 {
        let c = page.zoom_pinch(&spec, pinch, View::default(), 1.0 + 0.08 * frame as f32, focal);
        relaid += c.relaid as u32;
    }
    println!("continuous pinch, ten frames: {:?} per frame; {relaid} relaid", t.elapsed() / 10);
    let _ = page.layout(&spec);

    // The clock once everything has settled: what a host pays per frame for nothing.
    page.tick(10_000.0);
    let t = Instant::now();
    for i in 0..100_000 {
        let _ = page.tick(10_001.0 + i as f64);
    }
    println!("idle tick: {:?} per tick", t.elapsed() / 100_000);
    let t = Instant::now();
    for _ in 0..100_000 {
        let _ = page.colors();
    }
    println!("colors() after settling: {:?} per call", t.elapsed() / 100_000);
    println!(
        "surahs: {:?}",
        page.surahs().iter().map(|s| (s.number, s.latin.clone(), s.has_banner)).collect::<Vec<_>>()
    );
    println!("divisions: {:?}", page.divisions().iter().map(|d| (d.kind, d.n, d.surah, d.ayah)).collect::<Vec<_>>());
    println!("ayah_marks: {}", page.ayah_marks().len());
}
