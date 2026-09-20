use super::*;

fn square(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<Cmd> {
    vec![Cmd::MoveTo(x0, y0), Cmd::LineTo(x1, y0), Cmd::LineTo(x1, y1), Cmd::LineTo(x0, y1), Cmd::Close]
}

/// Two words on one line: A = [10,20]×[10,20] units (word 1:1:1), B = [30,40]×[10,20]
/// (word 1:1:2) with two marks above it; plus a second line with word C (1:2:1).
fn page() -> Page {
    Page::load(&encode(&page_data())).unwrap()
}

fn page_data() -> PageData {
    let mut ops = Vec::new();
    let mut paths = Vec::new();
    let mut strings = vec![];
    let mut add = |cmds: &[Cmd], kind: PathKind, mark: Mark, fam: Family| {
        let bb = cmds_bbox(cmds);
        let off = ops.len() as u32;
        encode_cmds(cmds, bb.x0, bb.y0, &mut ops);
        paths.push(PathRec {
            kind,
            mark,
            family: fam,
            flags: PF_EVENODD,
            ox: bb.x0,
            oy: bb.y0,
            op_off: off,
            op_len: ops.len() as u32 - off,
            bbox: bb,
        });
        bb
    };
    let a = add(&square(1000, 1000, 2000, 2000), PathKind::Body, Mark::None, Family::None);
    let mut b = add(&square(3000, 1000, 4000, 2000), PathKind::Body, Mark::None, Family::None);
    b.union(&add(&square(3200, 500, 3400, 700), PathKind::Mark, Mark::Fathah, Family::Diacritic));
    b.union(&add(&square(3600, 500, 3800, 700), PathKind::Mark, Mark::Dot, Family::Dots));
    let c = add(&square(1000, 5000, 4000, 6000), PathKind::Body, Mark::None, Family::None);
    let mut l1 = a;
    l1.union(&b);
    let mut st = |s: &str| {
        strings.push(s.to_owned());
        (strings.len() - 1) as u16
    };
    let (ta, ta_s, tb, tb_s, tc, tc_s) = (st("ذَٰلِكَ"), st("ذلك"), st("ٱلْكِتَٰبُ"), st("الكتاب"), st("لَا"), st("لا"));
    PageData {
        header: Header { version: VERSION, quant: 100, page: 1, flags: 0, width: 100.0, height: 100.0 },
        lines: vec![
            LineRec { line_number: 1, first_word: 0, n_words: 2, bbox: l1 },
            LineRec { line_number: 2, first_word: 2, n_words: 1, bbox: c },
        ],
        ayahs: vec![
            AyahRec {
                surah: 1,
                ayah: 1,
                fragment: 1,
                fragments: 1,
                flags: AF_JUZ_START | AF_HIZB_START | AF_RUBU_AL_HIZB_START,
                first_word: 0,
                n_words: 2,
                ayah_mark_decoration: NONE_U16,
                rubu_al_hizb: 1,
                bbox: l1,
            },
            AyahRec {
                surah: 1,
                ayah: 2,
                fragment: 1,
                fragments: 1,
                flags: 0,
                first_word: 2,
                n_words: 1,
                ayah_mark_decoration: NONE_U16,
                rubu_al_hizb: 0,
                bbox: c,
            },
        ],
        words: vec![
            WordRec {
                surah: 1,
                ayah: 1,
                word: 1,
                line_index: 0,
                ayah_index: 0,
                text: ta,
                rasm_imlai: ta,
                qpc: ta,
                rasm: ta_s,
                search: ta_s,
                first_path: 0,
                n_paths: 1,
                bbox: a,
            },
            WordRec {
                surah: 1,
                ayah: 1,
                word: 2,
                line_index: 0,
                ayah_index: 0,
                text: tb,
                rasm_imlai: tb,
                qpc: tb,
                rasm: tb_s,
                search: tb_s,
                first_path: 1,
                n_paths: 3,
                bbox: b,
            },
            WordRec {
                surah: 1,
                ayah: 2,
                word: 1,
                line_index: 1,
                ayah_index: 1,
                text: tc,
                rasm_imlai: tc,
                qpc: tc,
                rasm: tc_s,
                search: tc_s,
                first_path: 4,
                n_paths: 1,
                bbox: c,
            },
        ],
        paths,
        decorations: vec![],
        glyphs: vec![],
        insts: vec![],
        ops,
        strings,
    }
}

/// A tall sajdah sign on line 1 must not pull the line's centre. An ayah mark stored
/// without a line takes the line of the ayah it closes, not the nearest centre. The
/// sajdah line takes the line of the word under it, not the sign's line.
#[test]
fn decorations_follow_their_line() {
    let mut d = page_data();
    let mut add = |cmds: &[Cmd], kind: PathKind| {
        let bb = cmds_bbox(cmds);
        let off = d.ops.len() as u32;
        encode_cmds(cmds, bb.x0, bb.y0, &mut d.ops);
        let idx = d.paths.len() as u32;
        d.paths.push(PathRec {
            kind,
            mark: Mark::None,
            family: Family::None,
            flags: PF_EVENODD,
            ox: bb.x0,
            oy: bb.y0,
            op_off: off,
            op_len: d.ops.len() as u32 - off,
            bbox: bb,
        });
        (idx, bb)
    };
    // the sign spans from above line 1 to the middle of line 2
    let (sajdah, sb) = add(&square(200, 200, 400, 5500), PathKind::Other);
    // the sajdah line sits over word C on line 2, though its sign is on line 1
    let (sajdah_line, ob) = add(&square(1500, 4900, 3500, 4950), PathKind::Mark);
    // the mark sits low: its centre (y=40) is nearer line 2 (55) than line 1 (15)
    let (mark, mb) = add(&square(4200, 3800, 4600, 4200), PathKind::AyahMarkOrnament);
    d.paths[sajdah_line as usize].mark = Mark::SajdahLine;
    let mut sb2 = sb;
    sb2.union(&ob);
    d.decorations = vec![
        DecoRec {
            kind: DecoKind::SajdahMark,
            surah: 1,
            ayah: 1,
            text: NONE_U16,
            first_path: sajdah,
            n_paths: 2,
            line: 0,
            bbox: sb2,
        },
        DecoRec {
            kind: DecoKind::AyahMark,
            surah: 1,
            ayah: 1,
            text: NONE_U16,
            first_path: mark,
            n_paths: 1,
            line: NONE_U16,
            bbox: mb,
        },
    ];
    d.ayahs[0].ayah_mark_decoration = 1;
    let mut p = Page::load(&encode(&d)).unwrap();
    assert_eq!(p.line_centre(0), 15.0);
    assert_eq!(p.line_centre(1), 55.0);
    assert_eq!(p.geometry().table[sajdah as usize].line, 0);
    assert_eq!(p.geometry().table[sajdah_line as usize].line, 1);
    assert_eq!(p.geometry().table[mark as usize].line, 0);
    let l = p
        .layout(&LayoutSpec {
            viewport_w: 100.0,
            viewport_h: 400.0,
            fill_height: true,
            grid_lines: 2,
            ..Default::default()
        })
        .clone();
    assert!(l.line_dy[1] > l.line_dy[0]);
}

#[test]
fn hit_testing() {
    let p = page();
    assert_eq!(p.hit_test_exact(15.0, 15.0), Some(HitExact { word: 0, path: 0, decoration: NONE }));
    assert_eq!(p.hit_test_exact(35.0, 15.0), Some(HitExact { word: 1, path: 1, decoration: NONE }));
    assert_eq!(p.hit_test_exact(33.0, 6.0), Some(HitExact { word: 1, path: 2, decoration: NONE }));
    assert_eq!(p.hit_test_exact(33.0, 8.5), Some(HitExact { word: 1, path: NONE, decoration: NONE }));
    assert_eq!(p.hit_test_exact(25.0, 15.0), None);
    assert_eq!(p.hit_test_exact(50.0, 50.0), None);
    assert_eq!(p.word_text(1), "ٱلْكِتَٰبُ");
    assert_eq!(p.find_word(1, 1, 2), Some(1));
}

#[test]
fn gap_aware_hit_and_hit_boxes() {
    let p = page();
    let o = HitOptions::default();
    // in the gap between A (x 10-20) and B (x 30-40): gap 10, preceding = B (right) gets 60%
    let h = p.hit_test(25.0, 15.0, &o).unwrap();
    assert_eq!((h.word, h.is_exact), (1, false), "x=25 is within B's 60% share (threshold 24)");
    let h = p.hit_test(22.0, 15.0, &o).unwrap();
    assert_eq!((h.word, h.is_exact), (0, false));
    assert!((h.distance - 2.0).abs() < 1e-4);
    // above the line but within its line-spacing band → still resolves
    let h = p.hit_test(15.0, 1.0, &o).unwrap();
    assert_eq!(h.word, 0);
    assert!(p.hit_test(15.0, 1.0, &HitOptions { max_distance: 5.0, ..o }).is_none());
    let hb = p.hit_areas(0.6);
    assert_eq!(hb.len(), 3);
    assert!((hb[0].x1 - 24.0).abs() < 1e-4 && (hb[1].x0 - 24.0).abs() < 1e-4, "hit boxes meet at the gap split");
    let bands = p.line_bands();
    assert_eq!(bands.len(), 2);
    assert!(bands[0].y1 <= bands[1].y0 + 1e-3);
}

#[test]
fn styles_layers_handles_and_subword() {
    let mut p = page();
    assert!(p.styled_paths().is_empty());
    let h_ayah = p.style(Selector::Ayah(1, 1), Paint::new(0x0000ffff));
    let h_word = p.style(Selector::Word(1), Paint::new(0x00ff00ff));
    let h_fam = p.style(Selector::Family(Family::Diacritic), Paint::new(0xff0000ff));
    assert_eq!(p.color_of(0), 0x0000ffff, "ayah applies to word 0");
    assert_eq!(p.color_of(1), 0x00ff00ff, "word beats ayah");
    assert_eq!(p.color_of(2), 0x00ff00ff, "word beats family for its mark");
    p.remove_style(h_word);
    assert_eq!(p.color_of(2), 0x0000ffff, "ayah beats family");
    p.remove_style(h_ayah);
    assert_eq!(p.color_of(2), 0xff0000ff, "family applies to the mark only");
    // higher layer wins regardless of specificity
    let h_theme = p.style_in(LAYER_THEME, Selector::Page, Paint::new(0x111111ff));
    assert_eq!(p.color_of(2), 0x111111ff);
    p.remove_style(h_theme);
    p.remove_style(h_fam);
    // sub-word: the 2nd mark (index 1) of word 1 is the dot
    let h = p.style(Selector::WordMark(1, 1), Paint::new(0xabcdefff));
    assert_eq!(p.color_of(3), 0xabcdefff);
    assert_eq!(p.color_of(2), DEFAULT_INK);
    p.remove_style(h);
    let h = p.style(Selector::WordMarkNamed(1, Mark::Fathah, 0), Paint::new(0x123456ff));
    assert_eq!(p.color_of(2), 0x123456ff);
    p.remove_style(h);
    let h = p.style(Selector::Category(Category::LetterDot), Paint::new(0x0a0b0cff));
    assert_eq!(p.color_of(3), 0x0a0b0cff);
    assert_eq!(p.styled_paths(), vec![(3, 0x0a0b0cff)]);
    p.recolor_style(h, Paint::new(0x0d0e0fff));
    assert_eq!(p.color_of(3), 0x0d0e0fff);
    p.remove_style(h);
    assert!(p.styles.is_empty());
    let h = p.hide(Selector::Path(2));
    assert_eq!(p.color_of(2) & 0xff, 0, "alpha 0 hides");
    p.remove_style(h);
    // theme handle undoes as a whole
    let t = p.theme(&Theme { ink: Some(0x222222ff), dots: Some(0xcc0000ff), ..Default::default() });
    assert_eq!(p.color_of(0), 0x222222ff);
    assert_eq!(p.color_of(3), 0xcc0000ff);
    p.remove_style(t);
    assert_eq!(p.color_of(3), DEFAULT_INK);
}

#[test]
fn transitions_run_on_the_clock() {
    let mut p = page();
    p.tick(0.0);
    let h = p.style(Selector::Word(0), Paint::fade(0xff0000ff, 100));
    assert_eq!(p.color_of(0), DEFAULT_INK, "not moved before the clock ticks");
    assert!(p.tick(50.0));
    let mid = p.color_of(0);
    assert_ne!(mid, DEFAULT_INK);
    assert_ne!(mid, 0xff0000ff);
    assert!(!p.tick(200.0));
    assert_eq!(p.color_of(0), 0xff0000ff);
    p.remove_style(h);
    assert!(p.tick(210.0), "fade back uses the previous rule's duration");
    assert!(!p.tick(400.0));
    assert_eq!(p.color_of(0), DEFAULT_INK);
}

#[test]
fn highlights_bands_and_animation() {
    let mut p = page();
    p.tick(0.0);
    let st = HighlightStyle {
        mode: HighlightMode::Both,
        ink: 0x00aa00ff,
        band: 0xffcc0080,
        transition_ms: 100,
        ..Default::default()
    };
    let h = p.highlight(&Target::Ayah(1, 1), st);
    assert_eq!(p.color_of(0), DEFAULT_INK);
    p.tick(1000.0);
    assert_eq!(p.color_of(0), 0x00aa00ff);
    let boxes = p.highlight_boxes_view();
    assert_eq!(boxes.len(), 1, "one band box: both words on line 1");
    assert!((boxes[0].x0 - (10.0 - 1.2)).abs() < 1e-3 && (boxes[0].x1 - (40.0 + 1.2)).abs() < 1e-3);
    assert_eq!(boxes[0].color, 0xffcc0080);
    // slide to word C on line 2
    p.move_highlight(h, &Target::Word(2));
    p.tick(1050.0);
    let mid = p.highlight_boxes_view();
    assert_eq!(mid.len(), 1);
    assert!(mid[0].y0 > boxes[0].y0, "band is moving down");
    p.tick(2000.0);
    let end = p.highlight_boxes_view();
    assert!((end[0].x0 - (10.0 - 1.2)).abs() < 1e-3 && end[0].y0 > 30.0, "{end:?}");
    assert_eq!(p.color_of(4), 0x00aa00ff, "word C's body path");
    assert_eq!(p.color_of(0), DEFAULT_INK);
    p.remove_highlight(h);
    p.tick(2050.0);
    assert_eq!(p.highlight_boxes_view().len(), 1, "fading out");
    p.tick(3000.0);
    assert!(p.highlight_boxes_view().is_empty());
    assert!(p.highlight_handles().is_empty());
}

#[test]
fn text_search_selection_citation() {
    let p = page();
    assert_eq!(p.text_of(&[0, 1, 2], Form::RasmUthmani, " ", "\n"), "ذَٰلِكَ ٱلْكِتَٰبُ\nلَا");
    assert_eq!(p.text_of(&p.target_words(&Target::Page), Form::Search, " ", " / "), "ذلك الكتاب / لا");
    let m = p.search("الكتاب", &SearchOptions::default());
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].word, 1);
    assert!(p.search("كتاب", &SearchOptions { mode: SearchMode::Exact, ..Default::default() }).is_empty());
    assert_eq!(
        p.search("الكتب", &SearchOptions { loose: false, ..Default::default() }).len(),
        0,
        "strict pass: different letters"
    );
    assert_eq!(p.search("الكتب", &SearchOptions::default()).len(), 1, "loose pass drops bare alef");
    assert_eq!(p.search("ذَلِكَ", &SearchOptions::default())[0].word, 0, "query is normalised");
    let s = Selection { anchor: Some(2), focus: Some(0) };
    assert_eq!(s.words(), vec![0, 1, 2]);
    assert_eq!(p.citation(&s.words()), "1:1-2");
    assert_eq!(p.citation(&[1]), "1:1");
    assert_eq!(p.divisions().iter().map(|d| (d.kind, d.n)).collect::<Vec<_>>(), vec![(0, 1), (1, 1), (3, 1)]);
    assert_eq!(p.ayah_keys(), vec![(1, 1), (1, 2)]);
    assert_eq!(p.recite_map(1, 1, 2), Some(vec![0, 1]));
    assert_eq!(p.recite_map(1, 1, 3), None, "count mismatch → follow whole");
}

