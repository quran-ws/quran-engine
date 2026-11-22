//! The letter layer: attaching it, the pieces it appends, and reading its letters.
use super::*;
use crate::letters::*;
use crate::letters::{check_fits, word_runs_overlap};
use qvp_format::letters::{encode_letters, LetterData, PieceRec};

const RED: Rgba = 0xff0000ff;
const BLUE: Rgba = 0x0000ffff;

fn layer_bytes() -> Vec<u8> {
    encode_letters(&letters_layer())
}

fn page_without_layer() -> Page {
    Page::from_data(letters_page_data()).unwrap()
}

fn points(p: &Page, path: usize) -> Vec<(f32, f32)> {
    let g = p.geometry().table[path];
    let pts = &p.geometry().pts[g.pt_start as usize..(g.pt_start + g.pt_count) as usize];
    pts.chunks(2).map(|c| (c[0], c[1])).collect()
}

#[test]
fn a_page_without_a_layer_answers_nothing() {
    let p = page();
    assert!(!p.has_letters());
    assert_eq!(p.letter_count(0), 0);
    assert_eq!(p.letter_info(0, 0), None);
    assert!(p.letter_members(0, 0).is_empty());
    assert_eq!(p.path_letter(0), None);
    assert_eq!(p.letter_at_offset(0, 0), None);
    assert_eq!(p.draw_revision(), 0);
    assert_eq!(p.geometry().table.len(), 5);
}

#[test]
fn attaching_appends_the_pieces() {
    let mut p = letters_page();
    assert!(p.has_letters());
    assert_eq!(p.geometry().table.len(), 10);
    assert_eq!(p.colors().len(), 10);
    assert_eq!(p.draw_revision(), 1);
    let (row0, row8) = (p.geometry().table[0], p.geometry().table[8]);
    assert_eq!((row8.flags, row8.word, row8.line, row8.extra), (row0.flags, 0, 0, row0.extra));
    assert_eq!(points(&p, 8), vec![(50.0, 10.0), (60.0, 10.0), (60.0, 20.0), (50.0, 20.0)]);
    assert_eq!(points(&p, 9), vec![(39.0, 10.0), (50.0, 10.0), (50.0, 20.0), (39.0, 20.0)]);
    assert_eq!(p.path_deco(8), NONE);
    assert_eq!(p.path_deco(9), NONE);
}

#[test]
fn attaching_leaves_every_page_path_as_it_was() {
    let before = page_without_layer();
    let after = letters_page();
    let (b, a) = (before.geometry(), after.geometry());
    let n = b.table.len();
    assert_eq!(n, 8);
    assert_eq!(&a.ops[..b.ops.len()], &b.ops[..]);
    assert_eq!(&a.pts[..b.pts.len()], &b.pts[..]);
    for i in 0..n {
        let (x, y) = (b.table[i], a.table[i]);
        assert_eq!(
            (x.op_start, x.op_count, x.pt_start, x.pt_count, x.flags, x.word, x.line, x.extra),
            (y.op_start, y.op_count, y.pt_start, y.pt_count, y.flags, y.word, y.line, y.extra),
            "row {i}"
        );
    }
}

#[test]
fn the_word_ranges_hold_no_piece() {
    let p = letters_page();
    let ranges: Vec<(u32, u16)> = p.data().words.iter().map(|w| (w.first_path, w.n_paths)).collect();
    assert_eq!(ranges, vec![(0, 3), (3, 3), (6, 2)]);
    assert_eq!(p.data().paths.len(), 8, "the page's own path table keeps its length");
}

#[test]
fn letters_read_back() {
    let p = letters_page();
    assert_eq!([p.letter_count(0), p.letter_count(1), p.letter_count(2), p.letter_count(3)], [2, 2, 1, 0]);
    let ba = p.letter_info(0, 0).unwrap();
    assert_eq!((ba.word, ba.index, ba.at, ba.text), (0, 0, 0, 'ب'));
    assert_eq!((ba.n_members, ba.n_marks), (3, 2));
    assert!(ba.more_at.is_empty());
    assert_eq!((ba.x0, ba.y0, ba.x1, ba.y1), (50.0, 6.0, 60.0, 24.0));
    assert!(ba.has_ink);
    let lam = p.letter_info(1, 0).unwrap();
    assert_eq!((lam.text, lam.more_at.clone()), ('ل', vec![1]));
    let lam0 = p.letter_info(0, 1).unwrap();
    assert_eq!((lam0.at, lam0.x0, lam0.y0, lam0.x1, lam0.y1), (2, 39.0, 10.0, 50.0, 20.0), "a piece's own box");
    assert_eq!(p.letter_info(0, 2), None);
    assert_eq!(p.letter_info(3, 0), None);
}

#[test]
fn members_read_back() {
    let p = letters_page();
    let m = |path, at, mark, is_body, is_cut| LetterMember { path, at, mark, is_body, is_cut };
    assert_eq!(
        p.letter_members(0, 0),
        vec![m(8, 0, Mark::None, true, true), m(1, 0, Mark::Dot, false, false), m(2, 1, Mark::Fathah, false, false)]
    );
    assert_eq!(p.letter_members(0, 1), vec![m(9, 2, Mark::None, true, true)]);
    assert_eq!(p.letter_members(2, 0)[1].at, NONE);
    assert_eq!(p.letter_members(2, 0)[1].mark, Mark::OmittedAlif);
    assert!(p.letter_members(2, 1).is_empty());
    assert!(p.letter_members(9, 0).is_empty());
}

#[test]
fn a_path_names_its_letter() {
    let p = letters_page();
    assert_eq!(p.path_letter(0), None, "a cut path draws no one letter");
    assert_eq!(p.path_letter(8), Some((0, 0)));
    assert_eq!(p.path_letter(9), Some((1, 2)));
    assert_eq!(p.path_letter(2), Some((0, 1)));
    assert_eq!(p.path_letter(3), Some((0, 0)));
    assert_eq!(p.path_letter(5), Some((NONE, 3)), "a word-level mark");
    assert_eq!(p.path_letter(7), Some((0, NONE)));
    assert_eq!(p.path_letter(10), None);
    assert_eq!(p.path_letter(NONE), None);
}

#[test]
fn an_offset_names_its_letter() {
    let p = letters_page();
    assert_eq!(p.letter_at_offset(0, 0), Some(0));
    assert_eq!(p.letter_at_offset(0, 1), Some(0), "the fathah");
    assert_eq!(p.letter_at_offset(0, 2), Some(1));
    assert_eq!(p.letter_at_offset(1, 1), Some(0), "the kashida the lam draws");
    assert_eq!(p.letter_at_offset(1, 3), None, "a word-level mark is no letter");
    assert_eq!(p.letter_at_offset(1, 9), None);
    assert_eq!(p.letter_at_offset(1, 0x1_0000), None);
    assert_eq!(p.letter_at_offset(7, 0), None);
}

#[test]
fn a_bad_layer_is_refused_and_the_old_one_kept() {
    let mut p = letters_page();
    let check = |p: &mut Page| {
        assert_eq!(p.geometry().table.len(), 10);
        assert_eq!(p.colors().len(), 10);
        assert_eq!(p.letter_count(0), 2);
        assert_eq!(p.draw_revision(), 1);
    };
    let other = LetterData { page: 2, ..letters_layer() };
    assert!(p.attach_letters(&encode_letters(&other)).is_err());
    check(&mut p);
    let big = vec![0u8; defaults::MAX_LETTER_LAYER_BYTES + 1];
    assert_eq!(p.attach_letters(&big), Err(Error::Bounds("letter layer size")));
    check(&mut p);
    let bytes = layer_bytes();
    for n in 0..bytes.len() {
        assert!(p.attach_letters(&bytes[..n]).is_err(), "{n} bytes");
    }
    check(&mut p);
    // a layer for another page leaves a page with no layer with none
    let mut bare = page();
    assert!(bare.attach_letters(&bytes).is_err());
    assert!(!bare.has_letters());
    assert_eq!(bare.geometry().table.len(), 5);
}

