//! Styles, the clock, highlights and the renderer contract.
use super::*;

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
fn an_idle_tick_visits_no_path_and_leaves_the_colours_alone() {
    let mut p = page();
    p.style(Selector::Word(0), Paint::new(0xff0000ff));
    let h = p.style(Selector::Path(3), Paint::fade(0x00ff00ff, 100));
    p.tick(0.0);
    assert_eq!(p.animating, vec![3], "only the fading path is listed, by its index");
    assert!(p.tick(50.0));
    assert!(!p.tick(100.0), "the transition ends on time");
    assert!(p.animating.is_empty(), "a finished transition leaves the list");
    let settled = p.colors().to_vec();
    assert_eq!(settled[0], 0xff0000ff);
    assert_eq!(settled[3], 0x00ff00ff);
    for ms in 1..=5 {
        assert!(!p.tick(100.0 + 1000.0 * ms as f64));
        assert!(p.animating.is_empty());
        assert_eq!(p.colors(), &settled[..], "an idle tick changes no colour");
    }
    // A fade back restarts the clock for that path alone, and a rule with no transition
    // that lands mid-fade settles the path at once and takes it off the list.
    p.remove_style(h);
    p.tick(6000.0);
    assert_eq!(p.animating, vec![3]);
    p.style(Selector::Path(3), Paint::new(0x0000ffff));
    p.tick(6010.0);
    assert!(p.animating.is_empty());
    assert_eq!(p.color_of(3), 0x0000ffff);
    assert_eq!(p.colors()[3], 0x0000ffff);
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
