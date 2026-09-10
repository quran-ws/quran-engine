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
    let Some(bytes) = page_bytes("036.qvp") else { eprintln!("skip: no dist/pages"); return };
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
        let mut m = [QvpMatch { word: 0, index: 0, loose: 0 }; 64];
        let n = qvp_search(page, q.as_ptr(), q.len() as u32, 4, 0, 1, 1, 0, m.as_mut_ptr(), 64);
        assert!(n > 0 && n < 64, "{n}");
        let mut t = QvpStr { ptr: std::ptr::null(), len: 0 };
        let tg = QvpTarget { kind: 3, a: w.surah as u32, b: w.ayah as u32, c: 0, words: std::ptr::null(), n_words: 0 };
        qvp_text_target(page, &tg, 0, b" ".as_ptr(), 1, b"\n".as_ptr(), 1, &mut t);
        assert!(s(&t).contains(' '));
        // gap-aware hit at the centre of word 0's line but between words: always resolves
        let mut hx = std::mem::zeroed::<QvpHitEx>();
        assert_eq!(qvp_hit_test_ex(page, (w.x0 + w.x1) / 2.0, (w.y0 + w.y1) / 2.0, std::ptr::null(), &mut hx), 1);
        assert_eq!(hx.word, 0);
        // layout + view hit
        let spec = QvpLayoutSpec { viewport_w: 690.0, viewport_h: 1100.0, pad_top: 20.0, pad_bottom: 20.0, pad_left: 0.0, pad_right: 0.0, line_spacing: 1.0, line_gap: 0.0, fill_height: 1, nominal_lines: 15 };
        let mut lay = std::mem::zeroed::<QvpLayout>();
        qvp_layout(page, &spec, &mut lay);
        assert_eq!(lay.n_lines, 15);
        assert!((lay.scale - 2.0).abs() < 1e-5);
        let mut wb = [0f32; 4];
        assert_eq!(qvp_word_box_view(page, 0, wb.as_mut_ptr()), 1);
        let mut h = QvpHit { word: 0, path: 0, deco: 0 };
        assert_eq!(qvp_hit_test_view(page, (wb[0] + wb[2]) / 2.0, (wb[1] + wb[3]) / 2.0, &mut h), 1);
        assert_eq!(h.word, 0);
        // styles + highlight + tick
        let sel = QvpSelector { kind: 3, a: 0, b: 0, c: 0 }; // first mark of word 0
        let hd = qvp_style_add(page, 0, &sel, 0xff0000ff, 0);
        assert!(hd > 0);
        let mut pairs = vec![0u32; 4000];
        let n = qvp_styled(page, pairs.as_mut_ptr(), 2000);
        assert_eq!(n, 1);
        assert_eq!(qvp_style_remove(page, hd), 1);
        let st = QvpHighlightStyle { mode: 2, height: 0, ink: 0x00aa00ff, band: 0xffcc0080, pad_x: 1.2, pad_y: 0.0, radius: 1.0, seam: 0.25, transition_ms: 200, layer: 50 };
        let hh = qvp_highlight(page, &tg, &st);
        assert!(hh > 0);
        assert_eq!(qvp_tick(page, 100.0), 1, "animating");
        let mut boxes = [QvpBox { id: 0, line: 0, x0: 0.0, y0: 0.0, x1: 0.0, y1: 0.0, color: 0, radius: 0.0 }; 32];
        let nb = qvp_highlight_boxes(page, boxes.as_mut_ptr(), 32);
        assert!(nb >= 1 && nb <= 15);
        assert_eq!(qvp_tick(page, 1000.0), 0);
        qvp_unhighlight(page, hh);
        // metadata
        let mut d = [QvpDivision { kind: 0, line: 0, n: 0, surah: 0, ayah: 0, ayah_idx: 0 }; 8];
        let _ = qvp_divisions(page, d.as_mut_ptr(), 8);
        let mut mk = [QvpAyahMark { deco: 0, surah: 0, ayah: 0, line: 0, cx: 0.0, cy: 0.0, r: 0.0, ornament_path: 0, numeral_path: 0 }; 32];
        assert_eq!(qvp_ayah_marks(page, mk.as_mut_ptr(), 32), 6);
        // mask/reveal
        qvp_mask(page, &tg, 0);
        assert!(qvp_mask_hidden(page, std::ptr::null_mut(), 0) > 0);
        assert_eq!(qvp_reveal_next(page, 1), 1);
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
            assert_eq!(qvp_atlas_division_at(a, 0, 78, 1), 30);
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
            assert_eq!(qvp_atlas_find_surah(a, b"cow".as_ptr(), 3, found.as_mut_ptr(), 8), 1);
            assert_eq!(found[0], 2);
            qvp_atlas_free(a);
        }
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
                "", "fathah", "kasrah", "dammah", "tanwin_al_fath", "tanwin_al_kasr", "tanwin_al_damm", "shaddah", "sukun", "maddah",
                "hamzah", "hamzat_al_wasl", "omitted_alif", "small_waw", "small_yaa", "small_noon", "dot", "two_dots", "three_dots",
                "rounded_zero", "rectangular_zero", "waqf_jaiz_mustawi_al_tarafayn", "waqf_jaiz_waqf_awla", "waqf_jaiz_wasl_awla",
                "waqf_lazim", "waqf_al_muanaqah", "saktah", "small_meem", "hizb", "sajdah", "sajdah_mark", "sajdah_line",
                "seen_al_qiraah", "tashil", "ishmam", "imalah",
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
            ["body", "mark", "ayah_number", "ayah_mark_ornament", "header_ink", "ornament", "page_number", "running_head"]
        );
    }
}