#[test]
fn mask_reveal_and_crop() {
    let mut p = page();
    p.mask(&Target::Ayah(1, 1), MaskMode::Hide);
    assert_eq!(p.mask_hidden_count(), 2);
    assert_eq!(p.color_of(0) & 0xff, 0);
    assert_eq!(p.unmask_next(1), 1);
    assert_ne!(p.color_of(0) & 0xff, 0);
    assert_eq!(p.color_of(1) & 0xff, 0);
    p.unmask();
    assert_eq!(p.color_of(1), DEFAULT_INK);
    p.mask(&Target::Word(2), MaskMode::Block);
    assert_eq!(p.color_of(4), DEFAULT_INK, "block mode keeps ink; host draws the box");
    assert_eq!(p.mask_boxes_view().len(), 1);
    p.unmask();
    // reveal: grey page, window of 1 at step 0
    let steps = p.reveal_start(1, false, 0xc9c4b8ff, 0x000000ff, true, 0);
    assert_eq!(steps, 3);
    p.reveal_goto(0);
    assert_eq!(p.color_of(0), 0x000000ff);
    assert_eq!(p.color_of(1), 0xc9c4b8ff);
    let h = p.style(Selector::Word(1), Paint::new(0xff0000ff));
    assert_eq!(p.color_of(1), 0xff0000ff, "explicit rules beat reveal grey");
    p.remove_style(h);
    p.reveal_stop();
    assert_eq!(p.color_of(1), DEFAULT_INK);
    let cb = p.crop_bounds(&Target::Ayah(1, 1), 2.0, true).unwrap();
    assert!((cb.x0 - 8.0).abs() < 1e-4 && (cb.x1 - 42.0).abs() < 1e-4 && (cb.y0 - 3.0).abs() < 1e-4);
    let svg = p.crop_svg(&Target::Word(0), 1.0, false, Some(0xffffffff)).unwrap();
    assert!(svg.starts_with("<svg") && svg.contains("<rect") && svg.matches("<path").count() == 1);
}

