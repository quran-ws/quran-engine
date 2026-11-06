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

#[test]
fn a_page_that_fits_is_one_band() {
    let v = View { scale: 1.0, offset_x: 0.0, offset_y: -300.0 };
    assert_eq!(v.ink_band(2000.0, 400.0, 800.0, 0.0, 0.0), InkBand { top: 0.0, height: 2000.0 });
    // the default ceiling, in viewport pixels: the same page at twice the size no longer fits
    let tall = View { scale: 2.5, ..v }.ink_band(2000.0, 400.0, 800.0, 0.0, 0.0);
    assert!(tall.height < 2000.0);
    // exactly at the ceiling is still whole
    assert_eq!(v.ink_band(defaults::MAX_INK_HEIGHT, 400.0, 800.0, 0.0, 0.0).height, defaults::MAX_INK_HEIGHT);
    // a pixel budget too small for the whole page bands it
    let budget = v.ink_band(2000.0, 400.0, 800.0, 0.0, 400.0 * 1999.0);
    assert!(budget.height < 2000.0);
    assert_eq!(v.ink_band(2000.0, 400.0, 800.0, 0.0, 400.0 * 2000.0).height, 2000.0);
}

#[test]
fn no_screen_means_the_whole_page() {
    let v = View { scale: 1.0, offset_x: 0.0, offset_y: -3000.0 };
    let whole = InkBand { top: 0.0, height: 9000.0 };
    assert_eq!(v.ink_band(9000.0, 400.0, 0.0, 0.0, 0.0), whole);
    assert_eq!(v.ink_band(9000.0, 400.0, -10.0, 0.0, 0.0), whole);
    assert_eq!(View { scale: 0.0, ..v }.ink_band(9000.0, 400.0, 800.0, 0.0, 0.0), whole);
    assert_eq!(View { scale: -1.0, ..v }.ink_band(9000.0, 400.0, 800.0, 0.0, 0.0), whole);
    assert_eq!(View { scale: f32::NAN, ..v }.ink_band(9000.0, 400.0, 800.0, 0.0, 0.0), whole);
    assert_eq!(v.ink_band(f32::NAN, 400.0, 800.0, 0.0, 0.0), InkBand { top: 0.0, height: 0.0 });
}

#[test]
fn a_tall_page_is_drawn_two_screens_at_a_time() {
    // 800 px of screen: a band of 1600, a step of 400
    let at = |offset_y: f32| View { scale: 1.0, offset_x: 0.0, offset_y }.ink_band(9000.0, 400.0, 800.0, 0.0, 0.0);
    let b = at(-1000.0);
    assert_eq!(b, InkBand { top: 400.0, height: 1600.0 });
    // the band covers the screen, with slack either side
    assert!(b.top <= 1000.0 && b.top + b.height >= 1800.0);
    // a pan inside one step keeps the band where it is
    for y in [-800.0, -900.0, -1100.0, -1199.9] {
        assert_eq!(at(y).top, 400.0, "offset {y}");
    }
    // crossing the step moves it by exactly one step, either way
    assert_eq!(at(-1200.0).top, 800.0);
    assert_eq!(at(-799.9).top, 0.0);
    // the ceiling holds the band, and a screen taller than the ceiling is held to it too
    assert_eq!(at(0.0).height, 1600.0);
    let v = View { scale: 1.0, offset_x: 0.0, offset_y: 0.0 };
    assert_eq!(v.ink_band(9000.0, 400.0, 3000.0, 0.0, 0.0).height, defaults::MAX_INK_HEIGHT);
    assert_eq!(v.ink_band(9000.0, 400.0, 6000.0, 0.0, 0.0).height, defaults::MAX_INK_HEIGHT);
    assert_eq!(v.ink_band(9000.0, 400.0, 800.0, 1000.0, 0.0).height, 1000.0);
    // a pixel budget is a ceiling of its own: 400 px wide in 400 000 px is 1000 rows
    assert_eq!(v.ink_band(9000.0, 400.0, 800.0, 0.0, 400_000.0).height, 1000.0);
}

#[test]
fn the_band_is_in_layout_pixels_and_its_limits_on_the_screen() {
    // at a view scale of 2, an 800 px screen is 400 layout px and the band 800 of them
    let v = View { scale: 2.0, offset_x: 0.0, offset_y: -2000.0 };
    let b = v.ink_band(3000.0, 400.0, 800.0, 0.0, 0.0);
    assert_eq!(b, InkBand { top: 800.0, height: 800.0 });
    // the screen's top, 1000 layout px down, is inside the band, and so is its bottom
    assert!(b.top <= 1000.0 && b.top + b.height >= 1400.0);
    // the ceiling counts viewport pixels: 3000 layout px are 6000 on the screen
    assert_eq!(v.ink_band(3000.0, 400.0, 800.0, 6000.0, 0.0).height, 3000.0);
    assert!(v.ink_band(3000.0, 400.0, 800.0, 5999.0, 0.0).height < 3000.0);
}