#[test]
fn a_second_attach_replaces_the_first() {
    let mut p = letters_page();
    let ops = p.geometry().ops.len();
    let pts = p.geometry().pts.len();
    p.attach_letters(&layer_bytes()).unwrap();
    assert_eq!(p.geometry().table.len(), 10);
    assert_eq!((p.geometry().ops.len(), p.geometry().pts.len()), (ops, pts), "the first pieces are dropped");
    assert_eq!(p.colors().len(), 10);
    assert_eq!(p.draw_revision(), 2);
    assert_eq!(p.letter_members(0, 0)[0].path, 8);
    // a layer with no cut: the pieces of the first are gone and path 0 draws a letter again
    let mut uncut = letters_layer();
    uncut.pieces.clear();
    uncut.members[0].path = qvp_format::letters::MemberPath::Page(0);
    uncut.members.remove(3);
    uncut.letters.remove(1);
    for (l, first) in uncut.letters.iter_mut().zip([0, 3, 4, 5]) {
        l.first_member = first;
    }
    p.attach_letters(&encode_letters(&uncut)).unwrap();
    assert_eq!(p.geometry().table.len(), 8);
    assert_eq!(p.colors().len(), 8);
    assert_eq!(p.letter_count(0), 1);
    assert_eq!(p.path_letter(0), Some((0, 0)));
    assert_eq!(p.path_letter(8), None);
    assert_eq!(p.draw_revision(), 3);
}

#[test]
fn a_short_qpc_drops_the_offset() {
    let mut p = letters_page();
    assert_eq!(p.attach_words_json(r#"{"1:1:2": {"qpc": "لـه"}}"#.as_bytes()), Some(1));
    assert_eq!(p.path_letter(5), Some((NONE, NONE)));
    assert_eq!(p.letter_at_offset(1, 2), Some(1));
    assert_eq!(p.letter_at_offset(1, 1), Some(0), "the kashida is inside the text");
    // the same with the sidecar attached first
    let mut q = page_without_layer();
    q.attach_words_json(r#"{"1:1:2": {"qpc": "ل"}}"#.as_bytes()).unwrap();
    q.attach_letters(&layer_bytes()).unwrap();
    assert_eq!(q.path_letter(5), Some((NONE, NONE)));
    assert_eq!(q.letter_info(1, 1).unwrap().at, NONE, "a letter past the text has no offset");
    assert_eq!(q.letter_count(1), 2, "but keeps its index");
    assert_eq!(q.letter_info(1, 0).unwrap().more_at, vec![NONE]);
    assert_eq!(q.letter_at_offset(1, 1), None);
    assert_eq!(q.letter_at_offset(1, 0), Some(0));
    // a longer text later brings the offsets back
    q.attach_words_json(r#"{"1:1:2": {"qpc": "لـهۖ"}}"#.as_bytes()).unwrap();
    assert_eq!(q.path_letter(5), Some((NONE, 3)));
    assert_eq!(q.letter_at_offset(1, 1), Some(0));
}

#[test]
fn without_qpc_offsets_are_used_as_stored() {
    let mut data = letters_page_data();
    for w in &mut data.words {
        w.qpc = NONE_U16;
    }
    let mut p = Page::from_data(data).unwrap();
    p.attach_letters(&layer_bytes()).unwrap();
    assert_eq!(p.path_letter(5), Some((NONE, 3)));
    assert_eq!(p.letter_at_offset(1, 1), Some(0));
    // a sidecar with other forms only leaves them unchecked
    p.attach_words_json(r#"{"1:1:2": {"search": "له"}}"#.as_bytes()).unwrap();
    assert_eq!(p.path_letter(5), Some((NONE, 3)));
}

#[test]
fn a_reflowed_layout_groups_the_pieces_with_their_word() {
    let mut p = page_without_layer();
    let spec = LayoutSpec {
        viewport_w: 200.0,
        viewport_h: 400.0,
        reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }),
        ..Default::default()
    };
    let before = p.layout(&spec).clone();
    assert_eq!(before.path_group.len(), 8);
    p.attach_letters(&layer_bytes()).unwrap();
    let l = p.current_layout().unwrap();
    assert_eq!(l.path_group.len(), 10);
    assert_eq!(l.path_group[8], l.path_group[0]);
    assert_eq!(l.path_group[9], l.path_group[0]);
    assert_eq!(&l.path_group[..8], &before.path_group[..], "no page path changes group");
    assert_eq!(l.groups, before.groups, "positions do not change");
    // a new reflow after the attach covers the pieces too
    let again = p.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 2.5, ..Default::default() }), ..spec });
    assert_eq!(again.path_group.len(), 10);
    assert_eq!(again.path_group[8], again.path_group[0]);
}

#[test]
fn a_reflow_measured_after_the_attach_groups_the_pieces() {
    let mut p = letters_page();
    let spec = LayoutSpec {
        viewport_w: 200.0,
        viewport_h: 400.0,
        reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }),
        ..Default::default()
    };
    let l = p.layout(&spec);
    assert_eq!(l.path_group.len(), 10);
    assert_eq!(l.path_group[9], l.path_group[0]);
}

#[test]
fn attaching_after_a_layout_keeps_its_draw_list() {
    let mut p = page_without_layer();
    let spec = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    p.layout(&spec);
    let all = p.layout_draw_list();
    let band = p.layout_draw_list_in(Some((0.0, 60.0)));
    assert_eq!(all.len(), 8);
    p.attach_letters(&layer_bytes()).unwrap();
    assert_eq!(p.layout_draw_list(), all, "no piece is drawn while no cut path draws its pieces");
    assert_eq!(p.layout_draw_list_in(Some((0.0, 60.0))), band);
    let reflowed = LayoutSpec { reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }), ..spec };
    p.layout(&reflowed);
    let list = p.layout_draw_list_in(Some((0.0, 1000.0)));
    assert!(list.iter().all(|d| d.path < 8));
}

#[test]
fn a_piece_takes_its_sources_context() {
    let mut p = letters_page();
    p.style(Selector::Word(0), Paint::new(RED));
    p.style(Selector::WordMarks(0), Paint::new(BLUE));
    let c = p.colors().to_vec();
    assert_eq!((c[0], c[8], c[9]), (RED, RED, RED));
    assert_eq!((c[1], c[2]), (BLUE, BLUE));
    let mut q = letters_page();
    q.style(Selector::Ayah(1, 1), Paint::new(RED));
    q.style(Selector::Line(2), Paint::new(BLUE));
    q.style(Selector::WordPath(0, 0), Paint::new(BLUE));
    let c = q.colors().to_vec();
    assert_eq!((c[0], c[8], c[9], c[3], c[6]), (BLUE, BLUE, BLUE, RED, BLUE));
    let mut r = letters_page();
    r.style(Selector::Kind(PathKind::Body), Paint::new(RED));
    r.style(Selector::WordBody(1), Paint::new(BLUE));
    r.style(Selector::Category(Category::LetterDot), Paint::new(BLUE));
    let c = r.colors().to_vec();
    assert_eq!((c[8], c[9], c[3], c[1], c[2]), (RED, RED, BLUE, BLUE, DEFAULT_INK));
}

#[test]
fn a_mask_and_a_reveal_take_the_pieces_with_their_word() {
    let mut p = letters_page();
    p.mask(&Target::Word(0), MaskMode::Hide);
    let c = p.colors().to_vec();
    assert_eq!((c[0] & 0xff, c[8] & 0xff, c[9] & 0xff), (0, 0, 0));
    assert_eq!(c[3], DEFAULT_INK);
    // a reveal started before the attach lights the pieces with their word
    let mut r = page_without_layer();
    r.reveal_start(1, false, 0xc9c4b8ff, 0x000000ff, true, 0);
    r.reveal_goto(1);
    r.attach_letters(&layer_bytes()).unwrap();
    let c = r.colors().to_vec();
    assert_eq!((c[0], c[8], c[9], c[3]), (0xc9c4b8ff, 0xc9c4b8ff, 0xc9c4b8ff, 0x000000ff));
}

