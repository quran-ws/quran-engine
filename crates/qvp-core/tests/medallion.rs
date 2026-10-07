//! Where a medallion sits on a reflowed row: midway between the ayah it closes and the one
//! that follows, measured from the whole of each side, the signs that travel with a word
//! included.
//!
//! Needs dist/pages from scripts/sync-test-data.sh. Skipped when the data is missing unless
//! QVP_REQUIRE_DATA is set.
use qvp_core::qvp_format::DecoKind;
use qvp_core::{Fill, LayoutSpec, Page, ReflowSpec};
use std::path::{Path, PathBuf};

fn page_path(n: u32) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../dist/pages/{n:03}.qvp"))
}

fn page(n: u32) -> Option<Page> {
    if !page_path(n).exists() {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!("dist/pages is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
        }
        eprintln!("skip: dist/pages is missing; run scripts/sync-test-data.sh to enable this test");
        return None;
    }
    Some(Page::load(&std::fs::read(page_path(n)).unwrap()).unwrap())
}

/// Page 014: 2:91 ends on مُّؤْمِنِينَ and 2:92 opens a quarter, so its quarter star (۞) travels
/// with وَلَقَدْ. The medallion of 2:91 stands between them, the star on its far side.
#[test]
fn a_medallion_is_centred_between_its_word_and_the_quarter_star_after_it() {
    let Some(mut p) = page(14) else { return };
    let q = p.quant();
    let d = p.data().clone();
    let closing = d.words.iter().rposition(|w| w.surah == 2 && w.ayah == 91).expect("2:91 is on page 14");
    let opening = d.words.iter().position(|w| w.surah == 2 && w.ayah == 92).expect("2:92 is on page 14");
    let medallion = d
        .decorations
        .iter()
        .position(|x| x.kind == DecoKind::AyahMark && x.surah == 2 && x.ayah == 91)
        .expect("the medallion of 2:91");
    let star = d.decorations.iter().position(|x| x.kind == DecoKind::DivisionMark).expect("the quarter star of 2:92");
    for fill in [Fill::Centred, Fill::Justified] {
        for zoom in [1.4, 1.8, 2.2] {
            let spec = LayoutSpec {
                viewport_w: 390.0,
                viewport_h: 844.0,
                reflow: Some(ReflowSpec { zoom, fill, max_stretch: 5.0, ..Default::default() }),
                ..Default::default()
            };
            let flow = p.layout(&spec).reflow.clone().expect("reflowed");
            if flow.word_row[closing] != flow.word_row[opening] {
                continue;
            }
            let x = |placement: qvp_core::Placement, ix: i32| placement.apply(ix as f32 / q, 0.0).0;
            // right to left: the closing word, the medallion, then the star and the word it opens
            let word_left = x(flow.word_place[closing], d.words[closing].bbox.x0);
            let (m_left, m_right) = (
                x(flow.deco_place[medallion], d.decorations[medallion].bbox.x0),
                x(flow.deco_place[medallion], d.decorations[medallion].bbox.x1),
            );
            let star_right = x(flow.deco_place[star], d.decorations[star].bbox.x1);
            let (right_air, left_air) = (word_left - m_right, m_left - star_right);
            assert!(left_air > 0.0, "{fill:?} at {zoom}: the medallion overlaps the star ({left_air})");
            assert!(
                (right_air - left_air).abs() < 0.25 * right_air.max(left_air),
                "{fill:?} at {zoom}: {right_air} of air before the medallion against {left_air} after it"
            );
        }
    }
}
