//! Hit testing: the word and the path under a point.
use super::*;

#[test]
fn hit_testing() {
    let p = page();
    assert_eq!(p.hit_test_exact(15.0, 15.0), Some(HitExact { word: 0, path: 0, decoration: NONE }));
    assert_eq!(p.hit_test_exact(35.0, 15.0), Some(HitExact { word: 1, path: 1, decoration: NONE }));
    assert_eq!(p.hit_test_exact(33.0, 6.0), Some(HitExact { word: 1, path: 2, decoration: NONE }));
    assert_eq!(p.hit_test_exact(33.0, 8.5), Some(HitExact { word: 1, path: NONE, decoration: NONE }));
    assert_eq!(p.hit_test_exact(25.0, 15.0), None);
    assert_eq!(p.hit_test_exact(50.0, 50.0), None);
    assert_eq!(p.word_text(1), "ٱلْكِتَٰبُ");
    assert_eq!(p.find_word(1, 1, 2), Some(1));
}

#[test]
fn gap_aware_hit_and_hit_boxes() {
    let p = page();
    let o = HitOptions::default();
    // in the gap between A (x 10-20) and B (x 30-40): gap 10, preceding = B (right) gets 60%
    let h = p.hit_test(25.0, 15.0, &o).unwrap();
    assert_eq!((h.word, h.is_exact), (1, false), "x=25 is within B's 60% share (threshold 24)");
    let h = p.hit_test(22.0, 15.0, &o).unwrap();
    assert_eq!((h.word, h.is_exact), (0, false));
    assert!((h.distance - 2.0).abs() < 1e-4);
    // above the line but within its line-spacing band → still resolves
    let h = p.hit_test(15.0, 1.0, &o).unwrap();
    assert_eq!(h.word, 0);
    assert!(p.hit_test(15.0, 1.0, &HitOptions { max_distance: 5.0, ..o }).is_none());
    let hb = p.hit_areas(0.6);
    assert_eq!(hb.len(), 3);
    assert!((hb[0].x1 - 24.0).abs() < 1e-4 && (hb[1].x0 - 24.0).abs() < 1e-4, "hit boxes meet at the gap split");
    let bands = p.line_bands();
    assert_eq!(bands.len(), 2);
    assert!(bands[0].y1 <= bands[1].y0 + 1e-3);
}
