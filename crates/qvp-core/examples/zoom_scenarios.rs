//! Write the cross-wrapper zoom scenarios: the engine's answers for the reader's pan and zoom on
//! page 042 in one fixed viewport, which every wrapper's test suite replays and compares.
//!
//! The sequence is the one a host runs: lay the page out, read its fit, zoom about a point
//! under a floor and a ceiling, drag, hold the view against the viewport, turn three touches
//! into layout points, ask whether the reader has zoomed in, then pinch once under the stepped
//! control and once under the continuous one and read the spec each pinch asks for. Then the
//! spec the control asks for on the printed page, and three pinches whose fingers drift over
//! five frames: out under the stepped control and under the magnifying glass, and out one step
//! and back under the stepped control. The pure view
//! operations run on the fitted view, so each case stands on its own; the pinches start from
//! the printed layout, which the replay lays out again before each one.
//!
//! Run: cargo run -p qvp-core --release --example zoom_scenarios -- dist/pages/042.qvp conformance/scenarios/zoom.json
use qvp_core::*;

fn view(v: View) -> String {
    format!("{{\"scale\": {}, \"offsetX\": {}, \"offsetY\": {}}}", v.scale, v.offset_x, v.offset_y)
}

fn zoom(z: Zoom) -> String {
    let mode = match z.mode {
        ZoomMode::Stepped => "stepped",
        ZoomMode::Continuous => "continuous",
        ZoomMode::Magnify => "magnify",
    };
    format!("{{\"mode\": \"{}\", \"step\": {}, \"zoom\": {}}}", mode, z.step, z.zoom)
}

