//! The letter layer: attaching it, the pieces it appends, and reading its letters.
use super::*;
use crate::letters::*;
use crate::letters::{check_fits, word_runs_overlap};
use qvp_format::letters::{encode_letters, LetterData, PieceRec};
use qvp_format::PageData;

const RED: Rgba = 0xff0000ff;
const BLUE: Rgba = 0x0000ffff;

fn layer_bytes() -> Vec<u8> {
    layer_for(&letters_layer(), &letters_page_data())
}

/// A layer encoded against `data`, the page it is built for.
fn layer_for(layer: &LetterData, data: &PageData) -> Vec<u8> {
    encode_letters(layer, data).unwrap()
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
    assert!(p.attach_letters(&layer_for(&other, &letters_page_data())).is_err());
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
fn a_layer_built_for_other_bytes_of_the_page_is_refused() {
    // the same page, words and paths, but another file: one header field differs
    let mut other = letters_page_data();
    other.header.width = 101.0;
    let stale = layer_for(&letters_layer(), &other);
    let refused = Err(Error::Corrupt("letter layer page file"));
    // a page loaded from its file
    let mut p = Page::load(&encode(&letters_page_data())).unwrap();
    assert_eq!(p.attach_letters(&stale), refused);
    assert!(!p.has_letters());
    assert_eq!((p.geometry().table.len(), p.draw_revision()), (8, 0));
    // the layer built for this file attaches, and a refused one keeps it
    p.attach_letters(&layer_bytes()).unwrap();
    assert_eq!(p.attach_letters(&stale), refused);
    assert_eq!((p.geometry().table.len(), p.draw_revision(), p.letter_count(0)), (10, 1, 2));
    // a page built from data has the file that `encode` writes for it
    assert_eq!(page_without_layer().attach_letters(&stale), refused);
    let mut q = Page::from_data(other).unwrap();
    q.attach_letters(&stale).unwrap();
    assert_eq!(q.attach_letters(&layer_bytes()), refused);
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
    p.attach_letters(&layer_for(&uncut, &letters_page_data())).unwrap();
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
    let layer = layer_for(&letters_layer(), &data);
    let mut p = Page::from_data(data).unwrap();
    p.attach_letters(&layer).unwrap();
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
    assert!(b.attach_letters(&layer_for(&LetterData { page: 2, ..letters_layer() }, &letters_page_data())).is_err());
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
    let layer = layer_for(&letters_layer(), &data);
    let mut p = Page::from_data(data).unwrap();
    assert!(letters_layer().check_against(p.data()).is_ok(), "the format allows it");
    assert!(word_runs_overlap(p.data()));
    assert_eq!(p.attach_letters(&layer), Err(Error::Corrupt("overlapping word runs")));
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
    p.attach_letters(&layer_for(&more, &letters_page_data())).unwrap();
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
    let layer = layer_for(&letters_layer(), &data);
    let mut p = Page::from_data(data).unwrap();
    p.attach_letters(&layer).unwrap();
    assert!(!p.letter_info(2, 0).unwrap().has_ink);
    assert!(p.letter_info(0, 0).unwrap().has_ink);
}

#[test]
fn a_decoration_keeps_its_own_group_beside_the_pieces() {
    let (data, layer) = letters_decoration_fixture();
    let layer = layer_for(&layer, &data);
    let mut p = Page::from_data(data).unwrap();
    p.attach_letters(&layer).unwrap();
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
    p.attach_letters(&layer_for(&layer, &letters_page_data())).unwrap();
    p.style(Selector::LetterMarkNamed(2, 0, Mark::OmittedAlif), Paint::new(RED));
    assert_eq!(reds(&mut p), vec![7]);
    let mut q = page_without_layer();
    q.attach_letters(&layer_for(&layer, &letters_page_data())).unwrap();
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
fn the_draw_list_holds_the_rules_as_of_the_last_refresh() {
    // The Rust draw list takes `&self`, so it cannot apply a pending rule: it answers for the
    // last refresh, and a host refreshes first. The C draw-list call refreshes itself
    // (crates/qvp-ffi/tests/abi.rs, `the_draw_list_answers_for_every_rule_so_far`).
    let mut p = laid_letters_page();
    assert_eq!(draws(&p), PLAIN);
    let revision = p.draw_revision();
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::new(RED));
    assert_eq!(draws(&p), PLAIN, "no refresh yet: the rule is pending");
    assert_eq!(p.draw_revision(), revision);
    // no colour read first: the refresh alone applies the rule
    p.refresh_display();
    assert_eq!(draws(&p), SWAPPED);
    assert_eq!(p.draw_revision(), revision.wrapping_add(1));
    p.refresh_display();
    assert_eq!(p.draw_revision(), revision.wrapping_add(1), "a refresh with nothing pending changes nothing");
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
    p.attach_letters(&layer_for(&low, &letters_page_data())).unwrap();
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
    p.attach_letters(&layer_for(&uncut, &letters_page_data())).unwrap();
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
    p.attach_letters(&layer_for(&uncut, &letters_page_data())).unwrap();
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
    v.attach_letters(&layer_for(&moved, &letters_variant_page_data())).unwrap();
    assert_eq!(reds(&mut v), vec![6, 8]);
    assert_eq!(v.letter_members(2, 0).iter().map(|m| m.path).collect::<Vec<_>>(), vec![6, 8, 7, 9]);
}

// ── the letter under a point ──

/// The letter hit at a point in page units, with the default options.
fn letter_at(p: &Page, x: f32, y: f32) -> Option<LetterHit> {
    p.hit_test_letter(x, y, &HitOptions::default())
}

/// An exact letter hit, as the tests below expect one.
/// Every body of these fixtures has no mark and every other member has one, so `is_body`
/// follows from `mark` here.
fn exact(word: u32, letter: u32, path: u32, at: u32, mark: Mark, line: u32) -> LetterHit {
    LetterHit { word, letter, path, at, line, distance: 0.0, mark, is_exact: true, is_body: mark == Mark::None }
}

/// A grid of points over the fixture page and past its edges, in page units.
fn grid() -> Vec<(f32, f32)> {
    let mut out = Vec::new();
    let mut y = -5.0;
    while y <= 105.0 {
        let mut x = -5.0;
        while x <= 105.0 {
            out.push((x, y));
            x += 0.5;
        }
        y += 0.5;
    }
    out
}

/// The letter fixture with the ba's fathah (path 2) moved to `square(2400,600,2600,800)`,
/// inside word 1's box. Word 0's box then overlaps word 1's.
fn interlock_page() -> Page {
    let mut data = letters_page_data();
    reshape(&mut data, 2, &square(2400, 600, 2600, 800));
    attached(data, &letters_layer())
}

/// The letter fixture with a counter in the lam: the cut path and the lam's piece have a hole
/// `square(4200,1300,4700,1700)`, and the ba's fathah (path 2) is `square(4300,600,5600,800)`,
/// so the ba's box holds the hole too.
fn counter_page() -> Page {
    let hole = square(4200, 1300, 4700, 1700);
    let mut data = letters_page_data();
    let mut joined = square(4000, 1000, 6000, 2000);
    joined.extend(hole.iter().copied());
    reshape(&mut data, 0, &joined);
    reshape(&mut data, 2, &square(4300, 600, 5600, 800));
    let mut layer = letters_layer();
    let mut lam = square(3900, 1000, 5000, 2000);
    lam.extend(hole.iter().copied());
    layer.pieces[1].cmds = lam;
    attached(data, &layer)
}

#[test]
fn under_a_reflow_ink_names_its_owner() {
    let o = HitOptions::default();
    for mut p in [letters_page(), letters_variant_page(), overlap_page(), interlock_page(), seam_page(), counter_page()]
    {
        for zoom in [2.0, 3.0] {
            let l =
                p.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom, ..Default::default() }), ..printed() }).clone();
            let mut on_ink = 0;
            let mut y = -4.0;
            while y <= l.content_h + 4.0 {
                let mut x = -4.0;
                while x <= l.content_w + 4.0 {
                    let word = p.hit_test_view(x, y, &o).map(|h| h.word).filter(|&w| w != NONE);
                    let h = p.hit_test_letter_view(x, y, &o);
                    let place = |w: u32| {
                        let line = p.data().words[w as usize].line_index as usize;
                        l.word_placement(w, line).invert((x - l.offset_x) / l.scale, (y - l.offset_y) / l.scale)
                    };
                    on_ink += check_hit(&p, h, word, place, &format!("zoom {zoom} view ({x}, {y})")).0 as usize;
                    x += 1.0;
                }
                y += 1.0;
            }
            assert!(on_ink > 0);
        }
    }
}