/// One style with one square outline per asset, built in memory: enough to prove
/// the whole dress path over the ABI without a licensed artwork fixture.
fn ornament_bytes() -> Vec<u8> {
    use qvp_core::qvp_format::ornaments::*;
    use qvp_core::qvp_format::Cmd;
    let mut set = OrnamentSet::default();
    let q = set.quant as i32;
    let unit = |w: i32, h: i32| vec![Cmd::MoveTo(0, 0), Cmd::LineTo(w * q, 0), Cmd::LineTo(w * q, h * q), Cmd::LineTo(0, h * q), Cmd::Close];
    let asset = |w: i32, h: i32, slot: Option<[f32; 4]>, set: &mut OrnamentSet| {
        let first = set.paths.len() as u32;
        set.push_path(0, 0, 0, &unit(w, h));
        set.push_path(1, OPF_STROKE, q / 10, &unit(w, h));
        Asset { view_box: [0.0, 0.0, w as f32, h as f32], slot, first_path: first, n_paths: 2 }
    };
    let ayah_mark = asset(80, 100, Some([20.0, 20.0, 40.0, 60.0]), &mut set);
    let surah_header = asset(800, 100, Some([100.0, 10.0, 600.0, 80.0]), &mut set);
    let page_frame = asset(70, 100, Some([5.0, 5.0, 60.0, 90.0]), &mut set);
    set.styles.push(Style {
        name: "test".into(),
        riwayah: "Hafs 'an Asim".into(),
        license: License { id: "CC-BY-NC-SA-4.0".into(), status: "provisional".into(), redistributable: false, attribution: "a test".into() },
        parts: vec![Part { name: "c1".into(), color: 0x00649bff, stroke: false }, Part { name: "line".into(), color: 0x303641ff, stroke: true }],
        ayah_mark: Some(ayah_mark),
        surah_header: Some(surah_header),
        page_frame: Some(page_frame),
        slices: None,
    });
    set.encode()
}