#[test]
fn a_piece_starts_at_its_sources_colour_state() {
    let mut p = page_without_layer();
    p.style(Selector::Word(0), Paint::fade(RED, 1000));
    p.tick(0.0);
    p.tick(500.0);
    let mid = p.colors()[0];
    assert!(mid != RED && mid != DEFAULT_INK);
    p.attach_letters(&layer_bytes()).unwrap();
    let c = p.colors().to_vec();
    assert_eq!((c[8], c[9]), (mid, mid));
    assert!(p.tick(750.0));
    let c = p.colors().to_vec();
    assert_eq!((c[8], c[9]), (c[0], c[0]));
    // a second attach mid-transition lists each piece once
    p.attach_letters(&layer_bytes()).unwrap();
    let mut listed = p.animating.clone();
    listed.sort_unstable();
    assert_eq!(listed, vec![0, 1, 2, 8, 9]);
    assert!(!p.tick(1000.0));
    let c = p.colors().to_vec();
    assert_eq!((c[0], c[8], c[9]), (RED, RED, RED));
    assert!(p.animating.is_empty());
}

#[test]
fn styled_paths_and_render_leave_out_the_pieces_of_a_whole_path() {
    let mut p = letters_page();
    p.style(Selector::Word(0), Paint::new(RED));
    let styled: Vec<u32> = p.styled_paths().into_iter().map(|(i, _)| i).collect();
    assert_eq!(styled, vec![0, 1, 2]);
    struct Count(Vec<u32>);
    impl Renderer for Count {
        fn begin(&mut self, _: f32, _: f32) {}
        fn fill_path(&mut self, index: u32, _: &[u8], _: &[f32], _: Rgba, _: bool) {
            self.0.push(index);
        }
        fn fill_rect(&mut self, _: f32, _: f32, _: f32, _: f32, _: f32, _: Rgba) {}
        fn end(&mut self) {}
    }
    let mut r = Count(vec![]);
    p.render(&mut r);
    assert_eq!(r.0, (0..8).collect::<Vec<u32>>());
}

#[test]
fn the_letter_index_holds_the_boxes_and_the_cuts() {
    let p = letters_page();
    let lt = p.letters.as_deref().unwrap();
    assert_eq!(lt.first_piece, 8);
    assert_eq!(lt.word_letters, vec![(0, 2), (2, 2), (4, 1)]);
    assert_eq!(lt.sources, vec![(0, 8, 2)]);
    assert_eq!(lt.swapped, vec![false]);
    assert_eq!(lt.slot_of[0], 0);
    assert!(lt.slot_of[1..].iter().all(|&s| s == NONE));
    assert_eq!(lt.piece_box[0], IBox { x0: 5000, y0: 1000, x1: 6000, y1: 2000 });
    // piece 1 reaches past its word's box: its own box and its letter's hold it, the page's
    // word box does not, and the layer's word box does
    let (piece, word) = (lt.piece_box[1], p.data().words[0].bbox);
    assert_eq!(piece.x0, 3900);
    assert!(word.x0 > piece.x0);
    assert_eq!(lt.letter_box[1], piece);
    assert_eq!(lt.word_box[0], IBox { x0: 3900, ..word });
    assert_eq!(lt.word_box[1], p.data().words[1].bbox);
    assert_eq!(lt.path_box(p.data(), 9), lt.piece_box[1]);
    assert_eq!(lt.path_box(p.data(), 1), p.data().paths[1].bbox);
    assert!(lt.letter_has_mark(0, Mark::Dot) && lt.letter_has_mark(0, Mark::Fathah));
    assert!(!lt.letter_has_mark(1, Mark::Dot));
    assert!(lt.letter_has_mark(4, Mark::OmittedAlif));
    assert_eq!(lt.drawn_pieces(0), None);
    assert!(lt.is_drawn(0) && !lt.is_drawn(8) && lt.is_drawn(3));
    let c8 = p.path_ctx[8].letter;
    assert_eq!((c8.letter, c8.letter_in_word, c8.at, c8.is_body, c8.source), (0, 0, 0, true, 0));
    assert_eq!(p.path_ctx[0].letter.pieces, (8, 2));
    assert_eq!(p.path_ctx[2].letter.nth_letter_mark, 1);
    assert_eq!(p.path_ctx[1].letter.nth_letter_mark, 0);
    assert_eq!(p.path_ctx[8].letter.nth_letter_mark, u16::MAX);
    assert_eq!(p.path_ctx[5].letter.letter, NONE);
    assert_eq!(p.path_ctx[5].letter.raw_at, 3);
}

#[test]
fn a_letter_part_reads_from_its_code() {
    assert_eq!(LetterPart::from_u8(0), Some(LetterPart::All));
    assert_eq!(LetterPart::from_u8(1), Some(LetterPart::BodyAndDots));
    assert_eq!(LetterPart::from_u8(2), Some(LetterPart::Marks));
    assert_eq!(LetterPart::from_u8(3), None);
    assert!(LetterPart::All.takes(true) && LetterPart::All.takes(false));
    assert!(LetterPart::BodyAndDots.takes(true) && !LetterPart::BodyAndDots.takes(false));
    assert!(LetterPart::Marks.takes(false) && !LetterPart::Marks.takes(true));
}

/// A geometry row as a comparable tuple.
type Row = (u32, u32, u32, u32, u32, u32, u32, u32);

fn rows(p: &Page) -> Vec<Row> {
    let t = &p.geometry().table;
    t.iter().map(|g| (g.op_start, g.op_count, g.pt_start, g.pt_count, g.flags, g.word, g.line, g.extra)).collect()
}

#[test]
fn a_page_with_no_layer_behaves_as_before() {
    // two pages from the same bytes: one never touches the letter API, the other had a
    // layer refused three times
    let bytes = encode(&letters_page_data());
    let mut a = Page::load(&bytes).unwrap();
    let mut b = Page::load(&bytes).unwrap();
    let layer = layer_bytes();
    assert!(b.attach_letters(&layer[..layer.len() - 1]).is_err());
    assert!(b.attach_letters(&encode_letters(&LetterData { page: 2, ..letters_layer() })).is_err());
    assert!(b.attach_letters(b"not a layer").is_err());
    assert!(!b.has_letters());
    assert_eq!(b.draw_revision(), 0);
    for p in [&mut a, &mut b] {
        p.style(Selector::Word(0), Paint::new(RED));
        p.style(Selector::WordMark(1, 0), Paint::fade(BLUE, 400));
        p.tick(0.0);
        p.tick(100.0);
    }
    assert_eq!(rows(&a), rows(&b));
    assert_eq!(a.geometry().ops, b.geometry().ops);
    assert_eq!(a.geometry().pts, b.geometry().pts);
    assert_eq!(a.colors(), b.colors());
    assert_eq!(a.styled_paths(), b.styled_paths());
    assert_eq!(a.hit_test_exact(55.0, 15.0), b.hit_test_exact(55.0, 15.0));
    assert_eq!(a.hit_test_exact(15.0, 55.0), b.hit_test_exact(15.0, 55.0));
    let printed = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
    let reflowed = LayoutSpec { reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }), ..printed };
    for spec in [printed, reflowed] {
        let (la, lb) = (a.layout(&spec).clone(), b.layout(&spec).clone());
        assert_eq!(la, lb);
        assert_eq!(&la.path_group[..], &lb.path_group[..]);
        assert_eq!(a.layout_draw_list(), b.layout_draw_list());
        assert_eq!(a.layout_draw_list_in(Some((0.0, 60.0))), b.layout_draw_list_in(Some((0.0, 60.0))));
        assert_eq!(a.hit_test_exact_view(110.0, 30.0), b.hit_test_exact_view(110.0, 30.0));
    }
    assert_eq!(b.colors().len(), 8);
    assert_eq!(b.draw_revision(), 0);
}

#[test]
fn a_page_whose_word_runs_overlap_takes_no_layer() {
    // word 1 starts at path 2, so path 2 sits in the runs of words 0 and 1
    let mut data = letters_page_data();
    data.words[1].first_path = 2;
    data.words[1].n_paths = 4;
    let mut p = Page::from_data(data).unwrap();
    assert!(letters_layer().check_against(p.data()).is_ok(), "the format allows it");
    assert!(word_runs_overlap(p.data()));
    assert_eq!(p.attach_letters(&layer_bytes()), Err(Error::Corrupt("overlapping word runs")));
    assert!(!p.has_letters());
    assert_eq!(p.geometry().table.len(), 8);
    assert!(!word_runs_overlap(&letters_page_data()));
}

