//! The reader's zoom control: what a pinch commits to, and what it holds still.
//!
//! Needs dist/pages from scripts/sync-test-data.sh. Skipped when the data is missing unless
//! QVP_REQUIRE_DATA is set.
use qvp_core::{LayoutSpec, Page, Sideways, View, Zoom, ZoomMode};
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

fn spec() -> LayoutSpec {
    LayoutSpec { viewport_w: 390.0, viewport_h: 844.0, ..Default::default() }
}

/// The middle of the viewport, where a two-finger pinch usually sits.
fn middle(s: &LayoutSpec) -> (f32, f32) {
    (s.viewport_w / 2.0, s.viewport_h / 2.0)
}

#[test]
fn a_control_opens_on_the_printed_page() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    let z = Zoom::default();
    assert_eq!(z.mode, ZoomMode::Stepped, "stepped is what a host gets without asking");
    assert_eq!(z.step, 0);
    assert!(p.zoom_spec(&s, z).reflow.is_none(), "step 0 is the printed page, and asks for no reflow");
}

/// A step is left on the nearer of the midpoint to the next step and a fixed small reach, so
/// this measures against whichever of the two the page in hand puts first.
fn leaves_at(from: f32, to: f32) -> f32 {
    let midpoint = (from * to).sqrt();
    let reach = from * (1.0 + qvp_core::defaults::ZOOM_LEAVE_EFFORT);
    midpoint.min(reach) * (1.0 + qvp_core::defaults::ZOOM_SNAP_HYSTERESIS)
}

#[test]
fn a_pinch_short_of_the_reach_moves_nothing() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    p.layout(&s);
    let first = p.zoom_steps(&s)[0];
    // just short of what it takes to leave the printed page
    let line = leaves_at(1.0, first);
    let c = p.zoom_pinch(&s, Zoom::default(), View::default(), line * 0.99, middle(&s));
    assert!(!c.relaid, "the page was laid out again for a pinch that changed no row");
    assert_eq!(c.zoom.step, 0);
}

/// The reader's report: the control stuck at a step because that step sat above a wider gap
/// than its neighbours, so leaving it took a much bigger squeeze.
#[test]
fn every_step_takes_the_same_reach_to_leave() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    p.layout(&s);
    let n = p.zoom_steps(&s).len() as u32;
    let reach = 1.0 + qvp_core::defaults::ZOOM_LEAVE_EFFORT;
    let h = 1.0 + qvp_core::defaults::ZOOM_SNAP_HYSTERESIS;
    for step in 1..=n {
        let z = Zoom { mode: ZoomMode::Stepped, step, zoom: 0.0 };
        // closing the fingers by the reach and a hair leaves the step, whichever step it is
        let c = p.zoom_pinch(&s, z, View::default(), 0.99 / (reach * h), middle(&s));
        assert_eq!(c.zoom.step, step - 1, "step {step} would not let go of a pinch out");
    }
}

#[test]
fn a_pinch_past_the_reach_lands_on_the_step_exactly() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    p.layout(&s);
    let first = p.zoom_steps(&s)[0];
    let line = leaves_at(1.0, first);
    let c = p.zoom_pinch(&s, Zoom::default(), View::default(), line * 1.01, middle(&s));
    assert!(c.relaid);
    assert_eq!(c.zoom.step, 1);
    let r = p.zoom_spec(&s, c.zoom).reflow.expect("a step above the printed page reflows");
    assert!((r.zoom - first).abs() < 1e-4, "landed on {} rather than the page's own step {first}", r.zoom);
}

#[test]
fn the_hysteresis_does_not_oscillate_at_a_boundary() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    p.layout(&s);
    let steps = p.zoom_steps(&s);
    // fingers resting exactly on the line that leaves the first step
    let held = Zoom { mode: ZoomMode::Stepped, step: 1, zoom: 1.0 };
    let line = leaves_at(steps[0], steps[1]);
    let factor = line / steps[0];
    // A host holds the zoom the gesture began on and measures the fingers against that, so a
    // hand resting still asks for the same step every frame however many frames it rests for.
    let mut seen = Vec::new();
    for _ in 0..8 {
        seen.push(p.zoom_pinch(&s, held, View::default(), factor, middle(&s)).zoom.step);
    }
    assert!(seen.windows(2).all(|w| w[0] == w[1]), "a hand resting still walked the steps: {seen:?}");
}

