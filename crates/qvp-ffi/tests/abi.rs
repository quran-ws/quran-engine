//! Smoke test of the C ABI against a real converted page (needs dist/pages/036.qvp and atlas.qva).
use qvp_ffi::*;
use std::path::Path;

fn page_bytes(n: &str) -> Option<Vec<u8>> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../dist/pages/{n}"));
    std::fs::read(p).ok()
}

fn s(q: &QvpStr) -> String {
    unsafe { String::from_utf8_lossy(std::slice::from_raw_parts(q.ptr, q.len as usize)).into_owned() }
}

#[test]
fn abi_end_to_end() {
    let Some(bytes) = page_bytes("036.qvp") else {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!("dist/pages/036.qvp is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
        }
        eprintln!("skip: dist/pages/036.qvp is missing; run scripts/sync-test-data.sh to enable this test");
        return;
    };
    unsafe {
        let page = qvp_page_load(bytes.as_ptr(), bytes.len());
        assert!(!page.is_null());
        let mut info = std::mem::zeroed::<QvpPageInfo>();
        qvp_page_info(page, &mut info);
        assert_eq!((info.n_words, info.n_paths, info.n_lines), (150, 1162, 15));
        // word + forms
        let mut w = std::mem::zeroed::<QvpWordInfo>();
        assert_eq!(qvp_word_info(page, 0, &mut w), 1);
        assert_eq!(w.surah, 2);
        let mut f = QvpStr { ptr: std::ptr::null(), len: 0 };
        assert_eq!(qvp_word_form(page, 0, 4, &mut f), 1);
        assert!(!s(&f).is_empty());
        // search + text
        let q = "الله";
        let mut m = [QvpMatch { word: 0, index: 0, is_loose_match: 0 }; 64];
        let n = qvp_search(page, q.as_ptr(), q.len() as u32, 4, 0, 1, 1, 0, m.as_mut_ptr(), 64);
        assert!(n > 0 && n < 64, "{n}");
        let mut t = QvpStr { ptr: std::ptr::null(), len: 0 };
        let tg =
            QvpTarget { target: 3, a: w.surah as u32, b: w.ayah as u32, c: 0, words: std::ptr::null(), n_words: 0 };
        qvp_text(page, &tg, 0, b" ".as_ptr(), 1, b"\n".as_ptr(), 1, &mut t);
        assert!(s(&t).contains(' '));
        // gap-aware hit at the centre of word 0's line but between words: always resolves
        let mut hx = std::mem::zeroed::<QvpHit>();
        assert_eq!(qvp_hit_test(page, (w.x0 + w.x1) / 2.0, (w.y0 + w.y1) / 2.0, std::ptr::null(), &mut hx), 1);
        assert_eq!(hx.word, 0);
        // layout + view hit
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
        let mut lay = std::mem::zeroed::<QvpLayout>();
        qvp_layout(page, &spec, &mut lay);
        assert_eq!(lay.n_lines, 15);
        assert!((lay.scale - 2.0).abs() < 1e-5);
        let mut wb = [0f32; 4];
        assert_eq!(qvp_word_bounds_view(page, 0, wb.as_mut_ptr()), 1);
        // at the printed size a reflow spec changes nothing: the page is laid out as printed
        let at_one = QvpLayoutSpec { reflow_zoom: 1.0, ..spec };
        let mut lay1 = std::mem::zeroed::<QvpLayout>();
        qvp_layout(page, &at_one, &mut lay1);
        assert_eq!(lay1.reflowed, 0);
        assert!((lay1.scale - lay.scale).abs() < 1e-5 && (lay1.content_h - lay.content_h).abs() < 1e-3);
        let mut wb1 = [0f32; 4];
        assert_eq!(qvp_word_bounds_view(page, 0, wb1.as_mut_ptr()), 1);
        assert!(wb1.iter().zip(wb.iter()).all(|(a, b)| (a - b).abs() < 1e-3), "{wb1:?} vs {wb:?}");
        // zoomed in, the words break onto rows of their own
        let reflowed = QvpLayoutSpec { reflow_zoom: 2.0, ..spec };
        let mut lay2 = std::mem::zeroed::<QvpLayout>();
        qvp_layout(page, &reflowed, &mut lay2);
        assert_eq!(lay2.reflowed, 1);
        assert!(lay2.n_rows > lay.n_lines && lay2.content_h > lay.content_h);
        let n_groups = qvp_layout_groups(page, std::ptr::null_mut(), 0);
        let mut groups = vec![0f32; n_groups as usize * 4];
        assert_eq!(qvp_layout_groups(page, groups.as_mut_ptr(), n_groups), n_groups);
        let n_pg = qvp_layout_path_groups(page, std::ptr::null_mut(), 0);
        assert!(n_pg > 0);
        assert!(qvp_reflow_max_zoom(page, &reflowed) >= 1.0);
        let printed = QvpLayoutSpec { reflow_breaks: 3, ..reflowed };
        let cap = qvp_reflow_max_zoom(page, &printed);
        let mut fitted = std::mem::zeroed::<QvpLayout>();
        qvp_layout(page, &QvpLayoutSpec { reflow_zoom: cap, ..printed }, &mut fitted);
        let mut clamped = std::mem::zeroed::<QvpLayout>();
        qvp_layout(page, &QvpLayoutSpec { reflow_zoom: cap * 2.0, ..printed }, &mut clamped);
        assert_eq!(clamped.n_rows, lay.n_lines);
        assert_eq!(clamped.scale, fitted.scale);
        // the reflow knobs ride on the printed page: the pinch that leaves it, which the engine
        // lays out inside the call, reflows with the host's fill and not the default
        let first_step = |fill: u8| -> Vec<f32> {
            let printed = QvpLayoutSpec { reflow_fill: fill, ..spec };
            let view = QvpView { scale: 1.0, offset_x: 0.0, offset_y: 0.0 };
            let mut change = std::mem::zeroed::<QvpZoomChange>();
            qvp_zoom_to_step(page, &printed, &std::mem::zeroed::<QvpZoom>(), 1, &view, &mut change);
            assert_eq!(change.relaid, 1);
            let mut bounds = Vec::new();
            let mut b = [0f32; 4];
            while qvp_word_bounds_view(page, (bounds.len() / 4) as u32, b.as_mut_ptr()) == 1 {
                bounds.extend_from_slice(&b);
            }
            bounds
        };
        assert_ne!(first_step(1), first_step(2), "a justified first step is not the centred one");
        qvp_layout(page, &spec, &mut lay);
        let mut h = std::mem::zeroed::<QvpHit>();
        assert_eq!(qvp_hit_test_exact_view(page, (wb[0] + wb[2]) / 2.0, (wb[1] + wb[3]) / 2.0, &mut h), 1);
        assert_eq!(h.word, 0);
        // styles + highlight + tick
        let sel = QvpSelector { selector: 3, a: 0, b: 0, c: 0 }; // first mark of word 0
        let hd = qvp_style_add(page, 0, &sel, 0xff0000ff, 0);
        assert!(hd > 0);
        let mut pairs = vec![0u32; 4000];
        let n = qvp_styled_paths(page, pairs.as_mut_ptr(), 2000);
        assert_eq!(n, 1);
        assert_eq!(qvp_style_remove(page, hd), 1);
        let st = QvpHighlightStyle {
            mode: 2,
            height: 0,
            ink: 0x00aa00ff,
            band: 0xffcc0080,
            pad_x: 1.2,
            pad_y: 0.0,
            radius: 1.0,
            seam: 0.25,
            transition_ms: 200,
            layer: 50,
        };
        let hh = qvp_highlight_add(page, &tg, &st);
        assert!(hh > 0);
        assert_eq!(qvp_tick(page, 100.0), 1, "animating");
        let mut boxes = [QvpBox { id: 0, line: 0, x0: 0.0, y0: 0.0, x1: 0.0, y1: 0.0, color: 0, radius: 0.0 }; 32];
        let nb = qvp_highlight_boxes_view(page, boxes.as_mut_ptr(), 32);
        assert!((1..=15).contains(&nb));
        assert_eq!(qvp_tick(page, 1000.0), 0);
        qvp_highlight_remove(page, hh);
        // metadata
        let mut d = [QvpDivision { division: 0, line: 0, number: 0, surah: 0, ayah: 0, ayah_index: 0 }; 8];
        let _ = qvp_divisions(page, d.as_mut_ptr(), 8);
        let mut mk = [QvpAyahMark {
            decoration: 0,
            surah: 0,
            ayah: 0,
            line: 0,
            cx: 0.0,
            cy: 0.0,
            r: 0.0,
            ornament_path: 0,
            numeral_path: 0,
        }; 32];
        assert_eq!(qvp_ayah_marks(page, mk.as_mut_ptr(), 32), 6);
        // mask/reveal
        qvp_mask(page, &tg, 0);
        assert!(qvp_mask_hidden(page, std::ptr::null_mut(), 0) > 0);
        assert_eq!(qvp_unmask_next(page, 1), 1);
        qvp_unmask(page);
        // crop svg
        let mut svg = QvpStr { ptr: std::ptr::null(), len: 0 };
        assert_eq!(qvp_crop_svg(page, &tg, 2.0, 1, 0xffffffff, &mut svg), 1);
        assert!(s(&svg).starts_with("<svg"));
        // labels + citation
        let mut l = QvpStr { ptr: std::ptr::null(), len: 0 };
        qvp_word_label(page, 0, &mut l);
        assert!(s(&l).contains("2:"));
        let ws = [0u32, 1, 2];
        qvp_citation(page, ws.as_ptr(), 3, &mut l);
        assert!(s(&l).starts_with("2:"));
        qvp_page_free(page);
    }
    if let Some(ab) = page_bytes("atlas.qva") {
        unsafe {
            let a = qvp_atlas_load(ab.as_ptr(), ab.len());
            assert!(!a.is_null());
            assert_eq!(qvp_atlas_page_of(a, 2, 255), 42);
            assert_eq!(qvp_atlas_page_of(a, 1, 1), 1);
            assert_eq!(qvp_atlas_page_of(a, 114, 6), 604);
            assert_eq!(qvp_atlas_division_of(a, 0, 78, 1), 30);
            let mut r = QvpAtlasRubuAlHizb { rubu_al_hizb: 0, surah: 0, ayah: 0, page: 0 };
            assert_eq!(qvp_atlas_division(a, 0, 30, &mut r), 1);
            assert_eq!((r.surah, r.ayah, r.page), (78, 1, 582));
            let mut pj = [0u16; 2];
            assert_eq!(qvp_atlas_pages_of_juz(a, 30, pj.as_mut_ptr()), 1);
            assert_eq!(pj, [582, 604]);
            let mut su = std::mem::zeroed::<QvpAtlasSurah>();
            assert_eq!(qvp_atlas_surah(a, 2, &mut su), 1);
            // The bundle writes the bare name: "Baqarah", not "Al-Baqarah".
            assert_eq!(s(&su.latin), "Baqarah");
            let mut found = [0u16; 8];
            assert_eq!(qvp_atlas_search_surahs(a, b"cow".as_ptr(), 3, found.as_mut_ptr(), 8), 1);
            assert_eq!(found[0], 2);
            qvp_atlas_free(a);
        }
    }
}

