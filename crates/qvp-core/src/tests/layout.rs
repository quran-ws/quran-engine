//! The printed layout: fill height, crop, aspect bound and surah headers.
use super::*;

#[test]
fn layout_fill_height_and_view_hit() {
    let mut p = page();
    let spec = LayoutSpec {
        viewport_w: 200.0,
        viewport_h: 2500.0,
        pad_top: 50.0,
        pad_bottom: 50.0,
        fill_height: true,
        ..Default::default()
    };
    let l = p.layout(&spec).clone();
    assert_eq!(l.scale, 2.0);
    // a short page (2 lines) takes the rows of the 15-line grid: 2400 px / 15 = 160 px
    // = 80 units a row, so 40 units of leading on the printed line spacing of 40
    let delta = 40.0;
    assert!((l.line_spacing - (40.0 + delta)).abs() < 1e-3, "line spacing {}", l.line_spacing);
    assert_eq!(l.line_dy.len(), 2);
    // printed geometry kept: the two lines differ by exactly one delta; the grid (15 rows
    // = 1200 units) is centred in the padded viewport, the page centred on it (slot0 = 6.5)
    assert!((l.line_dy[1] - l.line_dy[0] - delta).abs() < 1e-3);
    assert!((l.line_dy[0] - (25.0 + (1200.0 - 100.0) / 2.0 - 7.0 * delta + 6.5 * delta)).abs() < 1e-3);
    assert!((l.content_h - 2500.0).abs() < 1e-3);
    // slot boundary halfway between the laid-out line centres
    let mid = ((15.0 + l.line_dy[0]) + (55.0 + l.line_dy[1])) / 2.0 * 2.0;
    assert!((l.line_slots[0].1 - mid).abs() < 1e-3 && (l.line_slots[1].0 - mid).abs() < 1e-3);
    let vx = l.offset_x + 15.0 * 2.0;
    let vy = l.offset_y + (15.0 + l.line_dy[0]) * 2.0;
    assert_eq!(p.hit_test_exact_view(vx, vy), Some(HitExact { word: 0, path: 0, decoration: NONE }));
    assert_eq!(p.hit_test_exact_view(vx, 5.0), None);
    let g = p.hit_test_view(l.offset_x + 25.0 * 2.0, vy, &HitOptions::default()).unwrap();
    assert_eq!((g.word, g.is_exact), (1, false));
    // rows shorter than the printed line spacing: the print is the floor and the grid outgrows the viewport
    let tight = p.layout(&LayoutSpec { viewport_h: 1100.0, ..spec }).clone();
    assert!((tight.line_spacing - 40.0).abs() < 1e-3 && (tight.content_h - (100.0 + 15.0 * 40.0 * 2.0)).abs() < 1e-3);
    // a full page (the grid is its own line count) spreads its printed height: (1200 − 100) units
    // over its one gap — and never squeezes below the print
    let full = p.layout(&LayoutSpec { grid_lines: 2, ..spec }).clone();
    assert!((full.line_spacing - (40.0 + 1100.0)).abs() < 1e-3 && (full.content_h - 2500.0).abs() < 1e-3);
    let squeezed =
        p.layout(&LayoutSpec { viewport_h: 150.0, pad_top: 0.0, pad_bottom: 0.0, grid_lines: 2, ..spec }).clone();
    assert!((squeezed.line_spacing - 40.0).abs() < 1e-3 && (squeezed.content_h - 200.0).abs() < 1e-3);
    let l2 = p
        .layout(&LayoutSpec { viewport_w: 200.0, viewport_h: 1100.0, line_spacing: 1.5, ..Default::default() })
        .clone();
    assert!((l2.line_spacing - p.line_spacing() * 1.5).abs() < 1e-3);
    assert!((l2.line_dy[1] - l2.line_dy[0] - 20.0).abs() < 1e-3, "×1.5 adds half a line spacing between lines");
    // as printed: no shift at all
    let l3 = p.layout(&LayoutSpec { viewport_w: 200.0, viewport_h: 1100.0, ..Default::default() }).clone();
    assert!(l3.line_dy.iter().all(|d| d.abs() < 1e-6) && (l3.content_h - 200.0).abs() < 1e-3);
    // spacing only ever opens up: below 1.0, a negative gap, and a fill that would
    // need to tighten all reproduce the print
    for spec in [
        LayoutSpec { viewport_w: 200.0, viewport_h: 1100.0, line_spacing: 0.5, ..Default::default() },
        LayoutSpec { viewport_w: 200.0, viewport_h: 60.0, fill_height: true, ..Default::default() },
    ] {
        let l = p.layout(&spec).clone();
        assert!((l.line_spacing - p.line_spacing()).abs() < 1e-3, "line spacing {} for {:?}", l.line_spacing, spec);
        assert!(l.line_dy[1] - l.line_dy[0] >= -1e-6, "lines never move closer");
    }
    // the multiplier that fills a viewport IS the spacing fill-height lays the page out at —
    // the crop, the aspect bound and the grid a short page sits on all included, because a host
    // that sizes its own furniture to a printed row reads it instead of laying the page out
    let tall = LayoutSpec { viewport_w: 390.0, viewport_h: 844.0, ..Default::default() };
    let grid2 = LayoutSpec { grid_lines: 2, ..tall };
    for spec in [
        tall,
        grid2,
        LayoutSpec { crop_left: 20.0, crop_right: 30.0, ..grid2 },
        LayoutSpec { max_aspect_slack: 1.15, ..grid2 },
    ] {
        let m = p.line_spacing_to_fill(&spec, f32::INFINITY);
        let filled = p.layout(&LayoutSpec { fill_height: true, ..spec }).line_spacing;
        assert!((filled - p.line_spacing() * m).abs() < 1e-3, "multiplier {m}, laid out at {filled}, for {spec:?}");
    }
    // a crop draws the page bigger, so less paper is left over and it takes less leading to fill
    assert!(
        p.line_spacing_to_fill(&LayoutSpec { crop_left: 20.0, crop_right: 30.0, ..grid2 }, f32::INFINITY)
            < p.line_spacing_to_fill(&grid2, f32::INFINITY)
    );
    assert_eq!(
        p.line_spacing_to_fill(&LayoutSpec { viewport_w: 820.0, viewport_h: 300.0, ..Default::default() }, 100.0),
        1.0
    );
    assert!(p.wasted_fraction(&tall) > 0.0 && p.wasted_fraction(&tall) < 1.0);
    // printed_height is content_h before any leading: exactly the flat layout's, never more
    // than the filled one's, and the filled one's again where the box is too short to fill
    let flat = LayoutSpec { fill_height: false, line_spacing: 1.0, ..tall };
    assert!((p.layout(&flat).content_h - p.printed_height(&flat)).abs() < 1e-3);
    let filled = LayoutSpec { fill_height: true, ..tall };
    assert!(p.layout(&filled).content_h >= p.printed_height(&filled) - 1e-3);
    let short = LayoutSpec { viewport_h: 200.0, fill_height: true, ..tall };
    assert!((p.layout(&short).content_h - p.printed_height(&short)).abs() < 1e-3);
    // a crop draws the page bigger, so it is taller for the same width
    assert!(p.printed_height(&LayoutSpec { crop_left: 20.0, crop_right: 30.0, ..grid2 }) > p.printed_height(&grid2));
    let wb = p.word_bounds_view(2);
    assert!(wb.1 > 0.0);
}