#[test]
fn mask_transition_fades_on_the_engine_clock() {
    let mut p = page();
    p.set_mask_transition(200);
    p.mask(&Target::Word(0), MaskMode::Hide);
    assert!(p.tick(1000.0), "hiding fades");
    assert_eq!(p.color_of(0), DEFAULT_INK, "a fade starts from the ink");
    p.tick(1100.0);
    let mid = p.color_of(0);
    assert!(mid & 0xff > 0 && mid & 0xff < 0xff, "half way: partly transparent, {mid:08x}");
    assert_eq!(mid >> 8, DEFAULT_INK >> 8, "fades the ink's own colour, never through black");
    assert!(!p.tick(1300.0));
    assert_eq!(p.color_of(0), DEFAULT_INK & 0xffff_ff00);
    assert!(p.unmask_word(0));
    assert!(p.tick(2000.0), "revealing fades back in");
    p.tick(2050.0);
    let quarter = p.color_of(0) & 0xff;
    assert!(quarter < 0x40, "a mask fade eases in: a quarter of the way it is still faint, {quarter:02x}");
    assert!(!p.tick(2300.0));
    assert_eq!(p.color_of(0), DEFAULT_INK);
    // a style fade keeps easing out: a quarter of the way it has done most of its change
    let _h = p.style(Selector::Word(1), Paint::fade(DEFAULT_INK & 0xffff_ff00, 200));
    p.tick(3000.0);
    p.tick(3050.0);
    let style_quarter = p.color_of(1) & 0xff;
    assert!(style_quarter < 0x80, "a style fade eases out, {style_quarter:02x}");
}