#[test]
fn a_pinch_up_then_down_returns_to_the_step_it_left() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    p.layout(&s);
    let steps = p.zoom_steps(&s);
    let up = p.zoom_pinch(&s, Zoom::default(), View::default(), steps[0].sqrt() * 1.2, middle(&s));
    assert_eq!(up.zoom.step, 1);
    let down = p.zoom_pinch(&s, up.zoom, up.view, 0.5, middle(&s));
    assert_eq!(down.zoom.step, 0);
    assert!(p.zoom_spec(&s, down.zoom).reflow.is_none(), "back at the printed page");
}

#[test]
fn a_free_pinch_stays_inside_the_page_s_own_limit() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    p.layout(&s);
    let cap = p.reflow_max_zoom(&Default::default());
    let z = Zoom { mode: ZoomMode::Continuous, step: 0, zoom: 1.0 };
    let c = p.zoom_pinch(&s, z, View::default(), 99.0, middle(&s));
    assert!(c.zoom.zoom <= cap + 1e-3, "{} is past the page's limit of {cap}", c.zoom.zoom);
    let back = p.zoom_pinch(&s, c.zoom, c.view, 0.0001, middle(&s));
    assert!(back.zoom.zoom >= 1.0 - 1e-4, "a free pinch went below the printed page");
}

#[test]
fn a_free_pinch_inside_one_quantum_lays_nothing_out() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    let z = Zoom { mode: ZoomMode::Continuous, step: 0, zoom: 1.5 };
    let at = p.zoom_spec(&s, z);
    p.layout(&at);
    let c = p.zoom_pinch(&s, z, View::default(), 1.001, middle(&s));
    assert!(!c.relaid, "a change of {} asked for a layout", c.zoom.zoom - 1.5);
    assert!((c.zoom.zoom - 1.5).abs() < 1e-4);
}

#[test]
fn a_magnifying_pinch_leaves_the_rows_alone() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    p.layout(&s);
    let z = Zoom { mode: ZoomMode::Magnify, step: 0, zoom: 1.0 };
    let c = p.zoom_pinch(&s, z, View::default(), 2.0, middle(&s));
    assert_eq!(c.zoom, z, "the control moved under a magnifying pinch");
    assert!(!c.relaid);
    assert!((c.view.scale - 2.0).abs() < 1e-4, "the view did not take the pinch");
    assert!(p.zoom_spec(&s, c.zoom).reflow.is_none());
}

#[test]
fn a_size_button_reaches_every_step_and_comes_back() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    p.layout(&s);
    let steps = p.zoom_steps(&s);
    let mut z = Zoom::default();
    for (i, want) in steps.iter().enumerate() {
        let c = p.zoom_to_step(&s, z, i as u32 + 1, View::default());
        assert_eq!(c.zoom.step, i as u32 + 1);
        let r = p.zoom_spec(&s, c.zoom).reflow.unwrap();
        assert!((r.zoom - want).abs() < 1e-4);
        z = c.zoom;
    }
    let c = p.zoom_to_step(&s, z, 0, View::default());
    assert!(p.zoom_spec(&s, c.zoom).reflow.is_none());
}

#[test]
fn a_commit_holds_the_word_under_the_fingers() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    let focal = middle(&s);
    let mut held_exactly = 0;
    for step in 1..=p.zoom_steps(&s).len() as u32 {
        p.layout(&s);
        // the word the fingers are on, and where inside it they are: what the engine holds
        let before = p.hit_test_view(focal.0, focal.1, &Default::default()).expect("a word in the middle");
        let (_, y0, _, y1) = p.word_bounds_view(before.word);
        let ny = ((focal.1 - y0) / (y1 - y0)).clamp(0.0, 1.0);
        let c = p.zoom_to_step(&s, Zoom::default(), step, View::default());
        assert!(c.relaid);
        let (_, y0, _, y1) = p.word_bounds_view(before.word);
        let on_screen = c.view.offset_y + c.view.scale * (y0 + ny * (y1 - y0));
        // The page runs out of room when holding the point would open a blank strip past the
        // first or the last row: `View::clamp` refuses that and the page sits against that
        // edge instead, which is the nearest it can come.
        let content_h = p.current_layout().unwrap().content_h;
        let at_edge = c.view.offset_y >= -0.01 || c.view.offset_y <= s.viewport_h - content_h * c.view.scale + 0.01;
        let off = (on_screen - focal.1).abs();
        assert!(off < 1.0 || at_edge, "step {step}: the point the fingers were on came back {off} px away");
        if off < 1.0 {
            held_exactly += 1;
        }
    }
    assert!(held_exactly > 0, "every step reached an edge; the anchor itself was never tested");
}