/// A passage through the C ABI: 2:255 from page 42, laid out, cut to two rows, and the
/// error returns.
#[test]
fn passage_end_to_end() {
    let Some(bytes) = page_bytes("042.qvp") else {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!("dist/pages/042.qvp is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
        }
        eprintln!("skip: dist/pages/042.qvp is missing; run scripts/sync-test-data.sh to enable this test");
        return;
    };
    unsafe {
        let page = qvp_page_load(bytes.as_ptr(), bytes.len());
        assert!(!page.is_null());
        let pages = [page as *const _];
        // An ayah that is not on the page, and a page given twice, load nothing.
        assert!(qvp_passage_load(pages.as_ptr(), 1, 2, 280, 280).is_null());
        assert!(qvp_passage_load(pages.as_ptr(), 1, 2, 1, 1).is_null());
        let twice = [page as *const _, page as *const _];
        assert!(qvp_passage_load(twice.as_ptr(), 2, 2, 255, 255).is_null());

        let passage = qvp_passage_load(pages.as_ptr(), 1, 2, 255, 255);
        assert!(!passage.is_null());
        // The passage keeps what it needs: the page can go.
        qvp_page_free(page);
        assert_eq!(qvp_passage_ayah_count(passage), 1);
        let mut text = QvpStr { ptr: std::ptr::null(), len: 0 };
        qvp_passage_ayah_text(passage, 0, &mut text);
        assert_eq!(s(&text).split(' ').count(), 50, "{}", s(&text));
        assert!(qvp_passage_line_spacing(passage) > 30.0);

        let mut spec = QvpPassageSpec {
            width: 366.0,
            scale: 0.9,
            line_spacing: 1.0,
            padding: 2.0,
            align: 0, // QVP_ALIGN_RIGHT
            keep_ayah_mark: 0,
            _pad: [0; 2],
            max_rows: 0,
            ellipsis_width: 0.0,
            max_stretch: 0.0, // the default
        };
        // The spec grew by its last field, the justified gaps' cap.
        assert_eq!(std::mem::size_of::<QvpPassageSpec>(), 32);
        let mut layout = QvpPassageLayout::default();
        assert_eq!(qvp_passage_layout(passage, &spec, &mut layout), 1);
        assert!(layout.n_rows > 1 && layout.n_words > 40 && layout.is_truncated == 0);
        assert_eq!(qvp_passage_rows(passage, std::ptr::null_mut(), 0), layout.n_rows);
        let mut words = vec![std::mem::zeroed::<QvpPassageWord>(); layout.n_words as usize];
        assert_eq!(qvp_passage_words(passage, words.as_mut_ptr(), layout.n_words), layout.n_words);
        assert!(words.iter().all(|w| w.page == 42 && w.surah == 2 && w.ayah == 255));
        assert_eq!((words[0].word, words[0].row), (1, 0));
        // The word alone leaves out its signs. Every word's own box is inside its box with
        // signs, and the word that closes the ayah holds the medallion, to its left.
        assert_eq!(std::mem::size_of::<QvpPassageWord>(), 44);
        assert!(words.iter().all(|w| w.word_x0 >= w.x0 && w.word_x1 <= w.x1 && w.word_y0 >= w.y0 && w.word_y1 <= w.y1));
        let closing = words[words.len() - 1];
        assert!(closing.word_x0 - closing.x0 > 10.0, "{} {}", closing.x0, closing.word_x0);
        let mut ayahs = vec![std::mem::zeroed::<QvpPassageAyah>(); 2];
        assert_eq!(qvp_passage_ayahs(passage, ayahs.as_mut_ptr(), 2), 1);
        let mut draws = vec![0u32; layout.n_draws as usize * 3];
        assert_eq!(qvp_passage_draw_list(passage, draws.as_mut_ptr(), layout.n_draws), layout.n_draws);
        let n_placements = qvp_passage_placements(passage, std::ptr::null_mut(), 0);
        assert!(draws.chunks(3).all(|d| d[0] == 42 && d[2] < n_placements));

        // Two rows, then an ellipsis and the medallion.
        spec.max_rows = 2;
        spec.keep_ayah_mark = 1;
        spec.ellipsis_width = 12.0;
        assert_eq!(qvp_passage_layout(passage, &spec, &mut layout), 1);
        assert_eq!((layout.n_rows, layout.is_truncated), (2, 1));
        assert!(layout.ellipsis_x1 - layout.ellipsis_x0 > 11.9);

        // Justified with no cap: every row but the last reaches both edges, and the last is centred.
        spec.max_rows = 0;
        spec.keep_ayah_mark = 0;
        spec.ellipsis_width = 0.0;
        spec.align = 2; // QVP_ALIGN_JUSTIFIED
        spec.max_stretch = -1.0;
        assert_eq!(qvp_passage_layout(passage, &spec, &mut layout), 1);
        let mut rows = vec![std::mem::zeroed::<QvpPassageRow>(); layout.n_rows as usize];
        assert_eq!(qvp_passage_rows(passage, rows.as_mut_ptr(), layout.n_rows), layout.n_rows);
        let (inner_left, inner_right) = (spec.padding, spec.width - spec.padding);
        for r in &rows[..rows.len() - 1] {
            assert!((r.x0 - inner_left).abs() < 0.01 && (r.x1 - inner_right).abs() < 0.01, "{} {}", r.x0, r.x1);
        }
        let last = rows[rows.len() - 1];
        assert!(((last.x0 - inner_left) - (inner_right - last.x1)).abs() < 0.01);

        // A spec out of range keeps the last layout.
        spec.align = 7;
        assert_eq!(qvp_passage_layout(passage, &spec, &mut layout), 0);
        assert_eq!(qvp_passage_rows(passage, std::ptr::null_mut(), 0), layout.n_rows);
        qvp_passage_free(passage);
    }
}