#[test]
fn layout_fit_crop_and_aspect_bound() {
    let mut p = page();
    // as printed the content is 200 px tall at scale 2; a 150 px viewport shrinks and centres it
    let l = p.layout(&LayoutSpec { viewport_w: 200.0, viewport_h: 150.0, ..Default::default() }).clone();
    assert!((l.fit_scale - 0.75).abs() < 1e-6 && (l.fit_x - 25.0).abs() < 1e-6 && l.fit_y.abs() < 1e-6, "{l:?}");
    // a tall viewport never enlarges: scale 1, centred vertically
    let l = p.layout(&LayoutSpec { viewport_w: 200.0, viewport_h: 1100.0, ..Default::default() }).clone();
    assert!((l.fit_scale - 1.0).abs() < 1e-6 && l.fit_x.abs() < 1e-6 && (l.fit_y - 450.0).abs() < 1e-6);
    // the aspect bound stops a wide viewport from stretching the lines: 100 px tall, 1:1 page,
    // slack 1.15 -> the content is 115 px wide and sits in the middle of the 1000 px viewport
    let l = p
        .layout(&LayoutSpec { viewport_w: 1000.0, viewport_h: 100.0, max_aspect_slack: 1.15, ..Default::default() })
        .clone();
    assert!((l.content_w - 115.0).abs() < 1e-3 && (l.scale - 1.15).abs() < 1e-6, "{l:?}");
    // 115 px of content in a 100 px viewport: the fit shrinks it and centres what remains
    let fit = 100.0 / 115.0;
    assert!((l.fit_scale - fit).abs() < 1e-6 && (l.fit_x - (1000.0 - 115.0 * fit) / 2.0).abs() < 1e-3, "{l:?}");
    // cropping 10 units off each printed margin: 80 units fill the 200 px width, the origin
    // moves left by the cropped margin
    let l = p
        .layout(&LayoutSpec {
            viewport_w: 200.0,
            viewport_h: 1100.0,
            crop_left: 10.0,
            crop_right: 10.0,
            ..Default::default()
        })
        .clone();
    assert!((l.scale - 2.5).abs() < 1e-6 && (l.offset_x + 25.0).abs() < 1e-6, "{l:?}");
}

