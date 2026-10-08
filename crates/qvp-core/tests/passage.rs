//! Passages on the real page data: every ayah of every page, at three widths and cut to one row.
//!
//! Every word is shown, every medallion moves with the word that closes its ayah, and every box
//! lies inside its layout. A range across two pages forms one passage. A justified passage
//! reaches both edges and centres what it cannot fill, and a surah's basmalah is a passage of
//! its own.
//!
//! Needs dist/pages from scripts/sync-test-data.sh. Skipped when the data is missing unless
//! QVP_REQUIRE_DATA is set.
use qvp_core::qvp_format::{DecoKind, Mark};
use qvp_core::{Align, Page, Passage, PassageError, PassageLayout, PassageSpec};
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

/// The air between a row's ink and each side of the layout, in layout pixels: right, then left.
fn margins(l: &PassageLayout, row: usize, padding: f32) -> (f32, f32) {
    let r = &l.rows[row];
    (l.width - padding - r.x1, r.x0 - padding)
}

#[test]
fn a_justified_passage_reaches_both_edges_and_centres_its_last_row() {
    let Some(page) = load(48) else { return };
    let mut passage = Passage::load(&[&page], 2, 282, 282).unwrap();
    // No cap: every row but the last opens until it reaches both edges.
    let spec =
        PassageSpec { width: 375.0, scale: 1.0, align: Align::Justified, max_stretch: 0.0, ..Default::default() };
    let l = passage.layout(&spec).unwrap().clone();
    bounded(&l, "2:282 justified");
    assert!(l.rows.len() > 2);
    let last = l.rows.len() - 1;
    for row in 0..last {
        let (right, left) = margins(&l, row, spec.padding);
        assert!(right.abs() < 0.01 && left.abs() < 0.01, "row {row}: {right} on the right, {left} on the left");
    }
    let (right, left) = margins(&l, last, spec.padding);
    assert!(left > 1.0, "the last row is short");
    assert!((right - left).abs() < 0.01, "the last row is centred: {right} against {left}");
}

#[test]
fn a_low_cap_leaves_a_justified_row_short_and_centred() {
    let Some(page) = load(48) else { return };
    let mut passage = Passage::load(&[&page], 2, 282, 282).unwrap();
    let spec =
        PassageSpec { width: 375.0, scale: 1.0, align: Align::Justified, max_stretch: 1.2, ..Default::default() };
    let l = passage.layout(&spec).unwrap().clone();
    let mut short = 0;
    for row in 0..l.rows.len() {
        let (right, left) = margins(&l, row, spec.padding);
        assert!((right - left).abs() < 0.01, "row {row} is centred: {right} against {left}");
        if left > 0.01 {
            short += 1;
        }
    }
    assert!(short > 1, "the cap leaves a row short besides the last");
}

#[test]
fn a_cut_justified_passage_keeps_its_ellipsis_after_its_last_word() {
    let Some(page) = load(48) else { return };
    let mut passage = Passage::load(&[&page], 2, 282, 282).unwrap();
    let spec = PassageSpec {
        width: 375.0,
        scale: 1.0,
        align: Align::Justified,
        max_rows: 2,
        ellipsis_width: 9.0,
        ..Default::default()
    };
    let l = passage.layout(&spec).unwrap().clone();
    bounded(&l, "2:282 justified and cut");
    assert!(l.is_truncated);
    let e = l.ellipsis.unwrap();
    let last = l.words.last().unwrap();
    assert!(e[2] <= last.x0 + 0.001 && last.x0 - e[2] < 10.0, "the ellipsis follows the last word: {e:?}, {}", last.x0);
    // The cut row and its ellipsis are centred together.
    let r = &l.rows[1];
    let (right, left) = (l.width - spec.padding - r.x1, e[0] - spec.padding);
    assert!((right - left).abs() < 0.01, "the cut row is centred: {right} against {left}");
}

