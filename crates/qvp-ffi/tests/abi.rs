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
        // a short buffer gets the prefix and the total, as the header says every array output does
        let mut short = vec![0f32; (n_groups as usize - 1) * 4];
        assert_eq!(qvp_layout_groups(page, short.as_mut_ptr(), n_groups - 1), n_groups);
        assert_eq!(short, groups[..short.len()]);
        let mut all_pg = vec![0u32; n_pg as usize];
        assert_eq!(qvp_layout_path_groups(page, all_pg.as_mut_ptr(), n_pg), n_pg);
        let mut short_pg = vec![u32::MAX; n_pg as usize - 1];
        assert_eq!(qvp_layout_path_groups(page, short_pg.as_mut_ptr(), n_pg - 1), n_pg);
        assert_eq!(short_pg, all_pg[..short_pg.len()]);
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

/// A page cut short is null from `qvp_page_load`, not a crash: the file a host hands the
/// engine may have been truncated by the network.
#[test]
fn a_truncated_page_loads_as_null() {
    let Some(bytes) = page_bytes("036.qvp") else {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!("dist/pages/036.qvp is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
        }
        eprintln!("skip: dist/pages/036.qvp is missing; run scripts/sync-test-data.sh to enable this test");
        return;
    };
    unsafe {
        for n in [0, 5, 63, 64, 1000, bytes.len() / 2, bytes.len() - 1] {
            assert!(qvp_page_load(bytes.as_ptr(), n).is_null(), "{n} of {} bytes loaded", bytes.len());
        }
        let page = qvp_page_load(bytes.as_ptr(), bytes.len());
        assert!(!page.is_null());
        qvp_page_free(page);
    }
}

/// The spec a phone in portrait asks for, at a reflow zoom (1 is the printed page).
fn phone_spec(reflow_zoom: f32) -> QvpLayoutSpec {
    QvpLayoutSpec {
        viewport_w: 390.0,
        viewport_h: 844.0,
        pad_top: 20.0,
        pad_bottom: 20.0,
        pad_left: 12.0,
        pad_right: 12.0,
        line_spacing: 1.0,
        fill_height: 1,
        grid_lines: 0,
        crop_left: 0.0,
        crop_right: 0.0,
        max_aspect_slack: 0.0,
        reflow_zoom,
        reflow_fill: 255,
        reflow_breaks: 255,
        reflow_gaps: 0,
        reflow_word_gap: 0.0,
        reflow_max_stretch: 0.0,
        reflow_relax: -1.0,
        banner_zoom: 0.0,
        surah_frames: 1,
    }
}

/// What a wrapper does every frame, the count with no buffer and then the fill, hands back
/// the list a page laid out fresh gives in one call: for the whole page and for bands, across
/// relayouts, printed and reflowed.
#[test]
fn draw_list_count_then_fill_matches_a_fresh_layout() {
    let Some(bytes) = page_bytes("036.qvp") else {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!("dist/pages/036.qvp is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
        }
        eprintln!("skip: dist/pages/036.qvp is missing; run scripts/sync-test-data.sh to enable this test");
        return;
    };
    unsafe {
        let page = qvp_page_load(bytes.as_ptr(), bytes.len());
        let fresh = qvp_page_load(bytes.as_ptr(), bytes.len());
        let mut info = std::mem::zeroed::<QvpPageInfo>();
        qvp_page_info(page, &mut info);
        let mut lay = std::mem::zeroed::<QvpLayout>();
        let bands = [(0.0, 0.0), (0.0, 400.0), (300.0, 900.0), (0.0, 400.0), (2000.0, 2600.0)];
        // 1 is the printed page; the repeat of 2 lays the page out again after a reflow
        for zoom in [1.0, 2.0, 3.0, 2.0, 1.0] {
            let spec = phone_spec(zoom);
            qvp_layout(page, &spec, &mut lay);
            for &(top, bottom) in &bands {
                let n = qvp_layout_draw_list(page, top, bottom, std::ptr::null_mut(), 0);
                let mut got = vec![0u32; n as usize * 2];
                assert_eq!(qvp_layout_draw_list(page, top, bottom, got.as_mut_ptr(), n), n);

                qvp_layout(fresh, &spec, &mut lay);
                let mut want = vec![0u32; info.n_paths as usize * 4];
                let m = qvp_layout_draw_list(fresh, top, bottom, want.as_mut_ptr(), info.n_paths * 2);
                want.truncate(m as usize * 2);
                assert_eq!(got, want, "zoom {zoom}, band {top}..{bottom}");
                assert!(n > 0 || top >= 2000.0, "zoom {zoom}, band {top}..{bottom} drew nothing");
            }
            let n = qvp_layout_placements(page, std::ptr::null_mut(), 0);
            let mut got = vec![0f32; n as usize * 4];
            assert_eq!(qvp_layout_placements(page, got.as_mut_ptr(), n), n);
            let mut want = vec![0f32; n as usize * 4];
            assert_eq!(qvp_layout_placements(fresh, want.as_mut_ptr(), n), n);
            assert_eq!(got, want, "zoom {zoom}");
        }
        qvp_page_free(fresh);
        qvp_page_free(page);
    }
}

/// The ink band over the ABI: a printed page fits and is one band, a page reflowed past the
/// ceiling is drawn a band at a time, and the band feeds the draw list as it comes back.
#[test]
fn ink_band_bands_a_tall_page_and_feeds_the_draw_list() {
    let Some(bytes) = page_bytes("036.qvp") else {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!("dist/pages/036.qvp is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
        }
        eprintln!("skip: dist/pages/036.qvp is missing; run scripts/sync-test-data.sh to enable this test");
        return;
    };
    unsafe {
        let page = qvp_page_load(bytes.as_ptr(), bytes.len());
        let mut lay = std::mem::zeroed::<QvpLayout>();
        let spec = phone_spec(1.0);
        qvp_layout(page, &spec, &mut lay);
        let view = QvpView { scale: 1.0, offset_x: 0.0, offset_y: 0.0 };
        let mut band = QvpInkBand { top: -1.0, height: -1.0 };
        qvp_layout_ink_band(page, &view, spec.viewport_w, spec.viewport_h, 0.0, 0.0, &mut band);
        assert_eq!((band.top, band.height), (0.0, lay.content_h), "the printed page is one band");

        // a ceiling under the content bands it, two screens tall around the reader
        let scrolled = QvpView { offset_y: -300.0, ..view };
        qvp_layout_ink_band(page, &scrolled, spec.viewport_w, 200.0, lay.content_h / 2.0, 0.0, &mut band);
        assert_eq!(band.height, 400.0);
        assert!(band.top <= 300.0 && band.top + band.height >= 500.0, "{} + {}", band.top, band.height);
        let n = qvp_layout_draw_list(page, band.top, band.top + band.height, std::ptr::null_mut(), 0);
        assert!(n > 0, "the band draws something");

        // a pixel budget is a ceiling of its own: the width times the rows it allows
        qvp_layout_ink_band(page, &view, 400.0, 200.0, 0.0, 400.0 * 300.0, &mut band);
        assert_eq!(band.height, 300.0);
        // no viewport height: the whole page
        qvp_layout_ink_band(page, &scrolled, 400.0, 0.0, 1.0, 0.0, &mut band);
        assert_eq!((band.top, band.height), (0.0, lay.content_h));
        qvp_page_free(page);
    }
}

