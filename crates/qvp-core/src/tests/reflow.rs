//! Reflow: breaking the words onto rows, filling them and spacing them.
use super::*;

#[test]
fn a_reflow_measures_the_page_once_and_lays_it_out_the_same_twice() {
    let mut p = packed_page();
    assert!(p.reflow_prepared.get().is_none(), "nothing is measured before a reflow asks");
    let spec = LayoutSpec {
        viewport_w: 200.0,
        viewport_h: 400.0,
        reflow: Some(ReflowSpec { zoom: 1.5, ..Default::default() }),
        ..Default::default()
    };
    let first = p.layout(&spec).clone();
    let prepared = p.reflow_prepared.get().expect("the first reflow measures the page") as *const _;
    let second = p.layout(&spec).clone();
    assert_eq!(first, second);
    assert!(std::ptr::eq(prepared, p.reflow_prepared.get().unwrap()), "the second reflow reads the same measure");
    // another zoom and the zoom ceiling read the same measure too
    let other = LayoutSpec { reflow: Some(ReflowSpec { zoom: 3.0, ..Default::default() }), ..spec };
    let _ = p.layout(&other);
    let _ = p.reflow_max_zoom(&ReflowSpec::default());
    let _ = p.reflow_max_zoom(&ReflowSpec { breaks: Breaks::Printed, ..Default::default() });
    assert!(std::ptr::eq(prepared, p.reflow_prepared.get().unwrap()));
    assert!(first.reflow.as_ref().is_some_and(|f| f.row_band.len() > 3), "the page did reflow");
}

#[test]
fn printed_breaks_clamp_whole_rows_and_apply_fill() {
    use crate::{Breaks, Fill, GapMode};
    let mut p = packed_page();
    let base = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    let mut r = ReflowSpec { breaks: Breaks::Printed, gaps: GapMode::Printed, relax: 0.0, ..Default::default() };
    let cap = p.reflow_max_zoom(&r);
    assert!((cap - 100.0 / 76.0).abs() < 1e-5);
    for fill in [Fill::Centred, Fill::Ragged, Fill::Justified] {
        r.fill = fill;
        let at_cap = p.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: cap, ..r }), ..base }).clone();
        let clamped = p.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 4.0, ..r }), ..base }).clone();
        assert_eq!(at_cap.scale, clamped.scale);
        assert_eq!(at_cap.reflow, clamped.reflow);
        let flow = clamped.reflow.unwrap();
        assert_eq!(flow.row_words, vec![vec![0, 1, 2, 3], vec![4, 5, 6, 7], vec![8, 9, 10, 11]]);
        assert!(!flow.as_printed);
        assert!((p.word_bounds_view(0).2 - base.viewport_w).abs() < 0.01);
        assert!(p.word_bounds_view(3).0.abs() < 0.01);
    }
    for fill in [Fill::Centred, Fill::Ragged] {
        let l = p.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 1.1, fill, ..r }), ..base }).clone();
        let right = p.word_bounds_view(0).2;
        let left = p.word_bounds_view(3).0;
        if fill == Fill::Centred {
            assert!((left - (base.viewport_w - right)).abs() < 0.01);
        } else {
            assert!((right - base.viewport_w).abs() < 0.01);
        }
        assert!(!l.reflow.unwrap().as_printed);
    }
}