#[test]
fn basmalah_end_to_end() {
    let (Some(two), Some(page42)) = (page_bytes("002.qvp"), page_bytes("042.qvp")) else {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!("dist/pages is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
        }
        eprintln!("skip: dist/pages is missing; run scripts/sync-test-data.sh to enable this test");
        return;
    };
    unsafe {
        let page = qvp_page_load(two.as_ptr(), two.len());
        let other = qvp_page_load(page42.as_ptr(), page42.len());
        assert!(!page.is_null() && !other.is_null());
        // Surah 1's basmalah is its first ayah, surah 9 has none, and page 42 prints no basmalah.
        let pages = [page as *const _];
        assert!(qvp_passage_load_basmalah(pages.as_ptr(), 1, 1).is_null());
        assert!(qvp_passage_load_basmalah(pages.as_ptr(), 1, 9).is_null());
        assert!(qvp_passage_load_basmalah([other as *const _].as_ptr(), 1, 2).is_null());

        let basmalah = qvp_passage_load_basmalah(pages.as_ptr(), 1, 2);
        assert!(!basmalah.is_null());
        qvp_page_free(page);
        qvp_page_free(other);
        assert_eq!(qvp_passage_ayah_count(basmalah), 0);
        assert!(qvp_passage_line_spacing(basmalah) > 0.0);
        let spec = QvpPassageSpec {
            width: 366.0,
            scale: 0.9,
            line_spacing: 1.0,
            padding: 2.0,
            align: 1, // QVP_ALIGN_CENTER
            keep_ayah_mark: 0,
            _pad: [0; 2],
            max_rows: 0,
            ellipsis_width: 0.0,
            max_stretch: 0.0,
        };
        let mut layout = QvpPassageLayout::default();
        assert_eq!(qvp_passage_layout(basmalah, &spec, &mut layout), 1);
        assert_eq!((layout.n_rows, layout.n_words, layout.n_ayahs), (1, 0, 0));
        assert!(layout.n_draws > 0);
        let mut draws = vec![0u32; layout.n_draws as usize * 3];
        assert_eq!(qvp_passage_draw_list(basmalah, draws.as_mut_ptr(), layout.n_draws), layout.n_draws);
        assert!(draws.chunks(3).all(|d| d[0] == 2));
        qvp_passage_free(basmalah);
    }
}