#[test]
fn a_members_word_is_its_letters_word() {
    let p = letters_page();
    let word = |i: usize| p.path_ctx[i].letter.word;
    // members, a word mark, a cut path and its pieces each carry the layer's word
    assert_eq!((0..10).map(word).collect::<Vec<_>>(), vec![0, 0, 0, 1, 1, 1, 2, 2, 0, 0]);
    let lt = p.letters.as_deref().unwrap();
    for (w, &(first, n)) in lt.word_letters.iter().enumerate() {
        for g in first..first + n {
            let l = &lt.data.letters[g as usize];
            assert_eq!(l.word as usize, w);
            for m in p.letter_members(w as u32, g - first) {
                assert_eq!(word(m.path as usize), w as u32);
            }
        }
    }
}

#[test]
fn a_layer_whose_counts_do_not_fit_is_refused() {
    let base = letters_layer();
    assert_eq!(check_fits(&base, 8, 0, 0), Ok(()));
    let one_word = |n: usize| LetterData {
        letters: (0..n)
            .map(|i| qvp_format::letters::LetterRec {
                word: 0,
                at: i as u16,
                text: 'ب',
                more_at: vec![],
                first_member: 0,
                n_members: 0,
            })
            .collect(),
        ..letters_layer()
    };
    assert_eq!(check_fits(&one_word(u16::MAX as usize), 8, 0, 0), Ok(()));
    assert_eq!(check_fits(&one_word(u16::MAX as usize + 1), 8, 0, 0), Err(Error::Bounds("letters in a word")));
    assert_eq!(check_fits(&base, NONE as usize - 2, 0, 0), Err(Error::Bounds("letter layer pieces")));
    assert_eq!(check_fits(&base, 8, u32::MAX as usize - 5, 0), Err(Error::Bounds("letter layer geometry")));
    assert_eq!(check_fits(&base, 8, 0, u32::MAX as usize - 5), Err(Error::Bounds("letter layer geometry")));
}

#[test]
fn a_second_attach_can_add_pieces() {
    // the lam of word 1 (path 3) is cut too: three pieces in place of two
    let mut more = letters_layer();
    more.members[4].path = qvp_format::letters::MemberPath::Piece(2);
    more.pieces.push(PieceRec { source: 3, cmds: square(2000, 1000, 3000, 2000) });
    let mut p = letters_page();
    p.style(Selector::Word(1), Paint::new(RED));
    p.attach_letters(&encode_letters(&more)).unwrap();
    assert_eq!(p.geometry().table.len(), 11);
    assert_eq!(p.colors().len(), 11);
    assert_eq!(p.colors()[10], RED, "the new piece takes its word's rule");
    assert_eq!(points(&p, 10), vec![(20.0, 10.0), (30.0, 10.0), (30.0, 20.0), (20.0, 20.0)]);
    assert_eq!(p.path_letter(3), None);
    assert_eq!(p.path_letter(10), Some((0, 0)));
    assert_eq!(
        p.letter_members(1, 0)[0],
        LetterMember { path: 10, at: 0, mark: Mark::None, is_body: true, is_cut: true }
    );
    assert_eq!(p.letters.as_deref().unwrap().sources, vec![(0, 8, 2), (3, 10, 1)]);
    assert_eq!(p.path_deco(10), NONE);
    // and back to two
    p.attach_letters(&layer_bytes()).unwrap();
    assert_eq!(p.geometry().table.len(), 10);
    assert_eq!(p.path_letter(3), Some((0, 0)));
    assert_eq!(p.path_ctx[3].letter.pieces, (0, 0));
}

#[test]
fn a_letter_drawn_off_the_page_has_no_ink() {
    // a page 45 units tall leaves word 2 (y 46 to 60) outside the viewBox, as p17's running
    // head is: its paths have no outline, and neither does a piece cut there
    let mut data = letters_page_data();
    data.header.height = 45.0;
    let mut p = Page::from_data(data).unwrap();
    p.attach_letters(&layer_bytes()).unwrap();
    assert!(!p.letter_info(2, 0).unwrap().has_ink);
    assert!(p.letter_info(0, 0).unwrap().has_ink);
}

#[test]
fn a_decoration_keeps_its_own_group_beside_the_pieces() {
    let mut data = letters_page_data();
    let (path, bbox) = add_path(&mut data, &square(7000, 5000, 8000, 6000), PathKind::AyahMarkOrnament);
    data.decorations.push(DecoRec {
        kind: DecoKind::Other,
        surah: 1,
        ayah: 0,
        text: NONE_U16,
        first_path: path,
        n_paths: 1,
        line: 1,
        bbox,
    });
    let mut p = Page::from_data(data).unwrap();
    p.attach_letters(&encode_letters(&LetterData { n_page_paths: 9, ..letters_layer() })).unwrap();
    assert_eq!(p.geometry().table.len(), 11);
    assert_eq!(p.path_deco(8), 0);
    assert_eq!(p.path_letter(8), None, "a decoration path has no letter");
    let groups = crate::reflow::path_groups(&p);
    assert_eq!(&groups[..], &[0, 0, 0, 1, 1, 1, 2, 2, 3, 0, 0], "words, the decoration after them, then the pieces");
    let spec = LayoutSpec {
        viewport_w: 200.0,
        viewport_h: 400.0,
        reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }),
        ..Default::default()
    };
    assert_eq!(&p.layout(&spec).path_group[..], &groups[..]);
}

// ── the letter selectors and the letter query ──

/// The paths of `p` in `RED`. Every other path must be in the default ink.
fn reds(p: &mut Page) -> Vec<u32> {
    let c = p.colors().to_vec();
    let mut red = vec![];
    for (i, &col) in c.iter().enumerate() {
        if col == RED {
            red.push(i as u32);
        } else {
            assert_eq!(col, DEFAULT_INK, "path {i}");
        }
    }
    red
}

/// The paths a rule in `RED` at `LAYER_BASE` takes on the letter fixture.
fn selected(sel: Selector) -> Vec<u32> {
    let mut p = letters_page();
    p.style(sel, Paint::new(RED));
    reds(&mut p)
}

fn query(scope: Target, text: Option<char>, mark: Mark, part: LetterPart) -> LetterQuery {
    LetterQuery { scope, text, mark, part }
}

#[test]
fn letter_all() {
    assert_eq!(selected(Selector::Letter(0, 1, LetterPart::All)), vec![9]);
    assert_eq!(selected(Selector::Letter(0, 0, LetterPart::All)), vec![1, 2, 8]);
}

#[test]
fn letter_body_and_dots() {
    assert_eq!(selected(Selector::Letter(0, 0, LetterPart::BodyAndDots)), vec![1, 8]);
}

#[test]
fn letter_marks() {
    assert_eq!(selected(Selector::Letter(0, 0, LetterPart::Marks)), vec![2]);
    assert_eq!(selected(Selector::Letter(2, 0, LetterPart::Marks)), vec![7]);
}

#[test]
fn letter_mark_nth() {
    assert_eq!(selected(Selector::LetterMark(0, 0, 0)), vec![1]);
    assert_eq!(selected(Selector::LetterMark(0, 0, 1)), vec![2]);
    assert_eq!(selected(Selector::LetterMark(0, 0, 2)), Vec::<u32>::new());
}

#[test]
fn letter_mark_named() {
    assert_eq!(selected(Selector::LetterMarkNamed(0, 0, Mark::Fathah)), vec![2]);
    assert_eq!(selected(Selector::LetterMarkNamed(0, 0, Mark::Dot)), vec![1]);
    assert_eq!(selected(Selector::LetterMarkNamed(2, 0, Mark::OmittedAlif)), vec![7]);
    assert_eq!(selected(Selector::LetterMarkNamed(0, 0, Mark::None)), Vec::<u32>::new());
}

#[test]
fn a_named_mark_takes_a_body_stored_as_a_mark() {
    // the omitted alif of word 2 as a second body of its letter, as the page stores a
    // dagger alif that draws a letter
    let mut layer = letters_layer();
    layer.members[7].is_body = true;
    let mut p = page_without_layer();
    p.attach_letters(&encode_letters(&layer)).unwrap();
    p.style(Selector::LetterMarkNamed(2, 0, Mark::OmittedAlif), Paint::new(RED));
    assert_eq!(reds(&mut p), vec![7]);
    let mut q = page_without_layer();
    q.attach_letters(&encode_letters(&layer)).unwrap();
    q.style(Selector::LetterMark(2, 0, 0), Paint::new(RED));
    assert_eq!(reds(&mut q), Vec::<u32>::new(), "a body is no nth mark");
}