#[test]
fn reflow_breaks_words_onto_rows_of_the_same_width() {
    let mut p = packed_page();
    let printed = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    let wide = p.layout(&printed).clone();
    let zoom = 3.0;
    let spec = LayoutSpec { reflow: Some(ReflowSpec { zoom, ..Default::default() }), ..printed };
    let l = p.layout(&spec).clone();
    // the page keeps its width and the ink grows by the zoom
    assert!((l.content_w - wide.content_w).abs() < 1e-3);
    assert!((l.scale - wide.scale * zoom).abs() < 1e-3);
    // zooming in breaks the words onto more rows, and the page grows taller
    let easy = LayoutSpec { reflow: Some(ReflowSpec { zoom: 1.5, ..Default::default() }), ..printed };
    let rows_at_1 = p.layout(&easy).reflow.as_ref().unwrap().row_band.len();
    let l = p.layout(&spec).clone();
    let flow = l.reflow.clone().unwrap();
    assert!(flow.row_band.len() > rows_at_1, "rows {} vs {rows_at_1}", flow.row_band.len());
    assert!(l.content_h > wide.content_h);
    // a scrolled page is never shrunk back to the viewport
    assert_eq!(l.fit_scale, 1.0);
    // every word sits inside its row, and rows never run backwards in reading order
    let q = p.quant();
    for (r, words) in flow.row_words.iter().enumerate() {
        let mut last_x1 = f32::INFINITY;
        for &wi in words {
            let w = &p.data().words[wi as usize];
            let pl = flow.word_place[wi as usize];
            let x0 = pl.apply(w.bbox.x0 as f32 / q, 0.0).0;
            let x1 = pl.apply(w.bbox.x1 as f32 / q, 0.0).0;
            // a word wider than the row is the documented overflow case (see reflow_max_zoom)
            if x1 - x0 <= flow.row_w + 0.01 {
                assert!(x0 >= -0.01 && x1 <= flow.row_w + 0.01, "word {wi} out of row: {x0}..{x1}");
            }
            assert!(x1 <= last_x1 + 0.01, "word {wi} overlaps its right neighbour on row {r}");
            last_x1 = x0;
            assert_eq!(flow.word_row[wi as usize], r as u32);
        }
    }
    // words keep their printed size: only whole words move
    for (wi, pl) in flow.word_place.iter().enumerate() {
        assert_eq!((pl.kx, pl.ky), (1.0, 1.0), "word {wi} was resized");
    }
    // a tap on a placed word finds that word
    for wi in 0..p.data().words.len() as u32 {
        let (x0, y0, x1, y1) = p.word_bounds_view(wi);
        let h = p.hit_test_view((x0 + x1) / 2.0, (y0 + y1) / 2.0, &HitOptions::default());
        assert_eq!(h.map(|h| h.word), Some(wi), "tap on word {wi}");
    }
}