/// The taxonomy is the contract: these names are the quran-svg `mark-taxonomy` v2
/// vocabulary, and every wrapper mirrors this table by index. Changing one here
/// means changing `qvp.h`, `web/qvp.js`, Kotlin, Dart, React Native and `docs/API.md`.
#[test]
fn taxonomy_names_are_the_pipeline_vocabulary() {
    unsafe fn name(f: unsafe extern "C" fn(u8, *mut QvpStr), i: u8) -> String {
        let mut o = QvpStr { ptr: std::ptr::null(), len: 0 };
        f(i, &mut o);
        s(&o)
    }
    unsafe {
        let marks: Vec<String> = (0u8..=35).map(|i| name(qvp_mark_name, i)).collect();
        assert_eq!(
            marks,
            [
                "",
                "fathah",
                "kasrah",
                "dammah",
                "tanwin_al_fath",
                "tanwin_al_kasr",
                "tanwin_al_damm",
                "shaddah",
                "sukun",
                "maddah",
                "hamzah",
                "hamzat_al_wasl",
                "omitted_alif",
                "small_waw",
                "small_yaa",
                "small_noon",
                "dot",
                "two_dots",
                "three_dots",
                "rounded_zero",
                "rectangular_zero",
                "waqf_jaiz_mustawi_al_tarafayn",
                "waqf_jaiz_waqf_awla",
                "waqf_jaiz_wasl_awla",
                "waqf_lazim",
                "waqf_al_muanaqah",
                "saktah",
                "small_meem",
                "hizb",
                "sajdah",
                "sajdah_mark",
                "sajdah_line",
                "seen_al_qiraah",
                "tashil",
                "ishmam",
                "imalah",
            ]
        );
        assert_eq!(name(qvp_mark_name, 255), "unknown");
        for (i, m) in marks.iter().enumerate().skip(1) {
            assert_eq!(qvp_mark_from_name(m.as_ptr(), m.len() as u32), i as u8, "round trip for {m}");
        }
        assert_eq!(qvp_mark_from_name(b"not-a-mark".as_ptr(), 10), 255);

        let families: Vec<String> = (0u8..=7).map(|i| name(qvp_family_name, i)).collect();
        assert_eq!(families, ["", "diacritic", "diacritic tanwin", "dots", "waqf", "sifr", "sajdah", "reading_sign"]);

        let categories: Vec<String> = (0u8..=8).map(|i| name(qvp_category_name, i)).collect();
        assert_eq!(
            categories,
            ["", "harakah", "tanwin", "letter_dot", "orthographic", "dabt", "waqf", "reading_sign", "standalone"]
        );

        let kinds: Vec<String> = (0u8..=7).map(|i| name(qvp_kind_name, i)).collect();
        assert_eq!(
            kinds,
            [
                "body",
                "mark",
                "ayah_number",
                "ayah_mark_ornament",
                "header_ink",
                "ornament",
                "page_number",
                "running_head"
            ]
        );
    }
}

