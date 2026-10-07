//! Passages on the real page data: every ayah of every page, at three widths and cut to one row.
//!
//! Every word is shown, every medallion moves with the word that closes its ayah, and every box
//! lies inside its layout. A range across two pages forms one passage.
//!
//! Needs dist/pages from scripts/sync-test-data.sh. Skipped when the data is missing unless
//! QVP_REQUIRE_DATA is set.
use qvp_core::qvp_format::DecoKind;
use qvp_core::{Page, Passage, PassageLayout, PassageSpec};
use std::path::{Path, PathBuf};

fn page_path(n: u32) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../dist/pages/{n:03}.qvp"))
}

fn load(n: u32) -> Option<Page> {
    if !page_path(n).exists() {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!("dist/pages is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
        }
        eprintln!("skip: dist/pages is missing; run scripts/sync-test-data.sh to enable this test");
        return None;
    }
    Some(Page::load(&std::fs::read(page_path(n)).unwrap()).unwrap())
}

fn bounded(l: &PassageLayout, at: &str) {
    let inside = |[x0, y0, x1, y1]: [f32; 4]| {
        [x0, y0, x1, y1].iter().all(|v| v.is_finite())
            && x0 >= -0.001
            && y0 >= -0.001
            && x1 <= l.width + 0.001
            && y1 <= l.height + 0.001
    };
    for w in &l.words {
        assert!(inside([w.x0, w.y0, w.x1, w.y1]), "{at}: word {}:{} outside {}×{}", w.ayah, w.word, l.width, l.height);
    }
    for r in &l.rows {
        assert!(inside([r.x0, r.y0, r.x1, r.y1]), "{at}: row outside {}×{}", l.width, l.height);
    }
    if let Some(e) = l.ellipsis {
        assert!(inside(e), "{at}: ellipsis outside {}×{}", l.width, l.height);
    }
}

#[test]
fn every_ayah_of_every_page_lays_out_whole() {
    let Some(_) = load(1) else { return };
    for n in 1..=604 {
        let page = load(n).unwrap();
        let data = page.data();
        let mut surahs: Vec<u16> = data.words.iter().map(|w| w.surah).collect();
        surahs.dedup();
        for surah in surahs {
            let ayahs: Vec<u16> = data.ayahs.iter().filter(|a| a.surah == surah).map(|a| a.ayah).collect();
            let (from, to) = (ayahs[0], ayahs[ayahs.len() - 1]);
            let mut passage = Passage::load(&[&page], surah, from, to).unwrap_or_else(|e| panic!("{n} {surah}: {e}"));
            let selected = data.words.iter().filter(|w| w.surah == surah).count();
            for width in [200.0, 320.0, 480.0] {
                let at = format!("page {n}, {surah}:{from}-{to} at {width}");
                let l = passage.layout(&PassageSpec { width, scale: 0.9, ..Default::default() }).unwrap().clone();
                bounded(&l, &at);
                assert_eq!(l.words.len(), selected, "{at}");
                assert!(!l.is_truncated, "{at}");
                // No ayah loses its number: its medallion is drawn where the last word is.
                for ayah in from..=to {
                    let Some(deco) = data
                        .decorations
                        .iter()
                        .find(|d| d.kind == DecoKind::AyahMark && d.surah == surah && d.ayah == ayah)
                    else {
                        panic!("{at}: no medallion for {surah}:{ayah}");
                    };
                    let last = data.words.iter().rposition(|w| w.surah == surah && w.ayah == ayah).unwrap();
                    let placement = |path: u32| l.draws.iter().find(|d| d.path == path).map(|d| d.placement);
                    let medallion =
                        placement(deco.first_path).unwrap_or_else(|| panic!("{at}: {surah}:{ayah} undrawn"));
                    assert_eq!(Some(medallion), placement(data.words[last].first_path), "{at}: {surah}:{ayah}");
                }
            }
            // Cut to one row, the last medallion still closes it.
            let cut = PassageSpec {
                width: 200.0,
                scale: 0.9,
                max_rows: 1,
                keep_ayah_mark: true,
                ellipsis_width: 9.0,
                ..Default::default()
            };
            let l = passage.layout(&cut).unwrap().clone();
            bounded(&l, &format!("page {n}, {surah}:{from}-{to} cut"));
            assert_eq!(l.rows.len(), 1);
            if l.is_truncated {
                let e = l.ellipsis.unwrap();
                let last = l.words.last().unwrap();
                assert!(e[2] <= last.x0 + 0.001, "page {n}: the ellipsis follows the last word");
            }
        }
    }
}

#[test]
fn a_range_across_two_pages_is_one_passage() {
    let (Some(two), Some(three)) = (load(2), load(3)) else { return };
    let mut passage = Passage::load(&[&three, &two], 2, 5, 6).unwrap();
    let l = passage.layout(&PassageSpec { width: 366.0, scale: 0.9, ..Default::default() }).unwrap().clone();
    assert_eq!(l.words.len(), 19);
    assert_eq!(l.ayahs.len(), 2);
    assert!(l.words.iter().any(|w| w.page == 2) && l.words.iter().any(|w| w.page == 3));
    bounded(&l, "2:5-6");
}

#[test]
fn the_longest_ayah_fits_any_width() {
    let Some(page) = load(48) else { return };
    let mut passage = Passage::load(&[&page], 2, 282, 282).unwrap();
    assert!(passage.word_count() > 100);
    for width in [120.0, 375.0, 1024.0] {
        let l = passage.layout(&PassageSpec { width, scale: 1.0, ..Default::default() }).unwrap().clone();
        assert_eq!(l.words.len(), passage.word_count());
        bounded(&l, &format!("2:282 at {width}"));
    }
    // One row: as wide as the ayah.
    let l = passage.layout(&PassageSpec::default()).unwrap().clone();
    assert_eq!(l.rows.len(), 1);
    bounded(&l, "2:282 on one row");
}