#[test]
fn layout_fill_height_and_view_hit() {
    let mut p = page();
    let spec = LayoutSpec {
        viewport_w: 200.0,
        viewport_h: 2500.0,
        pad_top: 50.0,
        pad_bottom: 50.0,
        fill_height: true,
        ..Default::default()
    };
    let l = p.layout(&spec).clone();
    assert_eq!(l.scale, 2.0);
    // a short page (2 lines) takes the rows of the 15-line grid: 2400 px / 15 = 160 px
    // = 80 units a row, so 40 units of leading on the printed line spacing of 40
    let delta = 40.0;
    assert!((l.line_spacing - (40.0 + delta)).abs() < 1e-3, "line spacing {}", l.line_spacing);
    assert_eq!(l.line_dy.len(), 2);
    // printed geometry kept: the two lines differ by exactly one delta; the grid (15 rows
    // = 1200 units) is centred in the padded viewport, the page centred on it (slot0 = 6.5)
    assert!((l.line_dy[1] - l.line_dy[0] - delta).abs() < 1e-3);
    assert!((l.line_dy[0] - (25.0 + (1200.0 - 100.0) / 2.0 - 7.0 * delta + 6.5 * delta)).abs() < 1e-3);
    assert!((l.content_h - 2500.0).abs() < 1e-3);
    // slot boundary halfway between the laid-out line centres
    let mid = ((15.0 + l.line_dy[0]) + (55.0 + l.line_dy[1])) / 2.0 * 2.0;
    assert!((l.line_slots[0].1 - mid).abs() < 1e-3 && (l.line_slots[1].0 - mid).abs() < 1e-3);
    let vx = l.offset_x + 15.0 * 2.0;
    let vy = l.offset_y + (15.0 + l.line_dy[0]) * 2.0;
    assert_eq!(p.hit_test_exact_view(vx, vy), Some(HitExact { word: 0, path: 0, decoration: NONE }));
    assert_eq!(p.hit_test_exact_view(vx, 5.0), None);
    let g = p.hit_test_view(l.offset_x + 25.0 * 2.0, vy, &HitOptions::default()).unwrap();
    assert_eq!((g.word, g.is_exact), (1, false));
    // rows shorter than the printed line spacing: the print is the floor and the grid outgrows the viewport
    let tight = p.layout(&LayoutSpec { viewport_h: 1100.0, ..spec }).clone();
    assert!((tight.line_spacing - 40.0).abs() < 1e-3 && (tight.content_h - (100.0 + 15.0 * 40.0 * 2.0)).abs() < 1e-3);
    // a full page (the grid is its own line count) spreads its printed height: (1200 − 100) units
    // over its one gap — and never squeezes below the print
    let full = p.layout(&LayoutSpec { grid_lines: 2, ..spec }).clone();
    assert!((full.line_spacing - (40.0 + 1100.0)).abs() < 1e-3 && (full.content_h - 2500.0).abs() < 1e-3);
    let squeezed =
        p.layout(&LayoutSpec { viewport_h: 150.0, pad_top: 0.0, pad_bottom: 0.0, grid_lines: 2, ..spec }).clone();
    assert!((squeezed.line_spacing - 40.0).abs() < 1e-3 && (squeezed.content_h - 200.0).abs() < 1e-3);
    let l2 = p
        .layout(&LayoutSpec { viewport_w: 200.0, viewport_h: 1100.0, line_spacing: 1.5, ..Default::default() })
        .clone();
    assert!((l2.line_spacing - p.line_spacing() * 1.5).abs() < 1e-3);
    assert!((l2.line_dy[1] - l2.line_dy[0] - 20.0).abs() < 1e-3, "×1.5 adds half a line spacing between lines");
    // as printed: no shift at all
    let l3 = p.layout(&LayoutSpec { viewport_w: 200.0, viewport_h: 1100.0, ..Default::default() }).clone();
    assert!(l3.line_dy.iter().all(|d| d.abs() < 1e-6) && (l3.content_h - 200.0).abs() < 1e-3);
    // spacing only ever opens up: below 1.0, a negative gap, and a fill that would
    // need to tighten all reproduce the print
    for spec in [
        LayoutSpec { viewport_w: 200.0, viewport_h: 1100.0, line_spacing: 0.5, ..Default::default() },
        LayoutSpec { viewport_w: 200.0, viewport_h: 60.0, fill_height: true, ..Default::default() },
    ] {
        let l = p.layout(&spec).clone();
        assert!((l.line_spacing - p.line_spacing()).abs() < 1e-3, "line spacing {} for {:?}", l.line_spacing, spec);
        assert!(l.line_dy[1] - l.line_dy[0] >= -1e-6, "lines never move closer");
    }
    // the multiplier that fills a viewport IS the spacing fill-height lays the page out at —
    // the crop, the aspect bound and the grid a short page sits on all included, because a host
    // that sizes its own furniture to a printed row reads it instead of laying the page out
    let tall = LayoutSpec { viewport_w: 390.0, viewport_h: 844.0, ..Default::default() };
    let grid2 = LayoutSpec { grid_lines: 2, ..tall };
    for spec in [
        tall,
        grid2,
        LayoutSpec { crop_left: 20.0, crop_right: 30.0, ..grid2 },
        LayoutSpec { max_aspect_slack: 1.15, ..grid2 },
    ] {
        let m = p.line_spacing_to_fill(&spec, f32::INFINITY);
        let filled = p.layout(&LayoutSpec { fill_height: true, ..spec }).line_spacing;
        assert!((filled - p.line_spacing() * m).abs() < 1e-3, "multiplier {m}, laid out at {filled}, for {spec:?}");
    }
    // a crop draws the page bigger, so less paper is left over and it takes less leading to fill
    assert!(
        p.line_spacing_to_fill(&LayoutSpec { crop_left: 20.0, crop_right: 30.0, ..grid2 }, f32::INFINITY)
            < p.line_spacing_to_fill(&grid2, f32::INFINITY)
    );
    assert_eq!(
        p.line_spacing_to_fill(&LayoutSpec { viewport_w: 820.0, viewport_h: 300.0, ..Default::default() }, 100.0),
        1.0
    );
    assert!(p.wasted_fraction(&tall) > 0.0 && p.wasted_fraction(&tall) < 1.0);
    // printed_height is content_h before any leading: exactly the flat layout's, never more
    // than the filled one's, and the filled one's again where the box is too short to fill
    let flat = LayoutSpec { fill_height: false, line_spacing: 1.0, ..tall };
    assert!((p.layout(&flat).content_h - p.printed_height(&flat)).abs() < 1e-3);
    let filled = LayoutSpec { fill_height: true, ..tall };
    assert!(p.layout(&filled).content_h >= p.printed_height(&filled) - 1e-3);
    let short = LayoutSpec { viewport_h: 200.0, fill_height: true, ..tall };
    assert!((p.layout(&short).content_h - p.printed_height(&short)).abs() < 1e-3);
    // a crop draws the page bigger, so it is taller for the same width
    assert!(p.printed_height(&LayoutSpec { crop_left: 20.0, crop_right: 30.0, ..grid2 }) > p.printed_height(&grid2));
    let wb = p.word_bounds_view(2);
    assert!(wb.1 > 0.0);
}

#[test]
fn renderer_draws_bands_ink_and_mask_boxes() {
    let mut p = page();
    struct Count(u32, u32, u32);
    impl Renderer for Count {
        fn begin(&mut self, _: f32, _: f32) {}
        fn fill_path(&mut self, _: u32, _: &[u8], pts: &[f32], _: Rgba, _: bool) {
            self.0 += 1;
            self.1 += pts.len() as u32;
        }
        fn fill_rect(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: Rgba) {
            self.2 += 1;
        }
        fn end(&mut self) {}
    }
    p.hide(Selector::Path(2));
    p.highlight(&Target::Word(0), HighlightStyle::default());
    let mut c = Count(0, 0, 0);
    p.render(&mut c);
    assert_eq!((c.0, c.2), (4, 1), "hidden path skipped; one band box");
}

#[test]
fn box_corner_radius_never_exceeds_half_the_shorter_side() {
    let mut p = page();
    p.layout(&LayoutSpec { viewport_w: 200.0, viewport_h: 200.0, ..Default::default() });
    p.highlight(&Target::Word(0), HighlightStyle { radius: 1000.0, ..HighlightStyle::default() });
    for b in p.highlight_boxes_view() {
        let half = ((b.x1 - b.x0).min(b.y1 - b.y0)) / 2.0;
        assert!(b.radius <= half + 1e-4 && b.radius > 0.0, "{b:?}");
    }
    p.mask(&Target::Word(1), MaskMode::Block);
    p.set_mask_options(0xff, 0.0, 0.0, 1000.0, false);
    for b in p.mask_boxes_view() {
        let half = ((b.x1 - b.x0).min(b.y1 - b.y0)) / 2.0;
        assert!(b.radius <= half + 1e-4 && b.radius > 0.0, "{b:?}");
    }
}