/// Three ornaments with one filled and one stroked square each, built in memory: enough
/// to prove the whole dress path over the ABI without a licensed artwork fixture.
fn ornament_bytes() -> Vec<u8> {
    use qvp_core::qvp_format::ornaments::*;
    use qvp_core::qvp_format::Cmd;
    let mut set = OrnamentSet::default();
    let q = set.quant as i32;
    let unit = |w: i32, h: i32| {
        vec![Cmd::MoveTo(0, 0), Cmd::LineTo(w * q, 0), Cmd::LineTo(w * q, h * q), Cmd::LineTo(0, h * q), Cmd::Close]
    };
    let parts = vec![
        Part { name: "c1".into(), color: 0x00649bff, stroke: false },
        Part { name: "line".into(), color: 0x303641ff, stroke: true },
    ];
    let asset = |w: i32, h: i32, slot: [f32; 4], set: &mut OrnamentSet| {
        let first = set.paths.len() as u32;
        set.push_path(0, 0, 0, &unit(w, h));
        set.push_path(1, OPF_STROKE, q / 10, &unit(w, h));
        Asset { view_box: [0.0, 0.0, w as f32, h as f32], slot: Some(slot), first_path: first, n_paths: 2 }
    };
    let mark = asset(80, 100, [20.0, 20.0, 40.0, 60.0], &mut set);
    let header = asset(800, 100, [100.0, 10.0, 600.0, 80.0], &mut set);
    let frame = asset(70, 100, [5.0, 5.0, 60.0, 90.0], &mut set);
    let orn = |kind, asset, style: &str, lineage| Ornament {
        kind,
        style: style.into(),
        lineage,
        riwayah: "Hafs 'an 'Asim".into(),
        license: License {
            id: "CC-BY-NC-SA-4.0".into(),
            status: "provisional".into(),
            redistributable: false,
            attribution: "a test".into(),
        },
        parts: parts.clone(),
        asset,
        slices: None,
    };
    set.ornaments.push(orn(OrnamentKind::AyahMark, mark, "test", Lineage::Font));
    set.ornaments.push(orn(OrnamentKind::SurahHeader, header, "test", Lineage::Scan));
    set.ornaments.push(orn(OrnamentKind::PageFrame, frame, "test", Lineage::Scan));
    set.encode()
}

