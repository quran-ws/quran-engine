//! Write the cross-wrapper ink band scenarios: the band of page 042 a host keeps drawn, for
//! views and viewports that cover each branch: a page that fits, a magnified page drawn a band
//! at a time, pans inside one step and across it, a page below the top of the screen, a host's
//! own height and pixel limits, a budget under one screen, a budget exactly at the whole page,
//! a screen taller than the ceiling, and no screen at all. A non-finite offset cannot be
//! written in JSON, so the core's unit tests hold that case. Every wrapper's test suite replays
//! them through its ink band call and compares the top and the height.
//!
//! The `dressed` block is the same page dressed in the synthetic ornament set of the dress
//! scenarios, whose border stands over the content's top and bottom. Its cases cover each
//! branch of a dressed band: a page that fits with its border, a ceiling and a budget that
//! the bare page fits but the border does not, which step in whole pixels, a magnified page
//! with its border above the screen, at the screen's top, in the middle and at the bottom, a
//! pan across a step, a budget
//! under one screen, a screen taller than the ceiling, one taller than a ceiling the bare page
//! fits, with no room, some room and room for the whole dress box above it, a band that
//! follows the reader one row over a budget of one screen, one with no room for that row, and
//! no screen.
//!
//! Run: cargo run -p qvp-core --release --example ink_band_scenarios -- dist/pages/042.qvp \
//!      conformance/scenarios/ink-band.json conformance/qvo1-dress.hex
use qvp_core::qvp_format::ornaments::OrnamentSet;
use qvp_core::*;
use std::sync::Arc;

