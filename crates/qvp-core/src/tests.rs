use super::*;

fn square(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<Cmd> {
    vec![Cmd::MoveTo(x0, y0), Cmd::LineTo(x1, y0), Cmd::LineTo(x1, y1), Cmd::LineTo(x0, y1), Cmd::Close]
}

/// Two words on one line: A = [10,20]×[10,20] units (word 1:1:1), B = [30,40]×[10,20]
/// (word 1:1:2) with two marks above it; plus a second line with word C (1:2:1).
fn page() -> Page {
    let mut ops = Vec::new();
    let mut paths = Vec::new();
    let mut strings = vec![];
    let mut add = |cmds: &[Cmd], kind: PathKind, mark: Mark, fam: Family| {
        let bb = cmds_bbox(cmds);
        let off = ops.len() as u32;
        encode_cmds(cmds, bb.x0, bb.y0, &mut ops);
        paths.push(PathRec { kind, mark, family: fam, flags: PF_EVENODD, ox: bb.x0, oy: bb.y0, op_off: off, op_len: ops.len() as u32 - off, bbox: bb });
        bb
    };
    let a = add(&square(1000, 1000, 2000, 2000), PathKind::Body, Mark::None, Family::None);
    let mut b = add(&square(3000, 1000, 4000, 2000), PathKind::Body, Mark::None, Family::None);
    b.union(&add(&square(3200, 500, 3400, 700), PathKind::Mark, Mark::Fathah, Family::Diacritic));
    b.union(&add(&square(3600, 500, 3800, 700), PathKind::Mark, Mark::Dot, Family::Dots));
    let c = add(&square(1000, 5000, 4000, 6000), PathKind::Body, Mark::None, Family::None);
    let mut l1 = a;
    l1.union(&b);
    let mut st = |s: &str| { strings.push(s.to_owned()); (strings.len() - 1) as u16 };
    let (ta, ta_s, tb, tb_s, tc, tc_s) = (st("ذَٰلِكَ"), st("ذلك"), st("ٱلْكِتَٰبُ"), st("الكتاب"), st("لَا"), st("لا"));
    let data = PageData {
        header: Header { version: VERSION, quant: 100, page: 1, flags: 0, width: 100.0, height: 100.0 },
        lines: vec![LineRec { line_no: 1, first_word: 0, n_words: 2, bbox: l1 }, LineRec { line_no: 2, first_word: 2, n_words: 1, bbox: c }],
        ayahs: vec![
            AyahRec { surah: 1, ayah: 1, fragment: 1, fragments: 1, flags: AF_JUZ_START | AF_HIZB_START | AF_RUBU_AL_HIZB_START, first_word: 0, n_words: 2, ayah_mark_deco: NONE_U16, rubu_al_hizb: 1, bbox: l1 },
            AyahRec { surah: 1, ayah: 2, fragment: 1, fragments: 1, flags: 0, first_word: 2, n_words: 1, ayah_mark_deco: NONE_U16, rubu_al_hizb: 0, bbox: c },
        ],
        words: vec![
            WordRec { surah: 1, ayah: 1, word: 1, line_idx: 0, ayah_idx: 0, text: ta, rasm_imlai: ta, qpc: ta, rasm: ta_s, search: ta_s, first_path: 0, n_paths: 1, bbox: a },
            WordRec { surah: 1, ayah: 1, word: 2, line_idx: 0, ayah_idx: 0, text: tb, rasm_imlai: tb, qpc: tb, rasm: tb_s, search: tb_s, first_path: 1, n_paths: 3, bbox: b },
            WordRec { surah: 1, ayah: 2, word: 1, line_idx: 1, ayah_idx: 1, text: tc, rasm_imlai: tc, qpc: tc, rasm: tc_s, search: tc_s, first_path: 4, n_paths: 1, bbox: c },
        ],
        paths,
        decos: vec![],
        glyphs: vec![],
        insts: vec![],
        ops,
        strings,
    };
    Page::load(&encode(&data)).unwrap()
}

#[test]
fn hit_testing() {
    let p = page();
    assert_eq!(p.hit_test(15.0, 15.0), Some(Hit { word: 0, path: 0, deco: NONE }));
    assert_eq!(p.hit_test(35.0, 15.0), Some(Hit { word: 1, path: 1, deco: NONE }));
    assert_eq!(p.hit_test(33.0, 6.0), Some(Hit { word: 1, path: 2, deco: NONE }));
    assert_eq!(p.hit_test(33.0, 8.5), Some(Hit { word: 1, path: NONE, deco: NONE }));
    assert_eq!(p.hit_test(25.0, 15.0), None);
    assert_eq!(p.hit_test(50.0, 50.0), None);
    assert_eq!(p.word_text(1), "ٱلْكِتَٰبُ");
    assert_eq!(p.find_word(1, 1, 2), Some(1));
}

#[test]
fn gap_aware_hit_and_hit_boxes() {
    let p = page();
    let o = HitOptions::default();
    // in the gap between A (x 10-20) and B (x 30-40): gap 10, preceding = B (right) gets 60%
    let h = p.hit_test_ex(25.0, 15.0, &o).unwrap();
    assert_eq!((h.word, h.exact), (1, false), "x=25 is within B's 60% share (threshold 24)");
    let h = p.hit_test_ex(22.0, 15.0, &o).unwrap();
    assert_eq!((h.word, h.exact), (0, false));
    assert!((h.distance - 2.0).abs() < 1e-4);
    // above the line but within its pitch band → still resolves
    let h = p.hit_test_ex(15.0, 1.0, &o).unwrap();
    assert_eq!(h.word, 0);
    assert!(p.hit_test_ex(15.0, 1.0, &HitOptions { max_distance: 5.0, ..o }).is_none());
    let hb = p.hit_boxes(0.6);
    assert_eq!(hb.len(), 3);
    assert!((hb[0].x1 - 24.0).abs() < 1e-4 && (hb[1].x0 - 24.0).abs() < 1e-4, "hit boxes meet at the gap split");
    let bands = p.line_bands();
    assert_eq!(bands.len(), 2);
    assert!(bands[0].y1 <= bands[1].y0 + 1e-3);
}

#[test]
fn styles_layers_handles_and_subword() {
    let mut p = page();
    assert!(p.styled().is_empty());
    let h_ayah = p.style(Selector::Ayah(1, 1), Paint::new(0x0000ffff));
    let h_word = p.style(Selector::Word(1), Paint::new(0x00ff00ff));
    let h_fam = p.style(Selector::Family(Family::Diacritic), Paint::new(0xff0000ff));
    assert_eq!(p.color_of(0), 0x0000ffff, "ayah applies to word 0");
    assert_eq!(p.color_of(1), 0x00ff00ff, "word beats ayah");
    assert_eq!(p.color_of(2), 0x00ff00ff, "word beats family for its mark");
    p.unstyle(h_word);
    assert_eq!(p.color_of(2), 0x0000ffff, "ayah beats family");
    p.unstyle(h_ayah);
    assert_eq!(p.color_of(2), 0xff0000ff, "family applies to the mark only");
    // higher layer wins regardless of specificity
    let h_theme = p.style_in(LAYER_THEME, Selector::Page, Paint::new(0x111111ff));
    assert_eq!(p.color_of(2), 0x111111ff);
    p.unstyle(h_theme);
    p.unstyle(h_fam);
    // sub-word: the 2nd mark (index 1) of word 1 is the dot
    let h = p.style(Selector::WordMark(1, 1), Paint::new(0xabcdefff));
    assert_eq!(p.color_of(3), 0xabcdefff);
    assert_eq!(p.color_of(2), DEFAULT_INK);
    p.unstyle(h);
    let h = p.style(Selector::WordMarkNamed(1, Mark::Fathah, 0), Paint::new(0x123456ff));
    assert_eq!(p.color_of(2), 0x123456ff);
    p.unstyle(h);
    let h = p.style(Selector::Category(Category::LetterDot), Paint::new(0x0a0b0cff));
    assert_eq!(p.color_of(3), 0x0a0b0cff);
    assert_eq!(p.styled(), vec![(3, 0x0a0b0cff)]);
    p.restyle(h, Paint::new(0x0d0e0fff));
    assert_eq!(p.color_of(3), 0x0d0e0fff);
    p.unstyle(h);
    assert!(p.styles.is_empty());
    let h = p.hide(Selector::Path(2));
    assert_eq!(p.color_of(2) & 0xff, 0, "alpha 0 hides");
    p.unstyle(h);
    // theme handle undoes as a whole
    let t = p.theme(&Theme { ink: Some(0x222222ff), dots: Some(0xcc0000ff), ..Default::default() });
    assert_eq!(p.color_of(0), 0x222222ff);
    assert_eq!(p.color_of(3), 0xcc0000ff);
    p.unstyle(t);
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
    p.unstyle(h);
    assert!(p.tick(210.0), "fade back uses the previous rule's duration");
    assert!(!p.tick(400.0));
    assert_eq!(p.color_of(0), DEFAULT_INK);
}

#[test]
fn highlights_bands_and_animation() {
    let mut p = page();
    p.tick(0.0);
    let st = HighlightStyle { mode: HighlightMode::Both, ink: 0x00aa00ff, band: 0xffcc0080, transition_ms: 100, ..Default::default() };
    let h = p.highlight(&Target::Ayah(1, 1), st);
    assert_eq!(p.color_of(0), DEFAULT_INK);
    p.tick(1000.0);
    assert_eq!(p.color_of(0), 0x00aa00ff);
    let boxes = p.highlight_boxes_view();
    assert_eq!(boxes.len(), 1, "one band box: both words on line 1");
    assert!((boxes[0].x0 - (10.0 - 1.2)).abs() < 1e-3 && (boxes[0].x1 - (40.0 + 1.2)).abs() < 1e-3);
    assert_eq!(boxes[0].color, 0xffcc0080);
    // slide to word C on line 2
    p.rehighlight(h, &Target::Word(2));
    p.tick(1050.0);
    let mid = p.highlight_boxes_view();
    assert_eq!(mid.len(), 1);
    assert!(mid[0].y0 > boxes[0].y0, "band is moving down");
    p.tick(2000.0);
    let end = p.highlight_boxes_view();
    assert!((end[0].x0 - (10.0 - 1.2)).abs() < 1e-3 && end[0].y0 > 30.0, "{end:?}");
    assert_eq!(p.color_of(4), 0x00aa00ff, "word C's body path");
    assert_eq!(p.color_of(0), DEFAULT_INK);
    p.unhighlight(h);
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
    assert_eq!(p.text_of(&p.resolve(&Target::Page), Form::Search, " ", " / "), "ذلك الكتاب / لا");
    let m = p.search("الكتاب", &SearchOptions::default());
    assert_eq!(m.len(), 1);
    assert_eq!(m[0].word, 1);
    assert!(p.search("كتاب", &SearchOptions { mode: SearchMode::Exact, ..Default::default() }).is_empty());
    assert_eq!(p.search("الكتب", &SearchOptions { loose: false, ..Default::default() }).len(), 0, "strict pass: different letters");
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
    assert_eq!(p.reveal_next(1), 1);
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
    p.unstyle(h);
    p.reveal_stop();
    assert_eq!(p.color_of(1), DEFAULT_INK);
    let cb = p.crop_box(&Target::Ayah(1, 1), 2.0, true).unwrap();
    assert!((cb.x0 - 8.0).abs() < 1e-4 && (cb.x1 - 42.0).abs() < 1e-4 && (cb.y0 - 3.0).abs() < 1e-4);
    let svg = p.crop_svg(&Target::Word(0), 1.0, false, Some(0xffffffff)).unwrap();
    assert!(svg.starts_with("<svg") && svg.contains("<rect") && svg.matches("<path").count() == 1);
}

#[test]
fn layout_fill_height_and_view_hit() {
    let mut p = page();
    let spec = LayoutSpec { viewport_w: 200.0, viewport_h: 2500.0, pad_top: 50.0, pad_bottom: 50.0, fill_height: true, ..Default::default() };
    let l = p.layout(&spec).clone();
    assert_eq!(l.scale, 2.0);
    // a short page (2 lines) takes the rows of the 15-line grid: 2400 px / 15 = 160 px
    // = 80 units a row, so 40 units of leading on the printed pitch of 40
    let delta = 40.0;
    assert!((l.pitch - (40.0 + delta)).abs() < 1e-3, "pitch {}", l.pitch);
    assert_eq!(l.line_dy.len(), 2);
    // printed geometry kept: the two lines differ by exactly one delta; the grid (15 rows
    // = 1200 units) is centred in the padded viewport, the page centred on it (slot0 = 6.5)
    assert!((l.line_dy[1] - l.line_dy[0] - delta).abs() < 1e-3);
    assert!((l.line_dy[0] - (25.0 + (1200.0 - 100.0) / 2.0 - 7.0 * delta + 6.5 * delta)).abs() < 1e-3);
    assert!((l.content_h - 2500.0).abs() < 1e-3);
    // slot boundary halfway between the laid-out line centres
    let mid = ((15.0 + l.line_dy[0]) + (55.0 + l.line_dy[1])) / 2.0 * 2.0;
    assert!((l.line_slots[0].1 - mid).abs() < 1e-3 && (l.line_slots[1].0 - mid).abs() < 1e-3);
    let vx = l.ox + 15.0 * 2.0;
    let vy = l.oy + (15.0 + l.line_dy[0]) * 2.0;
    assert_eq!(p.hit_test_view(vx, vy), Some(Hit { word: 0, path: 0, deco: NONE }));
    assert_eq!(p.hit_test_view(vx, 5.0), None);
    let g = p.hit_test_view_ex(l.ox + 25.0 * 2.0, vy, &HitOptions::default()).unwrap();
    assert_eq!((g.word, g.exact), (1, false));
    // rows shorter than the printed pitch: the print is the floor and the grid outgrows the viewport
    let tight = p.layout(&LayoutSpec { viewport_h: 1100.0, ..spec }).clone();
    assert!((tight.pitch - 40.0).abs() < 1e-3 && (tight.content_h - (100.0 + 15.0 * 40.0 * 2.0)).abs() < 1e-3);
    // a full page (the grid is its own line count) spreads its printed height: (1200 − 100) units
    // over its one gap — and never squeezes below the print
    let full = p.layout(&LayoutSpec { nominal_lines: 2, ..spec }).clone();
    assert!((full.pitch - (40.0 + 1100.0)).abs() < 1e-3 && (full.content_h - 2500.0).abs() < 1e-3);
    let squeezed = p.layout(&LayoutSpec { viewport_h: 150.0, pad_top: 0.0, pad_bottom: 0.0, nominal_lines: 2, ..spec }).clone();
    assert!((squeezed.pitch - 40.0).abs() < 1e-3 && (squeezed.content_h - 200.0).abs() < 1e-3);
    let l2 = p.layout(&LayoutSpec { viewport_w: 200.0, viewport_h: 1100.0, line_spacing: 1.5, ..Default::default() }).clone();
    assert!((l2.pitch - p.natural_pitch() * 1.5).abs() < 1e-3);
    assert!((l2.line_dy[1] - l2.line_dy[0] - 20.0).abs() < 1e-3, "×1.5 adds half a pitch between lines");
    // as printed: no shift at all
    let l3 = p.layout(&LayoutSpec { viewport_w: 200.0, viewport_h: 1100.0, ..Default::default() }).clone();
    assert!(l3.line_dy.iter().all(|d| d.abs() < 1e-6) && (l3.content_h - 200.0).abs() < 1e-3);
    assert!((gap_to_fill(345.0, 550.0, 15, 390.0, 844.0, f32::INFINITY) - 14.0).abs() < 0.1);
    assert_eq!(gap_to_fill(345.0, 550.0, 15, 820.0, 1180.0, 100.0), 0.0);
    assert!((wasted_fraction(345.0, 550.0, 390.0, 844.0) - 0.263).abs() < 0.01);
    let wb = p.word_box_view(2);
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
