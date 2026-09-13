//! Identity gate: SVG → QVP → SVG, rasterise both with resvg at 4×, pixel-diff.
//!
//! Run a subset:  QVP_TEST_PAGES=001,002,036 cargo test -p qvp-convert --release --test identity
//! Run all 604:   QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test identity
//! Diff images for failing pages are written to target/identity/.

use rayon::prelude::*;
use std::path::{Path, PathBuf};

const SCALE: f32 = 4.0;
/// A pixel counts as different when its coverage differs by more than this (0..255).
const PIX_TOL: i32 = 48;
/// Fraction of differing pixels allowed per page. Quantisation at 0.01 unit moves
/// edges by at most 0.02 px at 4×, which shows up only as sub-tolerance AA noise.
const MAX_DIFF_FRACTION: f64 = 0.0005;

fn pages_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../pages")
}

fn render(svg: &[u8]) -> tiny_skia::Pixmap {
    let tree = usvg::Tree::from_data(svg, &usvg::Options::default()).expect("usvg parse");
    let s = tree.size();
    let (w, h) = ((s.width() * SCALE).round() as u32, (s.height() * SCALE).round() as u32);
    let mut pm = tiny_skia::Pixmap::new(w, h).unwrap();
    resvg::render(&tree, tiny_skia::Transform::from_scale(SCALE, SCALE), &mut pm.as_mut());
    pm
}

struct Outcome {
    page: String,
    n_diff: usize,
    max_diff: i32,
    total: usize,
    svg_bytes: usize,
    qvp_bytes: usize,
}

fn check(page: &str) -> Outcome {
    let src = std::fs::read(pages_dir().join(format!("{page}.svg"))).expect("read svg");
    let conv = qvp_convert::convert(std::str::from_utf8(&src).unwrap()).expect("convert");
    let bytes = qvp_format::encode(&conv.page);
    let back = qvp_format::decode(&bytes).expect("decode");
    assert_eq!(back, conv.page, "encode/decode not identical for page {page}");
    let out_svg = qvp_convert::to_svg(&back).expect("to_svg");

    let a = render(&src);
    let b = render(out_svg.as_bytes());
    assert_eq!((a.width(), a.height()), (b.width(), b.height()), "size mismatch on page {page}");
    let (pa, pb) = (a.pixels(), b.pixels());
    let mut n_diff = 0usize;
    let mut max_diff = 0i32;
    let mut diff_img = tiny_skia::Pixmap::new(a.width(), a.height()).unwrap();
    let dp = diff_img.pixels_mut();
    for i in 0..pa.len() {
        let d = (pa[i].alpha() as i32 - pb[i].alpha() as i32).abs();
        max_diff = max_diff.max(d);
        if d > PIX_TOL {
            n_diff += 1;
            dp[i] = tiny_skia::PremultipliedColorU8::from_rgba(255, 0, 0, 255).unwrap();
        } else if pa[i].alpha() > 0 {
            dp[i] = tiny_skia::PremultipliedColorU8::from_rgba(200, 200, 200, 255).unwrap();
        }
    }
    let total = pa.len();
    if n_diff as f64 / total as f64 > MAX_DIFF_FRACTION {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/identity");
        std::fs::create_dir_all(&dir).ok();
        diff_img.save_png(dir.join(format!("{page}-diff.png"))).ok();
        a.save_png(dir.join(format!("{page}-src.png"))).ok();
        b.save_png(dir.join(format!("{page}-qvp.png"))).ok();
    }
    Outcome { page: page.to_owned(), n_diff, max_diff, total, svg_bytes: src.len(), qvp_bytes: bytes.len() }
}

fn selected_pages() -> Vec<String> {
    if std::env::var("QVP_TEST_ALL").is_ok() {
        let mut v: Vec<String> = std::fs::read_dir(pages_dir())
            .unwrap()
            .filter_map(|e| e.ok())
            .filter_map(|e| {
                let p = e.path();
                (p.extension()? == "svg").then(|| p.file_stem().unwrap().to_string_lossy().to_string())
            })
            .collect();
        v.sort();
        v
    } else if let Ok(s) = std::env::var("QVP_TEST_PAGES") {
        s.split(',').map(|x| x.trim().to_owned()).filter(|x| !x.is_empty()).collect()
    } else {
        ["001", "002", "017", "036", "144", "302", "485", "604"].iter().map(|s| s.to_string()).collect()
    }
}

#[test]
fn svg_qvp_svg_pixel_identity() {
    if !pages_dir().join("001.svg").exists() {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!(
                "pages/ (the source SVG bundle) is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh"
            );
        }
        eprintln!("skip: pages/ is missing; run scripts/sync-test-data.sh to enable the identity gate");
        return;
    }
    let pages = selected_pages();
    let results: Vec<Outcome> = pages.par_iter().map(|p| check(p)).collect();
    let mut failed = Vec::new();
    println!("{:>5} {:>10} {:>8} {:>9} {:>9} {:>7}", "page", "diff px", "max Δ", "svg B", "qvp B", "ratio");
    for r in &results {
        let frac = r.n_diff as f64 / r.total as f64;
        println!(
            "{:>5} {:>10} {:>8} {:>9} {:>9} {:>6.1}x{}",
            r.page,
            r.n_diff,
            r.max_diff,
            r.svg_bytes,
            r.qvp_bytes,
            r.svg_bytes as f64 / r.qvp_bytes as f64,
            if frac > MAX_DIFF_FRACTION { "  FAIL" } else { "" }
        );
        if frac > MAX_DIFF_FRACTION {
            failed.push(r.page.clone());
        }
    }
    assert!(failed.is_empty(), "pixel identity failed for pages {failed:?}; see target/identity/");
}