#[test]
fn ornament_and_dress_calls_take_null_handles() {
    let orn = ornament_bytes();
    unsafe {
        let set = qvp_ornaments_load(orn.as_ptr(), orn.len());
        assert!(!set.is_null());
        let none = std::ptr::null::<Ornaments>();
        assert_eq!(qvp_ornament_count(none), 0);
        assert_eq!(qvp_ornament_find(none, 0, b"test".as_ptr(), 4), -1);
        assert_eq!(qvp_ornament_find(set, 99, b"test".as_ptr(), 4), -1);
        assert_eq!(qvp_ornament_find(set, 256, b"test".as_ptr(), 4), -1);
        let mut o = std::mem::zeroed::<QvpOrnament>();
        assert_eq!(qvp_ornament_info(none, 0, &mut o), 0);
        assert_eq!(qvp_ornament_info(set, 0, std::ptr::null_mut()), 0);
        let mut part = std::mem::zeroed::<QvpOrnamentPart>();
        assert_eq!(qvp_ornament_part(none, 0, 0, &mut part), 0);
        assert_eq!(qvp_ornament_part(set, 0, 0, std::ptr::null_mut()), 0);

        let page = std::ptr::null_mut();
        let mut di = std::mem::zeroed::<QvpDressInfo>();
        assert_eq!(qvp_dress_info(page, &mut di), 0);
        qvp_undress(page);
        let view = QvpView { scale: 1.0, offset_x: 0.0, offset_y: 0.0 };
        let mut out = QvpView { scale: 7.0, offset_x: 7.0, offset_y: 7.0 };
        qvp_dress_clamp(page, &view, 100.0, 100.0, &mut out);
        qvp_dress_view(page, 1.0, 100.0, &mut out, std::ptr::null_mut());
        assert_eq!(out.scale, 7.0, "a null page writes nothing");
        let mut g = std::mem::zeroed::<QvpDressGeometry>();
        g.n_draws = 7;
        qvp_dress_geometry(page, &mut g);
        assert_eq!((g.n_draws, g.ops.is_null()), (0, true), "a null page is not dressed");
        qvp_dress_geometry(page, std::ptr::null_mut());
        qvp_ornaments_free(set);
    }
}