#[test]
fn layout_fit_crop_and_aspect_bound() {
    let mut p = page();
    // as printed the content is 200 px tall at scale 2; a 150 px viewport shrinks and centres it
    let l = p.layout(&LayoutSpec { viewport_w: 200.0, viewport_h: 150.0, ..Default::default() }).clone();
    assert!((l.fit_scale - 0.75).abs() < 1e-6 && (l.fit_x - 25.0).abs() < 1e-6 && l.fit_y.abs() < 1e-6, "{l:?}");
    // a tall viewport never enlarges: scale 1, centred vertically
    let l = p.layout(&LayoutSpec { viewport_w: 200.0, viewport_h: 1100.0, ..Default::default() }).clone();
    assert!((l.fit_scale - 1.0).abs() < 1e-6 && l.fit_x.abs() < 1e-6 && (l.fit_y - 450.0).abs() < 1e-6);
    // the aspect bound stops a wide viewport from stretching the lines: 100 px tall, 1:1 page,
    // slack 1.15 -> the content is 115 px wide and sits in the middle of the 1000 px viewport
    let l = p
        .layout(&LayoutSpec { viewport_w: 1000.0, viewport_h: 100.0, max_aspect_slack: 1.15, ..Default::default() })
        .clone();
    assert!((l.content_w - 115.0).abs() < 1e-3 && (l.scale - 1.15).abs() < 1e-6, "{l:?}");
    // 115 px of content in a 100 px viewport: the fit shrinks it and centres what remains
    let fit = 100.0 / 115.0;
    assert!((l.fit_scale - fit).abs() < 1e-6 && (l.fit_x - (1000.0 - 115.0 * fit) / 2.0).abs() < 1e-3, "{l:?}");
    // cropping 10 units off each printed margin: 80 units fill the 200 px width, the origin
    // moves left by the cropped margin
    let l = p
        .layout(&LayoutSpec {
            viewport_w: 200.0,
            viewport_h: 1100.0,
            crop_left: 10.0,
            crop_right: 10.0,
            ..Default::default()
        })
        .clone();
    assert!((l.scale - 2.5).abs() < 1e-6 && (l.offset_x + 25.0).abs() < 1e-6, "{l:?}");
}

#[test]
fn reflow_breaks_words_onto_rows_of_the_same_width() {
    let mut p = packed_page();
    let printed = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    let wide = p.layout(&printed).clone();
    let zoom = 3.0;
    let spec = LayoutSpec { reflow: Some(ReflowSpec { zoom, ..Default::default() }), ..printed };
    let l = p.layout(&spec).clone();
    // the page keeps its width and the ink grows by the zoom
    assert!((l.content_w - wide.content_w).abs() < 1e-3);
    assert!((l.scale - wide.scale * zoom).abs() < 1e-3);
    // zooming in breaks the words onto more rows, and the page grows taller
    let easy = LayoutSpec { reflow: Some(ReflowSpec { zoom: 1.5, ..Default::default() }), ..printed };
    let rows_at_1 = p.layout(&easy).reflow.as_ref().unwrap().row_band.len();
    let l = p.layout(&spec).clone();
    let flow = l.reflow.clone().unwrap();
    assert!(flow.row_band.len() > rows_at_1, "rows {} vs {rows_at_1}", flow.row_band.len());
    assert!(l.content_h > wide.content_h);
    // a scrolled page is never shrunk back to the viewport
    assert_eq!(l.fit_scale, 1.0);
    // every word sits inside its row, and rows never run backwards in reading order
    let q = p.quant();
    for (r, words) in flow.row_words.iter().enumerate() {
        let mut last_x1 = f32::INFINITY;
        for &wi in words {
            let w = &p.data().words[wi as usize];
            let pl = flow.word_place[wi as usize];
            let x0 = pl.apply(w.bbox.x0 as f32 / q, 0.0).0;
            let x1 = pl.apply(w.bbox.x1 as f32 / q, 0.0).0;
            // a word wider than the row is the documented overflow case (see reflow_max_zoom)
            if x1 - x0 <= flow.row_w + 0.01 {
                assert!(x0 >= -0.01 && x1 <= flow.row_w + 0.01, "word {wi} out of row: {x0}..{x1}");
            }
            assert!(x1 <= last_x1 + 0.01, "word {wi} overlaps its right neighbour on row {r}");
            last_x1 = x0;
            assert_eq!(flow.word_row[wi as usize], r as u32);
        }
    }
    // words keep their printed size: only whole words move
    for (wi, pl) in flow.word_place.iter().enumerate() {
        assert_eq!((pl.kx, pl.ky), (1.0, 1.0), "word {wi} was resized");
    }
    // a tap on a placed word finds that word
    for wi in 0..p.data().words.len() as u32 {
        let (x0, y0, x1, y1) = p.word_bounds_view(wi);
        let h = p.hit_test_view((x0 + x1) / 2.0, (y0 + y1) / 2.0, &HitOptions::default());
        assert_eq!(h.map(|h| h.word), Some(wi), "tap on word {wi}");
    }
}

/// A page wide enough to reflow onto mixed rows: 3 lines of 4 words, each word 16 units wide
/// with 4-unit gaps, inside a 100-unit page with 6-unit margins.
fn packed_page() -> Page {
    let mut ops = Vec::new();
    let mut paths = Vec::new();
    let mut words = Vec::new();
    let mut lines = Vec::new();
    let mut ayahs = Vec::new();
    let mut strings = vec!["كلمة".to_owned()];
    for li in 0..3u8 {
        let y0 = 10 + 30 * li as i32;
        let mut lb = IBox::default();
        let first = words.len() as u16;
        for w in 0..4u16 {
            // right to left: the first word of a line is the rightmost
            let x1 = 94 - 20 * w as i32;
            let (x0, y1) = (x1 - 16, y0 + 16);
            let cmds = square(x0 * 100, y0 * 100, x1 * 100, y1 * 100);
            let bb = cmds_bbox(&cmds);
            let off = ops.len() as u32;
            encode_cmds(&cmds, bb.x0, bb.y0, &mut ops);
            paths.push(PathRec {
                kind: PathKind::Body,
                mark: Mark::None,
                family: Family::None,
                flags: 0,
                ox: bb.x0,
                oy: bb.y0,
                op_off: off,
                op_len: ops.len() as u32 - off,
                bbox: bb,
            });
            if w == 0 {
                lb = bb;
            } else {
                lb.union(&bb);
            }
            words.push(WordRec {
                surah: 1,
                ayah: li as u16 + 1,
                word: w + 1,
                line_index: li as u16,
                ayah_index: li as u16,
                text: 0,
                rasm_imlai: 0,
                qpc: 0,
                rasm: 0,
                search: 0,
                first_path: paths.len() as u32 - 1,
                n_paths: 1,
                bbox: bb,
            });
        }
        lines.push(LineRec { line_number: li + 1, first_word: first, n_words: 4, bbox: lb });
        ayahs.push(AyahRec {
            surah: 1,
            ayah: li as u16 + 1,
            fragment: 1,
            fragments: 1,
            flags: 0,
            first_word: first,
            n_words: 4,
            ayah_mark_decoration: NONE_U16,
            rubu_al_hizb: 0,
            bbox: lb,
        });
    }
    let data = PageData {
        header: Header { version: VERSION, quant: 100, page: 1, flags: 0, width: 100.0, height: 100.0 },
        lines,
        ayahs,
        words,
        paths,
        decorations: vec![],
        glyphs: vec![],
        insts: vec![],
        ops,
        strings: std::mem::take(&mut strings),
    };
    Page::load(&encode(&data)).unwrap()
}