#[test]
fn a_point_on_a_piece_names_its_letter() {
    let p = letters_page();
    assert_eq!(letter_at(&p, 55.0, 15.0), Some(exact(0, 0, 8, 0, Mark::None, 0)));
    assert_eq!(letter_at(&p, 45.0, 15.0), Some(exact(0, 1, 9, 2, Mark::None, 0)));
    // the lam's piece reaches one unit past its word's box: the word hit test still picks
    // word 0 there, and the point is on the lam's ink
    assert_eq!(p.hit_test(39.5, 15.0, &HitOptions::default()).unwrap().word, 0);
    assert_eq!(letter_at(&p, 39.5, 15.0), Some(exact(0, 1, 9, 2, Mark::None, 0)));
    // an uncut body is its own path
    assert_eq!(letter_at(&p, 25.0, 15.0), Some(exact(1, 0, 3, 0, Mark::None, 0)));
    assert_eq!(letter_at(&p, 15.0, 55.0), Some(exact(2, 0, 6, 0, Mark::None, 1)));
}

#[test]
fn a_point_on_a_mark_names_the_mark() {
    let p = letters_page();
    assert_eq!(letter_at(&p, 55.0, 23.0), Some(exact(0, 0, 1, 0, Mark::Dot, 0)));
    assert_eq!(letter_at(&p, 55.0, 7.0), Some(exact(0, 0, 2, 1, Mark::Fathah, 0)));
    // a mark with no offset
    assert_eq!(letter_at(&p, 15.0, 47.0), Some(exact(2, 0, 7, NONE, Mark::OmittedAlif, 1)));
}