#[test]
fn changing_mode_keeps_the_size_the_reader_is_at() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    p.layout(&s);
    let steps = p.zoom_steps(&s);
    let stepped = Zoom { mode: ZoomMode::Stepped, step: 2, zoom: 1.0 };
    let free = p.zoom_mode(&s, stepped, ZoomMode::Continuous);
    assert_eq!(free.mode, ZoomMode::Continuous);
    assert!((free.zoom - steps[1]).abs() < 1e-4, "the free zoom did not start from the step's size");
    let back = p.zoom_mode(&s, free, ZoomMode::Stepped);
    assert_eq!(back.step, 2, "the nearest step at or below the free zoom is the one it came from");
}

/// One gesture, many frames: the fingers decide the step, not how many frames have gone by.
/// A pinch that opens a little must stop at the first step and stay there while it is held —
/// the page used to run away to the last step, because each frame measured from the step the
/// frame before had committed to.
#[test]
fn a_slow_pinch_does_not_run_away_through_the_steps() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    p.layout(&s);
    let steps = p.zoom_steps(&s);
    // the fingers open to just past the first step's boundary, then hold there
    let held = (steps[0] * 1.02).max((1.0 * steps[0]).sqrt() * 1.05);
    let mut z = Zoom::default();
    let start = z;
    for frame in 0..12 {
        let c = p.zoom_pinch(&s, start, View::default(), held, middle(&s));
        z = c.zoom;
        assert_eq!(z.step, 1, "frame {frame}: the fingers held at step 1 but the page went to step {}", z.step);
    }
}

/// The fingers lead the way through every step and back, inside one gesture.
#[test]
fn one_gesture_walks_up_the_steps_and_back_down() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    p.layout(&s);
    let steps = p.zoom_steps(&s);
    let start = Zoom::default();
    let mut seen = Vec::new();
    // open the fingers wider and wider, then close them again, always measured from the start
    let factors: Vec<f32> = steps.iter().map(|z| z * 1.05).chain(steps.iter().rev().map(|z| z * 0.8)).collect();
    for f in factors {
        seen.push(p.zoom_pinch(&s, start, View::default(), f, middle(&s)).zoom.step);
    }
    assert_eq!(seen.first().copied(), Some(1), "a small pinch should reach the first step, got {seen:?}");
    assert_eq!(seen.iter().max().copied(), Some(3), "a wide pinch should reach the last step, got {seen:?}");
    assert!(seen.last().copied().unwrap() < 3, "closing the fingers should come back down, got {seen:?}");
}

/// A reader who turns pages keeps the size they chose. Every page has its own steps, so
/// carrying the size instead of the step rounds it down whenever the next page's step sits a
/// little higher, and ten turns walked a reader from the largest step back to the printed page.
#[test]
fn the_size_survives_a_run_of_page_turns() {
    let Some(mut first) = page(420) else { return };
    let s = spec();
    first.layout(&s);
    let top = first.zoom_steps(&s).len() as u32;
    let mut z = Zoom { mode: ZoomMode::Stepped, step: top, zoom: first.zoom_at_step(&s, top) };
    let mut seen = Vec::new();
    for n in 421..=430 {
        let Some(mut p) = page(n) else { return };
        p.layout(&s);
        z = p.zoom_carried(&s, z);
        seen.push(z.step);
        // the size is this page's own value for that step, not the page before's
        let want = p.zoom_steps(&s)[z.step as usize - 1];
        assert!((z.zoom - want).abs() < 1e-4, "page {n}: {} is not its own step {}", z.zoom, z.step);
    }
    assert!(seen.iter().all(|&x| x == top), "the reader lost the size they chose: {seen:?}");
}

/// A free zoom carries as a size, and no further than the page it lands on can reach.
#[test]
fn a_free_zoom_carries_inside_the_new_page_s_limit() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    p.layout(&s);
    let cap = p.reflow_max_zoom(&Default::default());
    let carried = p.zoom_carried(&s, Zoom { mode: ZoomMode::Continuous, step: 0, zoom: cap + 5.0 });
    assert!(carried.zoom <= cap + 1e-3, "{} is past this page's limit of {cap}", carried.zoom);
    assert_eq!(carried.mode, ZoomMode::Continuous);
}