fn fill_name(f: Fill) -> &'static str {
    match f {
        Fill::Ragged => "ragged",
        Fill::Justified => "justified",
        Fill::Centred => "centred",
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let page_path = args.get(1).map(String::as_str).unwrap_or("dist/pages/042.qvp");
    let out_path = args.get(2).map(String::as_str).unwrap_or("conformance/scenarios/zoom.json");
    let bytes = std::fs::read(page_path).expect("read page");
    let mut page = Page::load(&bytes).expect("load page");

    let (viewport_w, viewport_h) = (390.0f32, 844.0f32);
    // The host's reflow knobs ride on the spec with zoom 0: the printed page, which the zoom
    // control leaves with the knobs in force. The replay checks that `zoomSpec` hands the whole
    // spec back, knobs included, and not the zoom alone.
    let knobs =
        ReflowSpec { zoom: 0.0, fill: Fill::Justified, word_gap: 1.5, max_stretch: 3.0, ..ReflowSpec::default() };
    let spec = LayoutSpec {
        viewport_w,
        viewport_h,
        pad_top: 24.0,
        pad_bottom: 24.0,
        pad_left: 16.0,
        pad_right: 16.0,
        line_spacing: 1.0,
        fill_height: false,
        grid_lines: 0,
        crop_left: 0.0,
        crop_right: 0.0,
        max_aspect_slack: 0.0,
        reflow: Some(knobs),
        banner_zoom: 0.0,
        surah_frames: true,
    };
    let l = page.layout(&spec).clone();
    let fitted = View { scale: l.fit_scale, offset_x: l.fit_x, offset_y: l.fit_y };
    let fit = format!(
        "{{\"fitScale\": {}, \"fitX\": {}, \"fitY\": {}, \"contentW\": {}, \"contentH\": {}}}",
        l.fit_scale, l.fit_x, l.fit_y, l.content_w, l.content_h
    );

    // zoom about a point: a plain pinch in, one the ceiling cuts, one the floor holds, and one
    // whose ceiling sits under the floor
    let mut zoom_about = Vec::new();
    for (fx, fy, factor, min, max) in [
        (195.0f32, 300.0f32, 2.5f32, l.fit_scale, 0.0f32),
        (120.0, 700.0, 40.0, l.fit_scale, 3.0),
        (195.0, 422.0, 0.3, l.fit_scale, 0.0),
        (195.0, 422.0, 4.0, l.fit_scale, 0.1),
    ] {
        let out = fitted.zoom_about(fx, fy, factor, min, max);
        zoom_about.push(format!(
            "    {{\"view\": {}, \"focalX\": {}, \"focalY\": {}, \"factor\": {}, \"min\": {}, \"max\": {}, \"result\": {}}}",
            view(fitted),
            fx,
            fy,
            factor,
            min,
            max,
            view(out)
        ));
    }
    let zoomed = fitted.zoom_about(195.0, 300.0, 2.5, l.fit_scale, 0.0);

    // a drag of the zoomed view, then the clamp that holds it against the viewport
    let panned = zoomed.pan(-80.0, -120.0);
    let pan = format!("{{\"view\": {}, \"dx\": -80, \"dy\": -120, \"result\": {}}}", view(zoomed), view(panned));
    let far = View { scale: zoomed.scale, offset_x: 1e5, offset_y: -1e5 };
    let mut clamps = Vec::new();
    for v in [panned, far, fitted] {
        let out = v.clamp(l.content_w, l.content_h, viewport_w, viewport_h);
        clamps.push(format!("    {{\"view\": {}, \"result\": {}}}", view(v), view(out)));
    }
    let held = panned.clamp(l.content_w, l.content_h, viewport_w, viewport_h);

    // three touches on the held view, as layout points
    let mut points = Vec::new();
    for (vx, vy) in [(0.0f32, 0.0f32), (195.0, 422.0), (390.0, 844.0)] {
        let (x, y) = page.view_to_layout(held, vx, vy);
        points.push(format!("[{}, {}, {}, {}]", vx, vy, x, y));
    }
    let view_to_layout = format!("{{\"view\": {}, \"points\": [{}]}}", view(held), points.join(", "));

    // zoomed in, by either road: the fitted view is not, the magnified one is
    let mut is_zoomed = Vec::new();
    for (z, v) in [(Zoom::default(), fitted), (Zoom::default(), held)] {
        is_zoomed.push(format!(
            "    {{\"zoom\": {}, \"view\": {}, \"fitScale\": {}, \"result\": {}}}",
            zoom(z),
            view(v),
            l.fit_scale,
            z.is_zoomed(v, l.fit_scale)
        ));
    }

    // one pinch under each reflowing control, from the printed layout, and the spec it asks for
    let mut pinches = Vec::new();
    let focal = (195.0f32, 422.0f32);
    let continuous = page.zoom_mode(&spec, Zoom::default(), ZoomMode::Continuous);
    for (name, start, factor) in [("stepped", Zoom::default(), 1.6f32), ("continuous", continuous, 1.37)] {
        page.layout(&spec);
        let c = page.zoom_pinch(&spec, start, fitted, factor, focal);
        let s = page.zoom_spec(&spec, c.zoom);
        let r = s.reflow.expect("a pinch past the printed page asks for a reflow");
        let fill = fill_name(r.fill);
        pinches.push(format!(
            concat!(
                "    {{\"mode\": \"{}\", \"start\": {}, \"factor\": {}, \"focalX\": {}, \"focalY\": {},\n",
                "     \"zoom\": {}, \"view\": {}, \"relaid\": {}, \"zoomedAfter\": {},\n",
                "     \"spec\": {{\"viewportW\": {}, \"padTop\": {}, \"surahFrames\": {}, \"reflowZoom\": {}, ",
                "\"fill\": \"{}\", \"wordGap\": {}, \"maxStretch\": {}}}}}"
            ),
            name,
            zoom(start),
            factor,
            focal.0,
            focal.1,
            zoom(c.zoom),
            view(c.view),
            c.relaid,
            c.zoom.is_zoomed(c.view, l.fit_scale),
            s.viewport_w,
            s.pad_top,
            s.surah_frames,
            r.zoom,
            fill,
            r.word_gap,
            r.max_stretch
        ));
    }

    // The spec the control asks for on the printed page: the engine reflows nothing there, and
    // the host's knobs come back as it set them with the zoom at 0, as `qvp_zoom_spec` writes
    // them. A wrapper that rebuilt the knobs from its defaults would fail this case.
    let printed = page.zoom_spec(&spec, Zoom::default());
    let knobs_out = printed.reflow.unwrap_or(ReflowSpec { zoom: 0.0, ..knobs });
    let printed_spec = format!(
        concat!(
            "{{\"zoom\": {}, \"spec\": {{\"viewportW\": {}, \"padTop\": {}, \"surahFrames\": {}, \"reflowZoom\": {}, ",
            "\"fill\": \"{}\", \"wordGap\": {}, \"maxStretch\": {}}}}}"
        ),
        zoom(Zoom::default()),
        printed.viewport_w,
        printed.pad_top,
        printed.surah_frames,
        knobs_out.zoom,
        fill_name(knobs_out.fill),
        knobs_out.word_gap,
        knobs_out.max_stretch
    );

    // A pinch whose fingers drift, frame by frame, under the rule every page view follows: the
    // engine works from where the gesture began (the control, the focal point, the factor of the
    // whole gesture), and the host pans the answer by the focal point's drift since then and
    // holds it against the viewport. Under the magnifying glass the engine zooms the view the
    // gesture began on. Under the reflowing controls it takes the view its last frame returned,
    // before the drift, because a step committed earlier in the gesture moved the page.
    let mut drifts = Vec::new();
    let start_focal = (195.0f32, 422.0f32);
    let outward: [(f32, f32, f32); 5] =
        [(1.1, 200.0, 410.0), (1.4, 210.0, 380.0), (1.7, 225.0, 350.0), (2.2, 240.0, 320.0), (2.6, 250.0, 300.0)];
    // out one step and back to where the fingers went down: the last frame lays the printed
    // page out again
    let and_back: [(f32, f32, f32); 5] =
        [(1.2, 200.0, 410.0), (1.5, 205.0, 400.0), (1.3, 210.0, 390.0), (1.1, 215.0, 380.0), (1.0, 220.0, 370.0)];
    let magnify = Zoom { mode: ZoomMode::Magnify, step: 0, zoom: 1.0 };
    for (name, start, frames) in
        [("stepped", Zoom::default(), outward), ("magnify", magnify, outward), ("stepped", Zoom::default(), and_back)]
    {
        page.layout(&spec);
        let mut base = fitted;
        let mut out = Vec::new();
        for (factor, fx, fy) in frames {
            let from = if start.mode == ZoomMode::Magnify { fitted } else { base };
            let c = page.zoom_pinch(&spec, start, from, factor, start_focal);
            if start.mode != ZoomMode::Magnify {
                base = c.view;
            }
            let drawn = page.dress_clamp(c.view.pan(fx - start_focal.0, fy - start_focal.1), viewport_w, viewport_h);
            out.push(format!(
                "      {{\"factor\": {}, \"focalX\": {}, \"focalY\": {}, \"zoom\": {}, \"relaid\": {}, \"view\": {}}}",
                factor,
                fx,
                fy,
                zoom(c.zoom),
                c.relaid,
                view(drawn)
            ));
        }
        drifts.push(format!(
            "    {{\"mode\": \"{}\", \"start\": {}, \"view\": {}, \"focalX\": {}, \"focalY\": {}, \"frames\": [\n{}\n    ]}}",
            name,
            zoom(start),
            view(fitted),
            start_focal.0,
            start_focal.1,
            out.join(",\n")
        ));
    }

    let json = format!(
        concat!(
            "{{\n  \"page\": \"042.qvp\",\n  \"tolerance\": 0.001,\n",
            "  \"spec\": {{\"viewportW\": {}, \"viewportH\": {}, \"padTop\": 24, \"padBottom\": 24, \"padLeft\": 16, \"padRight\": 16, ",
            "\"lineSpacing\": 1, \"fillHeight\": false, \"gridLines\": 0, \"cropLeft\": 0, \"cropRight\": 0, \"maxAspectSlack\": 0, ",
            "\"surahFrames\": true, \"reflow\": {{\"zoom\": 0, \"fill\": \"justified\", \"wordGap\": 1.5, \"maxStretch\": 3}}}},\n",
            "  \"fit\": {},\n",
            "  \"zoomAbout\": [\n{}\n  ],\n",
            "  \"pan\": {},\n",
            "  \"clamp\": [\n{}\n  ],\n",
            "  \"viewToLayout\": {},\n",
            "  \"isZoomed\": [\n{}\n  ],\n",
            "  \"pinches\": [\n{}\n  ],\n",
            "  \"printedSpec\": {},\n",
            "  \"drifts\": [\n{}\n  ]\n}}\n"
        ),
        viewport_w,
        viewport_h,
        fit,
        zoom_about.join(",\n"),
        pan,
        clamps.join(",\n"),
        view_to_layout,
        is_zoomed.join(",\n"),
        pinches.join(",\n"),
        printed_spec,
        drifts.join(",\n")
    );
    std::fs::create_dir_all(std::path::Path::new(out_path).parent().unwrap()).expect("mkdir");
    std::fs::write(out_path, json).expect("write scenarios");
    println!(
        "{out_path}: {} zoom cases, {} pinches, {} drifting pinches",
        zoom_about.len() + clamps.len() + 5,
        pinches.len(),
        drifts.len()
    );
}