#[test]
fn the_page_band_reads_the_current_layout() {
    let mut p = packed_page();
    let v = View { scale: 1.0, offset_x: 0.0, offset_y: 0.0 };
    // no layout: the page's own height
    assert_eq!(p.layout_ink_band(v, 200.0, 400.0, 0.0, 0.0).height, p.height());
    p.layout(&LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() });
    let h = p.current_layout().unwrap().content_h;
    assert_eq!(p.layout_ink_band(v, 200.0, 400.0, 0.0, 0.0), InkBand { top: 0.0, height: h });
    // a ceiling under the content bands it
    let b = p.layout_ink_band(v, 200.0, 100.0, h / 2.0, 0.0);
    assert!(b.height <= h / 2.0 && b.height > 0.0);
}

#[test]
fn a_pixel_budget_never_takes_the_band_below_one_screen() {
    let at = |offset_y: f32, max_pixels: f32| {
        View { scale: 1.0, offset_x: 0.0, offset_y }.ink_band(9000.0, 400.0, 800.0, 0.0, max_pixels)
    };
    // 300 rows of budget for an 800 px screen: the band is the screen, not 300 px
    assert_eq!(at(-1000.0, 400.0 * 300.0).height, 800.0);
    // and the band still covers the screen, wherever the reader is
    for y in [0.0, -1000.0, -1000.5, -4321.25] {
        let b = at(y, 400.0 * 300.0);
        assert!(b.top <= -y && b.top + b.height >= -y + 800.0, "offset {y}: {b:?}");
    }
    // between one screen and two, the budget sets the height
    assert_eq!(at(0.0, 400.0 * 1200.0).height, 1200.0);
    // a page under one screen is whole, however small the budget
    let v = View { scale: 1.0, offset_x: 0.0, offset_y: 0.0 };
    assert_eq!(v.ink_band(500.0, 400.0, 800.0, 0.0, 400.0 * 10.0), InkBand { top: 0.0, height: 500.0 });
    // the hard limit still holds under a screen taller than it
    assert_eq!(v.ink_band(9000.0, 400.0, 800.0, 600.0, 400.0 * 300.0).height, 600.0);
}

#[test]
fn a_non_finite_offset_means_the_whole_page() {
    let whole = InkBand { top: 0.0, height: 9000.0 };
    for offset_y in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
        let v = View { scale: 1.0, offset_x: 0.0, offset_y };
        assert_eq!(v.ink_band(9000.0, 400.0, 800.0, 0.0, 0.0), whole, "offset {offset_y}");
    }
    // so do a non-finite scale and viewport height
    let v = View { scale: f32::INFINITY, offset_x: 0.0, offset_y: 0.0 };
    assert_eq!(v.ink_band(9000.0, 400.0, 800.0, 0.0, 0.0), whole);
    let v = View { scale: 1.0, offset_x: 0.0, offset_y: 0.0 };
    assert_eq!(v.ink_band(9000.0, 400.0, f32::INFINITY, 0.0, 0.0), whole);
    // a NaN limit is no limit: the height takes the default and the budget none
    assert_eq!(v.ink_band(9000.0, 400.0, 800.0, f32::NAN, f32::NAN).height, 1600.0);
    // an infinite limit holds every page, so the page is whole
    assert_eq!(v.ink_band(9000.0, 400.0, 800.0, f32::INFINITY, f32::INFINITY), whole);
}

#[test]
fn a_page_below_the_top_of_the_screen_bands_from_above_it() {
    // the page starts 300 px down the screen: the band starts above the page
    let v = View { scale: 1.0, offset_x: 0.0, offset_y: 300.0 };
    let b = v.ink_band(9000.0, 400.0, 800.0, 0.0, 0.0);
    assert_eq!(b, InkBand { top: -800.0, height: 1600.0 });
    assert!(b.top <= -300.0 && b.top + b.height >= 500.0);
}

#[test]
fn a_band_with_under_two_pixels_of_slack_covers_the_screen() {
    // Slack of 0, 0.5, 1 and 1.5 px: a budget of about one screen, or a ceiling within 2 px of
    // the screen. No step of a whole pixel fits, so the band follows the reader.
    let cases = [
        (0.0, 400.0 * 800.0),
        (0.0, 400.0 * 800.5),
        (0.0, 400.0 * 801.0),
        (0.0, 400.0 * 801.5),
        (800.0, 0.0),
        (801.0, 0.0),
        (801.5, 0.0),
    ];
    for (max_height, max_pixels) in cases {
        let mut y = 0.0f32;
        while y < 8000.0 {
            let b =
                View { scale: 1.0, offset_x: 0.0, offset_y: -y }.ink_band(9000.0, 400.0, 800.0, max_height, max_pixels);
            assert!(
                b.top <= y && b.top + b.height >= y + 800.0,
                "ceiling {max_height}, budget {max_pixels}, offset {y}: {b:?}"
            );
            y += 0.37;
        }
    }
    // two whole pixels of slack still step: a pan inside one step keeps the band
    let at = |y: f32| View { scale: 1.0, offset_x: 0.0, offset_y: -y }.ink_band(9000.0, 400.0, 800.0, 802.0, 0.0);
    assert_eq!(at(1000.0).top, at(1000.9).top);
}