#[test]
fn reflow_omits_a_native_surah_frame_without_constraining_its_title() {
    let (mut framed, Some(frame_path), title_path, decoration) = page_with_surah_frame(true) else { unreachable!() };
    let (mut bare, None, bare_title, _) = page_with_surah_frame(false) else { unreachable!() };
    let printed_spec = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };

    let printed = framed.layout(&printed_spec).clone();
    assert!(printed.omitted_paths.is_empty());
    assert!(framed.layout_draw_list().iter().any(|draw| draw.path == frame_path));
    let frame_point = placed_point(&framed, &printed, frame_path, 11.0, -9.0);
    assert_eq!(
        framed.hit_test_exact_view(frame_point.0, frame_point.1),
        Some(HitExact { word: NONE, path: frame_path, decoration })
    );
    let hollow_point = placed_point(&framed, &printed, frame_path, 25.0, 0.0);
    assert_eq!(framed.hit_test_exact(25.0, 0.0), None);
    assert_eq!(framed.hit_test_exact_view(hollow_point.0, hollow_point.1), None);

    let hidden_spec = LayoutSpec { surah_frames: false, ..printed_spec };
    let hidden = framed.layout(&hidden_spec).clone();
    let bare_printed = bare.layout(&printed_spec).clone();
    assert_eq!(hidden.omitted_paths, vec![frame_path]);
    assert!(!framed.layout_draw_list().iter().any(|draw| draw.path == frame_path));
    assert!(framed.layout_draw_list().iter().any(|draw| draw.path == title_path));
    assert_eq!(hidden.content_h, bare_printed.content_h);
    assert_eq!(hidden.line_slots, bare_printed.line_slots);
    let hidden_frame_point = placed_point(&framed, &hidden, frame_path, 11.0, -9.0);
    assert_eq!(framed.hit_test_exact_view(hidden_frame_point.0, hidden_frame_point.1), None);
    let hidden_title_point = placed_point(&framed, &hidden, title_path, 25.0, 5.0);
    assert_eq!(
        framed.hit_test_exact_view(hidden_title_point.0, hidden_title_point.1),
        Some(HitExact { word: NONE, path: title_path, decoration })
    );

    let reflow_spec = LayoutSpec { reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }), ..printed_spec };
    let framed_layout = framed.layout(&reflow_spec).clone();
    let bare_layout = bare.layout(&reflow_spec).clone();
    assert!(!framed_layout.reflow.as_ref().unwrap().as_printed);
    assert_eq!(framed_layout.omitted_paths, vec![frame_path]);
    assert!(!framed.layout_draw_list().iter().any(|draw| draw.path == frame_path));
    assert!(framed.layout_draw_list().iter().any(|draw| draw.path == title_path));

    let title_placement = framed_layout.placement(title_path, 0);
    let bare_placement = bare_layout.placement(bare_title, 0);
    assert_eq!(title_placement, bare_placement, "the frame must not move or resize the title");
    assert!((framed_layout.content_h - bare_layout.content_h).abs() < 0.01);
    assert_eq!(framed_layout.line_slots, bare_layout.line_slots);

    let omitted_point = placed_point(&framed, &framed_layout, frame_path, 15.0, -5.0);
    assert_eq!(framed.hit_test_exact_view(omitted_point.0, omitted_point.1), None);
    let title_point = placed_point(&framed, &framed_layout, title_path, 25.0, 5.0);
    assert_eq!(
        framed.hit_test_exact_view(title_point.0, title_point.1),
        Some(HitExact { word: NONE, path: title_path, decoration })
    );

    // Asking for reflow at the printed size dispatches to the printed layout, so the native
    // frame remains. Only a page actually broken onto enlarged rows omits it.
    let at_print = framed
        .layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 1.0, ..Default::default() }), ..printed_spec })
        .clone();
    assert!(at_print.omitted_paths.is_empty());
    assert!(framed.layout_draw_list().iter().any(|draw| draw.path == frame_path));
}

#[test]
fn reflow_justified_fills_every_row_but_the_last() {
    let mut p = packed_page();
    let base = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    let spec = LayoutSpec {
        // no cap: every row that can reach the margins must
        reflow: Some(ReflowSpec { zoom: 1.5, fill: Fill::Justified, max_stretch: 0.0, ..Default::default() }),
        ..base
    };
    let l = p.layout(&spec).clone();
    let flow = l.reflow.clone().unwrap();
    let q = p.quant();
    let rows = flow.row_band.len();
    for (r, words) in flow.row_words.iter().enumerate() {
        if words.len() < 2 || r + 1 == rows {
            continue;
        }
        let first = &p.data().words[words[0] as usize];
        let last = &p.data().words[words[words.len() - 1] as usize];
        let right = flow.word_place[words[0] as usize].apply(first.bbox.x1 as f32 / q, 0.0).0;
        let left = flow.word_place[words[words.len() - 1] as usize].apply(last.bbox.x0 as f32 / q, 0.0).0;
        assert!((right - flow.row_w).abs() < 0.01, "row {r} does not touch the right margin");
        assert!(left.abs() < 0.01, "row {r} does not reach the left margin");
    }
}

