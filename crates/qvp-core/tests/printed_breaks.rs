//! Printed breaks keep each line whole while the ink grows to fill the page width.
//! Needs dist/pages from scripts/sync-test-data.sh.
use qvp_core::{Breaks, Fill, GapMode, LayoutSpec, Page, ReflowSpec};
use std::path::Path;

#[test]
fn printed_rows_keep_their_words_and_fit_at_max_zoom() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dist/pages");
    if !dir.join("001.qvp").exists() {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!("dist/pages is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
        }
        eprintln!("skip: dist/pages is missing; run scripts/sync-test-data.sh to enable this test");
        return;
    }
    for n in [1, 2] {
        let mut page = Page::load(&std::fs::read(dir.join(format!("{n:03}.qvp"))).unwrap()).unwrap();
        let base = LayoutSpec { viewport_w: 390.0, viewport_h: 844.0, ..Default::default() };
        let printed_scale = page.layout(&base).scale;
        let expected: Vec<Vec<u32>> = page
            .data()
            .lines
            .iter()
            .map(|line| {
                let mut words: Vec<u32> = (line.first_word..line.first_word + line.n_words).map(u32::from).collect();
                words.sort_by(|&a, &b| {
                    page.data().words[b as usize].bbox.x0.cmp(&page.data().words[a as usize].bbox.x0)
                });
                words
            })
            .collect();
        if n == 1 {
            assert_eq!(expected.iter().filter(|ws| !ws.is_empty()).count(), 7);
        }
        for gaps in [GapMode::Printed, GapMode::Uniform] {
            for fill in [Fill::Centred, Fill::Justified, Fill::Ragged] {
                let reflow = ReflowSpec { breaks: Breaks::Printed, gaps, fill, relax: 0.0, ..Default::default() };
                let cap = page.reflow_max_zoom(&reflow);
                assert!(cap > 1.2, "page {n}: ink should grow clearly, got {cap}");
                let mut capped = None;
                for zoom in [1.1, cap, cap * 2.0] {
                    let layout =
                        page.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom, ..reflow }), ..base }).clone();
                    let flow = layout.reflow.as_ref().unwrap();
                    assert!(!flow.as_printed);
                    assert_eq!(flow.row_words, expected, "page {n}, {gaps:?}, {fill:?}, zoom {zoom}");
                    let mut widest = 0.0f32;
                    for (row, words) in flow.row_words.iter().enumerate().filter(|(_, ws)| !ws.is_empty()) {
                        let (mut left, mut right) = (f32::INFINITY, f32::NEG_INFINITY);
                        for &wi in words {
                            let b = page.word_bounds_view(wi);
                            left = left.min(b.0);
                            right = right.max(b.2);
                        }
                        // A medallion can extend past the last word of its row.
                        let q = page.quant();
                        for (di, deco) in page.data().decorations.iter().enumerate() {
                            if deco.kind == qvp_core::qvp_format::DecoKind::AyahMark && deco.line as usize == row {
                                let p = flow.deco_place[di];
                                left =
                                    left.min(layout.offset_x + p.apply(deco.bbox.x0 as f32 / q, 0.0).0 * layout.scale);
                                right =
                                    right.max(layout.offset_x + p.apply(deco.bbox.x1 as f32 / q, 0.0).0 * layout.scale);
                            }
                        }
                        assert!(left >= -0.01 && right <= base.viewport_w + 0.01, "page {n}: {left}..{right}");
                        widest = widest.max(right - left);
                    }
                    if zoom == cap {
                        assert!(layout.scale > printed_scale * 1.2);
                        assert!((widest - base.viewport_w).abs() < 0.01, "page {n}: widest row {widest}");
                        capped = Some(layout);
                    } else if zoom > cap {
                        let at_cap = capped.as_ref().unwrap();
                        assert_eq!(layout.scale, at_cap.scale);
                        assert_eq!(layout.reflow, at_cap.reflow);
                    }
                }
            }
        }
    }
}
