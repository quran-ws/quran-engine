//! Write the cross-wrapper hit scenarios: the engine's hits on page 042 at points on a word's
//! outline, on a word's box alone, on a decoration's outline and on a decoration's box alone,
//! in page units and in viewport px. Every wrapper's test suite replays them through its exact
//! and gap-aware hit tests and compares each field.
//!
//! Run: cargo run -p qvp-core --release --example hit_scenarios -- dist/pages/042.qvp conformance/scenarios/hit.json
use qvp_core::*;

/// What an exact hit landed on. The scenarios hold one point of each.
const KINDS: [&str; 4] = ["word outline", "word box", "decoration outline", "decoration box"];

fn kind(h: HitExact) -> usize {
    match (h.word != NONE, h.path != NONE) {
        (true, true) => 0,
        (true, false) => 1,
        (false, true) => 2,
        (false, false) => 3,
    }
}

/// A hit as JSON, with the wrappers' -1 for an absent index.
fn json(h: Option<Hit>) -> String {
    let index = |v: u32| if v == NONE { -1 } else { v as i64 };
    match h {
        None => "null".into(),
        Some(h) => format!(
            "{{\"word\": {}, \"path\": {}, \"decoration\": {}, \"line\": {}, \"distance\": {}, \"isExact\": {}}}",
            index(h.word),
            index(h.path),
            index(h.decoration),
            index(h.line),
            h.distance,
            h.is_exact
        ),
    }
}

/// The first point of each kind on a grid of `step`, scanning rows from the top. A point is
/// taken only when the gap-aware test also finds it and the four points `step / 2` away give
/// the same hits, so a wrapper that rounds a coordinate differently still lands on the same
/// outline or box.
fn find(
    w: f32,
    h: f32,
    step: f32,
    exact: impl Fn(f32, f32) -> Option<HitExact>,
    hit: impl Fn(f32, f32) -> Option<Hit>,
) -> Vec<(f32, f32)> {
    let same = |a: Option<Hit>, b: Option<Hit>| match (a, b) {
        (Some(a), Some(b)) => {
            (a.word, a.path, a.decoration, a.line, a.is_exact) == (b.word, b.path, b.decoration, b.line, b.is_exact)
        }
        (a, b) => a.is_none() && b.is_none(),
    };
    let mut found: [Option<(f32, f32)>; 4] = [None; 4];
    let (mut y, d) = (step, step / 2.0);
    while y < h && found.iter().any(Option::is_none) {
        let mut x = step;
        while x < w {
            if let Some(e) = exact(x, y) {
                let k = kind(e);
                let steady = hit(x, y).is_some()
                    && [(x - d, y), (x + d, y), (x, y - d), (x, y + d)]
                        .iter()
                        .all(|&(nx, ny)| exact(nx, ny) == Some(e) && same(hit(nx, ny), hit(x, y)));
                if found[k].is_none() && steady {
                    found[k] = Some((x, y));
                }
            }
            x += step;
        }
        y += step;
    }
    found.iter().enumerate().map(|(k, p)| p.unwrap_or_else(|| panic!("no steady point on a {}", KINDS[k]))).collect()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let page_path = args.get(1).map(String::as_str).unwrap_or("dist/pages/042.qvp");
    let out_path = args.get(2).map(String::as_str).unwrap_or("conformance/scenarios/hit.json");
    let bytes = std::fs::read(page_path).expect("read page");
    let mut page = Page::load(&bytes).expect("load page");
    let opt = HitOptions::default();
    let mut cases = Vec::new();

    let points =
        find(page.width(), page.height(), 0.5, |x, y| page.hit_test_exact(x, y), |x, y| page.hit_test(x, y, &opt));
    for (k, &(x, y)) in points.iter().enumerate() {
        let exact = page.hit_test_exact(x, y).map(|h| page.exact_to_hit(h));
        cases.push(format!(
            "    {{\"name\": \"{}\", \"units\": \"page\", \"x\": {x}, \"y\": {y},\n     \"exact\": {},\n     \"hit\": {}}}",
            KINDS[k],
            json(exact),
            json(page.hit_test(x, y, &opt))
        ));
    }

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
    let points = find(
        spec.viewport_w,
        spec.viewport_h,
        0.5,
        |x, y| page.hit_test_exact_view(x, y),
        |x, y| page.hit_test_view(x, y, &opt),
    );
    for (k, &(x, y)) in points.iter().enumerate() {
        let exact = page.hit_test_exact_view(x, y).map(|h| page.exact_to_hit(h));
        cases.push(format!(
            "    {{\"name\": \"{}\", \"units\": \"view\", \"x\": {x}, \"y\": {y},\n     \"exact\": {},\n     \"hit\": {}}}",
            KINDS[k],
            json(exact),
            json(page.hit_test_view(x, y, &opt))
        ));
    }

    let json = format!(
        concat!(
            "{{\n  \"page\": \"042.qvp\",\n  \"tolerance\": 0.001,\n",
            "  \"spec\": {{\"viewportW\": {}, \"viewportH\": {}, \"padTop\": {}, \"padBottom\": {}, \"padLeft\": {}, \"padRight\": {}}},\n",
            "  \"cases\": [\n{}\n  ]\n}}\n"
        ),
        spec.viewport_w,
        spec.viewport_h,
        spec.pad_top,
        spec.pad_bottom,
        spec.pad_left,
        spec.pad_right,
        cases.join(",\n")
    );
    std::fs::create_dir_all(std::path::Path::new(out_path).parent().unwrap()).expect("mkdir");
    std::fs::write(out_path, json).expect("write scenarios");
    println!("{out_path}: {} cases", cases.len());
}