fn page_with_surah_frame(frame: bool) -> (Page, Option<u32>, u32, u32) {
    let mut data = page_data();
    for line in &mut data.lines {
        line.line_number += 1;
    }
    data.lines.insert(0, LineRec { line_number: 1, first_word: 0, n_words: 0, bbox: IBox::EMPTY });
    for word in &mut data.words {
        word.line_index += 1;
    }

    let add = |data: &mut PageData, cmds: &[Cmd], kind: PathKind| {
        let bbox = cmds_bbox(cmds);
        let op_off = data.ops.len() as u32;
        encode_cmds(cmds, bbox.x0, bbox.y0, &mut data.ops);
        let pi = data.paths.len() as u32;
        data.paths.push(PathRec {
            kind,
            mark: Mark::None,
            family: Family::None,
            flags: PF_EVENODD,
            ox: bbox.x0,
            oy: bbox.y0,
            op_off,
            op_len: data.ops.len() as u32 - op_off,
            bbox,
        });
        pi
    };

    // The frame spans the printed text column and deliberately sits off the title's vertical
    // centre. It is hollow, like the real opening frames: empty space inside it must not become
    // a decoration hit. Reflow must ignore both extents, not merely hide paint after measuring it.
    let frame_path = frame.then(|| {
        let mut outline = square(1000, -1000, 4000, 1000);
        outline.extend(square(1200, -800, 3800, 800));
        add(&mut data, &outline, PathKind::Ornament)
    });
    let title_path = add(&mut data, &square(2000, 300, 3000, 700), PathKind::HeaderInk);
    let first_path = frame_path.unwrap_or(title_path);
    let mut bbox = data.paths[title_path as usize].bbox;
    if let Some(frame_path) = frame_path {
        bbox.union(&data.paths[frame_path as usize].bbox);
    }
    let decoration = data.decorations.len() as u32;
    data.decorations.push(DecoRec {
        kind: DecoKind::SurahName,
        surah: 1,
        ayah: 0,
        text: NONE_U16,
        first_path,
        n_paths: u16::from(frame) + 1,
        line: 0,
        bbox,
    });
    data.canonicalize_ops();
    (Page::from_data(data), frame_path, title_path, decoration)
}

fn placed_point(page: &Page, layout: &Layout, path: u32, x: f32, y: f32) -> (f32, f32) {
    let line = page.geometry().table[path as usize].line;
    let (x, y) = layout.placement(path, line).apply(x, y);
    (layout.offset_x + x * layout.scale, layout.offset_y + y * layout.scale)
}

#[test]
fn reflow_omits_a_native_surah_frame_without_constraining_its_title() {
    let (mut framed, Some(frame_path), title_path, decoration) = page_with_surah_frame(true) else { unreachable!() };
    let (mut bare, None, bare_title, _) = page_with_surah_frame(false) else { unreachable!() };
    let printed_spec = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };

    let printed = framed.layout(&printed_spec).clone();
    assert!(printed.omitted_paths.is_empty());
    assert!(framed.layout_draw_list().iter().any(|draw| draw.path == frame_path));
    let frame_point = placed_point(&framed, &printed, frame_path, 11.0, -9.0);
    assert_eq!(
        framed.hit_test_exact_view(frame_point.0, frame_point.1),
        Some(HitExact { word: NONE, path: frame_path, decoration })
    );
    let hollow_point = placed_point(&framed, &printed, frame_path, 25.0, 0.0);
    assert_eq!(framed.hit_test_exact(25.0, 0.0), None);
    assert_eq!(framed.hit_test_exact_view(hollow_point.0, hollow_point.1), None);

    let hidden_spec = LayoutSpec { surah_frames: false, ..printed_spec };
    let hidden = framed.layout(&hidden_spec).clone();
    let bare_printed = bare.layout(&printed_spec).clone();
    assert_eq!(hidden.omitted_paths, vec![frame_path]);
    assert!(!framed.layout_draw_list().iter().any(|draw| draw.path == frame_path));
    assert!(framed.layout_draw_list().iter().any(|draw| draw.path == title_path));
    assert_eq!(hidden.content_h, bare_printed.content_h);
    assert_eq!(hidden.line_slots, bare_printed.line_slots);
    let hidden_frame_point = placed_point(&framed, &hidden, frame_path, 11.0, -9.0);
    assert_eq!(framed.hit_test_exact_view(hidden_frame_point.0, hidden_frame_point.1), None);
    let hidden_title_point = placed_point(&framed, &hidden, title_path, 25.0, 5.0);
    assert_eq!(
        framed.hit_test_exact_view(hidden_title_point.0, hidden_title_point.1),
        Some(HitExact { word: NONE, path: title_path, decoration })
    );

    let reflow_spec = LayoutSpec { reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }), ..printed_spec };
    let framed_layout = framed.layout(&reflow_spec).clone();
    let bare_layout = bare.layout(&reflow_spec).clone();
    assert!(!framed_layout.reflow.as_ref().unwrap().as_printed);
    assert_eq!(framed_layout.omitted_paths, vec![frame_path]);
    assert!(!framed.layout_draw_list().iter().any(|draw| draw.path == frame_path));
    assert!(framed.layout_draw_list().iter().any(|draw| draw.path == title_path));

    let title_placement = framed_layout.placement(title_path, 0);
    let bare_placement = bare_layout.placement(bare_title, 0);
    assert_eq!(title_placement, bare_placement, "the frame must not move or resize the title");
    assert!((framed_layout.content_h - bare_layout.content_h).abs() < 0.01);
    assert_eq!(framed_layout.line_slots, bare_layout.line_slots);

    let omitted_point = placed_point(&framed, &framed_layout, frame_path, 15.0, -5.0);
    assert_eq!(framed.hit_test_exact_view(omitted_point.0, omitted_point.1), None);
    let title_point = placed_point(&framed, &framed_layout, title_path, 25.0, 5.0);
    assert_eq!(
        framed.hit_test_exact_view(title_point.0, title_point.1),
        Some(HitExact { word: NONE, path: title_path, decoration })
    );

    // Asking for reflow at the printed size dispatches to the printed layout, so the native
    // frame remains. Only a page actually broken onto enlarged rows omits it.
    let at_print = framed
        .layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 1.0, ..Default::default() }), ..printed_spec })
        .clone();
    assert!(at_print.omitted_paths.is_empty());
    assert!(framed.layout_draw_list().iter().any(|draw| draw.path == frame_path));
}