#[test]
fn a_sajdah_line_covers_its_words_on_a_justified_row() {
    for (n, surah, from, to) in [(272, 16, 49, 50), (598, 96, 19, 19)] {
        let Some(page) = load(n) else { return };
        let data = page.data();
        let q = page.quant();
        let mut passage = Passage::load(&[&page], surah, from, to).unwrap();
        let spec =
            PassageSpec { width: 200.0, scale: 0.9, align: Align::Justified, max_stretch: 0.0, ..Default::default() };
        let l = passage.layout(&spec).unwrap().clone();
        let units = |v: i32| v as f32 / q;
        let mut checked = 0;
        for (pi, path) in data.paths.iter().enumerate().filter(|(_, p)| p.mark == Mark::SajdahLine) {
            let (x0, y0, x1) = (units(path.bbox.x0), units(path.bbox.y0), units(path.bbox.x1));
            let across = |w: &qvp_core::qvp_format::WordRec| units(w.bbox.x0) < x1 && units(w.bbox.x1) > x0;
            // The words under the stroke: those across it on the line of the highest one it reaches.
            let Some(top) =
                data.words.iter().filter(|w| across(w) && units(w.bbox.y0) >= y0 - 2.0).min_by_key(|w| w.bbox.y0)
            else {
                continue;
            };
            let under: Vec<_> = data.words.iter().filter(|w| w.line_index == top.line_index && across(w)).collect();
            for draw in l.draws.iter().filter(|d| d.path == pi as u32) {
                let p = l.placements[draw.placement as usize];
                let (s0, s1) = (l.scale * (p.kx * x0 + p.dx), l.scale * (p.kx * x1 + p.dx));
                // The stroke is drawn above its words, inside the ink of the row that holds them.
                let middle = l.scale * (p.dy + (y0 + units(path.bbox.y1)) / 2.0);
                let row = l.rows.iter().position(|r| (r.y0..=r.y1).contains(&middle)).expect("the stroke is on a row");
                let row_words: Vec<_> = under
                    .iter()
                    .filter_map(|w| l.words.iter().find(|lw| lw.ayah == w.ayah && lw.word == w.word))
                    .filter(|lw| lw.row as usize == row)
                    .collect();
                assert!(!row_words.is_empty(), "page {n}: a stroke drawn over none of its words");
                for w in row_words {
                    assert!(
                        s0 <= w.x1 && s1 >= w.x0,
                        "page {n}: the stroke [{s0}, {s1}] misses {}:{} at [{}, {}]",
                        w.ayah,
                        w.word,
                        w.x0,
                        w.x1
                    );
                }
                checked += 1;
            }
        }
        assert!(checked > 0, "page {n} draws its sajdah line");
    }
}

#[test]
fn a_surahs_basmalah_is_a_passage_of_its_own() {
    for (n, surah) in [(2, 2), (604, 114)] {
        let Some(page) = load(n) else { return };
        let deco = page.data().decorations.iter().find(|d| d.kind == DecoKind::Basmalah && d.surah == surah).unwrap();
        let paths: Vec<u32> = (deco.first_path..deco.first_path + deco.n_paths as u32).collect();
        let mut basmalah = Passage::load_basmalah(&[&page], surah).unwrap();
        assert_eq!(basmalah.word_count(), 0);
        assert_eq!(basmalah.ayahs().count(), 0);
        // It is sized by the printed line, as an ayah of its page is.
        let ayah = Passage::load(&[&page], surah, 1, 1).unwrap();
        assert_eq!(basmalah.line_spacing(), ayah.line_spacing());
        for (spec, at) in [
            (
                PassageSpec {
                    width: 366.0,
                    scale: 22.0 / basmalah.line_spacing(),
                    align: Align::Center,
                    ..Default::default()
                },
                "centred",
            ),
            (PassageSpec { width: 366.0, scale: 0.9, align: Align::Justified, ..Default::default() }, "justified"),
            (PassageSpec { width: 0.0, scale: 0.5, padding: 0.0, ..Default::default() }, "one row"),
            (PassageSpec { width: 40.0, scale: 0.9, ..Default::default() }, "narrow"),
        ] {
            let at = format!("basmalah of {surah}, {at}");
            let l = basmalah.layout(&spec).unwrap().clone();
            bounded(&l, &at);
            assert_eq!(l.rows.len(), 1, "{at}");
            assert!(l.words.is_empty() && l.ayahs.is_empty(), "{at}");
            assert_eq!(l.draws.len(), paths.len(), "{at}");
            assert!(
                l.draws.iter().all(|d| d.page == n as u16 && paths.contains(&d.path)),
                "{at}: draws only its own paths"
            );
            let r = &l.rows[0];
            assert!(r.x1 > r.x0 && r.y1 > r.y0, "{at}: it has ink");
        }
        // Too narrow for it, it shrinks to fit rather than overflow.
        let l = basmalah.layout(&PassageSpec { width: 40.0, scale: 0.9, ..Default::default() }).unwrap().clone();
        assert!(l.scale < 0.9);
    }
}

#[test]
fn a_surah_without_a_basmalah_of_its_own_fails() {
    let (Some(one), Some(two), Some(tawbah)) = (load(1), load(2), load(187)) else { return };
    assert_eq!(Passage::load_basmalah(&[&one], 1).err(), Some(PassageError::NoBasmalah { surah: 1 }));
    assert_eq!(Passage::load_basmalah(&[&tawbah], 9).err(), Some(PassageError::NoBasmalah { surah: 9 }));
    assert_eq!(Passage::load_basmalah(&[&two], 3).err(), Some(PassageError::NoBasmalah { surah: 3 }));
    assert_eq!(Passage::load_basmalah(&[&two], 115).err(), Some(PassageError::Range));
    assert_eq!(Passage::load_basmalah(&[&two, &two], 2).err(), Some(PassageError::RepeatedPage));
}