#[test]
fn a_word_mark_has_no_letter() {
    let p = letters_page();
    assert_eq!(letter_at(&p, 15.0, 5.0), Some(exact(1, NONE, 5, 3, Mark::WaqfJaizWaslAwla, 0)));
}

#[test]
fn between_words_the_nearest_letter() {
    let p = letters_page();
    // the gap goes to word 0; its lam's box starts at 39 (the piece reaches past the word)
    let h = letter_at(&p, 35.0, 15.0).unwrap();
    assert_eq!(
        h,
        LetterHit {
            word: 0,
            letter: 1,
            path: 9,
            at: 2,
            line: 0,
            distance: 4.0,
            mark: Mark::None,
            is_exact: false,
            is_body: false
        }
    );
    // inside a letter's box but off its ink: distance 0, still not exact
    let h = letter_at(&p, 55.0, 21.0).unwrap();
    assert_eq!((h.word, h.letter, h.path, h.at, h.distance, h.is_exact), (0, 0, 8, 0, 0.0, false));
    assert_eq!(h.mark, Mark::None);
    // `max_distance` refuses the word, as in `hit_test`
    let far = HitOptions { max_distance: 1.0, ..HitOptions::default() };
    assert_eq!(p.hit_test(35.0, 15.0, &far), None);
    assert_eq!(p.hit_test_letter(35.0, 15.0, &far), None);
}