#[test]
fn reflow_justified_leaves_a_row_short_rather_than_gap_it_out() {
    let mut p = packed_page();
    let base = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    let spec = |max_stretch| LayoutSpec {
        reflow: Some(ReflowSpec { zoom: 1.5, fill: Fill::Justified, max_stretch, ..Default::default() }),
        ..base
    };
    let q = p.quant();
    let reach = |p: &mut Page, max_stretch: f32| -> Vec<f32> {
        let l = p.layout(&spec(max_stretch)).clone();
        let flow = l.reflow.clone().unwrap();
        flow.row_words
            .iter()
            .filter(|ws| ws.len() > 1)
            .map(|ws| {
                let last = &p.data().words[ws[ws.len() - 1] as usize];
                flow.word_place[ws[ws.len() - 1] as usize].apply(last.bbox.x0 as f32 / q, 0.0).0
            })
            .collect()
    };
    let uncapped = reach(&mut p, 0.0);
    // a cap of 1.0 forbids any stretch at all, so no row is pulled out to the left margin
    let capped = reach(&mut p, 1.0);
    assert_eq!(uncapped.len(), capped.len());
    assert!(
        capped.iter().zip(&uncapped).any(|(c, u)| c > u),
        "with gaps capped, at least one row should stay short instead of reaching the margin"
    );
}

#[test]
fn reflow_justified_centres_every_row_it_cannot_fill() {
    let mut p = packed_page();
    let base = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    // a cap of 1.0 forbids any stretch, so no row reaches both margins, the last one included
    let spec = LayoutSpec {
        reflow: Some(ReflowSpec { zoom: 1.5, fill: Fill::Justified, max_stretch: 1.0, ..Default::default() }),
        ..base
    };
    let l = p.layout(&spec).clone();
    let flow = l.reflow.clone().unwrap();
    let q = p.quant();
    let mut short = 0;
    for (r, words) in flow.row_words.iter().enumerate() {
        let (Some(&first), Some(&last)) = (words.first(), words.last()) else { continue };
        let (fw, lw) = (&p.data().words[first as usize], &p.data().words[last as usize]);
        let right = flow.row_w - flow.word_place[first as usize].apply(fw.bbox.x1 as f32 / q, 0.0).0;
        let left = flow.word_place[last as usize].apply(lw.bbox.x0 as f32 / q, 0.0).0;
        assert!((right - left).abs() < 0.01, "row {r} is not centred: {right} on the right, {left} on the left");
        if left > 0.01 {
            short += 1;
        }
    }
    assert!(short > 0, "the cap leaves at least one row short");
}

#[test]
fn reflow_at_zoom_1_gives_the_printed_page_back() {
    let mut p = packed_page();
    let base = LayoutSpec {
        viewport_w: 200.0,
        viewport_h: 400.0,
        pad_top: 12.0,
        pad_left: 8.0,
        pad_right: 8.0,
        ..Default::default()
    };
    let printed = p.layout(&base).clone();
    let boxes: Vec<(f32, f32, f32, f32)> = (0..p.data().words.len() as u32).map(|i| p.word_bounds_view(i)).collect();
    let l = p.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 1.0, ..Default::default() }), ..base }).clone();
    assert!(!l.is_reflowed(), "at the printed size the page is laid out as printed");
    // same scale, same page height, same word in the same pixel
    assert!((l.scale - printed.scale).abs() < 1e-4);
    assert!((l.content_h - printed.content_h).abs() < 1e-3, "{} vs {}", l.content_h, printed.content_h);
    for (i, want) in boxes.iter().enumerate() {
        let got = p.word_bounds_view(i as u32);
        assert!(
            (got.0 - want.0).abs() < 1e-3 && (got.1 - want.1).abs() < 1e-3,
            "word {i} moved: {:?} vs {:?}",
            got,
            want
        );
    }
    // and the leading still opens the lines up, as it does without reflow
    let spread = LayoutSpec { line_spacing: 1.5, ..base };
    let want = p.layout(&spread).clone();
    let got = p.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 1.0, ..Default::default() }), ..spread }).clone();
    assert_eq!(want.line_dy, got.line_dy, "line spacing must work at the printed size");
    assert!((want.content_h - got.content_h).abs() < 1e-3);
}

