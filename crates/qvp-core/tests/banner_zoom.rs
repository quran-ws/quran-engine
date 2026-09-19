//! How big a banner gets as the reader zooms in ([`ReflowSpec::banner_zoom`]).
//!
//! A surah name is drawn at `header_scale · layout scale`, and the layout scale carries the
//! zoom — so a banner left alone grows with the words around it until it fills the row, which
//! for a surah name is about five times the print. A host that draws its own frame around the
//! name needs it to stop somewhere.
//!
//! Needs dist/pages from scripts/sync-test-data.sh. Skipped when the data is missing unless
//! QVP_REQUIRE_DATA is set.
use qvp_core::qvp_format::DecoKind;
use qvp_core::{LayoutSpec, Page, ReflowSpec};
use std::path::{Path, PathBuf};

fn page_path(n: u32) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../dist/pages/{n:03}.qvp"))
}

/// 582 opens سورة النبأ, so it carries a surah name to measure.
fn page() -> Option<Page> {
    if !page_path(582).exists() {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!("dist/pages is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
        }
        eprintln!("skip: dist/pages is missing; run scripts/sync-test-data.sh to enable this test");
        return None;
    }
    Some(Page::load(&std::fs::read(page_path(582)).unwrap()).unwrap())
}

/// What a banner is drawn at, against its printed size: the scale the reflow placed it under,
/// times the zoom the whole layout is at. `kind` picks which banner — a surah name is about a
/// fifth of the text block wide and a basmalah about half, so the row stops the basmalah
/// growing long before it stops the name.
fn banner_against_print(p: &mut Page, kind: DecoKind, zoom: f32, banner_zoom: f32) -> f32 {
    let spec = LayoutSpec {
        viewport_w: 390.0,
        viewport_h: 844.0,
        reflow: Some(ReflowSpec { zoom, ..Default::default() }),
        banner_zoom,
        ..Default::default()
    };
    let n_words = p.data().words.len();
    let di = p.data().decorations.iter().position(|d| d.kind == kind).expect("page 582 opens a surah");
    let k = p.layout(&spec).placements()[n_words + di].kx;
    k * zoom
}

#[test]
fn uncapped_a_banner_grows_with_the_reader() {
    let Some(mut p) = page() else { return };
    // Measured from just above 1: a zoom of exactly 1 asks for the printed rows, and the
    // engine answers with the printed layout, which has no per-decoration placement to read.
    let barely = banner_against_print(&mut p, DecoKind::SurahName, 1.2, 0.0);
    assert!((barely - 1.2).abs() < 1e-3, "the name grows with the page, got {barely}");
    let zoomed = banner_against_print(&mut p, DecoKind::SurahName, 2.5, 0.0);
    assert!(zoomed > 2.0, "uncapped, the name keeps growing with the words around it, got {zoomed}");
}

#[test]
fn a_cap_holds_the_banner_where_the_host_asked() {
    let Some(mut p) = page() else { return };
    // 1.0: the name stays exactly as printed, however far the reader zooms in.
    for zoom in [1.5_f32, 2.5, 4.0] {
        let got = banner_against_print(&mut p, DecoKind::SurahName, zoom, 1.0);
        assert!((got - 1.0).abs() < 1e-3, "at zoom {zoom} the name should stay printed, got {got}");
    }
    // 1.5: it grows to half again its printed size and stops.
    assert!((banner_against_print(&mut p, DecoKind::SurahName, 4.0, 1.5) - 1.5).abs() < 1e-3);
    // A cap the zoom has not reached yet does not shrink anything.
    assert!((banner_against_print(&mut p, DecoKind::SurahName, 1.2, 3.0) - 1.2).abs() < 1e-3);
}

#[test]
fn a_cap_never_widens_a_banner_past_its_row() {
    let Some(mut p) = page() else { return };
    // The row is the other bound and it still holds. Measured on the BASMALAH, which is about
    // half the block wide: its row stops it around twice the print, well before a surah name —
    // a fifth of the block — would reach a row's edge at any zoom the page allows. Asking for
    // ten times the print cannot widen it past the row it has to sit on.
    let uncapped = banner_against_print(&mut p, DecoKind::Basmalah, 4.0, 0.0);
    assert!(uncapped < 4.0 - 1e-3, "the row, not the zoom, is what answers here — got {uncapped}");
    let wide = banner_against_print(&mut p, DecoKind::Basmalah, 4.0, 10.0);
    assert!((wide - uncapped).abs() < 1e-3, "the row bound is what answers, got {wide} vs {uncapped}");
}