#[test]
fn reflow_justified_fills_every_row_but_the_last() {
    let mut p = packed_page();
    let base = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    let spec = LayoutSpec {
        // no cap: every row that can reach the margins must
        reflow: Some(ReflowSpec { zoom: 1.5, fill: Fill::Justified, max_stretch: 0.0, ..Default::default() }),
        ..base
    };
    let l = p.layout(&spec).clone();
    let flow = l.reflow.clone().unwrap();
    let q = p.quant();
    let rows = flow.row_band.len();
    for (r, words) in flow.row_words.iter().enumerate() {
        if words.len() < 2 || r + 1 == rows {
            continue;
        }
        let first = &p.data().words[words[0] as usize];
        let last = &p.data().words[words[words.len() - 1] as usize];
        let right = flow.word_place[words[0] as usize].apply(first.bbox.x1 as f32 / q, 0.0).0;
        let left = flow.word_place[words[words.len() - 1] as usize].apply(last.bbox.x0 as f32 / q, 0.0).0;
        assert!((right - flow.row_w).abs() < 0.01, "row {r} does not touch the right margin");
        assert!(left.abs() < 0.01, "row {r} does not reach the left margin");
    }
}

#[test]
fn reflow_justified_leaves_a_row_ragged_rather_than_gap_it_out() {
    let mut p = packed_page();
    let base = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    let spec = |max_stretch| LayoutSpec {
        reflow: Some(ReflowSpec { zoom: 1.5, fill: Fill::Justified, max_stretch, ..Default::default() }),
        ..base
    };
    let q = p.quant();
    let reach = |p: &mut Page, max_stretch: f32| -> Vec<f32> {
        let l = p.layout(&spec(max_stretch)).clone();
        let flow = l.reflow.clone().unwrap();
        flow.row_words
            .iter()
            .filter(|ws| ws.len() > 1)
            .map(|ws| {
                let last = &p.data().words[ws[ws.len() - 1] as usize];
                flow.word_place[ws[ws.len() - 1] as usize].apply(last.bbox.x0 as f32 / q, 0.0).0
            })
            .collect()
    };
    let uncapped = reach(&mut p, 0.0);
    // a cap of 1.0 forbids any stretch at all, so no row is pulled out to the left margin
    let capped = reach(&mut p, 1.0);
    assert_eq!(uncapped.len(), capped.len());
    assert!(
        capped.iter().zip(&uncapped).any(|(c, u)| c > u),
        "with gaps capped, at least one row should stay short instead of reaching the margin"
    );
}

#[test]
fn reflow_at_zoom_1_gives_the_printed_page_back() {
    let mut p = packed_page();
    let base = LayoutSpec {
        viewport_w: 200.0,
        viewport_h: 400.0,
        pad_top: 12.0,
        pad_left: 8.0,
        pad_right: 8.0,
        ..Default::default()
    };
    let printed = p.layout(&base).clone();
    let boxes: Vec<(f32, f32, f32, f32)> = (0..p.data().words.len() as u32).map(|i| p.word_bounds_view(i)).collect();
    let l = p.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 1.0, ..Default::default() }), ..base }).clone();
    assert!(!l.is_reflowed(), "at the printed size the page is laid out as printed");
    // same scale, same page height, same word in the same pixel
    assert!((l.scale - printed.scale).abs() < 1e-4);
    assert!((l.content_h - printed.content_h).abs() < 1e-3, "{} vs {}", l.content_h, printed.content_h);
    for (i, want) in boxes.iter().enumerate() {
        let got = p.word_bounds_view(i as u32);
        assert!(
            (got.0 - want.0).abs() < 1e-3 && (got.1 - want.1).abs() < 1e-3,
            "word {i} moved: {:?} vs {:?}",
            got,
            want
        );
    }
    // and the leading still opens the lines up, as it does without reflow
    let spread = LayoutSpec { line_spacing: 1.5, ..base };
    let want = p.layout(&spread).clone();
    let got = p.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 1.0, ..Default::default() }), ..spread }).clone();
    assert_eq!(want.line_dy, got.line_dy, "line spacing must work at the printed size");
    assert!((want.content_h - got.content_h).abs() < 1e-3);
}

#[test]
fn view_zoom_pans_about_the_fingers_and_clamps() {
    let v = View { scale: 1.0, offset_x: 0.0, offset_y: 0.0 };
    // the point under the fingers stays under them
    let z = v.zoom_about(100.0, 50.0, 2.0, 0.0, 0.0);
    assert_eq!(z.scale, 2.0);
    assert!((z.offset_x + 100.0).abs() < 1e-4 && (z.offset_y + 50.0).abs() < 1e-4);
    // and still does when the clamp cuts the factor down
    let hit = v.zoom_about(100.0, 50.0, 100.0, 0.0, 4.0);
    assert_eq!(hit.scale, 4.0);
    assert!((100.0 - (100.0 - hit.offset_x) / 4.0).abs() < 1e-3);
    // content smaller than the viewport is centred; larger content keeps it covered
    let small = View { scale: 1.0, offset_x: -80.0, offset_y: 0.0 }.clamp(100.0, 100.0, 300.0, 300.0);
    assert!((small.offset_x - 100.0).abs() < 1e-4);
    let big = View { scale: 1.0, offset_x: 50.0, offset_y: -900.0 }.clamp(400.0, 1000.0, 300.0, 300.0);
    assert_eq!((big.offset_x, big.offset_y), (0.0, -700.0));
    assert_eq!(swipe_direction(60.0, 5.0, 0.0, 0.0), 1);
    assert_eq!(swipe_direction(-60.0, 5.0, 0.0, 0.0), -1);
    assert_eq!(swipe_direction(10.0, 5.0, 0.0, 0.0), 0);
    assert_eq!(swipe_direction(60.0, 300.0, 0.0, 0.0), 0);
}

#[test]
fn view_anchor_holds_a_word_across_a_relayout() {
    let mut p = packed_page();
    let base = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    p.layout(&base);
    let word = 5u32;
    let v = View { scale: 1.0, offset_x: 0.0, offset_y: 0.0 };
    let (_, y0, _, y1) = p.word_bounds_view(word);
    let was = y0 + 0.5 * (y1 - y0);
    // the reader zooms in: the page reflows and the word moves to another row
    p.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }), ..base });
    let moved = {
        let (_, a, _, b) = p.word_bounds_view(word);
        a + 0.5 * (b - a)
    };
    assert!((moved - was).abs() > 1.0, "the relayout should have moved the word");
    // anchoring brings that point back to where the fingers are
    let v2 = p.view_anchor(v, word, (0.5, 0.5), (0.0, was), (200.0, 400.0));
    let (_, a, _, b) = p.word_bounds_view(word);
    let now = v2.offset_y + v2.scale * (a + 0.5 * (b - a));
    assert!((now - was).abs() < 0.01, "anchored to {now}, wanted {was}");
}