#[test]
fn word_chars_letter() {
    assert_eq!(selected(Selector::WordChars(0, 0, 0)), vec![1, 8]);
}

#[test]
fn word_chars_mark() {
    assert_eq!(selected(Selector::WordChars(0, 1, 1)), vec![2]);
}

#[test]
fn word_chars_kashida() {
    assert_eq!(selected(Selector::WordChars(1, 1, 1)), vec![3]);
}

#[test]
fn word_chars_word_mark() {
    assert_eq!(selected(Selector::WordChars(1, 3, 3)), vec![5]);
}

#[test]
fn word_chars_no_offset() {
    assert_eq!(selected(Selector::WordChars(2, 0, 5)), vec![6]);
}

#[test]
fn a_cut_path_taken_whole() {
    assert_eq!(selected(Selector::WordChars(0, 0, 2)), vec![0, 1, 2, 8, 9]);
    assert_eq!(selected(Selector::WordChars(0, 1, 2)), vec![2, 9], "one piece leaves the cut path");
}

#[test]
fn word_reaches_the_pieces() {
    assert_eq!(selected(Selector::Word(0)), vec![0, 1, 2, 8, 9]);
    assert_eq!(selected(Selector::Kind(PathKind::Body)), vec![0, 3, 4, 6, 8, 9]);
    let mut p = letters_page();
    p.theme(&Theme { ink: Some(RED), ..Default::default() });
    assert_eq!(reds(&mut p), (0..10).collect::<Vec<u32>>());
}

#[test]
fn path_reaches_the_pieces() {
    let mut p = letters_page();
    p.hide(Selector::Path(0));
    let c = p.colors().to_vec();
    for (i, col) in c.iter().enumerate() {
        let hidden = [0, 8, 9].contains(&i);
        assert_eq!(col & 0xff == 0, hidden, "path {i}");
    }
    assert_eq!(selected(Selector::Path(8)), vec![8], "a piece takes its own index");
    assert_eq!(selected(Selector::Path(3)), vec![3]);
}

#[test]
fn no_layer_selects_nothing() {
    let mut p = page();
    let n = p.colors().len();
    let sels = [
        Selector::Letter(0, 0, LetterPart::All),
        Selector::LetterMark(0, 0, 0),
        Selector::LetterMarkNamed(0, 0, Mark::Fathah),
        Selector::WordChars(0, 0, u16::MAX),
    ];
    for s in sels {
        assert!(p.style(s, Paint::new(RED)) > 0);
    }
    let q = query(Target::Page, None, Mark::None, LetterPart::All);
    assert!(p.style_letters(LAYER_BASE, &q, Paint::new(RED)) > 0);
    assert!(p.colors().iter().all(|&c| c == DEFAULT_INK));
    assert_eq!(p.colors().len(), n);
}

#[test]
fn an_out_of_range_selector_selects_nothing() {
    let none = Vec::<u32>::new();
    for s in [
        Selector::Letter(3, 0, LetterPart::All),
        Selector::Letter(NONE, 0, LetterPart::All),
        Selector::Letter(0, 2, LetterPart::All),
        Selector::Letter(0, u16::MAX, LetterPart::All),
        Selector::LetterMark(0, 0, u16::MAX),
        Selector::LetterMark(9, 0, 0),
        Selector::LetterMarkNamed(0, 0, Mark::from_u8(255)),
        Selector::LetterMarkNamed(0, u16::MAX, Mark::Dot),
        Selector::WordChars(0, 2, 1),
        Selector::WordChars(0, 3, u16::MAX - 1),
        Selector::WordChars(0, u16::MAX, u16::MAX),
        Selector::WordChars(3, 0, u16::MAX),
        Selector::WordChars(NONE, 0, u16::MAX),
    ] {
        assert_eq!(selected(s.clone()), none, "{s:?}");
    }
    let q = |w: Vec<u32>| {
        Selector::Letters(LetterSelection { words: w, text: None, mark: Mark::None, part: LetterPart::All })
    };
    assert_eq!(selected(q(vec![3, NONE])), none);
    assert_eq!(selected(q(vec![])), none);
}

#[test]
fn a_word_mark_is_no_letters() {
    for s in [Selector::Letter(1, 0, LetterPart::All), Selector::Letter(1, 1, LetterPart::Marks)] {
        assert!(!selected(s).contains(&5));
    }
    let mut p = letters_page();
    p.style_letters(LAYER_BASE, &query(Target::Word(1), None, Mark::None, LetterPart::All), Paint::new(RED));
    assert_eq!(reds(&mut p), vec![3, 4]);
}

#[test]
fn a_short_qpc_hides_offsets_from_a_range() {
    // the sidecar before the layer and after it
    for before in [true, false] {
        let mut p = page_without_layer();
        let json = r#"{"1:1:2": {"qpc": "ل"}}"#.as_bytes();
        if before {
            p.attach_words_json(json).unwrap();
        }
        p.attach_letters(&layer_bytes()).unwrap();
        p.style(Selector::WordChars(1, 0, 5), Paint::new(RED));
        p.style(Selector::Letter(1, 1, LetterPart::All), Paint::new(RED));
        if !before {
            assert_eq!(reds(&mut p), vec![3, 4, 5], "unchecked before the sidecar");
            p.attach_words_json(json).unwrap();
        }
        // the lam at 0 stays; its kashida (1), the ha (2) and the waqf sign (3) are past the
        // text. The ha keeps its index, so the letter selector still takes it
        assert_eq!(reds(&mut p), vec![3, 4], "sidecar first: {before}");
    }
}

#[test]
fn a_rule_waits_for_the_layer() {
    let mut p = page_without_layer();
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::new(RED));
    let q = query(Target::Page, Some('ه'), Mark::None, LetterPart::All);
    p.style_letters(LAYER_BASE, &q, Paint::new(RED));
    assert!(p.colors().iter().all(|&c| c == DEFAULT_INK));
    p.attach_letters(&layer_bytes()).unwrap();
    assert_eq!(p.color_of(9), RED);
    assert_eq!(reds(&mut p), vec![4, 9]);
}

#[test]
fn a_more_specific_letter_rule_wins() {
    let mut p = letters_page();
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::new(BLUE));
    p.style(Selector::Word(0), Paint::new(RED));
    let c = p.colors().to_vec();
    assert_eq!((c[9], c[8], c[0]), (BLUE, RED, RED));
    // a query rule is as specific as a word's body
    let mut q = letters_page();
    q.style_letters(LAYER_BASE, &query(Target::Page, Some('ل'), Mark::None, LetterPart::All), Paint::new(BLUE));
    q.style(Selector::Word(0), Paint::new(RED));
    q.style(Selector::WordBody(1), Paint::new(RED));
    let c = q.colors().to_vec();
    assert_eq!((c[9], c[8], c[3]), (BLUE, RED, RED), "the later rule of the same weight wins");
}

#[test]
fn find_letters_by_text() {
    let p = letters_page();
    let find = |scope, text, mark| p.find_letters(&query(scope, text, mark, LetterPart::All));
    assert_eq!(find(Target::Page, Some('ل'), Mark::None), vec![(0, 1), (1, 0)]);
    assert_eq!(find(Target::Page, char::from_u32(0x0628), Mark::None), vec![(0, 0)]);
    assert_eq!(find(Target::Page, None, Mark::Fathah), vec![(0, 0)]);
    assert_eq!(find(Target::Word(2), None, Mark::None), vec![(2, 0)]);
    assert_eq!(find(Target::Page, None, Mark::None), vec![(0, 0), (0, 1), (1, 0), (1, 1), (2, 0)]);
    assert_eq!(find(Target::Words(vec![2, 0, 9]), Some('ل'), Mark::None), vec![(0, 1)]);
    assert_eq!(find(Target::Page, Some('ل'), Mark::Fathah), vec![]);
    assert_eq!(find(Target::Page, None, Mark::WaqfJaizWaslAwla), vec![], "a word mark is no letter's");
    // part does not filter
    let marks = p.find_letters(&query(Target::Page, Some('ل'), Mark::None, LetterPart::Marks));
    assert_eq!(marks, vec![(0, 1), (1, 0)]);
    assert_eq!(page_without_layer().find_letters(&query(Target::Page, None, Mark::None, LetterPart::All)), vec![]);
}