#[test]
fn marks_win_over_the_body_and_the_mark_drawn_last_wins() {
    let p = overlap_page();
    assert_eq!(p.geometry().table.len(), 12);
    // inside piece 0, the lam's sukun and the ba's shaddah: the shaddah is drawn last
    assert_eq!(letter_at(&p, 55.0, 15.0), Some(exact(0, 0, 4, 0, Mark::Shaddah, 0)));
    // inside piece 0 and the sukun only: the lam's mark, over the ba's body
    assert_eq!(letter_at(&p, 53.5, 13.5), Some(exact(0, 1, 3, 2, Mark::Sukun, 0)));
    // inside piece 0 only
    assert_eq!(letter_at(&p, 51.0, 11.0), Some(exact(0, 0, 10, 0, Mark::None, 0)));
    assert_eq!(letter_at(&p, 45.0, 15.0), Some(exact(0, 1, 11, 2, Mark::None, 0)));
    // the word mark moved to path 7
    assert_eq!(letter_at(&p, 15.0, 5.0), Some(exact(1, NONE, 7, 3, Mark::WaqfJaizWaslAwla, 0)));
}

#[test]
fn the_hit_does_not_follow_the_swap() {
    let mut p = laid_letters_page();
    let before: Vec<_> = grid().iter().map(|&(x, y)| letter_at(&p, x, y)).collect();
    p.style(Selector::Letter(0, 1, LetterPart::All), Paint::new(RED));
    p.refresh_display();
    assert!(is_swapped(&p));
    let after: Vec<_> = grid().iter().map(|&(x, y)| letter_at(&p, x, y)).collect();
    assert_eq!(before, after);
    // never the cut path, whether it draws whole or as its pieces
    assert!(after.iter().flatten().all(|h| h.path != 0));
}

/// The words whose ink holds the point: a page path of the word, a cut path included, or a
/// piece of one, which can reach past its cut path. `place` takes a word to the page-unit
/// point under that word's placement.
fn ink_owners(p: &Page, place: impl Fn(u32) -> (f32, f32)) -> Vec<u32> {
    let d = p.data();
    let n = d.paths.len() as u32;
    let total = p.geometry().table.len() as u32;
    (0..d.words.len() as u32)
        .filter(|&w| {
            let (x, y) = place(w);
            let r = &d.words[w as usize];
            let run = r.first_path..r.first_path + r.n_paths as u32;
            run.clone().any(|q| p.point_in_path(q, x, y))
                || (n..total).any(|q| run.contains(&source_of(p, q)) && p.point_in_path(q, x, y))
        })
        .collect()
}

/// The source of a piece, or the path itself.
fn source_of(p: &Page, path: u32) -> u32 {
    p.letters.as_deref().map_or(path, |lt| {
        if path < lt.first_piece {
            path
        } else {
            lt.data.pieces[(path - lt.first_piece) as usize].source
        }
    })
}

/// True when the member `path` of a hit is a body member of its letter.
fn member_is_body(p: &Page, h: &LetterHit) -> bool {
    h.letter != NONE && p.letter_members(h.word, h.letter).iter().any(|m| m.path == h.path && m.is_body)
}