#[test]
fn a_surah_header_reports_a_frame_box_and_the_title_ink_without_the_frame() {
    let (mut framed, Some(frame_path), title_path, _) = page_with_surah_frame(true) else { unreachable!() };
    let data = framed.data().clone();
    let title = data.paths[title_path as usize].bbox;
    let q = framed.quant();

    let headers = framed.surah_headers();
    assert_eq!(headers.len(), 1);
    let h = headers[0];
    assert_eq!(h.surah, 1);
    // the title box is the title's own ink: the frame is left out even though it is drawn
    assert!((h.title_x0 - title.x0 as f32 / q).abs() < 1e-4);
    assert!((h.title_x1 - title.x1 as f32 / q).abs() < 1e-4);
    // the frame box spans the page's text block and holds the title
    assert!(h.x0 <= h.title_x0 && h.x1 >= h.title_x1);
    assert!(h.y0 < h.y1);
    let frame = data.paths[frame_path as usize].bbox;
    assert!(h.title_x1 - h.title_x0 < (frame.x1 - frame.x0) as f32 / q);

    // hiding the frame changes neither box
    let printed = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    framed.layout(&printed);
    let with_frame = framed.surah_headers();
    framed.layout(&LayoutSpec { surah_frames: false, ..printed });
    assert_eq!(framed.surah_headers(), with_frame);

    // the view boxes follow the layout: same order, and inside the laid-out content
    let l = framed.layout(&printed).clone();
    let view = framed.surah_headers_view();
    assert_eq!(view.len(), 1);
    assert!(view[0].title_y0 >= 0.0 && view[0].title_y1 <= l.content_h);
    let placed = placed_point(&framed, &l, title_path, title.x0 as f32 / q, title.y0 as f32 / q);
    assert!((view[0].title_y0 - placed.1).abs() < 1e-3, "{} vs {}", view[0].title_y0, placed.1);

    // On a reflowed page the frame box is the row: the printed block scaled by the banner's
    // growth was wider than the row, and so wider than the screen, as soon as the reader
    // zoomed in.
    let zoomed =
        framed.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }), ..printed }).clone();
    let flow = zoomed.reflow.as_ref().unwrap();
    let view = framed.surah_headers_view();
    assert_eq!(view.len(), 1);
    assert!((view[0].x0 - zoomed.offset_x).abs() < 1e-3, "{} vs {}", view[0].x0, zoomed.offset_x);
    assert!((view[0].x1 - (zoomed.offset_x + flow.row_w * zoomed.scale)).abs() < 1e-3);
    assert!(view[0].x1 <= zoomed.offset_x + zoomed.content_w + 1e-3);
    assert!(view[0].x0 <= view[0].title_x0 && view[0].title_x1 <= view[0].x1, "the frame holds the title");
}

#[test]
fn a_surah_name_of_ornaments_alone_keeps_the_decorations_box() {
    let (framed, Some(frame_path), _, decoration) = page_with_surah_frame(true) else { unreachable!() };
    let mut data = framed.data().clone();
    let deco = &mut data.decorations[decoration as usize];
    deco.first_path = frame_path;
    deco.n_paths = 1;
    deco.bbox = data.paths[frame_path as usize].bbox;
    let page = Page::from_data(data).unwrap();
    let q = page.quant();
    let frame = page.data().paths[frame_path as usize].bbox;
    let headers = page.surah_headers();
    assert_eq!(headers.len(), 1);
    let h = headers[0];
    assert!(h.title_x0 <= h.title_x1 && h.title_y0 <= h.title_y1, "a box, not the empty sentinel: {h:?}");
    assert!((h.title_x0 - frame.x0 as f32 / q).abs() < 1e-4);
    assert!((h.title_x1 - frame.x1 as f32 / q).abs() < 1e-4);
    assert!(h.y0 < h.y1 && h.y0.is_finite() && h.y1.is_finite());
}