#[test]
fn style_letters_lasts() {
    let mut p = letters_page();
    let h = p.style_letters(LAYER_BASE, &query(Target::Page, Some('ل'), Mark::None, LetterPart::All), Paint::new(RED));
    assert!(h > 0);
    assert_eq!(reds(&mut p), vec![3, 9]);
    p.attach_letters(&layer_bytes()).unwrap();
    assert_eq!(reds(&mut p), vec![3, 9]);
    assert_eq!(p.remove_style(h), 1);
    assert_eq!(reds(&mut p), Vec::<u32>::new());
}

#[test]
fn style_letters_takes_a_part_and_a_mark() {
    let style = |q: LetterQuery| {
        let mut p = letters_page();
        p.style_letters(LAYER_BASE, &q, Paint::new(RED));
        reds(&mut p)
    };
    assert_eq!(style(query(Target::Page, None, Mark::Fathah, LetterPart::All)), vec![1, 2, 8]);
    assert_eq!(style(query(Target::Page, None, Mark::Fathah, LetterPart::BodyAndDots)), vec![1, 8]);
    assert_eq!(style(query(Target::Page, None, Mark::Fathah, LetterPart::Marks)), vec![2]);
    assert_eq!(style(query(Target::Page, None, Mark::None, LetterPart::BodyAndDots)), vec![0, 1, 3, 4, 6, 8, 9]);
    assert_eq!(style(query(Target::Line(2), None, Mark::None, LetterPart::Marks)), vec![7]);
    assert_eq!(style(query(Target::Word(9), None, Mark::None, LetterPart::All)), Vec::<u32>::new());
}

// ── the swap: a cut path draws as its pieces while they differ ──

/// The printed layout of the swap tests: scale 2, every `line_dy` 0.
fn printed() -> LayoutSpec {
    LayoutSpec { viewport_w: 200.0, viewport_h: 200.0, ..Default::default() }
}

/// The current layout's draw list as (path, placement) pairs.
fn draws(p: &Page) -> Vec<(u32, u32)> {
    p.layout_draw_list().iter().map(|d| (d.path, d.placement)).collect()
}

fn draws_in(p: &Page, band: (f32, f32)) -> Vec<(u32, u32)> {
    p.layout_draw_list_in(Some(band)).iter().map(|d| (d.path, d.placement)).collect()
}

const PLAIN: [(u32, u32); 8] = [(0, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (6, 1), (7, 1)];
const SWAPPED: [(u32, u32); 9] = [(8, 0), (9, 0), (1, 0), (2, 0), (3, 0), (4, 0), (5, 0), (6, 1), (7, 1)];

/// The letter page, laid out as printed.
fn laid_letters_page() -> Page {
    let mut p = letters_page();
    p.layout(&printed());
    p
}

/// True while the fixture's cut path, path 0, draws its pieces.
fn is_swapped(p: &Page) -> bool {
    p.letters.as_deref().is_some_and(|lt| lt.drawn_pieces(0).is_some())
}

/// A renderer that keeps the index of each path it fills, and every argument as bytes.
#[derive(Default)]
struct Fills {
    paths: Vec<u32>,
    bytes: Vec<u8>,
}

impl Renderer for Fills {
    fn begin(&mut self, w: f32, h: f32) {
        self.bytes.extend(w.to_le_bytes());
        self.bytes.extend(h.to_le_bytes());
    }
    fn fill_path(&mut self, index: u32, ops: &[u8], pts: &[f32], color: Rgba, evenodd: bool) {
        self.paths.push(index);
        self.bytes.extend(ops);
        pts.iter().for_each(|v| self.bytes.extend(v.to_le_bytes()));
        self.bytes.extend(color.to_le_bytes());
        self.bytes.push(evenodd as u8);
    }
    fn fill_rect(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, radius: f32, color: Rgba) {
        [x0, y0, x1, y1, radius].iter().for_each(|v| self.bytes.extend(v.to_le_bytes()));
        self.bytes.extend(color.to_le_bytes());
    }
    fn end(&mut self) {}
}

#[test]
fn no_layer_draws_as_before() {
    let mut p = page();
    p.layout(&printed());
    assert_eq!(draws(&p), vec![(0, 0), (1, 0), (2, 0), (3, 0), (4, 1)]);
    p.style(Selector::Word(0), Paint::fade(RED, 200));
    p.tick(0.0);
    p.tick(300.0);
    assert_eq!(draws(&p), vec![(0, 0), (1, 0), (2, 0), (3, 0), (4, 1)]);
    assert_eq!(p.draw_revision(), 0, "a page with no layer never counts");
}

#[test]
fn a_layer_alone_changes_nothing() {
    let mut p = laid_letters_page();
    assert_eq!(draws(&p), PLAIN);
    assert!(p.styled_paths().is_empty());
    let mut bare = page_without_layer();
    bare.layout(&printed());
    assert_eq!(draws(&p), draws(&bare));
    let (mut a, mut b) = (Fills::default(), Fills::default());
    p.render(&mut a);
    bare.render(&mut b);
    assert_eq!(a.bytes, b.bytes);
}

#[test]
fn a_letter_colour_draws_the_pieces() {
    let mut p = laid_letters_page();
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::new(RED));
    p.refresh_display();
    assert_eq!(draws(&p), SWAPPED);
    assert_eq!(p.draw_revision(), 2);
    assert_eq!(p.colors()[9], RED);
    assert_eq!(p.colors()[8], DEFAULT_INK);
}

#[test]
fn a_word_colour_draws_the_path_whole() {
    let mut p = laid_letters_page();
    p.style(Selector::Word(0), Paint::new(RED));
    p.refresh_display();
    assert_eq!(draws(&p), PLAIN);
    assert_eq!(p.styled_paths(), vec![(0, RED), (1, RED), (2, RED)]);
    assert_eq!(p.draw_revision(), 1);
}

#[test]
fn styled_paths_follow_the_swap() {
    let mut p = laid_letters_page();
    p.style(Selector::Word(0), Paint::new(RED));
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::new(BLUE));
    assert_eq!(p.styled_paths(), vec![(1, RED), (2, RED), (8, RED), (9, BLUE)]);
    assert_eq!(draws(&p), SWAPPED);
}

#[test]
fn an_uncut_letter_needs_no_pieces() {
    let mut p = laid_letters_page();
    p.style(Selector::Letter(1, 1, LetterPart::All), Paint::new(RED));
    p.refresh_display();
    assert_eq!(draws(&p), PLAIN);
    assert_eq!(p.draw_revision(), 1);
}

#[test]
fn the_pieces_stay_until_the_fade_ends() {
    let mut p = laid_letters_page();
    p.tick(0.0);
    let h = p.style(Selector::Letter(0, 1, LetterPart::All), Paint::fade(RED, 200));
    p.refresh_display();
    assert_eq!(draws(&p), SWAPPED);
    assert_eq!(p.draw_revision(), 2);
    p.tick(100.0);
    p.remove_style(h);
    // rules resolve when the display is next refreshed: the fade back starts here, at 100 ms
    p.refresh_display();
    for t in [150.0, 200.0, 250.0, 299.0] {
        p.tick(t);
        assert_eq!(draws(&p), SWAPPED, "at {t} ms");
        assert_eq!(p.draw_revision(), 2, "the colours move, the drawn set does not");
    }
    assert!(!p.tick(300.0));
    assert_eq!(draws(&p), PLAIN);
    assert_eq!(p.draw_revision(), 3);
    p.tick(400.0);
    assert_eq!(p.draw_revision(), 3);
}

#[test]
fn a_rule_added_before_the_draw_list_counts() {
    let mut p = laid_letters_page();
    assert_eq!(draws(&p), PLAIN);
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::new(RED));
    // no colour read first: the pending rule applies here
    p.refresh_display();
    assert_eq!(draws(&p), SWAPPED);
}