/// What every letter hit must be, at a point whose page-unit form under word `w`'s placement
/// is `place(w)`. On the ink of a word path: exact, in an owner's word, on a path that holds
/// the point or on a piece of the cut path that holds it (a seam), with `is_body` the member's
/// role. Off ink: the word `word` (the word hit test's), and not exact. Returns (exact, seam).
fn check_hit(
    p: &Page,
    h: Option<LetterHit>,
    word: Option<u32>,
    place: impl Fn(u32) -> (f32, f32),
    at: &str,
) -> (bool, bool) {
    let owners = ink_owners(p, &place);
    if owners.is_empty() {
        // in a counter, the letter whose body encloses the point. Else the word hit's word
        let counter = h.is_some_and(|h| {
            let (x, y) = place(h.word);
            h.letter != NONE && p.letter_members(h.word, h.letter).iter().any(|m| m.is_body && p.encloses(m.path, x, y))
        });
        if !counter {
            assert_eq!(h.map(|h| h.word), word, "off ink, the word hit's word, {at}");
        }
        if let Some(h) = h {
            assert!(!h.is_exact && !h.is_body && h.mark == Mark::None && h.letter != NONE, "{at}: {h:?}");
        }
        return (false, false);
    }
    let h = h.unwrap_or_else(|| panic!("on ink, no letter hit {at}"));
    assert!(h.is_exact && h.distance == 0.0, "on ink, an exact hit {at}: {h:?}");
    assert!(owners.contains(&h.word), "on the ink of {owners:?}, the hit names word {} {at}", h.word);
    let (x, y) = place(h.word);
    let lt = p.letters.as_deref().unwrap();
    assert!(lt.slot_of_path(h.path).is_none(), "never a cut path {at}");
    let seam = !p.point_in_path(h.path, x, y);
    if seam {
        let s = source_of(p, h.path);
        assert!(s != h.path && p.point_in_path(s, x, y), "neither the path nor its source holds the point {at}: {h:?}");
    }
    assert_eq!(h.is_body, member_is_body(p, &h), "{at}: {h:?}");
    (true, seam)
}

#[test]
fn the_letter_hit_prefers_ink_and_else_takes_the_word_hit_tests_word() {
    let o = HitOptions::default();
    let opts = [
        o,
        HitOptions { prefer_exact: false, ..o },
        HitOptions { gap_bias: 0.1, ..o },
        HitOptions { max_distance: 2.0, ..o },
    ];
    for p in [letters_page(), letters_variant_page(), overlap_page(), interlock_page(), seam_page(), counter_page()] {
        let (mut exact_hits, mut near) = (0, 0);
        for opt in &opts {
            for (x, y) in grid() {
                let word = p.hit_test(x, y, opt).map(|h| h.word).filter(|&w| w != NONE);
                let h = p.hit_test_letter(x, y, opt);
                let (on_ink, _) = check_hit(&p, h, word, |_| (x, y), &format!("at ({x}, {y}) with {opt:?}"));
                if on_ink {
                    exact_hits += 1;
                } else if h.is_some() {
                    near += 1;
                }
            }
        }
        assert!(exact_hits > 0 && near > 0);
    }
}

#[test]
fn ink_wins_over_the_gap_split() {
    let p = interlock_page();
    // the fathah of word 0 lies inside word 1's box. The gap split alone picks word 1
    let gap = HitOptions { prefer_exact: false, ..HitOptions::default() };
    assert_eq!(p.hit_test(25.0, 7.0, &gap).unwrap().word, 1);
    assert_eq!(p.hit_test_letter(25.0, 7.0, &gap), Some(exact(0, 0, 2, 1, Mark::Fathah, 0)));
    assert_eq!(letter_at(&p, 25.0, 7.0), Some(exact(0, 0, 2, 1, Mark::Fathah, 0)));
    // off the ink, the gap split still decides
    assert_eq!(p.hit_test_letter(25.0, 9.0, &gap).map(|h| (h.word, h.is_exact)), Some((1, false)));
}