#[test]
fn dress_over_the_abi() {
    let Some(bytes) = page_bytes("604.qvp") else { eprintln!("skip: no dist/pages"); return };
    let orn = ornament_bytes();
    unsafe {
        let page = qvp_page_load(bytes.as_ptr(), bytes.len());
        assert!(!page.is_null());
        let set = qvp_ornaments_load(orn.as_ptr(), orn.len());
        assert!(!set.is_null());
        assert_eq!(qvp_ornament_styles(set), 1);
        assert_eq!(qvp_ornament_find_style(set, b"test".as_ptr(), 4), 0);
        assert_eq!(qvp_ornament_find_style(set, b"nope".as_ptr(), 4), -1);

        let mut st = std::mem::zeroed::<QvpOrnamentStyle>();
        assert_eq!(qvp_ornament_style(set, 0, &mut st), 1);
        assert_eq!(s(&st.name), "test");
        assert_eq!(s(&st.license_id), "CC-BY-NC-SA-4.0");
        assert_eq!(st.redistributable, 0);
        assert_eq!(st.assets, 0b0111); // three assets, no slices
        assert_eq!(st.n_parts, 2);
        let mut part = std::mem::zeroed::<QvpOrnamentPart>();
        assert_eq!(qvp_ornament_part(set, 0, 1, &mut part), 1);
        assert_eq!((s(&part.name), part.color, part.stroke), ("line".into(), 0x303641ff, 1));

        // undressed, the page's viewBox is its own and there is no display list
        let mut vb = [0f32; 4];
        qvp_page_view_box(page, vb.as_mut_ptr());
        let mut pi = std::mem::zeroed::<QvpPageInfo>();
        qvp_page_info(page, &mut pi);
        assert_eq!(vb, [0.0, 0.0, pi.width, pi.height]);
        let mut di = std::mem::zeroed::<QvpDressInfo>();
        assert_eq!(qvp_dress_info(page, &mut di), 0);

        // the printed rings are visible before the dress
        let ring = (0..pi.n_paths).find(|i| qvp_color_of(page, *i) != 0 && path_kind(page, *i) == 3).expect("a printed ring");
        assert_ne!(qvp_color_of(page, ring) & 0xff, 0);

        let colors: [u32; 2] = [0, 0xff0000ff]; // repaint part 0 red
        let spec = QvpDressSpec { style: 0, gap: 5.0, line_art: 0, ayah_marks: 1, surah_headers: 1, page_frame: 1, colors: colors.as_ptr(), n_colors: 1 };
        assert_eq!(qvp_dress(page, set, &spec), 1);
        assert_eq!(qvp_dress_info(page, &mut di), 1);
        assert!(di.n_ayah_marks > 0 && di.n_surah_headers > 0);
        assert_eq!(di.frame_stretched, 1); // no slices in this set
        assert_eq!(di.n_frame_repeats, 0);
        // the border grew the page around the text, and nothing moved
        qvp_page_view_box(page, vb.as_mut_ptr());
        assert_eq!(vb, di.view_box);
        assert!(vb[0] < 0.0 && vb[1] < 0.0 && vb[2] > pi.width && vb[3] > pi.height, "{vb:?}");
        // and a host that fits content+overflow shows all of it
        let mut over = [0f32; 4];
        qvp_dress_overflow(page, over.as_mut_ptr());
        assert!(over.iter().all(|v| *v > 0.0), "{over:?}");

        // A LAYOUT THAT RESPACES THE LINES MAKES THE PAGE TALLER, and the border
        // goes around what is on the page: it grows with it, unasked.
        let tall = QvpLayoutSpec { viewport_w: 690.0, viewport_h: 2000.0, pad_top: 0.0, pad_bottom: 0.0, pad_left: 0.0, pad_right: 0.0, line_spacing: 1.0, line_gap: 0.0, fill_height: 1, nominal_lines: 15 };
        let mut lay = std::mem::zeroed::<QvpLayout>();
        qvp_layout(page, &tall, &mut lay);
        let mut vb2 = [0f32; 4];
        qvp_page_view_box(page, vb2.as_mut_ptr());
        assert!(vb2[3] > vb[3], "the border must follow the laid-out page: {vb:?} → {vb2:?}");
        assert_eq!(vb2[2], vb[2], "only the height changed");
        // and a host's raster cache of that layer is told it went stale
        let before = di.revision;
        assert_eq!(qvp_dress_info(page, &mut di), 1);
        assert_ne!(di.revision, before, "a relayout rebuilds the ornament layer");
        // the print's own rings are hidden; its numerals are not
        assert_eq!(qvp_color_of(page, ring) & 0xff, 0);

        let mut g = std::mem::zeroed::<QvpDressGeometry>();
        qvp_dress_geometry(page, &mut g);
        assert_eq!(g.n_draws, di.n_draws);
        // one filled + one stroked outline per placed asset
        assert_eq!(g.n_draws, (di.n_ayah_marks + di.n_surah_headers + 1) * 2);
        let table = std::slice::from_raw_parts(g.table, g.n_draws as usize * 9);
        let (mut filled, mut stroked) = (0, 0);
        for d in table.chunks_exact(9) {
            let (color, flags, width) = (d[4], d[5], f32::from_bits(d[6]));
            assert!(d[1] > 0 && d[3] > 0);
            if flags & (2 << 8) != 0 {
                stroked += 1;
                assert!(width > 0.0, "a stroke needs a width in page units");
                assert_eq!(color, 0x303641ff);
            } else {
                filled += 1;
                assert_eq!(color, 0xff0000ff, "part 0 was repainted");
            }
            assert!(flags & 0xff <= 2, "ornament kind");
            // the border is placed from the page and belongs to no line; the rest follow one
            assert_eq!(d[8] == u32::MAX, flags & 0xff == 2, "kind {} line {}", flags & 0xff, d[8]);
        }
        assert_eq!((filled, stroked), (g.n_draws / 2, g.n_draws / 2));

        // line art draws one ink, and only one
        let spec = QvpDressSpec { line_art: 1, ..spec };
        assert_eq!(qvp_dress(page, set, &spec), 1);
        qvp_dress_geometry(page, &mut g);
        assert_eq!(g.n_draws, di.n_draws / 2);

        // a style the set does not hold changes nothing
        let spec = QvpDressSpec { style: 9, ..spec };
        assert_eq!(qvp_dress(page, set, &spec), 0);
        assert_eq!(qvp_dress_info(page, &mut di), 0);

        qvp_undress(page);
        assert_eq!(qvp_dress_info(page, &mut di), 0);
        qvp_page_view_box(page, vb.as_mut_ptr());
        assert_eq!(vb, [0.0, 0.0, pi.width, pi.height]);
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