#[test]
fn render_draws_each_ink_once() {
    let mut p = letters_page();
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::new(RED));
    let mut r = Fills::default();
    p.render(&mut r);
    assert_eq!(r.paths, vec![8, 9, 1, 2, 3, 4, 5, 6, 7]);
}

#[test]
fn crop_writes_the_pieces() {
    let mut p = letters_page();
    let whole = p.crop_svg(&Target::Word(0), 0.0, false, None).unwrap();
    assert_eq!(whole.matches("<path").count(), 3);
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::new(RED));
    let svg = p.crop_svg(&Target::Word(0), 0.0, false, None).unwrap();
    assert_eq!(svg.matches("<path").count(), 4);
    assert_eq!(svg.matches("fill=\"#ff0000\"").count(), 1);
    assert_eq!(svg.matches("fill-rule=\"evenodd\"").count(), 4, "a piece takes its source's fill rule");
    // the lam's piece, at its own outline
    assert!(svg.contains("M39 10"), "{svg}");
}

#[test]
fn a_band_culls_pieces_by_their_box() {
    let mut p = laid_letters_page();
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::new(RED));
    p.refresh_display();
    assert!(draws_in(&p, (0.0, 1.0)).is_empty());
    // page y 10 to 20: the bodies of line 1, with the pieces in place of path 0
    assert_eq!(draws_in(&p, (20.0, 40.0)), vec![(8, 0), (9, 0), (3, 0), (4, 0)]);
}

#[test]
fn a_band_keeps_the_piece_that_reaches_past_its_source() {
    // the lam's piece reaches 6 units below its source (page y 20 to 26)
    let mut low = letters_layer();
    low.pieces[1].cmds = square(3900, 1000, 5000, 2600);
    let mut p = page_without_layer();
    p.attach_letters(&encode_letters(&low)).unwrap();
    p.layout(&printed());
    // page y 21 to 25: the dot under the ba (y 22 to 24) and the low piece, nothing else
    let band = (42.0, 50.0);
    assert_eq!(draws_in(&p, band), vec![(1, 0)], "a whole path is culled by its own box");
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::new(RED));
    p.refresh_display();
    assert_eq!(draws_in(&p, band), vec![(9, 0), (1, 0)], "each piece is culled by its own box");
    assert_eq!(draws_in(&p, (0.0, 19.0)), vec![(2, 0), (5, 0)], "above the bodies: the fathah and the waqf sign");
}

#[test]
fn every_letter_of_the_path_in_one_colour_draws_it_whole() {
    let mut p = laid_letters_page();
    p.tick(0.0);
    p.style(Selector::Letter(0, 0, LetterPart::All), Paint::fade(RED, 200));
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::fade(RED, 200));
    for t in [0.0, 50.0, 199.0, 200.0, 300.0] {
        p.tick(t);
        assert_eq!(draws(&p), PLAIN, "at {t} ms");
    }
    assert_eq!(p.draw_revision(), 1);
    assert_eq!(p.colors()[0], RED, "the cut path takes the colour its pieces share");
    assert_eq!(p.styled_paths(), vec![(0, RED), (1, RED), (2, RED)]);
    // one letter, then the other: apart while the second fades in, whole once it has
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::fade(BLUE, 200));
    p.tick(400.0);
    assert_eq!(draws(&p), SWAPPED);
    p.style(Selector::Letter(0, 0, LetterPart::All), Paint::fade(BLUE, 200));
    p.tick(500.0);
    assert_eq!(draws(&p), SWAPPED);
    p.tick(600.0);
    assert_eq!(draws(&p), SWAPPED, "the second letter is still fading");
    p.tick(700.0);
    assert_eq!(draws(&p), PLAIN);
    assert_eq!(p.colors()[0], BLUE);
    assert_eq!(p.draw_revision(), 3);
}

#[test]
fn a_theme_a_highlight_and_a_mask_leave_every_path_whole() {
    let mut p = laid_letters_page();
    p.tick(0.0);
    p.theme(&Theme { ink: Some(0x202020ff), dots: Some(BLUE), transition_ms: 300, ..Default::default() });
    let style = HighlightStyle { mode: HighlightMode::Ink, ink: RED, transition_ms: 200, ..Default::default() };
    let h = p.highlight(&Target::Word(0), style);
    p.set_mask_transition(150);
    p.mask(&Target::Word(0), MaskMode::Hide);
    for t in [0.0, 100.0, 200.0, 400.0] {
        p.tick(t);
        assert_eq!(draws(&p), PLAIN, "at {t} ms");
    }
    p.unmask();
    p.remove_highlight(h);
    for t in [450.0, 600.0, 900.0] {
        p.tick(t);
        assert_eq!(draws(&p), PLAIN, "at {t} ms");
    }
    p.style(Selector::Path(0), Paint::fade(BLUE, 100));
    p.tick(1000.0);
    p.tick(1200.0);
    assert_eq!(draws(&p), PLAIN, "a rule on the cut path takes its pieces with it");
    assert_eq!(p.draw_revision(), 1);
}

#[test]
fn a_swap_under_a_reflow() {
    let mut p = letters_page();
    let spec = LayoutSpec {
        viewport_w: 200.0,
        viewport_h: 400.0,
        reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }),
        ..Default::default()
    };
    let l = p.layout(&spec).clone();
    let plain = draws(&p);
    let at = plain.iter().position(|d| d.0 == 0).unwrap();
    let g = l.path_group[0];
    assert_eq!(plain[at], (0, g));
    p.style(Selector::Letter(0, 0, LetterPart::BodyAndDots), Paint::new(RED));
    p.refresh_display();
    let mut want = plain.clone();
    want.splice(at..at + 1, [(8, g), (9, g)]);
    assert_eq!(draws(&p), want);
    // the band of the whole page keeps both pieces under the word's placement
    let all = draws_in(&p, (0.0, l.content_h));
    assert!(all.contains(&(8, g)) && all.contains(&(9, g)) && !all.iter().any(|d| d.0 == 0));
}

#[test]
fn a_swap_survives_every_change_of_state() {
    // a layout made before the attach
    let mut p = page_without_layer();
    p.layout(&printed());
    p.tick(0.0);
    let lam = p.style(Selector::Letter(0, 1, LetterPart::All), Paint::fade(RED, 200));
    p.attach_letters(&layer_bytes()).unwrap();
    assert_eq!(draws(&p), SWAPPED, "the attach applies the waiting rule");
    // a second rule on the other letter of the swapped path
    let ba = p.style(Selector::Letter(0, 0, LetterPart::All), Paint::fade(BLUE, 200));
    p.tick(50.0);
    assert_eq!(draws(&p), SWAPPED);
    // a second attach while swapped
    p.attach_letters(&layer_bytes()).unwrap();
    assert_eq!(draws(&p), SWAPPED);
    // the rules go mid-transition
    p.tick(100.0);
    p.remove_style(lam);
    p.tick(120.0);
    p.remove_style(ba);
    p.tick(150.0);
    assert_eq!(draws(&p), SWAPPED);
    while p.tick(p.clock_ms + 25.0) {}
    assert_eq!(draws(&p), PLAIN);
    // a layer with no cut, attached while swapped
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::new(RED));
    p.refresh_display();
    assert_eq!(draws(&p), SWAPPED);
    let mut uncut = letters_layer();
    uncut.pieces.clear();
    uncut.members[0].path = qvp_format::letters::MemberPath::Page(0);
    uncut.members.remove(3);
    uncut.letters.remove(1);
    for (l, first) in uncut.letters.iter_mut().zip([0, 3, 4, 5]) {
        l.first_member = first;
    }
    let rev = p.draw_revision();
    p.attach_letters(&encode_letters(&uncut)).unwrap();
    assert_eq!(draws(&p), PLAIN);
    assert!(p.draw_revision() != rev);
    assert!(!is_swapped(&p));
    let mut r = Fills::default();
    p.render(&mut r);
    assert_eq!(r.paths, (0..8).collect::<Vec<u32>>());
}

#[test]
fn a_new_layout_keeps_the_swap() {
    let mut p = laid_letters_page();
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::new(RED));
    p.refresh_display();
    // a new layout keeps the swap
    p.layout(&LayoutSpec { viewport_w: 400.0, viewport_h: 400.0, ..Default::default() });
    assert_eq!(draws(&p), SWAPPED);
}