#[test]
fn a_seam_between_pieces_is_an_exact_hit() {
    let p = seam_page();
    // the cut path holds the point and neither piece does
    assert!(p.point_in_path(0, 49.5, 15.0) && !p.point_in_path(8, 49.5, 15.0) && !p.point_in_path(9, 49.5, 15.0));
    // the nearest piece: the ba's piece is 0.5 away, the lam's 1.5
    assert_eq!(letter_at(&p, 49.5, 15.0), Some(exact(0, 0, 8, 0, Mark::None, 0)));
    assert_eq!(letter_at(&p, 48.5, 15.0), Some(exact(0, 1, 9, 2, Mark::None, 0)));
    // the same under a reflow, through the view call
    let mut p = p;
    let l = p.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }), ..printed() }).clone();
    let (vx, vy) = placed_point(&p, &l, 0, 49.5, 15.0);
    assert_eq!(p.hit_test_letter_view(vx, vy, &HitOptions::default()), Some(exact(0, 0, 8, 0, Mark::None, 0)));
}

#[test]
fn a_point_in_a_counter_names_its_own_letter() {
    let p = counter_page();
    // the hole of the lam: inside the boxes of both letters, on no ink
    let info = |l| p.letter_info(0, l).unwrap();
    for l in [0, 1] {
        let i = info(l);
        assert!(i.x0 <= 44.5 && 44.5 <= i.x1 && i.y0 <= 15.0 && 15.0 <= i.y1, "letter {l} box {i:?}");
    }
    assert!(ink_owners(&p, |_| (44.5, 15.0)).is_empty());
    let h = letter_at(&p, 44.5, 15.0).unwrap();
    assert_eq!((h.word, h.letter, h.path, h.at, h.distance, h.is_exact, h.is_body), (0, 1, 9, 2, 0.0, false, false));
    // beside the hole, in the lam's stroke: exact on its piece
    assert_eq!(letter_at(&p, 40.0, 15.0), Some(exact(0, 1, 9, 2, Mark::None, 0)));
}

#[test]
fn a_counter_names_its_letter_under_another_words_box() {
    // the counter page with word 1's waqf sign moved over the lam's hole: word 1's box then
    // reaches over the hole, and the word hit test picks word 1 there
    let (data, layer) = counter_under_a_box_fixture();
    let p = attached(data, &layer);
    let o = HitOptions::default();
    assert_eq!(p.hit_test(44.5, 15.0, &o).unwrap().word, 1);
    assert!(p.encloses(9, 44.5, 15.0) && !p.point_in_path(9, 44.5, 15.0));
    let h = letter_at(&p, 44.5, 15.0).unwrap();
    assert_eq!((h.word, h.letter, h.path, h.distance, h.is_exact, h.is_body), (0, 1, 9, 0.0, false, false));
    // off the counter, the word hit test decides
    assert_eq!(letter_at(&p, 35.0, 15.0).map(|h| (h.word, h.is_exact)), Some((1, false)));
    // a solid outline encloses nothing
    assert!(!p.encloses(3, 25.0, 15.0) && !p.encloses(3, 5.0, 15.0));
}

#[test]
fn is_body_is_the_members_role() {
    let p = letters_page();
    let h = letter_at(&p, 55.0, 15.0).unwrap();
    assert!(h.is_body && h.mark == Mark::None, "a body piece");
    let h = letter_at(&p, 55.0, 23.0).unwrap();
    assert!(!h.is_body && h.mark == Mark::Dot, "a dot is a mark member");
    let h = letter_at(&p, 15.0, 5.0).unwrap();
    assert!(!h.is_body && h.letter == NONE, "a word mark");
    let h = letter_at(&p, 35.0, 15.0).unwrap();
    assert!(!h.is_body && !h.is_exact, "a near hit has no member");
    // a body the page stores as a mark, as a dagger alif: the omitted alif is word 2's body
    let p = body_mark_page();
    let h = letter_at(&p, 15.0, 47.0).unwrap();
    assert_eq!((h.letter, h.path, h.is_body, h.mark, h.is_exact), (0, 7, true, Mark::OmittedAlif, true));
}