#[test]
fn view_geometry_follows_the_words_it_describes() {
    let mut p = packed_page();
    let base = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    for reflow in [None, Some(ReflowSpec { zoom: 1.6, ..Default::default() })] {
        p.layout(&LayoutSpec { reflow, ..base });
        let tag = if reflow.is_some() { "reflowed" } else { "printed" };
        for w in 0..p.data().words.len() as u32 {
            let (x0, y0, x1, y1) = p.word_bounds_view(w);
            // an ink band sits on the word's own ink
            let b = p.word_bands_view(&[w], BandHeight::Ink, 0.0, 0.0);
            assert_eq!(b.len(), 1, "{tag}: one band per word");
            assert!(
                (b[0].x0 - x0).abs() < 0.01 && (b[0].y0 - y0).abs() < 0.01 && (b[0].x1 - x1).abs() < 0.01,
                "{tag}: band {:?} off word {w} {:?}",
                (b[0].x0, b[0].y0, b[0].x1, b[0].y1),
                (x0, y0, x1, y1)
            );
            // and the hit box contains that ink, and answers where a tap in it does
            let area = p.hit_areas_view(0.6).into_iter().find(|a| a.word == w).expect("an area per word");
            assert!(
                area.x0 <= x0 + 0.01 && area.x1 >= x1 - 0.01 && area.y0 <= y0 + 0.01 && area.y1 >= y1 - 0.01,
                "{tag}: area misses word {w}"
            );
            let hit = p.hit_test_view((area.x0 + area.x1) / 2.0, (area.y0 + area.y1) / 2.0, &HitOptions::default());
            assert_eq!(hit.map(|h| h.word), Some(w), "{tag}: a tap in word {w}'s box");
        }
    }
}

#[test]
fn reflow_centred_splits_what_is_left_over_between_the_margins() {
    let mut p = packed_page();
    let base = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    let q = p.quant();
    let edges = |p: &mut Page, fill: Fill| -> Vec<(f32, f32)> {
        let l = p
            .layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 1.5, fill, ..Default::default() }), ..base })
            .clone();
        let flow = l.reflow.clone().unwrap();
        flow.row_words
            .iter()
            .filter(|ws| !ws.is_empty())
            .map(|ws| {
                let (first, last) = (&p.data().words[ws[0] as usize], &p.data().words[ws[ws.len() - 1] as usize]);
                (
                    flow.row_w - flow.word_place[ws[0] as usize].apply(first.bbox.x1 as f32 / q, 0.0).0,
                    flow.word_place[ws[ws.len() - 1] as usize].apply(last.bbox.x0 as f32 / q, 0.0).0,
                )
            })
            .collect()
    };
    let ragged = edges(&mut p, Fill::Ragged);
    let centred = edges(&mut p, Fill::Centred);
    assert_eq!(ragged.len(), centred.len());
    for (r, c) in ragged.iter().zip(&centred) {
        // ragged rows start at the right margin; centred rows share the leftover evenly
        assert!(r.0.abs() < 0.01, "a ragged row starts at the right margin");
        assert!((c.0 - c.1).abs() < 0.01, "a centred row has equal margins: {} and {}", c.0, c.1);
    }
    // a row that fills its width is left where it is
    assert!(centred.iter().any(|(l, _)| *l > 0.01), "some row should have room to centre");
}

#[test]
fn reflow_even_gaps_keep_the_letters_the_same_distance_apart() {
    let mut p = packed_page();
    let base = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    // nothing relaxed: this is about the gap the engine picks, not about evening the rows
    let spread = |p: &mut Page, gaps: GapMode| -> f32 {
        let spec = ReflowSpec { zoom: 1.5, gaps, relax: 0.0, ..Default::default() };
        let l = p.layout(&LayoutSpec { reflow: Some(spec), ..base }).clone();
        let flow = l.reflow.clone().unwrap();
        let mut v: Vec<f32> = Vec::new();
        for ws in &flow.row_words {
            for pair in ws.windows(2) {
                // the distance between the letters of two words, as placed
                let a = flow.word_place[pair[0] as usize].apply(p.word_body(pair[0]).0, 0.0).0;
                let b = flow.word_place[pair[1] as usize].apply(p.word_body(pair[1]).1, 0.0).0;
                v.push(a - b);
            }
        }
        let mean = v.iter().sum::<f32>() / v.len() as f32;
        (v.iter().map(|x| (x - mean) * (x - mean)).sum::<f32>() / v.len() as f32).sqrt()
    };
    let even = spread(&mut p, GapMode::Uniform);
    assert!(even < 0.01, "even gaps should not vary: spread {even}");
    // and the engine asks for them by default
    assert_eq!(ReflowSpec::default().gaps, GapMode::Uniform);
}

#[test]
fn words_are_spaced_by_the_air_between_their_strokes() {
    let p = packed_page();
    // the test page sets its words apart, so none of them is drawn inside another
    for w in 0..p.data().words.len() as u32 - 1 {
        assert_eq!(p.words_interlock(w, w + 1), None, "word {w} should not be drawn inside the next");
    }
    // its words are plain squares 16 wide with 4 between them, so the air is that 4
    let air = p.words_clearance(1, 2, 0.0).expect("the two share a band");
    assert!((air - 4.0).abs() < 0.01, "air {air}");
    // and the shift that would leave 10 units of air moves the second word by the difference
    let shift = p.shift_for_clearance(1, 2, 10.0).unwrap();
    assert!((shift + 6.0).abs() < 0.01, "shift {shift}");
    assert!((p.words_clearance(1, 2, shift).unwrap() - 10.0).abs() < 0.01);
}

/// A page's viewBox is what the page is, and ink outside it gets no outline to draw.
///
/// p17 of the Hafs KFGQPC mushaf is the only page of 604 whose artwork draws anything outside
/// the box: a page number below it and a running head above it, four paths in all. The record
/// and the box stay — a crop still has them — but there is nothing for a host to stroke.
///
/// Needs dist/pages from scripts/sync-test-data.sh, and is skipped without it unless
/// QVP_REQUIRE_DATA is set.
#[test]
fn ink_outside_the_page_box_gets_no_outline() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dist/pages/017.qvp");
    if !path.exists() {
        assert!(
            std::env::var("QVP_REQUIRE_DATA").is_err(),
            "dist/pages is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh"
        );
        return;
    }
    let p = Page::load(&std::fs::read(&path).unwrap()).unwrap();
    let (w, h, q) = (p.width(), p.height(), p.quant());
    let mut off_page = 0;
    for (i, rec) in p.data.paths.iter().enumerate() {
        let bb = rec.bbox;
        let (x0, y0) = (bb.x0 as f32 / q, bb.y0 as f32 / q);
        let (x1, y1) = (bb.x1 as f32 / q, bb.y1 as f32 / q);
        let g = &p.geom.table[i];
        if x1 <= 0.0 || y1 <= 0.0 || x0 >= w || y0 >= h {
            off_page += 1;
            assert_eq!(g.op_count, 0, "path {i} is outside the {w}x{h} box and still has an outline");
            assert_eq!(g.pt_count, 0);
        } else {
            assert!(g.op_count > 0, "path {i} is on the page and lost its outline");
        }
    }
    assert_eq!(off_page, 4, "p17 carries its page number and its running head outside the box");
    // the paths are still there to be counted, cropped and exported
    assert_eq!(p.geom.table.len(), p.data.paths.len());
}