// ── fix round 1: parts by offset, marks in text order ──

/// The paths a rule in `RED` takes on a page with a layer. The rule is removed after.
fn taken(p: &mut Page, sel: Selector) -> Vec<u32> {
    let h = p.style(sel, Paint::new(RED));
    let v = reds(p);
    p.remove_style(h);
    v
}

#[test]
fn parts_go_by_offset() {
    let mut p = letters_variant_page();
    // the hamzah sign (9) is at the letter's own offset: it is part of the letter's
    // character, as the body is. The kasrah (8) is at another offset, the omitted alif (7)
    // has none
    assert_eq!(taken(&mut p, Selector::Letter(2, 0, LetterPart::BodyAndDots)), vec![6, 9]);
    assert_eq!(taken(&mut p, Selector::Letter(2, 0, LetterPart::Marks)), vec![7, 8]);
    assert_eq!(taken(&mut p, Selector::Letter(2, 0, LetterPart::All)), vec![6, 7, 8, 9]);
    // the mark selectors are not limited by part
    assert_eq!(taken(&mut p, Selector::LetterMarkNamed(2, 0, Mark::Hamzah)), vec![9]);
    assert_eq!(taken(&mut p, Selector::LetterMark(2, 0, 0)), vec![9]);
    let q = |part| query(Target::Page, None, Mark::Hamzah, part);
    p.style_letters(LAYER_BASE, &q(LetterPart::BodyAndDots), Paint::new(RED));
    assert_eq!(reds(&mut p), vec![6, 9]);
    let mut r = letters_variant_page();
    r.style_letters(LAYER_BASE, &q(LetterPart::Marks), Paint::new(RED));
    assert_eq!(reds(&mut r), vec![7, 8]);
    assert_eq!(r.find_letters(&q(LetterPart::Marks)), vec![(2, 0)]);
}

#[test]
fn body_and_dots_is_what_the_letters_offset_takes() {
    for mut p in [letters_page(), letters_variant_page()] {
        let mut n = 0;
        for w in 0..p.data().words.len() as u32 {
            for l in 0..p.letter_count(w) {
                let at = p.letter_info(w, l).unwrap().at;
                assert_ne!(at, NONE);
                let own = taken(&mut p, Selector::Letter(w, l as u16, LetterPart::BodyAndDots));
                assert!(!own.is_empty());
                assert_eq!(own, taken(&mut p, Selector::WordChars(w, at as u16, at as u16)), "word {w} letter {l}");
                n += 1;
            }
        }
        assert_eq!(n, 5);
    }
}

#[test]
fn a_letters_marks_are_in_text_order() {
    let mut p = letters_variant_page();
    // stored as 7 (no offset), 8 (at 1), 9 (at 0); in text order 9, 8, 7
    assert_eq!(taken(&mut p, Selector::LetterMark(2, 0, 0)), vec![9]);
    assert_eq!(taken(&mut p, Selector::LetterMark(2, 0, 1)), vec![8]);
    assert_eq!(taken(&mut p, Selector::LetterMark(2, 0, 2)), vec![7]);
    assert_eq!(taken(&mut p, Selector::LetterMark(2, 0, 3)), Vec::<u32>::new());
    let paths: Vec<u32> = p.letter_members(2, 0).iter().map(|m| m.path).collect();
    assert_eq!(paths, vec![6, 9, 8, 7], "the body first, then the marks in text order");
    let ats: Vec<u32> = p.letter_members(2, 0).iter().map(|m| m.at).collect();
    assert_eq!(ats, vec![0, 0, 1, NONE]);
    // the layer's own data keeps its order
    let lt = p.letters.as_deref().unwrap();
    let stored: Vec<_> = lt.data.members[6..10].iter().map(|m| m.path).collect();
    use qvp_format::letters::MemberPath::Page as P;
    assert_eq!(stored, vec![P(6), P(7), P(8), P(9)]);
    assert_eq!(p.path_letter(9), Some((0, 0)));
    assert_eq!(p.path_letter(7), Some((0, NONE)));
}

#[test]
fn letter_members_list_the_marks_as_letter_mark_counts_them() {
    for mut p in [letters_page(), letters_variant_page()] {
        for w in 0..p.data().words.len() as u32 {
            for l in 0..p.letter_count(w) {
                let members = p.letter_members(w, l);
                let first_mark = members.iter().position(|m| !m.is_body).unwrap_or(members.len());
                assert!(members[first_mark..].iter().all(|m| !m.is_body), "the bodies come first");
                for (n, m) in members[first_mark..].iter().enumerate() {
                    assert_eq!(taken(&mut p, Selector::LetterMark(w, l as u16, n as u16)), vec![m.path]);
                }
                let past = (members.len() - first_mark) as u16;
                assert_eq!(taken(&mut p, Selector::LetterMark(w, l as u16, past)), Vec::<u32>::new());
            }
        }
    }
}

#[test]
fn a_query_takes_an_ayah_scope() {
    let p = letters_page();
    let find = |scope| p.find_letters(&query(scope, None, Mark::None, LetterPart::All));
    assert_eq!(find(Target::Ayah(1, 2)), vec![(2, 0)]);
    assert_eq!(find(Target::Ayah(1, 1)), vec![(0, 0), (0, 1), (1, 0), (1, 1)]);
    assert_eq!(find(Target::Ayah(2, 1)), vec![]);
    assert_eq!(find(Target::AyahRange(1, 1, 2)).len(), 5);
    assert_eq!(find(Target::AyahRange(1, 2, 9)), vec![(2, 0)]);
    assert_eq!(find(Target::AyahRange(1, 3, 9)), vec![]);
    let mut q = letters_page();
    q.style_letters(LAYER_BASE, &query(Target::Ayah(1, 1), Some('ل'), Mark::None, LetterPart::All), Paint::new(RED));
    assert_eq!(reds(&mut q), vec![3, 9]);
    let mut r = letters_page();
    r.style_letters(LAYER_BASE, &query(Target::AyahRange(1, 2, 2), None, Mark::None, LetterPart::All), Paint::new(RED));
    assert_eq!(reds(&mut r), vec![6, 7]);
}

#[test]
fn letter_rules_follow_a_different_second_layer() {
    // the first layer cuts path 0; the second does not, and has one letter fewer in word 0
    let mut uncut = letters_layer();
    uncut.pieces.clear();
    uncut.members[0].path = qvp_format::letters::MemberPath::Page(0);
    uncut.members.remove(3);
    uncut.letters.remove(1);
    for (l, first) in uncut.letters.iter_mut().zip([0, 3, 4, 5]) {
        l.first_member = first;
    }
    let mut p = letters_page();
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::new(RED));
    p.style(Selector::LetterMark(0, 0, 1), Paint::new(RED));
    p.style(Selector::WordChars(0, 2, 2), Paint::new(RED));
    p.style_letters(LAYER_BASE, &query(Target::Page, Some('ل'), Mark::None, LetterPart::All), Paint::new(RED));
    assert_eq!(reds(&mut p), vec![2, 3, 9]);
    p.attach_letters(&encode_letters(&uncut)).unwrap();
    // word 0 has no letter 1 and no offset 2 now; the fathah is still mark 1 of the ba
    assert_eq!(p.colors().len(), 8);
    assert_eq!(reds(&mut p), vec![2, 3]);
    p.attach_letters(&layer_bytes()).unwrap();
    assert_eq!(reds(&mut p), vec![2, 3, 9]);

    // on the variant page: a second layer that moves the hamzah sign off the letter's offset
    // changes the parts and the mark order
    let mut moved = letters_variant_layer();
    moved.members[9].at = qvp_format::letters::NO_AT;
    let mut v = letters_variant_page();
    v.style(Selector::LetterMark(2, 0, 0), Paint::new(RED));
    v.style(Selector::Letter(2, 0, LetterPart::BodyAndDots), Paint::new(RED));
    assert_eq!(reds(&mut v), vec![6, 9]);
    v.attach_letters(&encode_letters(&moved)).unwrap();
    assert_eq!(reds(&mut v), vec![6, 8]);
    assert_eq!(v.letter_members(2, 0).iter().map(|m| m.path).collect::<Vec<_>>(), vec![6, 8, 7, 9]);
}