#[test]
fn the_view_variant_goes_through_the_layout() {
    let mut p = laid_letters_page();
    let o = HitOptions::default();
    let h = p.hit_test_letter_view(110.0, 30.0, &o).unwrap();
    assert_eq!((h.word, h.letter, h.path, h.is_exact), (0, 0, 8, true));
    // printed: every grid point in view px agrees with the word hit test, and maps to the
    // page-unit letter hit
    let l = p.current_layout().unwrap().clone();
    for (x, y) in grid() {
        let (vx, vy) = (l.offset_x + x * l.scale, l.offset_y + y * l.scale);
        let word = p.hit_test_view(vx, vy, &o).map(|h| h.word).filter(|&w| w != NONE);
        let letter = p.hit_test_letter_view(vx, vy, &o);
        let place = |w: u32| {
            let line = p.data().words[w as usize].line_index as usize;
            l.word_placement(w, line).invert((vx - l.offset_x) / l.scale, (vy - l.offset_y) / l.scale)
        };
        check_hit(&p, letter, word, place, &format!("at view ({vx}, {vy})"));
    }
    // reflowed
    let spec = LayoutSpec {
        viewport_w: 200.0,
        viewport_h: 400.0,
        reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }),
        ..Default::default()
    };
    let l = p.layout(&spec).clone();
    let (vx, vy) = placed_point(&p, &l, 8, 55.0, 15.0);
    let h = p.hit_test_letter_view(vx, vy, &o).unwrap();
    assert_eq!((h.word, h.letter, h.path, h.at, h.is_exact), (0, 0, 8, 0, true));
    let (vx, vy) = placed_point(&p, &l, 9, 45.0, 15.0);
    assert_eq!(p.hit_test_letter_view(vx, vy, &o).map(|h| (h.letter, h.path)), Some((1, 9)));
    let (vx, vy) = placed_point(&p, &l, 5, 15.0, 5.0);
    assert_eq!(p.hit_test_letter_view(vx, vy, &o), Some(exact(1, NONE, 5, 3, Mark::WaqfJaizWaslAwla, 0)));
    let (vx, vy) = placed_point(&p, &l, 6, 15.0, 55.0);
    assert_eq!(p.hit_test_letter_view(vx, vy, &o), Some(exact(2, 0, 6, 0, Mark::None, 1)));
    let mut seen = 0;
    let mut y = -10.0;
    while y <= l.content_h + 10.0 {
        let mut x = -10.0;
        while x <= l.content_w + 10.0 {
            let word = p.hit_test_view(x, y, &o).map(|h| h.word).filter(|&w| w != NONE);
            let letter = p.hit_test_letter_view(x, y, &o);
            let place = |w: u32| {
                let line = p.data().words[w as usize].line_index as usize;
                l.word_placement(w, line).invert((x - l.offset_x) / l.scale, (y - l.offset_y) / l.scale)
            };
            let (on_ink, _) = check_hit(&p, letter, word, place, &format!("at reflowed view ({x}, {y})"));
            seen += on_ink as usize;
            x += 1.0;
        }
        y += 1.0;
    }
    assert!(seen > 0);
}

#[test]
fn the_view_variant_with_no_layout_is_in_page_units() {
    let p = letters_page();
    assert!(p.current_layout().is_none());
    for (x, y) in grid() {
        let o = HitOptions::default();
        assert_eq!(p.hit_test_letter_view(x, y, &o), p.hit_test_letter(x, y, &o));
    }
}

#[test]
fn no_layer_no_letter() {
    let mut p = page();
    let o = HitOptions::default();
    assert!(grid().iter().all(|&(x, y)| p.hit_test_letter(x, y, &o).is_none()));
    assert!(grid().iter().all(|&(x, y)| p.hit_test_letter_view(x, y, &o).is_none()));
    p.layout(&printed());
    assert!(grid().iter().all(|&(x, y)| p.hit_test_letter_view(x * 2.0, y * 2.0, &o).is_none()));
    let q = page_without_layer();
    assert_eq!(q.hit_test_letter(55.0, 15.0, &o), None);
}