#[test]
fn dress_over_the_abi() {
    let Some(bytes) = page_bytes("604.qvp") else {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!("dist/pages/604.qvp is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
        }
        eprintln!("skip: dist/pages/604.qvp is missing; run scripts/sync-test-data.sh to enable this test");
        return;
    };
    let orn = ornament_bytes();
    unsafe {
        let page = qvp_page_load(bytes.as_ptr(), bytes.len());
        assert!(!page.is_null());
        let set = qvp_ornaments_load(orn.as_ptr(), orn.len());
        assert!(!set.is_null());
        assert_eq!(qvp_ornament_count(set), 3);
        assert_eq!(qvp_ornament_find(set, 2, b"test".as_ptr(), 4), 2);
        assert_eq!(qvp_ornament_find(set, 0, b"nope".as_ptr(), 4), -1);
        assert_eq!(qvp_ornament_find(set, 99, b"test".as_ptr(), 4), -1, "no such kind");
        assert_eq!(qvp_ornament_find(set, 256, b"test".as_ptr(), 4), -1, "a kind past u8 does not wrap to 0");

        let mut o = std::mem::zeroed::<QvpOrnament>();
        assert_eq!(qvp_ornament_info(set, 0, &mut o), 1);
        assert_eq!((o.kind, o.lineage, o.n_parts, o.has_slices, o.redistributable), (0, 1, 2, 0, 0));
        assert_eq!(s(&o.style), "test");
        assert_eq!(s(&o.license_id), "CC-BY-NC-SA-4.0");
        assert_eq!(qvp_ornament_info(set, 3, &mut o), 0);
        let mut part = std::mem::zeroed::<QvpOrnamentPart>();
        assert_eq!(qvp_ornament_part(set, 0, 1, &mut part), 1);
        assert_eq!((s(&part.name), part.color, part.stroke), ("line".into(), 0x303641ff, 1));

        let mut pi = std::mem::zeroed::<QvpPageInfo>();
        qvp_page_info(page, &mut pi);
        let mut di = std::mem::zeroed::<QvpDressInfo>();
        assert_eq!(qvp_dress_info(page, &mut di), 0, "undressed");

        // the printed rings are visible before the dress
        let ring =
            (0..pi.n_paths).find(|i| qvp_color_of(page, *i) != 0 && path_kind(page, *i) == 3).expect("a printed ring");

        let colors: [u32; 3] = [2, 0, 0xff0000ff]; // repaint the border's fill red
        let spec = QvpDressSpec {
            ayah_mark: 0,
            surah_header: 1,
            page_frame: 2,
            gap: 5.0,
            band_gap: 5.0,
            ayah_mark_scale: 1.0,
            page_frame_scale: 1.0,
            line_art: 0,
            colors: colors.as_ptr(),
            n_colors: 1,
        };
        assert_eq!(qvp_dress(page, set, &spec), 1);
        assert_eq!(qvp_dress_info(page, &mut di), 1);
        assert!(di.n_ayah_marks > 0 && di.n_surah_headers > 0, "{} {}", di.n_ayah_marks, di.n_surah_headers);
        assert_eq!((di.frame_stretched, di.n_frame_repeats), (1, 0), "no slices in this set");
        let vb = di.view_box;
        assert!(vb[0] < 0.0 && vb[1] < 0.0 && vb[2] > pi.width && vb[3] > pi.height, "{vb:?}");
        assert_eq!(qvp_color_of(page, ring) & 0xff, 0, "the print's own ring is hidden");

        // the border goes around the laid-out page: a taller layout grows it, unasked
        let spec_layout = QvpLayoutSpec {
            viewport_w: 690.0,
            viewport_h: 1100.0,
            pad_top: 20.0,
            pad_bottom: 20.0,
            pad_left: 0.0,
            pad_right: 0.0,
            line_spacing: 1.0,
            fill_height: 0,
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
        let mut lay = std::mem::zeroed::<QvpLayout>();
        qvp_layout(page, &spec_layout, &mut lay);
        let before = di.revision;
        assert_eq!(qvp_dress_info(page, &mut di), 1);
        assert_ne!(di.revision, before, "a layout rebuilds the ornament layer");
        assert!(di.overflow.iter().all(|v| *v > 0.0), "{:?}", di.overflow);
        let printed_h = di.view_box[3];
        let tall = QvpLayoutSpec { viewport_h: 2000.0, fill_height: 1, ..spec_layout };
        qvp_layout(page, &tall, &mut lay);
        assert_eq!(qvp_dress_info(page, &mut di), 1);
        assert!(di.view_box[3] > printed_h, "{printed_h} -> {}", di.view_box[3]);
        // the layout's fit holds the border: content plus overflow inside the viewport
        assert_eq!(qvp_layout_current(page, &mut lay), 1);
        let (w, h) = (lay.content_w + di.overflow[0] + di.overflow[2], lay.content_h + di.overflow[1] + di.overflow[3]);
        assert!(lay.fit_y - lay.fit_scale * di.overflow[1] >= -1e-3, "{} {}", lay.fit_y, lay.fit_scale);
        assert!(lay.fit_y + lay.fit_scale * (h - di.overflow[1]) <= tall.viewport_h + 1e-3);
        assert!(lay.fit_x - lay.fit_scale * di.overflow[0] >= -1e-3 && lay.fit_scale * w <= tall.viewport_w + 1e-3);
        // a drag past the top stops at the border's top edge
        let pushed = QvpView { scale: 3.0, offset_x: 0.0, offset_y: 1e5 };
        let mut held = QvpView { scale: 0.0, offset_x: 0.0, offset_y: 0.0 };
        qvp_dress_clamp(page, &pushed, tall.viewport_w, tall.viewport_h, &mut held);
        assert!((held.offset_y - held.scale * di.overflow[1]).abs() < 1e-2, "{} {}", held.offset_y, held.scale);
        // a host that scrolls the page: the border's top at 0, the size the whole dressed box
        let mut shown = QvpView { scale: 0.0, offset_x: 0.0, offset_y: 0.0 };
        let mut size = [0.0f32; 2];
        qvp_dress_view(page, 1.0, 2000.0, &mut shown, size.as_mut_ptr());
        assert_eq!(shown.scale, 1.0);
        assert!((shown.offset_y - di.overflow[1]).abs() < 1e-3, "{}", shown.offset_y);
        assert!((size[0] - w).abs() < 1e-3 && (size[1] - h).abs() < 1e-3, "{size:?} vs {w} {h}");
        assert!((shown.offset_x - di.overflow[0] - (2000.0 - w) / 2.0).abs() < 1e-3, "{}", shown.offset_x);

        let mut g = std::mem::zeroed::<QvpDressGeometry>();
        qvp_dress_geometry(page, &mut g);
        assert_eq!(g.n_draws, di.n_draws);
        assert_eq!(
            g.n_draws,
            (di.n_ayah_marks + di.n_surah_headers + 1) * 2,
            "one filled and one stroked outline per placed asset"
        );
        let table = std::slice::from_raw_parts(g.table, g.n_draws as usize * 9);
        let mut red = 0;
        for d in table.as_chunks::<9>().0 {
            let (color, flags, width, placement) = (d[4], d[5], f32::from_bits(d[6]), d[8]);
            assert!(d[1] > 0 && d[3] > 0);
            let kind = flags & 0xff;
            assert!(kind <= 2, "ornament kind {kind}");
            if flags & QVP_DRAW_STROKE != 0 {
                assert!(width > 0.0, "a stroke needs a width in page units");
                assert_eq!(color, 0x303641ff);
            } else if kind == 2 {
                assert_eq!(color, 0xff0000ff, "the border's fill was repainted");
                red += 1;
            } else {
                assert_eq!(color, 0x00649bff);
            }
            // the border belongs to no placement; the rest are drawn under one
            assert_eq!(placement == u32::MAX, kind == 2, "kind {kind} placement {placement}");
        }
        assert_eq!(red, 1);

        // line art draws one ink, and only one
        let spec = QvpDressSpec { line_art: QVP_LINE_ART_ALL, ..spec };
        assert_eq!(qvp_dress(page, set, &spec), 1);
        qvp_dress_geometry(page, &mut g);
        assert_eq!(g.n_draws, di.n_draws / 2);
        // line art for the bands alone: each band loses its fill, nothing else changes
        let spec = QvpDressSpec { line_art: QVP_LINE_ART_SURAH_HEADER, ..spec };
        assert_eq!(qvp_dress(page, set, &spec), 1);
        qvp_dress_geometry(page, &mut g);
        assert_eq!(g.n_draws, di.n_draws - di.n_surah_headers);

        // an ornament of the wrong kind, or none at all, changes nothing
        assert_eq!(qvp_dress(page, set, &QvpDressSpec { ayah_mark: 1, ..spec }), 0);
        assert_eq!(qvp_dress_info(page, &mut di), 0);
        assert_eq!(
            qvp_dress(
                page,
                set,
                &QvpDressSpec { ayah_mark: u32::MAX, surah_header: u32::MAX, page_frame: u32::MAX, ..spec }
            ),
            0
        );

        assert_eq!(qvp_dress(page, set, &spec), 1);
        qvp_undress(page);
        assert_eq!(qvp_dress_info(page, &mut di), 0);
        assert_ne!(qvp_color_of(page, ring) & 0xff, 0, "undressing puts the printed rings back");

        qvp_ornaments_free(set);
        qvp_page_free(page);
    }
}

/// `flags` byte 0 of the page geometry table.
unsafe fn path_kind(page: *mut qvp_core::Page, i: u32) -> u32 {
    let mut g = std::mem::zeroed::<QvpGeometry>();
    qvp_geometry(page, &mut g);
    *g.table.add(i as usize * 8 + 4) & 0xff
}