/// One case: a name, the view's scale and vertical offset, the viewport, and the two limits.
type Case = (&'static str, f32, f32, f32, f32, f32, f32);

const CASES: [Case; 12] = [
    ("the page fits", 1.0, 0.0, 390.0, 844.0, 0.0, 0.0),
    ("magnified, a band", 8.0, -2000.0, 390.0, 844.0, 0.0, 0.0),
    ("a pan inside the step", 8.0, -2100.0, 390.0, 844.0, 0.0, 0.0),
    ("a pan across the step", 8.0, -2150.0, 390.0, 844.0, 0.0, 0.0),
    ("the host's own height", 2.0, -300.0, 390.0, 844.0, 1000.0, 0.0),
    ("the page below the top of the screen", 8.0, 300.0, 390.0, 844.0, 0.0, 0.0),
    ("a budget under one screen", 8.0, -2000.0, 390.0, 844.0, 0.0, 390.0 * 300.0),
    ("the host's pixel budget", 3.0, -600.0, 390.0, 844.0, 0.0, 390.0 * 1500.0),
    ("a budget the page fits", 2.0, -600.0, 390.0, 844.0, 0.0, 390.0 * 4096.0),
    ("a screen taller than the ceiling", 8.0, -1000.0, 390.0, 5000.0, 0.0, 0.0),
    ("no viewport height", 8.0, -2000.0, 390.0, 0.0, 0.0, 0.0),
    ("no scale", 0.0, -2000.0, 390.0, 844.0, 0.0, 0.0),
];

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let page_path = args.get(1).map(String::as_str).unwrap_or("dist/pages/042.qvp");
    let out_path = args.get(2).map(String::as_str).unwrap_or("conformance/scenarios/ink-band.json");
    let hex_path = args.get(3).map(String::as_str).unwrap_or("conformance/qvo1-dress.hex");
    let bytes = std::fs::read(page_path).expect("read page");
    let mut page = Page::load(&bytes).expect("load page");
    let spec = LayoutSpec {
        viewport_w: 390.0,
        viewport_h: 844.0,
        pad_top: 24.0,
        pad_bottom: 24.0,
        pad_left: 16.0,
        pad_right: 16.0,
        ..Default::default()
    };
    page.layout(&spec);
    let content_h = page.current_layout().expect("laid out").content_h;

    // A budget exactly at the whole page at a view scale of 2: the smallest budget that keeps
    // the page whole, and one row of pixels under it, which bands it.
    let (edge_scale, edge_w) = (2.0f32, 390.0f32);
    let at = |max_pixels: f32| {
        page.layout_ink_band(
            View { scale: edge_scale, offset_x: 0.0, offset_y: -600.0 },
            edge_w,
            844.0,
            0.0,
            max_pixels,
        )
    };
    let mut edge = edge_w * content_h * edge_scale;
    while at(edge).height < content_h {
        edge = edge.next_up();
    }
    while at(edge.next_down()).height >= content_h {
        edge = edge.next_down();
    }
    assert!(at(edge - edge_w).height < content_h, "a row under the edge bands the page");
    let edges: [Case; 2] = [
        ("a budget exactly at the whole page", edge_scale, -600.0, edge_w, 844.0, 0.0, edge),
        ("a budget a row under the whole page", edge_scale, -600.0, edge_w, 844.0, 0.0, edge - edge_w),
    ];

    let write = |page: &Page, cases: &[Case], indent: &str| -> Vec<String> {
        cases
            .iter()
            .map(|&(name, scale, offset_y, viewport_w, viewport_h, max_height, max_pixels)| {
                let view = View { scale, offset_x: 0.0, offset_y };
                let b = page.layout_ink_band(view, viewport_w, viewport_h, max_height, max_pixels);
                format!(
                    concat!(
                        "{i}  {{\"name\": \"{}\", \"view\": {{\"scale\": {}, \"offsetX\": 0, \"offsetY\": {}}},\n",
                        "{i}   \"viewportW\": {}, \"viewportH\": {}, \"maxHeight\": {}, \"maxPixels\": {},\n",
                        "{i}   \"band\": {{\"top\": {}, \"height\": {}}}}}"
                    ),
                    name,
                    scale,
                    offset_y,
                    viewport_w,
                    viewport_h,
                    max_height,
                    max_pixels,
                    b.top,
                    b.height,
                    i = indent
                )
            })
            .collect()
    };
    let all: Vec<Case> = CASES.iter().chain(edges.iter()).copied().collect();
    let cases = write(&page, &all, "  ");

    // The same page in the dress scenarios' synthetic set. The ceiling and the budget the
    // bare page fits at a view scale of 2 sit between the bare page and the dressed box.
    let hex: String = std::fs::read_to_string(hex_path).expect("read the ornament set").split_whitespace().collect();
    let qvo: Vec<u8> =
        (0..hex.len()).step_by(2).map(|i| u8::from_str_radix(&hex[i..i + 2], 16).expect("hex")).collect();
    let set = Arc::new(OrnamentSet::decode(&qvo).expect("decode the ornament set"));
    let dress = DressSpec {
        ayah_mark: 0,
        surah_header: 1,
        page_frame: 2,
        gap: 5.0,
        band_gap: 5.0,
        ayah_mark_scale: 1.0,
        page_frame_scale: 1.0,
        line_art: 0,
        colors: vec![],
    };
    page.dress(&set, &dress).expect("the fixture dresses page 042");
    let dressed_h = page.current_layout().expect("laid out").content_h;
    assert_eq!(dressed_h, content_h, "a dress does not change the printed content");
    let (ol, ot, or, ob) = page.dress_overflow();
    let bare = content_h * 2.0;
    // the dressed box at a view scale of 8, in viewport px, and its overflow above
    let (above8, box8) = ((ot * 8.0).ceil(), content_h * 8.0 + (ot * 8.0).ceil() + (ob * 8.0).ceil());
    let dressed_cases: [Case; 17] = [
        ("the dressed page fits", 1.0, 0.0, 390.0, 844.0, 0.0, 0.0),
        ("a ceiling the bare page fits, an odd slack", 2.0, -500.0, 390.0, 401.0, bare, 0.0),
        ("a budget the bare page fits", 2.0, -300.0, 390.0, 400.0, 0.0, 390.0 * (bare + 0.5)),
        ("magnified, the page at the top of the screen", 8.0, 0.0, 390.0, 844.0, 0.0, 0.0),
        ("magnified, the border at the top of the screen", 8.0, above8, 390.0, 844.0, 0.0, 0.0),
        ("magnified, the middle of the page", 8.0, -2000.0, 390.0, 844.0, 0.0, 0.0),
        ("magnified, a pan across the step", 8.0, -2150.0, 390.0, 844.0, 0.0, 0.0),
        ("magnified, the border at the bottom of the screen", 8.0, 844.0 - box8 + above8, 390.0, 844.0, 0.0, 0.0),
        ("a budget under one screen", 8.0, -2000.0, 390.0, 844.0, 0.0, 390.0 * 300.0),
        ("a screen taller than the ceiling", 8.0, -1000.0, 390.0, 5000.0, 0.0, 0.0),
        ("a screen taller than a ceiling the bare page fits", 2.0, -300.0, 390.0, 5000.0, bare, 0.0),
        ("a screen taller than a ceiling with room for some border", 2.0, -300.0, 390.0, 5000.0, bare + 22.5, 0.0),
        ("a screen taller than a ceiling with room for the box", 2.0, -300.0, 390.0, 5000.0, bare + 40.0, 0.0),
        ("a budget of one screen the bare page fits, a row over", 1.35, -100.6, 390.0, 844.0, 0.0, 390.0 * 844.0),
        ("a ceiling half a pixel over the screen the bare page fits", 1.35, -100.6, 390.0, 844.0, 844.5, 0.0),
        ("no viewport height", 8.0, -2000.0, 390.0, 0.0, 0.0, 0.0),
        ("no scale", 0.0, -2000.0, 390.0, 844.0, 0.0, 0.0),
    ];
    let dressed = write(&page, &dressed_cases, "    ");

    let json = format!(
        concat!(
            "{{\n  \"page\": \"042.qvp\",\n  \"tolerance\": 0.001,\n",
            "  \"spec\": {{\"viewportW\": {}, \"viewportH\": {}, \"padTop\": {}, \"padBottom\": {}, \"padLeft\": {}, \"padRight\": {}}},\n",
            "  \"contentH\": {},\n",
            "  \"cases\": [\n{}\n  ],\n",
            "  \"dressed\": {{\n",
            "    \"ornaments\": \"qvo1-dress.hex\",\n",
            "    \"dress\": {{\"ayahMark\": 0, \"surahHeader\": 1, \"pageFrame\": 2, \"gap\": 5, \"bandGap\": 5, \"ayahMarkScale\": 1, \"pageFrameScale\": 1}},\n",
            "    \"overflow\": [{}, {}, {}, {}],\n",
            "    \"cases\": [\n{}\n    ]\n  }}\n}}\n"
        ),
        spec.viewport_w,
        spec.viewport_h,
        spec.pad_top,
        spec.pad_bottom,
        spec.pad_left,
        spec.pad_right,
        content_h,
        cases.join(",\n"),
        ol,
        ot,
        or,
        ob,
        dressed.join(",\n")
    );
    std::fs::create_dir_all(std::path::Path::new(out_path).parent().unwrap()).expect("mkdir");
    std::fs::write(out_path, json).expect("write scenarios");
    println!("{out_path}: {} cases, {} dressed", cases.len(), dressed.len());
}
