//! Write the cross-wrapper passage scenarios: the engine's layouts of fixed ayah ranges, and of
//! two surahs' basmalahs, at fixed specs, which every wrapper that binds passages replays and
//! compares.
//!
//! Run: cargo run -p qvp-core --release --example passage_scenarios -- dist/pages conformance/scenarios/passage.json
use qvp_core::{Align, Page, Passage, PassageSpec};
use std::path::Path;

/// The ranges: the Fatihah on the smaller lines of page 1, 2:255, a range across two
/// pages, the longest ayah, and two sajdah ayahs.
const PASSAGES: [(&[u32], u16, u16, u16); 6] = [
    (&[1], 1, 1, 7),
    (&[42], 2, 255, 255),
    (&[2, 3], 2, 5, 6),
    (&[48], 2, 282, 282),
    (&[272], 16, 49, 50),
    (&[598], 96, 19, 19),
];

/// The basmalahs: the first in the book, on the smaller lines of page 2, and the last.
const BASMALAHS: [(&[u32], u16); 2] = [(&[2], 2), (&[604], 114)];

/// One thing to lay out at every spec of its list: an ayah range, or a surah's basmalah.
struct Run<'a> {
    pages: &'a [u32],
    surah: u16,
    from: u16,
    to: u16,
    basmalah: bool,
    specs: &'a [PassageSpec],
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let directory = args.get(1).map(String::as_str).unwrap_or("dist/pages");
    let out_path = args.get(2).map(String::as_str).unwrap_or("conformance/scenarios/passage.json");
    let specs = [
        PassageSpec { width: 200.0, scale: 0.9, ..Default::default() },
        PassageSpec { width: 366.0, scale: 0.9, ..Default::default() },
        PassageSpec { width: 480.0, scale: 0.9, ..Default::default() },
        PassageSpec { width: 366.0, scale: 0.9, align: Align::Center, line_spacing: 1.5, ..Default::default() },
        PassageSpec {
            width: 200.0,
            scale: 0.9,
            max_rows: 1,
            keep_ayah_mark: true,
            ellipsis_width: 9.0,
            ..Default::default()
        },
        PassageSpec { width: 0.0, scale: 0.5, padding: 0.0, ..Default::default() },
        PassageSpec { width: 366.0, scale: 0.9, align: Align::Justified, ..Default::default() },
        PassageSpec { width: 200.0, scale: 0.9, align: Align::Justified, max_stretch: 5.0, ..Default::default() },
    ];
    let basmalah_specs = [
        PassageSpec { width: 366.0, scale: 0.9, align: Align::Center, ..Default::default() },
        PassageSpec { width: 0.0, scale: 0.5, padding: 0.0, ..Default::default() },
    ];
    let load = |numbers: &[u32]| -> Vec<Page> {
        numbers
            .iter()
            .map(|n| {
                Page::load(&std::fs::read(Path::new(directory).join(format!("{n:03}.qvp"))).expect("read page"))
                    .expect("load page")
            })
            .collect()
    };
    let mut runs: Vec<Run> = PASSAGES
        .iter()
        .map(|&(pages, surah, from, to)| Run { pages, surah, from, to, basmalah: false, specs: &specs })
        .collect();
    runs.extend(BASMALAHS.iter().map(|&(pages, surah)| Run {
        pages,
        surah,
        from: 0,
        to: 0,
        basmalah: true,
        specs: &basmalah_specs,
    }));
    let mut cases = Vec::new();
    for Run { pages: numbers, surah, from, to, basmalah, specs } in runs {
        let pages = load(numbers);
        let refs: Vec<&Page> = pages.iter().collect();
        let mut passage = if basmalah {
            Passage::load_basmalah(&refs, surah).expect("load basmalah")
        } else {
            Passage::load(&refs, surah, from, to).expect("load passage")
        };
        for spec in specs {
            let l = passage.layout(spec).expect("lay out").clone();
            let rows: Vec<String> =
                l.rows.iter().map(|r| format!("[{}, {}, {}, {}, {}]", r.x0, r.y0, r.x1, r.y1, r.baseline)).collect();
            let words: Vec<String> = l
                .words
                .iter()
                .map(|w| {
                    format!("[{}, {}, {}, {}, {}, {}, {}, {}]", w.page, w.ayah, w.word, w.row, w.x0, w.y0, w.x1, w.y1)
                })
                .collect();
            let ellipsis =
                l.ellipsis.map_or("null".to_owned(), |e| format!("[{}, {}, {}, {}]", e[0], e[1], e[2], e[3]));
            cases.push(format!(
                concat!(
                    "    {{\"pages\": {:?}, \"surah\": {}, \"from\": {}, \"to\": {}, \"basmalah\": {},\n",
                    "     \"spec\": {{\"width\": {}, \"scale\": {}, \"lineSpacing\": {}, \"padding\": {}, \"align\": \"{}\", ",
                    "\"maxRows\": {}, \"keepAyahMark\": {}, \"ellipsisWidth\": {}, \"maxStretch\": {}}},\n",
                    "     \"layout\": {{\"width\": {}, \"height\": {}, \"scale\": {}, \"lineSpacing\": {}, \"isTruncated\": {}, ",
                    "\"ellipsis\": {}, \"draws\": {},\n",
                    "      \"rows\": [{}],\n",
                    "      \"words\": [{}]}}}}"
                ),
                numbers, surah, from, to, basmalah,
                spec.width, spec.scale, spec.line_spacing, spec.padding,
                match spec.align {
                    Align::Right => "right",
                    Align::Center => "center",
                    Align::Justified => "justified",
                },
                spec.max_rows, spec.keep_ayah_mark, spec.ellipsis_width, spec.max_stretch,
                l.width, l.height, l.scale, l.line_spacing, l.is_truncated, ellipsis, l.draws.len(),
                rows.join(", "),
                words.join(", ")
            ));
        }
    }
    let json = format!(
        "{{\n  \"note\": \"rows: [x0, y0, x1, y1, baseline]; words: [page, ayah, word, row, x0, y0, x1, y1]; layout px\",\n  \"tolerance\": 0.01,\n  \"cases\": [\n{}\n  ]\n}}\n",
        cases.join(",\n")
    );
    std::fs::write(out_path, json).expect("write scenarios");
}
