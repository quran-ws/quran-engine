//! Data gate: every element moves with its line under layout.
//!
//! A path is shifted by the `line_dy` of the line index it is laid out with. So an ayah
//! mark must carry the line of the ayah it closes, a sajdah line the line of the word
//! it is drawn over, and a line's reference centre must come from its body ink alone. A
//! sajdah sign is taller than the text and used to pull the centre, which snapped the
//! ayah mark to the neighbouring line on five pages. The sajdah line used to move with the
//! sign's line and dropped into the diacritics of the sajdah word.
//! Needs dist/pages from scripts/sync-test-data.sh. Skipped when the data is missing
//! unless QVP_REQUIRE_DATA is set.
use qvp_core::Page;
use qvp_format::{IBox, Mark, PathKind};
use std::path::{Path, PathBuf};

fn pages_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dist/pages")
}

#[test]
fn ayah_marks_and_line_centres_follow_the_text() {
    if !pages_dir().join("001.qvp").exists() {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!("dist/pages is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
        }
        eprintln!("skip: dist/pages is missing; run scripts/sync-test-data.sh to enable this gate");
        return;
    }
    let mut failures = Vec::new();
    let mut marks = 0;
    let mut sajdah_lines = 0;
    for n in 1..=604u32 {
        let Ok(bytes) = std::fs::read(pages_dir().join(format!("{n:03}.qvp"))) else { continue };
        let p = Page::load(&bytes).expect("decode page");
        let d = p.data();
        let q = p.quant();
        let g = p.geometry();
        for a in d.ayahs.iter().filter(|a| a.fragment == a.fragments && a.n_words > 0) {
            let Some(deco) = d.decorations.get(a.ayah_mark_decoration as usize) else { continue };
            let last = &d.words[(a.first_word + a.n_words - 1) as usize];
            marks += 1;
            for pi in deco.first_path..deco.first_path + deco.n_paths as u32 {
                let line = g.table[pi as usize].line;
                if line != last.line_index as u32 {
                    failures.push(format!(
                        "page {n:03} {}:{} mark on line {line}, its last word on line {}",
                        a.surah, a.ayah, last.line_index
                    ));
                    break;
                }
            }
        }
        for (pi, pr) in d.paths.iter().enumerate().filter(|(_, pr)| pr.mark == Mark::SajdahLine) {
            sajdah_lines += 1;
            let line = g.table[pi].line;
            // the word under the sajdah line: same columns, top edge at or below the line
            let under = d
                .words
                .iter()
                .filter(|w| w.bbox.x0 < pr.bbox.x1 && w.bbox.x1 > pr.bbox.x0 && w.bbox.y0 >= pr.bbox.y0 - 2 * q as i32)
                .min_by_key(|w| w.bbox.y0 - pr.bbox.y1);
            match under {
                Some(w) if w.line_index as u32 == line => {}
                Some(w) => failures.push(format!(
                    "page {n:03} sajdah line on line {line}, the word under it on line {}",
                    w.line_index
                )),
                None => failures.push(format!("page {n:03} sajdah line on line {line} with no word under it")),
            }
        }
        for (li, l) in d.lines.iter().enumerate() {
            let mut ink = IBox::EMPTY;
            for wi in l.first_word..l.first_word + l.n_words {
                let w = &d.words[wi as usize];
                for pi in w.first_path..w.first_path + w.n_paths as u32 {
                    let pr = &d.paths[pi as usize];
                    if pr.kind == PathKind::Body {
                        ink.union(&pr.bbox);
                    }
                }
            }
            if ink.is_empty() {
                continue;
            }
            let want = (ink.y0 + ink.y1) as f32 / 2.0 / q;
            let got = p.line_centre(li);
            if (got - want).abs() > 0.01 {
                failures.push(format!("page {n:03} line {li} centre {got:.2}, body ink centre {want:.2}"));
            }
        }
    }
    assert!(marks > 6000, "only {marks} ayah marks seen; is dist/pages complete?");
    assert_eq!(sajdah_lines, 15, "the mushaf has 15 sajdah lines");
    assert!(
        failures.is_empty(),
        "{} elements would not move with their line:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
