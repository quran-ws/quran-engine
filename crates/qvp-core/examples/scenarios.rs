//! Write the cross-wrapper layout scenarios: the engine's answers for page 042 over a fixed
//! set of viewports, which every wrapper's test suite replays and compares.
//!
//! Run: cargo run -p qvp-core --release --example scenarios -- dist/pages/042.qvp conformance/scenarios/layout.json
use qvp_core::*;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let page_path = args.get(1).map(String::as_str).unwrap_or("dist/pages/042.qvp");
    let out_path = args.get(2).map(String::as_str).unwrap_or("conformance/scenarios/layout.json");
    let bytes = std::fs::read(page_path).expect("read page");
    let mut page = Page::load(&bytes).expect("load page");

    let viewports: [(f32, f32); 10] = [
        (390.0, 844.0),
        (375.0, 667.0),
        (430.0, 932.0),
        (768.0, 1024.0),
        (1024.0, 768.0),
        (1440.0, 900.0),
        (320.0, 480.0),
        (600.0, 600.0),
        (2000.0, 600.0),
        (345.0, 550.0),
    ];
    let mut cases = Vec::new();
    for (w, h) in viewports {
        for (fill, slack, crop) in [(false, 0.0, 0.0), (true, 0.0, 0.0), (false, 1.15, 0.0), (true, 1.15, 12.0)] {
            let spec = LayoutSpec {
                viewport_w: w,
                viewport_h: h,
                pad_top: 24.0,
                pad_bottom: 24.0,
                pad_left: 16.0,
                pad_right: 16.0,
                line_spacing: 1.0,
                fill_height: fill,
                grid_lines: 0,
                crop_left: crop,
                crop_right: crop,
                max_aspect_slack: slack,
                reflow: None,
                banner_zoom: 0.0,
                surah_frames: true,
            };
            let to_fill = page.line_spacing_to_fill(&spec, f32::INFINITY);
            let wasted = page.wasted_fraction(&spec);
            let l = page.layout(&spec).clone();
            cases.push(format!(
                concat!(
                    "    {{\"spec\": {{\"viewportW\": {}, \"viewportH\": {}, \"padTop\": 24, \"padBottom\": 24, \"padLeft\": 16, \"padRight\": 16, ",
                    "\"lineSpacing\": 1, \"fillHeight\": {}, \"gridLines\": 0, \"cropLeft\": {}, \"cropRight\": {}, \"maxAspectSlack\": {}}},\n",
                    "     \"layout\": {{\"scale\": {}, \"offsetX\": {}, \"offsetY\": {}, \"contentW\": {}, \"contentH\": {}, \"lineSpacing\": {}, ",
                    "\"fitScale\": {}, \"fitX\": {}, \"fitY\": {}, \"lineDy0\": {}, \"lineDyLast\": {}}},\n",
                    "     \"lineSpacingToFill\": {}, \"wastedFraction\": {}}}"
                ),
                w, h, fill, crop, crop, slack,
                l.scale, l.offset_x, l.offset_y, l.content_w, l.content_h, l.line_spacing,
                l.fit_scale, l.fit_x, l.fit_y, l.line_dy[0], l.line_dy[l.line_dy.len() - 1],
                to_fill, wasted
            ));
        }
    }
    let json = format!(
        "{{\n  \"page\": \"042.qvp\",\n  \"tolerance\": 0.001,\n  \"cases\": [\n{}\n  ]\n}}\n",
        cases.join(",\n")
    );
    std::fs::create_dir_all(std::path::Path::new(out_path).parent().unwrap()).expect("mkdir");
    std::fs::write(out_path, json).expect("write scenarios");
    println!("{out_path}: {} cases", cases.len());
}