/// A negative `gap_bias` is the engine's default, as a `max_distance` of 0 is, so a wrapper
/// that leaves both to the engine taps the same word as one that passes no options.
#[test]
fn a_negative_gap_bias_is_the_default() {
    let Some(bytes) = page_bytes("036.qvp") else {
        if std::env::var("QVP_REQUIRE_DATA").is_ok() {
            panic!("dist/pages/036.qvp is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh");
        }
        eprintln!("skip: dist/pages/036.qvp is missing; run scripts/sync-test-data.sh to enable this test");
        return;
    };
    unsafe {
        let page = qvp_page_load(bytes.as_ptr(), bytes.len());
        let mut line = std::mem::zeroed::<QvpLineInfo>();
        assert_eq!(qvp_line_info(page, 3, &mut line), 1);
        let y = (line.y0 + line.y1) / 2.0;
        let hit = |opt: *const QvpHitOptions, x: f32| {
            let mut h = std::mem::zeroed::<QvpHit>();
            assert_eq!(qvp_hit_test(page, x, y, opt, &mut h), 1);
            h.word
        };
        let default = QvpHitOptions { max_distance: 0.0, gap_bias: -1.0, prefer_exact: 1 };
        let none = QvpHitOptions { max_distance: 0.0, gap_bias: 0.0, prefer_exact: 1 };
        let mut moved = 0;
        for i in 0..=400 {
            let x = line.x0 + (line.x1 - line.x0) * i as f32 / 400.0;
            assert_eq!(hit(&default, x), hit(std::ptr::null(), x), "x {x}");
            moved += (hit(&none, x) != hit(std::ptr::null(), x)) as u32;
        }
        // the bias decides some taps on this line, so the comparison above is not empty
        assert!(moved > 0);
        // the hit areas read a negative bias the same way, so an overlay still matches the taps
        type Areas = unsafe extern "C" fn(*const qvp_core::Page, f32, *mut QvpHitArea, u32) -> u32;
        let areas = |f: Areas, bias: f32| {
            let n = f(page, bias, std::ptr::null_mut(), 0);
            let mut out = vec![std::mem::zeroed::<QvpHitArea>(); n as usize];
            assert_eq!(f(page, bias, out.as_mut_ptr(), n), n);
            out.iter().map(|a| (a.word, a.line, a.x0, a.y0, a.x1, a.y1, a.ink_x0, a.ink_x1)).collect::<Vec<_>>()
        };
        for f in [qvp_hit_areas as Areas, qvp_hit_areas_view] {
            assert_eq!(areas(f, -1.0), areas(f, 0.6));
            assert_ne!(areas(f, 0.0), areas(f, 0.6));
        }
        qvp_page_free(page);
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

// ───────────── the letter layer ─────────────

use qvp_core::qvp_format::letters::{
    encode_letters, LetterData, LetterRec, MemberPath, MemberRec, PieceRec, WordMarkRec, NO_AT,
};
use qvp_core::qvp_format::{
    encode, encode_cmds, AyahRec, Cmd, Family, Header, IBox, LineRec, Mark, PageData, PathKind, PathRec, WordRec,
    NONE_U16, PF_EVENODD, VERSION,
};

fn square(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<Cmd> {
    vec![Cmd::MoveTo(x0, y0), Cmd::LineTo(x1, y0), Cmd::LineTo(x1, y1), Cmd::LineTo(x0, y1), Cmd::Close]
}

/// The letter fixture's page, a copy of `letters_page_data` in `crates/qvp-core/src/fixtures.rs`:
/// three words on two lines, eight paths, quant 100. Path 0 joins two letters.
fn letters_page_data() -> PageData {
    let mut ops = Vec::new();
    let mut paths: Vec<PathRec> = Vec::new();
    let mut add = |cmds: &[Cmd], kind: PathKind, mark: Mark, family: Family| -> IBox {
        let bbox = qvp_core::qvp_format::cmds_bbox(cmds);
        let op_off = ops.len() as u32;
        encode_cmds(cmds, bbox.x0, bbox.y0, &mut ops);
        paths.push(PathRec {
            kind,
            mark,
            family,
            flags: PF_EVENODD,
            ox: bbox.x0,
            oy: bbox.y0,
            op_off,
            op_len: ops.len() as u32 - op_off,
            bbox,
        });
        bbox
    };
    let mut w0 = add(&square(4000, 1000, 6000, 2000), PathKind::Body, Mark::None, Family::None);
    w0.union(&add(&square(5400, 2200, 5600, 2400), PathKind::Mark, Mark::Dot, Family::Dots));
    w0.union(&add(&square(5400, 600, 5600, 800), PathKind::Mark, Mark::Fathah, Family::Diacritic));
    let mut w1 = add(&square(2000, 1000, 3000, 2000), PathKind::Body, Mark::None, Family::None);
    w1.union(&add(&square(1000, 1000, 2000, 2000), PathKind::Body, Mark::None, Family::None));
    w1.union(&add(&square(1400, 400, 1600, 600), PathKind::Mark, Mark::WaqfJaizWaslAwla, Family::Waqf));
    let mut w2 = add(&square(1000, 5000, 2000, 6000), PathKind::Body, Mark::None, Family::None);
    w2.union(&add(&square(1400, 4600, 1600, 4800), PathKind::Mark, Mark::OmittedAlif, Family::Diacritic));
    let mut l1 = w0;
    l1.union(&w1);
    let word = |i: u16, ayah: u16, n: u16, line: u16, first_path: u32, n_paths: u16, bbox: IBox| WordRec {
        surah: 1,
        ayah,
        word: n,
        line_index: line,
        ayah_index: ayah - 1,
        text: i,
        rasm_imlai: i,
        qpc: i,
        rasm: i,
        search: i,
        first_path,
        n_paths,
        bbox,
    };
    let ayah = |n: u16, first_word: u16, n_words: u16, bbox: IBox| AyahRec {
        surah: 1,
        ayah: n,
        fragment: 1,
        fragments: 1,
        flags: 0,
        first_word,
        n_words,
        ayah_mark_decoration: NONE_U16,
        rubu_al_hizb: 0,
        bbox,
    };
    PageData {
        header: Header { version: VERSION, quant: 100, page: 1, flags: 0, width: 100.0, height: 100.0 },
        lines: vec![
            LineRec { line_number: 1, first_word: 0, n_words: 2, bbox: l1 },
            LineRec { line_number: 2, first_word: 2, n_words: 1, bbox: w2 },
        ],
        ayahs: vec![ayah(1, 0, 2, l1), ayah(2, 2, 1, w2)],
        words: vec![word(0, 1, 1, 0, 0, 3, w0), word(1, 1, 2, 0, 3, 3, w1), word(2, 2, 1, 1, 6, 2, w2)],
        paths,
        decorations: vec![],
        glyphs: vec![],
        insts: vec![],
        ops,
        strings: ["بَل", "لـهۖ", "و"].iter().map(|s| s.to_string()).collect(),
    }
}

/// The letter fixture's layer, a copy of `letters_layer` in `crates/qvp-core/src/fixtures.rs`:
/// path 0 cut into piece 0 (the ba, paths 8 once attached) and piece 1 (the lam, path 9).
fn letters_layer() -> LetterData {
    let letter = |word: u16, at: u16, text: char, more_at: Vec<u16>, first_member: u32, n_members: u16| LetterRec {
        word,
        at,
        text,
        more_at,
        first_member,
        n_members,
    };
    let member = |path: MemberPath, at: u16, is_body: bool| MemberRec { path, at, is_body };
    LetterData {
        page: 1,
        quant: 100,
        n_page_paths: 8,
        n_page_words: 3,
        letters: vec![
            letter(0, 0, 'ب', vec![], 0, 3),
            letter(0, 2, 'ل', vec![], 3, 1),
            letter(1, 0, 'ل', vec![1], 4, 1),
            letter(1, 2, 'ه', vec![], 5, 1),
            letter(2, 0, 'و', vec![], 6, 2),
        ],
        members: vec![
            member(MemberPath::Piece(0), 0, true),
            member(MemberPath::Page(1), 0, false),
            member(MemberPath::Page(2), 1, false),
            member(MemberPath::Piece(1), 2, true),
            member(MemberPath::Page(3), 0, true),
            member(MemberPath::Page(4), 2, true),
            member(MemberPath::Page(6), 0, true),
            member(MemberPath::Page(7), NO_AT, false),
        ],
        word_marks: vec![WordMarkRec { word: 1, path: 5, at: 3 }],
        pieces: vec![
            PieceRec { source: 0, cmds: square(5000, 1000, 6000, 2000) },
            PieceRec { source: 0, cmds: square(3900, 1000, 5000, 2000) },
        ],
    }
}

/// The bytes of a layer for a page. The one place the test encodes a layer, so a change to
/// the encoder's signature is a change to this line.
fn layer_bytes(page: &PageData, layer: &LetterData) -> Vec<u8> {
    encode_letters(layer, page).unwrap()
}

/// The letter fixture's page, loaded through the ABI, with no layer, and the layer's bytes.
unsafe fn letters_page() -> (*mut qvp_core::Page, Vec<u8>) {
    let data = letters_page_data();
    let bytes = encode(&data);
    let page = qvp_page_load(bytes.as_ptr(), bytes.len());
    assert!(!page.is_null());
    (page, layer_bytes(&data, &letters_layer()))
}

/// The fixture's page as printed: 200 × 200 px for a 100 × 100 page.
fn letters_spec() -> QvpLayoutSpec {
    QvpLayoutSpec {
        viewport_w: 200.0,
        viewport_h: 200.0,
        pad_top: 0.0,
        pad_bottom: 0.0,
        pad_left: 0.0,
        pad_right: 0.0,
        line_spacing: 1.0,
        fill_height: 0,
        grid_lines: 0,
        crop_left: 0.0,
        crop_right: 0.0,
        max_aspect_slack: 0.0,
        reflow_zoom: 0.0,
        reflow_fill: 255,
        reflow_breaks: 255,
        reflow_gaps: 0,
        reflow_word_gap: 0.0,
        reflow_max_stretch: 0.0,
        reflow_relax: -1.0,
        banner_zoom: 0.0,
        surah_frames: 1,
    }
}

/// The whole draw list of the current layout as (path, placement) pairs: the count call, then
/// the fill, as a wrapper does it.
unsafe fn draw_pairs(page: *const qvp_core::Page) -> Vec<(u32, u32)> {
    let n = qvp_layout_draw_list(page, 0.0, 0.0, std::ptr::null_mut(), 0);
    let mut out = vec![0u32; n as usize * 2];
    assert_eq!(qvp_layout_draw_list(page, 0.0, 0.0, out.as_mut_ptr(), n), n);
    out.chunks(2).map(|c| (c[0], c[1])).collect()
}

/// The printed draw list of the fixture with path 0 whole, and with it drawn as its pieces.
const PLAIN: [(u32, u32); 8] = [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (6, 1), (7, 1)];
const SWAPPED: [(u32, u32); 9] = [(8, 0), (9, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (6, 1), (7, 1)];
const RED: u32 = 0xff0000ff;
const QVP_NONE: u32 = 0xFFFF_FFFF;
const SEL_LETTER: u8 = 16;
const SEL_LETTER_MARK: u8 = 17;
const SEL_LETTER_MARK_NAMED: u8 = 18;
const SEL_WORD_CHARS: u8 = 19;

fn page_target() -> QvpTarget {
    QvpTarget { target: 0, a: 0, b: 0, c: 0, words: std::ptr::null(), n_words: 0 }
}

#[test]
fn letters_over_the_abi() {
    use std::mem::{offset_of, size_of};
    // the struct layouts the header states and the wrappers write
    assert_eq!(size_of::<QvpLetterInfo>(), 52);
    let info_offsets = [
        offset_of!(QvpLetterInfo, word),
        offset_of!(QvpLetterInfo, index),
        offset_of!(QvpLetterInfo, at),
        offset_of!(QvpLetterInfo, text),
        offset_of!(QvpLetterInfo, n_members),
        offset_of!(QvpLetterInfo, n_marks),
        offset_of!(QvpLetterInfo, more_at),
        offset_of!(QvpLetterInfo, n_more_at),
        offset_of!(QvpLetterInfo, x0),
        offset_of!(QvpLetterInfo, y0),
        offset_of!(QvpLetterInfo, x1),
        offset_of!(QvpLetterInfo, y1),
        offset_of!(QvpLetterInfo, has_ink),
        offset_of!(QvpLetterInfo, _pad),
    ];
    assert_eq!(info_offsets, [0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 49]);
    assert_eq!(size_of::<QvpLetterMember>(), 12);
    let member_offsets = [
        offset_of!(QvpLetterMember, path),
        offset_of!(QvpLetterMember, at),
        offset_of!(QvpLetterMember, mark),
        offset_of!(QvpLetterMember, is_body),
        offset_of!(QvpLetterMember, is_cut),
        offset_of!(QvpLetterMember, _pad),
    ];
    assert_eq!(member_offsets, [0, 4, 8, 9, 10, 11]);
    assert_eq!(size_of::<QvpLetterHit>(), 28);
    let hit_offsets = [
        offset_of!(QvpLetterHit, word),
        offset_of!(QvpLetterHit, letter),
        offset_of!(QvpLetterHit, path),
        offset_of!(QvpLetterHit, at),
        offset_of!(QvpLetterHit, line),
        offset_of!(QvpLetterHit, distance),
        offset_of!(QvpLetterHit, mark),
        offset_of!(QvpLetterHit, is_exact),
        offset_of!(QvpLetterHit, is_body),
        offset_of!(QvpLetterHit, _pad),
    ];
    assert_eq!(hit_offsets, [0, 4, 8, 12, 16, 20, 24, 25, 26, 27]);
    let query_offsets = [
        offset_of!(QvpLetterQuery, scope),
        offset_of!(QvpLetterQuery, text),
        offset_of!(QvpLetterQuery, mark),
        offset_of!(QvpLetterQuery, part),
        offset_of!(QvpLetterQuery, _pad),
    ];
    // QvpTarget holds a pointer: 32 bytes on a 64-bit host, 24 on wasm32
    if cfg!(target_pointer_width = "64") {
        assert_eq!(size_of::<QvpTarget>(), 32);
        assert_eq!(size_of::<QvpLetterQuery>(), 40);
        assert_eq!(query_offsets, [0, 32, 36, 37, 38]);
    } else {
        assert_eq!(size_of::<QvpTarget>(), 24);
        assert_eq!(size_of::<QvpLetterQuery>(), 32);
        assert_eq!(query_offsets, [0, 24, 28, 29, 30]);
    }

    unsafe {
        let (p, layer) = letters_page();
        let sel = |selector: u8, a: u32, b: u32, c: u32| QvpSelector { selector, a, b, c };
        let mut li = std::mem::zeroed::<QvpLetterInfo>();
        let mut hit = std::mem::zeroed::<QvpLetterHit>();
        let (mut letter, mut at) = (7u32, 7u32);

        // ── no layer: every call answers 0, QVP_NONE or nothing ──
        assert_eq!(qvp_has_letters(p), 0);
        assert_eq!(qvp_letter_count(p, 0), 0);
        assert_eq!(qvp_letter_info(p, 0, 0, &mut li), 0);
        assert_eq!(qvp_letter_members(p, 0, 0, std::ptr::null_mut(), 0), 0);
        assert_eq!(qvp_path_letter(p, 8, &mut letter, &mut at), 0);
        assert_eq!(qvp_path_letter(p, 1, &mut letter, &mut at), 0);
        assert_eq!((letter, at), (7, 7), "a 0 answer writes nothing");
        assert_eq!(qvp_letter_at_offset(p, 1, 1, &mut letter), 0);
        let any = QvpLetterQuery { scope: page_target(), text: 0, mark: 0, part: 0, _pad: [0; 2] };
        assert_eq!(qvp_find_letters(p, &any, std::ptr::null_mut(), 0), 0);
        assert_eq!(qvp_hit_test_letter(p, 15.0, 5.0, std::ptr::null(), &mut hit), 0);
        assert_eq!(qvp_hit_test_letter_view(p, 15.0, 5.0, std::ptr::null(), &mut hit), 0);
        assert_eq!(qvp_layout_draw_revision(p), 0);
        // a letter rule is a rule with no layer: a handle that selects nothing yet
        let waiting = qvp_style_add(p, 0, &sel(SEL_LETTER, 0, 1, 0), RED, 0);
        assert!(waiting > 0);
        assert!(qvp_style_add_letters(p, 0, &any, RED, 0) > 0);
        let mut info = std::mem::zeroed::<QvpPageInfo>();
        qvp_page_info(p, &mut info);
        assert_eq!(info.n_paths, 8);
        assert!((0..8).all(|i| qvp_color_of(p, i) != RED), "with no layer the letter rules select nothing");
        qvp_style_clear(p);

        // ── attach ──
        assert_eq!(qvp_attach_letters(p, layer.as_ptr(), 3), -1, "three bytes are no layer");
        assert_eq!(qvp_attach_letters(p, std::ptr::null(), 0), -1);
        assert_eq!(qvp_attach_letters(std::ptr::null_mut(), layer.as_ptr(), layer.len() as u32), -1);
        let mut other = letters_layer();
        other.page = 2;
        let other = layer_bytes(&letters_page_data(), &other);
        assert_eq!(qvp_attach_letters(p, other.as_ptr(), other.len() as u32), -1, "another page's layer");
        assert_eq!(qvp_attach_letters(p, layer.as_ptr(), (1 << 20) + 1), -1, "over the size limit, before a read");
        assert_eq!(qvp_attach_letters(p, layer.as_ptr(), u32::MAX), -1);
        assert_eq!(qvp_has_letters(p), 0, "a refused attach changes nothing");
        qvp_page_info(p, &mut info);
        assert_eq!(info.n_paths, 8);
        assert_eq!(qvp_attach_letters(p, layer.as_ptr(), layer.len() as u32), 0);
        assert_eq!(qvp_has_letters(p), 1);
        qvp_page_info(p, &mut info);
        assert_eq!(info.n_paths, 10, "the two pieces follow the page's eight paths");
        let mut g = std::mem::zeroed::<QvpGeometry>();
        qvp_geometry(p, &mut g);
        assert_eq!(g.n_paths, 10);
        assert_eq!(qvp_letter_count(p, 0), 2);
        assert_eq!(qvp_letter_count(p, 1), 2);
        assert_eq!(qvp_letter_count(p, 2), 1);
        assert_eq!(qvp_letter_count(p, 3), 0, "a word past the page");

        // ── letters and members ──
        assert_eq!(qvp_letter_info(p, 0, 0, &mut li), 1);
        assert_eq!(
            (li.word, li.index, li.at, li.text, li.n_members, li.n_marks, li.more_at, li.n_more_at),
            (0, 0, 0, 0x0628, 3, 2, QVP_NONE, 0)
        );
        assert_eq!((li.x0, li.y0, li.x1, li.y1, li.has_ink, li._pad), (50.0, 6.0, 60.0, 24.0, 1, [0; 3]));
        assert_eq!(qvp_letter_info(p, 0, 1, &mut li), 1);
        // every pair of neighbouring fields differs here, so a swap of two shows
        assert_eq!(
            (li.word, li.index, li.at, li.text, li.n_members, li.n_marks, li.more_at, li.n_more_at),
            (0, 1, 2, 0x0644, 1, 0, QVP_NONE, 0)
        );
        // the lam's piece reaches one unit past its source and its word
        assert_eq!((li.x0, li.y0, li.x1, li.y1), (39.0, 10.0, 50.0, 20.0));
        assert_eq!(qvp_letter_info(p, 1, 0, &mut li), 1);
        assert_eq!((li.at, li.text, li.more_at, li.n_more_at), (0, 0x0644, 1, 1));
        assert_eq!(qvp_letter_info(p, 0, 2, &mut li), 0, "no such letter");
        assert_eq!(qvp_letter_info(p, 9, 0, &mut li), 0, "no such word");
        assert_eq!(qvp_letter_info(p, 2, 0, std::ptr::null_mut()), 1, "a NULL out still answers");

        assert_eq!(qvp_letter_members(p, 0, 0, std::ptr::null_mut(), 0), 3);
        let mut members = [std::mem::zeroed::<QvpLetterMember>(); 4];
        assert_eq!(qvp_letter_members(p, 0, 0, members.as_mut_ptr(), 4), 3);
        let m = |r: &QvpLetterMember| (r.path, r.at, r.mark, r.is_body, r.is_cut, r._pad);
        // the body first, then the marks in text order
        assert_eq!(m(&members[0]), (8, 0, 0, 1, 1, 0));
        assert_eq!(m(&members[1]), (1, 0, Mark::Dot as u8, 0, 0, 0));
        assert_eq!(m(&members[2]), (2, 1, Mark::Fathah as u8, 0, 0, 0));
        // a short buffer gets the prefix and the total
        let mut one = [std::mem::zeroed::<QvpLetterMember>(); 1];
        assert_eq!(qvp_letter_members(p, 0, 0, one.as_mut_ptr(), 1), 3);
        assert_eq!(m(&one[0]), (8, 0, 0, 1, 1, 0));
        // cap 0 writes nothing, whatever the pointer
        let mut untouched = [QvpLetterMember { path: 77, at: 77, mark: 77, is_body: 77, is_cut: 77, _pad: 77 }; 1];
        assert_eq!(qvp_letter_members(p, 0, 0, untouched.as_mut_ptr(), 0), 3);
        assert_eq!(m(&untouched[0]), (77, 77, 77, 77, 77, 77));
        let mut omitted = [std::mem::zeroed::<QvpLetterMember>(); 2];
        assert_eq!(qvp_letter_members(p, 2, 0, omitted.as_mut_ptr(), 2), 2);
        assert_eq!(m(&omitted[1]), (7, QVP_NONE, Mark::OmittedAlif as u8, 0, 0, 0), "a mark with no offset");
        // a body that is a page path, not a piece: is_body and is_cut apart
        assert_eq!(qvp_letter_members(p, 1, 1, omitted.as_mut_ptr(), 2), 1);
        assert_eq!(m(&omitted[0]), (4, 2, 0, 1, 0, 0));
        assert_eq!(qvp_letter_members(p, 0, 5, members.as_mut_ptr(), 4), 0);

        // ── a path's letter, an offset's letter ──
        assert_eq!(qvp_path_letter(p, 5, &mut letter, &mut at), 1, "a word-level mark");
        assert_eq!((letter, at), (QVP_NONE, 3));
        assert_eq!(qvp_path_letter(p, 9, &mut letter, &mut at), 1, "a piece");
        assert_eq!((letter, at), (1, 2));
        assert_eq!(qvp_path_letter(p, 2, &mut letter, &mut at), 1, "a mark of a letter");
        assert_eq!((letter, at), (0, 1));
        assert_eq!(qvp_path_letter(p, 0, &mut letter, &mut at), 0, "a cut path has no letter of its own");
        assert_eq!(qvp_path_letter(p, 10, &mut letter, &mut at), 0, "past the table");
        assert_eq!(qvp_path_letter(p, 3, std::ptr::null_mut(), std::ptr::null_mut()), 1, "NULL outs are allowed");
        assert_eq!(qvp_letter_at_offset(p, 1, 1, &mut letter), 1, "the tatweel the lam draws");
        assert_eq!(letter, 0);
        assert_eq!(qvp_letter_at_offset(p, 0, 1, &mut letter), 1, "the ba's fathah");
        assert_eq!(letter, 0);
        assert_eq!(qvp_letter_at_offset(p, 1, 3, &mut letter), 0, "a word-level mark is no letter's");
        assert_eq!(qvp_letter_at_offset(p, 1, QVP_NONE, &mut letter), 0);
        assert_eq!(qvp_letter_at_offset(p, 1, 2, std::ptr::null_mut()), 1, "a NULL out still answers");

        // ── selectors ──
        assert_eq!(qvp_style_add(p, 0, &sel(SEL_LETTER, 0, 0, 3), RED, 0), 0, "part 3 is no part");
        assert_eq!(qvp_style_add(p, 0, &sel(SEL_LETTER, 0, 0, 256), RED, 0), 0, "part 256 is no part");
        assert_eq!(qvp_style_add(p, 0, &sel(SEL_WORD_CHARS, 0, 2, 1), RED, 0), 0, "from after to");
        assert_eq!(qvp_style_add(p, 0, &sel(20, 0, 0, 0), RED, 0), 0, "no selector 20");
        assert_eq!(qvp_style_hide(p, &sel(SEL_LETTER, 0, 0, 3)), 0);
        let checks: [(QvpSelector, &[u32]); 6] = [
            (sel(SEL_LETTER, 0, 0, 1), &[1, 8]),   // the ba's own ink: body and dot
            (sel(SEL_LETTER, 0, 0, 2), &[2]),      // its marks: the fathah
            (sel(SEL_LETTER_MARK, 0, 0, 1), &[2]), // its second mark in text order
            (sel(SEL_LETTER_MARK_NAMED, 0, 0, Mark::Dot as u32), &[1]),
            (sel(SEL_WORD_CHARS, 1, 1, 3), &[3, 4, 5]), // the lam's tatweel, the ha, the waqf sign
            (sel(SEL_WORD_CHARS, 0, 0, u32::MAX), &[0, 1, 2, 8, 9]), // a range past the text clamps
        ];
        for (s, want) in checks {
            let h = qvp_style_add(p, 0, &s, RED, 0);
            assert!(h > 0, "selector {}", s.selector);
            let got: Vec<u32> = (0..10).filter(|&i| qvp_color_of(p, i) == RED).collect();
            assert_eq!(got, want, "selector {} ({}, {}, {})", s.selector, s.a, s.b, s.c);
            assert_eq!(qvp_style_remove(p, h), 1);
        }

        // ── the query ──
        let lam = QvpLetterQuery { text: 0x0644, ..any };
        assert_eq!(qvp_find_letters(p, &lam, std::ptr::null_mut(), 0), 2);
        let mut pairs = [7u32; 6];
        assert_eq!(qvp_find_letters(p, &lam, pairs.as_mut_ptr(), 3), 2);
        assert_eq!(pairs, [0, 1, 1, 0, 7, 7], "two pairs written, the rest untouched");
        let mut first = [7u32; 4];
        assert_eq!(qvp_find_letters(p, &lam, first.as_mut_ptr(), 1), 2, "cap counts pairs");
        assert_eq!(first, [0, 1, 7, 7]);
        assert_eq!(qvp_find_letters(p, &lam, first.as_mut_ptr(), 0), 2);
        assert_eq!(qvp_find_letters(p, &QvpLetterQuery { part: 3, ..lam }, std::ptr::null_mut(), 0), 0);
        assert_eq!(qvp_find_letters(p, &QvpLetterQuery { text: 0xD800, ..lam }, std::ptr::null_mut(), 0), 0);
        assert_eq!(qvp_find_letters(p, &QvpLetterQuery { text: 0x110000, ..lam }, std::ptr::null_mut(), 0), 0);
        assert_eq!(qvp_find_letters(p, std::ptr::null(), std::ptr::null_mut(), 0), 0);
        // mark and part apart: a swap of the two bytes gives another answer for both queries
        let dot = QvpLetterQuery { mark: Mark::Dot as u8, ..any };
        assert_eq!(qvp_find_letters(p, &dot, pairs.as_mut_ptr(), 3), 1);
        assert_eq!(&pairs[..2], &[0, 0]);
        let marks_part = QvpLetterQuery { part: 2, ..any };
        assert_eq!(qvp_find_letters(p, &marks_part, std::ptr::null_mut(), 0), 5, "part does not filter");
        let word_1 = QvpTarget { target: 1, a: 1, ..page_target() };
        assert_eq!(qvp_find_letters(p, &QvpLetterQuery { scope: word_1, ..lam }, pairs.as_mut_ptr(), 3), 1);
        assert_eq!(&pairs[..2], &[1, 0]);
        assert_eq!(qvp_style_add_letters(p, 0, &QvpLetterQuery { part: 3, ..lam }, RED, 0), 0);
        assert_eq!(qvp_style_add_letters(p, 0, &QvpLetterQuery { text: 0xD800, ..lam }, RED, 0), 0);
        assert_eq!(qvp_style_add_letters(p, 0, std::ptr::null(), RED, 0), 0);
        let lams = qvp_style_add_letters(p, 0, &lam, RED, 0);
        assert!(lams > 0);
        let colors = std::slice::from_raw_parts(qvp_colors(p), 10);
        assert_eq!(colors[3], RED, "the lam of word 1");
        assert_eq!(colors[9], RED, "the lam's piece of word 0");
        assert_eq!(colors.iter().filter(|&&c| c == RED).count(), 2);
        assert_eq!(qvp_style_remove(p, lams), 1);

        // ── the swap in the draw list ──
        let mut lay = std::mem::zeroed::<QvpLayout>();
        qvp_layout(p, &letters_spec(), &mut lay);
        assert_eq!(draw_pairs(p), PLAIN);
        let before = qvp_layout_draw_revision(p);
        let h = qvp_style_add(p, 0, &sel(SEL_LETTER, 0, 1, 0), RED, 0);
        assert!(h > 0);
        // no qvp_colors, no tick: the draw list itself applies the new rule
        assert_eq!(draw_pairs(p), SWAPPED);
        assert_eq!(qvp_layout_draw_revision(p), before.wrapping_add(1));
        let mut sp = [0u32; 8];
        assert_eq!(qvp_styled_paths(p, sp.as_mut_ptr(), 4), 1, "the piece, never the cut path too");
        assert_eq!(&sp[..2], &[9, RED]);

        // ── hit tests ──
        assert_eq!(qvp_hit_test_letter(p, 15.0, 5.0, std::ptr::null(), &mut hit), 1);
        let hv = |h: &QvpLetterHit| {
            (h.word, h.letter, h.path, h.at, h.line, h.distance, h.mark, h.is_exact, h.is_body, h._pad)
        };
        // a word-level mark: no letter, not a body
        assert_eq!(hv(&hit), (1, QVP_NONE, 5, 3, 0, 0.0, Mark::WaqfJaizWaslAwla as u8, 1, 0, 0));
        // a mark member of a letter: the dot of the ba
        assert_eq!(qvp_hit_test_letter(p, 55.0, 23.0, std::ptr::null(), &mut hit), 1);
        assert_eq!(hv(&hit), (0, 0, 1, 0, 0, 0.0, Mark::Dot as u8, 1, 0, 0));
        // the piece of the lam, which reaches past its word's box
        assert_eq!(qvp_hit_test_letter(p, 39.5, 15.0, std::ptr::null(), &mut hit), 1);
        assert_eq!(hv(&hit), (0, 1, 9, 2, 0, 0.0, 0, 1, 1, 0));
        // the view variant goes through the layout: the middle of piece 0 at (55, 15)
        let dy = *lay.lines;
        let (vx, vy) = (lay.offset_x + 55.0 * lay.scale, lay.offset_y + (15.0 + dy) * lay.scale);
        assert_eq!(qvp_hit_test_letter_view(p, vx, vy, std::ptr::null(), &mut hit), 1);
        assert_eq!(hv(&hit), (0, 0, 8, 0, 0, 0.0, 0, 1, 1, 0));
        // between the letters of word 2, off the ink: the nearest letter's box
        assert_eq!(qvp_hit_test_letter(p, 21.0, 55.0, std::ptr::null(), &mut hit), 1);
        assert_eq!((hit.word, hit.letter, hit.path, hit.is_exact, hit.mark, hit.is_body), (2, 0, 6, 0, 0, 0));
        assert!((hit.distance - 1.0).abs() < 1e-5, "{}", hit.distance);
        let tight = QvpHitOptions { max_distance: 0.5, gap_bias: -1.0, prefer_exact: 1 };
        assert_eq!(qvp_hit_test_letter(p, 80.0, 90.0, &tight, &mut hit), 0, "too far from every word");
        assert_eq!(qvp_hit_test_letter(p, 15.0, 5.0, std::ptr::null(), std::ptr::null_mut()), 1);

        // ── a second attach replaces the first; a refused one keeps it ──
        let rev = qvp_layout_draw_revision(p);
        assert_eq!(qvp_attach_letters(p, layer.as_ptr(), layer.len() as u32 - 1), -1);
        assert_eq!(qvp_layout_draw_revision(p), rev, "a refused attach changes nothing");
        assert_eq!(qvp_attach_letters(p, layer.as_ptr(), layer.len() as u32), 0);
        qvp_page_info(p, &mut info);
        assert_eq!(info.n_paths, 10);
        assert!(qvp_layout_draw_revision(p) != rev, "an attach counts up");
        assert_eq!(draw_pairs(p), SWAPPED, "the rule stays and takes the new layer");

        qvp_page_free(p);
        // a NULL page answers as no layer
        let null = std::ptr::null_mut::<qvp_core::Page>();
        assert_eq!(qvp_has_letters(null), 0);
        assert_eq!(qvp_letter_count(null, 0), 0);
        assert_eq!(qvp_letter_info(null, 0, 0, &mut li), 0);
        assert_eq!(qvp_letter_members(null, 0, 0, std::ptr::null_mut(), 0), 0);
        assert_eq!(qvp_path_letter(null, 0, &mut letter, &mut at), 0);
        assert_eq!(qvp_letter_at_offset(null, 0, 0, &mut letter), 0);
        assert_eq!(qvp_find_letters(null, &any, std::ptr::null_mut(), 0), 0);
        assert_eq!(qvp_hit_test_letter(null, 0.0, 0.0, std::ptr::null(), &mut hit), 0);
        assert_eq!(qvp_hit_test_letter_view(null, 0.0, 0.0, std::ptr::null(), &mut hit), 0);
        assert_eq!(qvp_layout_draw_revision(null), 0);
        assert_eq!(qvp_style_add_letters(null, 0, &any, RED, 0), 0);
    }
}

/// `is_body` is the member's role, and `mark` the path's mark: a body that the page stores as
/// a mark, as an `omitted_alif`, has both.
#[test]
fn a_letter_hit_says_whether_it_is_body_ink() {
    unsafe {
        let data = letters_page_data();
        let bytes = encode(&data);
        let p = qvp_page_load(bytes.as_ptr(), bytes.len());
        assert!(!p.is_null());
        // the omitted alif (path 7, a mark of the page) as a second body of word 2's letter
        let mut layer = letters_layer();
        layer.members[7] = MemberRec { path: MemberPath::Page(7), at: 0, is_body: true };
        let bytes = layer_bytes(&data, &layer);
        assert_eq!(qvp_attach_letters(p, bytes.as_ptr(), bytes.len() as u32), 0);
        let mut hit = std::mem::zeroed::<QvpLetterHit>();
        let kind = |hit: &QvpLetterHit| (hit.word, hit.letter, hit.path, hit.mark, hit.is_exact, hit.is_body);
        assert_eq!(qvp_hit_test_letter(p, 15.0, 47.0, std::ptr::null(), &mut hit), 1);
        assert_eq!(kind(&hit), (2, 0, 7, Mark::OmittedAlif as u8, 1, 1), "a body stored as a mark");
        assert_eq!(qvp_hit_test_letter(p, 15.0, 55.0, std::ptr::null(), &mut hit), 1);
        assert_eq!(kind(&hit), (2, 0, 6, 0, 1, 1), "body ink");
        assert_eq!(qvp_hit_test_letter(p, 55.0, 7.0, std::ptr::null(), &mut hit), 1);
        assert_eq!(kind(&hit), (0, 0, 2, Mark::Fathah as u8, 1, 0), "a mark member");
        qvp_page_free(p);
    }
}

/// A host that changes a rule and asks for the draw list, or for the draw revision, before it
/// reads colours gets an answer for every rule change made so far, never last frame's swap.
#[test]
fn the_draw_list_answers_for_every_rule_so_far() {
    unsafe {
        let (p, layer) = letters_page();
        assert_eq!(qvp_attach_letters(p, layer.as_ptr(), layer.len() as u32), 0);
        let mut lay = std::mem::zeroed::<QvpLayout>();
        qvp_layout(p, &letters_spec(), &mut lay);
        assert_eq!(draw_pairs(p), PLAIN);
        let r0 = qvp_layout_draw_revision(p);
        let lam = QvpSelector { selector: SEL_LETTER, a: 0, b: 1, c: 0 };

        // a rule with no transition, then the draw list alone: no colours, no tick
        let h = qvp_style_add(p, 0, &lam, RED, 0);
        assert_eq!(draw_pairs(p), SWAPPED);
        assert_eq!(qvp_layout_draw_revision(p), r0.wrapping_add(1));
        // nothing changed: the same list and the same revision, however often a host asks
        assert_eq!(draw_pairs(p), SWAPPED);
        assert_eq!(qvp_layout_draw_revision(p), r0.wrapping_add(1));
        assert_eq!(qvp_layout_draw_revision(p), r0.wrapping_add(1));

        // the rule removed, then the revision alone: it counts the change back to whole
        assert_eq!(qvp_style_remove(p, h), 1);
        assert_eq!(qvp_layout_draw_revision(p), r0.wrapping_add(2));
        assert_eq!(draw_pairs(p), PLAIN);
        assert_eq!(qvp_layout_draw_revision(p), r0.wrapping_add(2));

        // a recolour to the default ink and a mask change act the same way
        let h = qvp_style_add(p, 0, &lam, RED, 0);
        assert_eq!(qvp_layout_draw_revision(p), r0.wrapping_add(3));
        assert_eq!(qvp_style_recolor(p, h, qvp_core::DEFAULT_INK, 0), 1);
        assert_eq!(draw_pairs(p), PLAIN);
        assert_eq!(qvp_layout_draw_revision(p), r0.wrapping_add(4));
        qvp_page_free(p);
    }
}

/// A page whose words share paths takes no layer, and keeps its answers.
#[test]
fn a_page_whose_words_share_paths_takes_no_layer() {
    unsafe {
        let mut data = letters_page_data();
        // word 1 starts at path 2, so path 2 sits in the runs of words 0 and 1
        data.words[1].first_path = 2;
        data.words[1].n_paths = 4;
        let bytes = encode(&data);
        let p = qvp_page_load(bytes.as_ptr(), bytes.len());
        assert!(!p.is_null());
        let mut w = std::mem::zeroed::<QvpWordInfo>();
        assert_eq!(qvp_word_info(p, 1, &mut w), 1);
        assert_eq!((w.first_path, w.n_paths), (2, 4), "the file keeps the overlap");
        let layer = layer_bytes(&data, &letters_layer());
        assert_eq!(qvp_attach_letters(p, layer.as_ptr(), layer.len() as u32), -1);
        assert_eq!(qvp_has_letters(p), 0);
        let mut info = std::mem::zeroed::<QvpPageInfo>();
        qvp_page_info(p, &mut info);
        assert_eq!(info.n_paths, 8);
        qvp_page_free(p);
    }
}

/// A mark id with no name counts as 255, unknown, in the letter selector and the query, as in
/// QVP_SELECTOR_MARK and QVP_SELECTOR_WORD_MARK_NAMED: it takes the paths whose mark the
/// converter did not recognise. An id above 255 takes nothing in the letter selector.
#[test]
fn a_mark_id_with_no_name_is_the_unknown_mark() {
    unsafe {
        let mut data = letters_page_data();
        data.paths[2].mark = Mark::Unknown; // the ba's fathah, as a mark the converter did not know
        let bytes = encode(&data);
        let p = qvp_page_load(bytes.as_ptr(), bytes.len());
        assert!(!p.is_null());
        let layer = layer_bytes(&data, &letters_layer());
        assert_eq!(qvp_attach_letters(p, layer.as_ptr(), layer.len() as u32), 0);
        let reds = |p: *mut qvp_core::Page, s: QvpSelector| -> Vec<u32> {
            let h = qvp_style_add(p, 0, &s, RED, 0);
            assert!(h > 0);
            let got = (0..10).filter(|&i| qvp_color_of(p, i) == RED).collect();
            qvp_style_remove(p, h);
            got
        };
        for id in [36, 200, 254, 255] {
            assert_eq!(reds(p, QvpSelector { selector: SEL_LETTER_MARK_NAMED, a: 0, b: 0, c: id }), [2], "id {id}");
            assert_eq!(reds(p, QvpSelector { selector: 10, a: id, b: 0, c: 0 }), [2], "QVP_SELECTOR_MARK {id}");
            let q = QvpLetterQuery { scope: page_target(), text: 0, mark: id as u8, part: 0, _pad: [0; 2] };
            let mut pairs = [7u32; 4];
            assert_eq!(qvp_find_letters(p, &q, pairs.as_mut_ptr(), 2), 1, "query mark {id}");
            assert_eq!(&pairs[..2], &[0, 0]);
        }
        for id in [0, 256, 256 + 255, u32::MAX] {
            assert!(reds(p, QvpSelector { selector: SEL_LETTER_MARK_NAMED, a: 0, b: 0, c: id }).is_empty(), "id {id}");
        }
        qvp_page_free(p);

        // on the shared fixture no path has an unknown mark, so an id with no name takes nothing
        let (p, layer) = letters_page();
        assert_eq!(qvp_attach_letters(p, layer.as_ptr(), layer.len() as u32), 0);
        let h = qvp_style_add(p, 0, &QvpSelector { selector: SEL_LETTER_MARK_NAMED, a: 0, b: 0, c: 200 }, RED, 0);
        assert!(h > 0);
        assert!((0..10).all(|i| qvp_color_of(p, i) != RED));
        qvp_page_free(p);
    }
}

/// On a page with no layer the draw list and the draw revision leave the rules alone, so a
/// fade added after a tick starts at the next tick, as it did before the letter layer existed.
#[test]
fn a_page_with_no_layer_keeps_its_fade() {
    unsafe {
        let (p, _) = letters_page();
        let mut lay = std::mem::zeroed::<QvpLayout>();
        qvp_layout(p, &letters_spec(), &mut lay);
        let word_0 = QvpSelector { selector: 7, a: 0, b: 0, c: 0 };
        for read in 0..3 {
            qvp_tick(p, 1000.0);
            let h = qvp_style_add(p, 0, &word_0, RED, 200);
            match read {
                0 => {
                    draw_pairs(p);
                }
                1 => {
                    qvp_layout_draw_revision(p);
                }
                _ => {} // neither: the behaviour before the letter layer
            }
            assert_eq!(qvp_tick(p, 60000.0), 1, "call {read}: the fade starts at this tick");
            assert_eq!(qvp_color_of(p, 0), qvp_core::DEFAULT_INK, "call {read}");
            assert_eq!(qvp_tick(p, 60100.0), 1);
            let mid = qvp_color_of(p, 0);
            assert!(mid != qvp_core::DEFAULT_INK && mid != RED, "call {read}: mid-fade {mid:08x}");
            assert_eq!(qvp_tick(p, 60200.0), 0);
            assert_eq!(qvp_color_of(p, 0), RED);
            assert_eq!(qvp_style_remove(p, h), 1);
            qvp_tick(p, 70000.0);
            qvp_tick(p, 80000.0);
            assert_eq!(qvp_color_of(p, 0), qvp_core::DEFAULT_INK);
        }
        assert_eq!(draw_pairs(p), PLAIN);
        qvp_page_free(p);
    }
}
