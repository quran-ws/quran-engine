//! Memorisation masks and their transitions.
use super::*;

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