#[test]
fn a_decoration_or_no_word_is_no_letter() {
    let (data, layer) = letters_decoration_fixture();
    let layer = layer_for(&layer, &data);
    let mut p = Page::from_data(data).unwrap();
    p.attach_letters(&layer).unwrap();
    let o = HitOptions::default();
    let h = p.hit_test(75.0, 55.0, &o).unwrap();
    assert_eq!((h.word, h.decoration, h.path), (NONE, 0, 8));
    assert_eq!(p.hit_test_letter(75.0, 55.0, &o), None, "a decoration is no letter");
    p.layout(&printed());
    let l = p.current_layout().unwrap().clone();
    let (vx, vy) = (l.offset_x + 75.0 * l.scale, l.offset_y + 55.0 * l.scale);
    assert_eq!(p.hit_test_view(vx, vy, &o).map(|h| h.word), Some(NONE));
    assert_eq!(p.hit_test_letter_view(vx, vy, &o), None);
    // a point too far from every word is no word
    let near = HitOptions { max_distance: 1.0, ..o };
    assert_eq!(p.hit_test(50.0, 90.0, &near), None);
    assert_eq!(p.hit_test_letter(50.0, 90.0, &near), None);
}

#[test]
fn the_old_hit_test_is_unchanged() {
    let with = letters_page();
    let without = page_without_layer();
    assert_eq!(with.hit_test_exact(55.0, 15.0), Some(HitExact { word: 0, path: 0, decoration: NONE }));
    let o = HitOptions::default();
    for (x, y) in grid() {
        assert_eq!(with.hit_test_exact(x, y), without.hit_test_exact(x, y));
        assert_eq!(with.hit_test(x, y, &o), without.hit_test(x, y, &o));
        assert_eq!(with.hit_test_view(x, y, &o), without.hit_test_view(x, y, &o));
    }
}

#[test]
fn an_offset_past_the_text_is_reported_as_none() {
    let mut p = letters_page();
    p.attach_words_json(r#"{"1:1:2": {"qpc": "ل"}}"#.as_bytes()).unwrap();
    // the ha (offset 2) and the waqf sign (offset 3) are past the text
    assert_eq!(letter_at(&p, 15.0, 15.0), Some(exact(1, 1, 4, NONE, Mark::None, 0)));
    assert_eq!(letter_at(&p, 15.0, 5.0), Some(exact(1, NONE, 5, NONE, Mark::WaqfJaizWaslAwla, 0)));
    // a near hit reports its letter's checked offset too
    let h = letter_at(&p, 15.0, 25.0).unwrap();
    assert_eq!((h.letter, h.at, h.is_exact), (1, NONE, false));
}

#[test]
fn any_point_is_safe() {
    let mut pages = vec![letters_page(), overlap_page(), page()];
    let odd = [f32::NAN, f32::INFINITY, f32::NEG_INFINITY, f32::MAX, f32::MIN, 1e30, -1e30, 0.0];
    let opts = [
        HitOptions::default(),
        HitOptions { max_distance: f32::NAN, gap_bias: f32::NAN, prefer_exact: false },
        HitOptions { max_distance: -1.0, gap_bias: f32::INFINITY, prefer_exact: true },
    ];
    for round in 0..3 {
        for p in &mut pages {
            if round == 1 {
                p.layout(&printed());
            }
            if round == 2 {
                p.layout(&LayoutSpec { reflow: Some(ReflowSpec { zoom: 3.0, ..Default::default() }), ..printed() });
            }
            for &x in &odd {
                for &y in &odd {
                    for o in &opts {
                        let _ = p.hit_test_letter(x, y, o);
                        let _ = p.hit_test_letter_view(x, y, o);
                        let _ = p.hit_test_letter(x, 15.0, o);
                        let _ = p.hit_test_letter_view(110.0, y, o);
                    }
                }
            }
        }
    }
}
