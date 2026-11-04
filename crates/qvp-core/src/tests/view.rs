//! The view: pan, zoom, anchors and geometry through a relayout.
use super::*;

#[test]
fn view_zoom_pans_about_the_fingers_and_clamps() {
    let v = View { scale: 1.0, offset_x: 0.0, offset_y: 0.0 };
    // the point under the fingers stays under them
    let z = v.zoom_about(100.0, 50.0, 2.0, 0.0, 0.0);
    assert_eq!(z.scale, 2.0);
    assert!((z.offset_x + 100.0).abs() < 1e-4 && (z.offset_y + 50.0).abs() < 1e-4);
    // and still does when the clamp cuts the factor down
    let hit = v.zoom_about(100.0, 50.0, 100.0, 0.0, 4.0);
    assert_eq!(hit.scale, 4.0);
    assert!((100.0 - (100.0 - hit.offset_x) / 4.0).abs() < 1e-3);
    // content smaller than the viewport is centred; larger content keeps it covered
    let small = View { scale: 1.0, offset_x: -80.0, offset_y: 0.0 }.clamp(100.0, 100.0, 300.0, 300.0);
    assert!((small.offset_x - 100.0).abs() < 1e-4);
    let big = View { scale: 1.0, offset_x: 50.0, offset_y: -900.0 }.clamp(400.0, 1000.0, 300.0, 300.0);
    assert_eq!((big.offset_x, big.offset_y), (0.0, -700.0));
    assert_eq!(swipe_direction(60.0, 5.0, 0.0, 0.0), 1);
    assert_eq!(swipe_direction(-60.0, 5.0, 0.0, 0.0), -1);
    assert_eq!(swipe_direction(10.0, 5.0, 0.0, 0.0), 0);
    assert_eq!(swipe_direction(60.0, 300.0, 0.0, 0.0), 0);
}

#[test]
fn a_zoom_ceiling_under_the_floor_is_the_floor() {
    let v = View { scale: 0.1, offset_x: 0.0, offset_y: 0.0 };
    // the floor holds against a ceiling under it, and against a factor that would pass it
    assert_eq!(v.zoom_about(0.0, 0.0, 1.0, 0.5, 0.1).scale, 0.5);
    assert_eq!(v.zoom_about(0.0, 0.0, 100.0, 1.0, 0.5).scale, 1.0);
    // a ceiling of 0 is the engine's own, not the floor
    assert_eq!(v.zoom_about(0.0, 0.0, 1000.0, 1.0, 0.0).scale, defaults::MAX_ZOOM);
}

#[test]
fn view_anchor_holds_a_word_across_a_relayout() {
    let mut p = packed_page();
    let base = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    p.layout(&base);
    let word = 5u32;
    let v = View { scale: 1.0, offset_x: 0.0, offset_y: 0.0 };
    let (_, y0, _, y1) = p.word_bounds_view(word);
    let was = y0 + 0.5 * (y1 - y0);
    // the reader zooms in: the page reflows and the word moves to another row
    p.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }), ..base });
    let moved = {
        let (_, a, _, b) = p.word_bounds_view(word);
        a + 0.5 * (b - a)
    };
    assert!((moved - was).abs() > 1.0, "the relayout should have moved the word");
    // anchoring brings that point back to where the fingers are
    let v2 = p.view_anchor(v, word, (0.5, 0.5), (0.0, was), (200.0, 400.0));
    let (_, a, _, b) = p.word_bounds_view(word);
    let now = v2.offset_y + v2.scale * (a + 0.5 * (b - a));
    assert!((now - was).abs() < 0.01, "anchored to {now}, wanted {was}");
}

#[test]
fn view_geometry_follows_the_words_it_describes() {
    let mut p = packed_page();
    let base = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    for reflow in [None, Some(ReflowSpec { zoom: 1.6, ..Default::default() })] {
        p.layout(&LayoutSpec { reflow, ..base });
        let tag = if reflow.is_some() { "reflowed" } else { "printed" };
        for w in 0..p.data().words.len() as u32 {
            let (x0, y0, x1, y1) = p.word_bounds_view(w);
            // an ink band sits on the word's own ink
            let b = p.word_bands_view(&[w], BandHeight::Ink, 0.0, 0.0);
            assert_eq!(b.len(), 1, "{tag}: one band per word");
            assert!(
                (b[0].x0 - x0).abs() < 0.01 && (b[0].y0 - y0).abs() < 0.01 && (b[0].x1 - x1).abs() < 0.01,
                "{tag}: band {:?} off word {w} {:?}",
                (b[0].x0, b[0].y0, b[0].x1, b[0].y1),
                (x0, y0, x1, y1)
            );
            // and the hit box contains that ink, and answers where a tap in it does
            let area = p.hit_areas_view(0.6).into_iter().find(|a| a.word == w).expect("an area per word");
            assert!(
                area.x0 <= x0 + 0.01 && area.x1 >= x1 - 0.01 && area.y0 <= y0 + 0.01 && area.y1 >= y1 - 0.01,
                "{tag}: area misses word {w}"
            );
            let hit = p.hit_test_view((area.x0 + area.x1) / 2.0, (area.y0 + area.y1) / 2.0, &HitOptions::default());
            assert_eq!(hit.map(|h| h.word), Some(w), "{tag}: a tap in word {w}'s box");
        }
    }
}