#[test]
fn reflow_centred_splits_what_is_left_over_between_the_margins() {
    let mut p = packed_page();
    let base = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    let q = p.quant();
    let edges = |p: &mut Page, fill: Fill| -> Vec<(f32, f32)> {
        let l = p
            .layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 1.5, fill, ..Default::default() }), ..base })
            .clone();
        let flow = l.reflow.clone().unwrap();
        flow.row_words
            .iter()
            .filter(|ws| !ws.is_empty())
            .map(|ws| {
                let (first, last) = (&p.data().words[ws[0] as usize], &p.data().words[ws[ws.len() - 1] as usize]);
                (
                    flow.row_w - flow.word_place[ws[0] as usize].apply(first.bbox.x1 as f32 / q, 0.0).0,
                    flow.word_place[ws[ws.len() - 1] as usize].apply(last.bbox.x0 as f32 / q, 0.0).0,
                )
            })
            .collect()
    };
    let ragged = edges(&mut p, Fill::Ragged);
    let centred = edges(&mut p, Fill::Centred);
    assert_eq!(ragged.len(), centred.len());
    for (r, c) in ragged.iter().zip(&centred) {
        // ragged rows start at the right margin; centred rows share the leftover evenly
        assert!(r.0.abs() < 0.01, "a ragged row starts at the right margin");
        assert!((c.0 - c.1).abs() < 0.01, "a centred row has equal margins: {} and {}", c.0, c.1);
    }
    // a row that fills its width is left where it is
    assert!(centred.iter().any(|(l, _)| *l > 0.01), "some row should have room to centre");
}

#[test]
fn reflow_even_gaps_keep_the_letters_the_same_distance_apart() {
    let mut p = packed_page();
    let base = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    // nothing relaxed: this is about the gap the engine picks, not about evening the rows
    let spread = |p: &mut Page, gaps: GapMode| -> f32 {
        let spec = ReflowSpec { zoom: 1.5, gaps, relax: 0.0, ..Default::default() };
        let l = p.layout(&LayoutSpec { reflow: Some(spec), ..base }).clone();
        let flow = l.reflow.clone().unwrap();
        let mut v: Vec<f32> = Vec::new();
        for ws in &flow.row_words {
            for pair in ws.windows(2) {
                // the distance between the letters of two words, as placed
                let a = flow.word_place[pair[0] as usize].apply(p.word_body(pair[0]).0, 0.0).0;
                let b = flow.word_place[pair[1] as usize].apply(p.word_body(pair[1]).1, 0.0).0;
                v.push(a - b);
            }
        }
        let mean = v.iter().sum::<f32>() / v.len() as f32;
        (v.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / v.len() as f32).sqrt()
    };
    let even = spread(&mut p, GapMode::Uniform);
    assert!(even < 0.01, "even gaps should not vary: spread {even}");
    // and the engine asks for them by default
    assert_eq!(ReflowSpec::default().gaps, GapMode::Uniform);
}

#[test]
fn words_are_spaced_by_the_air_between_their_strokes() {
    let p = packed_page();
    // the test page sets its words apart, so none of them is drawn inside another
    for w in 0..p.data().words.len() as u32 - 1 {
        assert_eq!(p.words_interlock(w, w + 1), None, "word {w} should not be drawn inside the next");
    }
    // its words are plain squares 16 wide with 4 between them, so the air is that 4
    let air = p.words_clearance(1, 2, 0.0).expect("the two share a band");
    assert!((air - 4.0).abs() < 0.01, "air {air}");
    // and the shift that would leave 10 units of air moves the second word by the difference
    let shift = p.shift_for_clearance(1, 2, 10.0).unwrap();
    assert!((shift + 6.0).abs() < 0.01, "shift {shift}");
    assert!((p.words_clearance(1, 2, shift).unwrap() - 10.0).abs() < 0.01);
}
