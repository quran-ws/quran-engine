//! Real-page numbers for the array getters a renderer calls every frame, through the C ABI:
//! cargo run -p qvp-ffi --release --example bench -- dist/pages/042.qvp
use qvp_ffi::*;
use std::time::Instant;

const ROUNDS: u32 = 20_000;

fn per_call(name: &str, mut call: impl FnMut() -> u32) {
    let n = call();
    let t = Instant::now();
    for _ in 0..ROUNDS {
        std::hint::black_box(call());
    }
    println!("{name}: {n} records, {:?} per call", t.elapsed() / ROUNDS);
}

fn main() {
    let path = std::env::args().nth(1).unwrap_or_else(|| "dist/pages/042.qvp".into());
    let bytes = std::fs::read(&path).expect("read qvp");
    unsafe {
        let page = qvp_page_load(bytes.as_ptr(), bytes.len());
        assert!(!page.is_null(), "{path} did not load");
        let mut info = std::mem::zeroed::<QvpPageInfo>();
        qvp_page_info(page, &mut info);
        println!("{path}: {} words, {} paths", info.n_words, info.n_paths);

        let spec = QvpLayoutSpec {
            viewport_w: 690.0,
            viewport_h: 1100.0,
            pad_top: 20.0,
            pad_bottom: 20.0,
            pad_left: 0.0,
            pad_right: 0.0,
            line_spacing: 1.0,
            fill_height: 1,
            grid_lines: 0,
            crop_left: 0.0,
            crop_right: 0.0,
            max_aspect_slack: 0.0,
            reflow_zoom: 0.0,
            reflow_fill: 0,
            reflow_breaks: 255,
            reflow_gaps: 0,
            reflow_word_gap: 0.0,
            reflow_max_stretch: 0.0,
            reflow_relax: -1.0,
            banner_zoom: 0.0,
            surah_frames: 1,
        };
        let mut layout = std::mem::zeroed::<QvpLayout>();
        qvp_layout(page, &spec, &mut layout);

        let n = qvp_layout_draw_list(page, 0.0, 0.0, std::ptr::null_mut(), 0);
        let mut draws = vec![0u32; n as usize * 2];
        per_call("layout draw list", || qvp_layout_draw_list(page, 0.0, 0.0, draws.as_mut_ptr(), n));
        per_call("layout draw list, count only", || qvp_layout_draw_list(page, 0.0, 0.0, std::ptr::null_mut(), 0));
        // what a wrapper does every frame: ask the count with a null buffer, then fill
        per_call("layout draw list, count then fill", || {
            let n = qvp_layout_draw_list(page, 0.0, 0.0, std::ptr::null_mut(), 0);
            qvp_layout_draw_list(page, 0.0, 0.0, draws.as_mut_ptr(), n)
        });
        // a drag moves the band every frame, so no two frames ask for the same list
        let mut frame = 0u32;
        per_call("layout draw list, count then fill, moving band", || {
            frame += 1;
            let top = (frame % 400) as f32;
            let n = qvp_layout_draw_list(page, top, top + 600.0, std::ptr::null_mut(), 0);
            qvp_layout_draw_list(page, top, top + 600.0, draws.as_mut_ptr(), n)
        });
        let mut placements = vec![0f32; qvp_layout_placements(page, std::ptr::null_mut(), 0) as usize * 4];
        per_call("layout placements", || {
            qvp_layout_placements(page, placements.as_mut_ptr(), placements.len() as u32 / 4)
        });
        per_call("layout placements, count then fill", || {
            let n = qvp_layout_placements(page, std::ptr::null_mut(), 0);
            qvp_layout_placements(page, placements.as_mut_ptr(), n)
        });

        // a band over the first 40 words, which cross several lines, and a mask over the page
        let words = QvpTarget { target: 7, a: 0, b: 39, c: 0, words: std::ptr::null(), n_words: 0 };
        let handle = qvp_highlight_add(page, &words, std::ptr::null());
        let mut boxes = vec![std::mem::zeroed::<QvpBox>(); 64];
        per_call("highlight boxes, count then fill", || {
            let n = qvp_highlight_boxes_view(page, std::ptr::null_mut(), 0);
            qvp_highlight_boxes_view(page, boxes.as_mut_ptr(), n)
        });
        qvp_highlight_remove(page, handle);
        let page_target = QvpTarget { target: 0, a: 0, b: 0, c: 0, words: std::ptr::null(), n_words: 0 };
        qvp_mask(page, &page_target, 1);
        qvp_mask_all(page);
        let mut masks = vec![std::mem::zeroed::<QvpBox>(); info.n_words as usize];
        per_call("mask boxes, count then fill", || {
            let n = qvp_mask_boxes_view(page, std::ptr::null_mut(), 0);
            qvp_mask_boxes_view(page, masks.as_mut_ptr(), n)
        });
        qvp_unmask(page);

        // two selectors: a word and a mark, as a reader with a highlight and a theme has
        qvp_style_add(page, 0, &QvpSelector { selector: 7, a: 3, b: 0, c: 0 }, 0xff0000ff, 0);
        qvp_style_add(page, 0, &QvpSelector { selector: 10, a: 7, b: 0, c: 0 }, 0x0000ffff, 0);
        qvp_tick(page, 0.0);
        let mut pairs = vec![0u32; info.n_paths as usize * 2];
        per_call("styled paths", || qvp_styled_paths(page, pairs.as_mut_ptr(), info.n_paths));

        // the first complete ayah on the page, laid out as a passage
        let mut ayah = std::mem::zeroed::<QvpAyahInfo>();
        let mut i = 0;
        while qvp_ayah_info(page, i, &mut ayah) == 1 && ayah.fragments != 1 {
            i += 1;
        }
        let pages = [page as *const _];
        let passage = qvp_passage_load(pages.as_ptr(), 1, ayah.surah, ayah.ayah, ayah.ayah);
        assert!(!passage.is_null(), "ayah {}:{} did not load as a passage", ayah.surah, ayah.ayah);
        let pspec = QvpPassageSpec {
            width: 366.0,
            scale: 0.9,
            line_spacing: 1.0,
            padding: 2.0,
            align: 0,
            keep_ayah_mark: 0,
            _pad: [0; 2],
            max_rows: 0,
            ellipsis_width: 0.0,
            max_stretch: 0.0,
        };
        let mut playout = QvpPassageLayout::default();
        assert_eq!(qvp_passage_layout(passage, &pspec, &mut playout), 1);
        let mut pdraws = vec![0u32; playout.n_draws as usize * 3];
        per_call("passage draw list", || qvp_passage_draw_list(passage, pdraws.as_mut_ptr(), playout.n_draws));

        qvp_passage_free(passage);
        qvp_page_free(page);
    }
}
