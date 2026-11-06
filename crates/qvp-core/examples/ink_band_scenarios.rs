//! Write the cross-wrapper ink band scenarios: the band of page 042 a host keeps drawn, for
//! views and viewports that cover each branch: a page that fits, a magnified page drawn a band
//! at a time, pans inside one step and across it, a host's own height and pixel limits, a
//! screen taller than the ceiling, and no screen at all. Every wrapper's test suite replays
//! them through its ink band call and compares the top and the height.
//!
//! Run: cargo run -p qvp-core --release --example ink_band_scenarios -- dist/pages/042.qvp conformance/scenarios/ink-band.json
use qvp_core::*;

/// One case: a name, the view's scale and vertical offset, the viewport, and the two limits.
type Case = (&'static str, f32, f32, f32, f32, f32, f32);

const CASES: [Case; 10] = [
    ("the page fits", 1.0, 0.0, 390.0, 844.0, 0.0, 0.0),
    ("magnified, a band", 8.0, -2000.0, 390.0, 844.0, 0.0, 0.0),
    ("a pan inside the step", 8.0, -2100.0, 390.0, 844.0, 0.0, 0.0),
    ("a pan across the step", 8.0, -2150.0, 390.0, 844.0, 0.0, 0.0),
    ("the host's own height", 2.0, -300.0, 390.0, 844.0, 1000.0, 0.0),
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

    let cases: Vec<String> = CASES
        .iter()
        .map(|&(name, scale, offset_y, viewport_w, viewport_h, max_height, max_pixels)| {
            let view = View { scale, offset_x: 0.0, offset_y };
            let b = page.layout_ink_band(view, viewport_w, viewport_h, max_height, max_pixels);
            format!(
                concat!(
                    "    {{\"name\": \"{}\", \"view\": {{\"scale\": {}, \"offsetX\": 0, \"offsetY\": {}}},\n",
                    "     \"viewportW\": {}, \"viewportH\": {}, \"maxHeight\": {}, \"maxPixels\": {},\n",
                    "     \"band\": {{\"top\": {}, \"height\": {}}}}}"
                ),
                name, scale, offset_y, viewport_w, viewport_h, max_height, max_pixels, b.top, b.height
            )
        })
        .collect();

    let json = format!(
        concat!(
            "{{\n  \"page\": \"042.qvp\",\n  \"tolerance\": 0.001,\n",
            "  \"spec\": {{\"viewportW\": {}, \"viewportH\": {}, \"padTop\": {}, \"padBottom\": {}, \"padLeft\": {}, \"padRight\": {}}},\n",
            "  \"contentH\": {},\n",
            "  \"cases\": [\n{}\n  ]\n}}\n"
        ),
        spec.viewport_w,
        spec.viewport_h,
        spec.pad_top,
        spec.pad_bottom,
        spec.pad_left,
        spec.pad_right,
        content_h,
        cases.join(",\n")
    );
    std::fs::create_dir_all(std::path::Path::new(out_path).parent().unwrap()).expect("mkdir");
    std::fs::write(out_path, json).expect("write scenarios");
    println!("{out_path}: {} cases", cases.len());
}