/// A sideways drag turns the page wherever there is no sideways travel to spend.
#[test]
fn a_sideways_drag_turns_the_page_unless_there_is_room_to_pan() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    p.layout(&s);
    let settled = View::default();
    assert_eq!(p.sideways_drag(Zoom::default(), settled, 1.0), Sideways::TurnPage, "a page nobody zoomed");
    let magnified = View { scale: 2.0, ..View::default() };
    let z = Zoom { mode: ZoomMode::Magnify, step: 0, zoom: 1.0 };
    assert_eq!(p.sideways_drag(z, magnified, 1.0), Sideways::Pan, "a magnified page has room to pan");
    // the same magnified view, but the page is reflowed: its width is the screen's
    let stepped = p.zoom_to_step(&s, Zoom::default(), 2, settled).zoom;
    assert_eq!(p.sideways_drag(stepped, magnified, 1.0), Sideways::TurnPage, "a reflowed page has none");
}

/// The reader has zoomed in by either road: a magnified view, or a reflowed page.
#[test]
fn zoomed_in_counts_both_roads() {
    let settled = View::default();
    assert!(!Zoom::default().is_zoomed(settled, 1.0));
    assert!(Zoom::default().is_zoomed(View { scale: 2.0, ..settled }, 1.0), "a magnified view");
    let reflowed = Zoom { mode: ZoomMode::Stepped, step: 1, zoom: 1.35 };
    assert!(reflowed.is_zoomed(settled, 1.0), "a reflowed page, whatever the view is at");
}

/// A muṣḥaf is read right to left: a flick to the right turns to the next page. Every host used
/// to work this out for itself, and a host that got the sign the wrong way round turned the
/// book backwards.
#[test]
fn a_flick_to_the_right_turns_to_the_next_page() {
    use qvp_core::{swipe_direction, swipe_pages};
    let far = 200.0;
    assert_eq!(swipe_pages(far, 0.0, 0.0, 0.0), 1, "a flick right is the next page");
    assert_eq!(swipe_pages(-far, 0.0, 0.0, 0.0), -1, "a flick left is the page before");
    assert_eq!(swipe_pages(0.0, far, 0.0, 0.0), 0, "a drag up and down turns nothing");
    // the physical answer is still there for a host that wants it
    assert_eq!(swipe_direction(far, 0.0, 0.0, 0.0), 1);
}

/// The reader's report: the control sticks at a step and a pinch out will not leave it, while
/// the same pinch from a step above walks all the way down.
#[test]
fn a_pinch_out_leaves_every_step() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    let f = middle(&s);
    let n = p.zoom_steps(&s).len() as u32;
    for start in 1..=n {
        // the reader is at `start`, having got there however; now they pinch out
        let mut z = Zoom { mode: ZoomMode::Stepped, step: start, zoom: 0.0 };
        z.zoom = p.zoom_at_step(&s, start);
        let held = z;
        // what a host actually passes: the spec its current layout is in, which moves with every
        // commit, while the gesture keeps measuring from where the fingers went down
        let mut live_zoom = held;
        let mut view = View::default();
        let mut reached = start;
        // one gesture, closing the fingers the whole way
        let mut factor = 1.0f32;
        while factor > 0.05 {
            factor *= 0.97;
            let in_force = p.zoom_spec(&s, live_zoom);
            let c = p.zoom_pinch(&in_force, held, view, factor, f);
            view = c.view;
            live_zoom = c.zoom;
            reached = reached.min(c.zoom.step);
        }
        assert_eq!(reached, 0, "a pinch out from step {start} only reached step {reached}");
    }
}

#[test]
fn a_magnifying_pinch_never_shrinks_the_page_inside_the_screen() {
    let Some(mut p) = page(428) else { return };
    let s = spec();
    let fit = p.layout(&s).fit_scale;
    let z = Zoom { mode: ZoomMode::Magnify, step: 0, zoom: 1.0 };
    let settled = View { scale: fit, offset_x: 0.0, offset_y: 0.0 };
    for factor in [0.9, 0.5, 0.1, 0.01] {
        let c = p.zoom_pinch(&s, z, settled, factor, middle(&s));
        assert!(c.view.scale >= fit - 1e-4, "a pinch out took the page to {} below its settled {fit}", c.view.scale);
    }
}
