//! Synthesised pages and the path builder the unit tests share.
use crate::{Layout, Page};
use qvp_format::letters::*;
use qvp_format::*;

pub(crate) fn square(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<Cmd> {
    vec![Cmd::MoveTo(x0, y0), Cmd::LineTo(x1, y0), Cmd::LineTo(x1, y1), Cmd::LineTo(x0, y1), Cmd::Close]
}

/// Encode `cmds` onto `ops` and add its path record to `paths`. Returns the new path's index
/// and its box.
pub(crate) fn push_path(
    ops: &mut Vec<u8>,
    paths: &mut Vec<PathRec>,
    cmds: &[Cmd],
    kind: PathKind,
    mark: Mark,
    family: Family,
    flags: u8,
) -> (u32, IBox) {
    let bbox = cmds_bbox(cmds);
    let op_off = ops.len() as u32;
    encode_cmds(cmds, bbox.x0, bbox.y0, ops);
    let pi = paths.len() as u32;
    paths.push(PathRec {
        kind,
        mark,
        family,
        flags,
        ox: bbox.x0,
        oy: bbox.y0,
        op_off,
        op_len: ops.len() as u32 - op_off,
        bbox,
    });
    (pi, bbox)
}

/// Add a filled path of `kind` with no mark and no family to `data`. Returns the new path's
/// index and its box.
pub(crate) fn add_path(data: &mut PageData, cmds: &[Cmd], kind: PathKind) -> (u32, IBox) {
    push_path(&mut data.ops, &mut data.paths, cmds, kind, Mark::None, Family::None, PF_EVENODD)
}

/// Two words on one line: A = [10,20]×[10,20] units (word 1:1:1), B = [30,40]×[10,20]
/// (word 1:1:2) with two marks above it; plus a second line with word C (1:2:1).
pub(crate) fn page() -> Page {
    Page::load(&encode(&page_data())).unwrap()
}

/// The data of [`page`].
pub(crate) fn page_data() -> PageData {
    let mut ops = Vec::new();
    let mut paths = Vec::new();
    let mut strings = vec![];
    let mut add = |cmds: &[Cmd], kind: PathKind, mark: Mark, fam: Family| {
        push_path(&mut ops, &mut paths, cmds, kind, mark, fam, PF_EVENODD).1
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

/// A page wide enough to reflow onto mixed rows: 3 lines of 4 words, each word 16 units wide
/// with 4-unit gaps, inside a 100-unit page with 6-unit margins.
pub(crate) fn packed_page() -> Page {
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
            let (pi, bb) = push_path(&mut ops, &mut paths, &cmds, PathKind::Body, Mark::None, Family::None, 0);
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
                first_path: pi,
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

/// [`page_data`] with a surah name on a new first line: a title and, when `frame` is true, a
/// hollow frame around it. Returns the page, the frame path, the title path and the
/// decoration.
pub(crate) fn page_with_surah_frame(frame: bool) -> (Page, Option<u32>, u32, u32) {
    let mut data = page_data();
    for line in &mut data.lines {
        line.line_number += 1;
    }
    data.lines.insert(0, LineRec { line_number: 1, first_word: 0, n_words: 0, bbox: IBox::EMPTY });
    for word in &mut data.words {
        word.line_index += 1;
    }

    // The frame spans the printed text column and deliberately sits off the title's vertical
    // centre. It is hollow, like the real opening frames: empty space inside it must not become
    // a decoration hit. Reflow must ignore both extents, not merely hide paint after measuring it.
    let frame_path = frame.then(|| {
        let mut outline = square(1000, -1000, 4000, 1000);
        outline.extend(square(1200, -800, 3800, 800));
        add_path(&mut data, &outline, PathKind::Ornament).0
    });
    let title_path = add_path(&mut data, &square(2000, 300, 3000, 700), PathKind::HeaderInk).0;
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
    (Page::from_data(data).unwrap(), frame_path, title_path, decoration)
}

/// Where a point of a path, in page units, lands in the viewport under `layout`.
pub(crate) fn placed_point(page: &Page, layout: &Layout, path: u32, x: f32, y: f32) -> (f32, f32) {
    let line = page.geometry().table[path as usize].line;
    let (x, y) = layout.placement(path, line).apply(x, y);
    (layout.offset_x + x * layout.scale, layout.offset_y + y * layout.scale)
}

/// The page of the letter fixture: three words on two lines, eight paths. Path 0 joins two
/// letters, and the layer cuts it into two pieces.
pub(crate) fn letters_page_data() -> PageData {
    let mut ops = Vec::new();
    let mut paths = Vec::new();
    let mut add = |cmds: &[Cmd], kind: PathKind, mark: Mark, fam: Family| {
        push_path(&mut ops, &mut paths, cmds, kind, mark, fam, PF_EVENODD).1
    };
    // word 0: the joined ba and lam, a dot under the ba, a fathah over it
    let mut w0 = add(&square(4000, 1000, 6000, 2000), PathKind::Body, Mark::None, Family::None);
    w0.union(&add(&square(5400, 2200, 5600, 2400), PathKind::Mark, Mark::Dot, Family::Dots));
    w0.union(&add(&square(5400, 600, 5600, 800), PathKind::Mark, Mark::Fathah, Family::Diacritic));
    // word 1: lam with its kashida, ha, and a waqf sign that belongs to the word
    let mut w1 = add(&square(2000, 1000, 3000, 2000), PathKind::Body, Mark::None, Family::None);
    w1.union(&add(&square(1000, 1000, 2000, 2000), PathKind::Body, Mark::None, Family::None));
    w1.union(&add(&square(1400, 400, 1600, 600), PathKind::Mark, Mark::WaqfJaizWaslAwla, Family::Waqf));
    // word 2: waw and an omitted alif with no offset
    let mut w2 = add(&square(1000, 5000, 2000, 6000), PathKind::Body, Mark::None, Family::None);
    w2.union(&add(&square(1400, 4600, 1600, 4800), PathKind::Mark, Mark::OmittedAlif, Family::Diacritic));
    let mut l1 = w0;
    l1.union(&w1);
    let strings: Vec<String> = ["بَل", "لـهۖ", "و"].iter().map(|s| s.to_string()).collect();
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
        strings,
    }
}

/// The layer of [`letters_page_data`]: path 0 cut into two pieces, five letters, one word
/// mark. It passes `check_against` on that page. Piece 1 reaches past its word's box.
pub(crate) fn letters_layer() -> LetterData {
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
            // the lam's tail reaches one unit past its word's box, as a real piece can
            PieceRec { source: 0, cmds: square(3900, 1000, 5000, 2000) },
        ],
    }
}

/// The letter fixture's page with its layer attached.
pub(crate) fn letters_page() -> Page {
    let mut p = Page::from_data(letters_page_data()).unwrap();
    p.attach_letters(&encode_letters(&letters_layer(), &letters_page_data()).unwrap()).unwrap();
    p
}

/// A variant of [`letters_page_data`] for the parts and the order of a letter's marks.
/// Word 2 is `ؤِ` (U+0624 0, U+0650 1) and gets two more paths at its end: path 8, a kasrah,
/// and path 9, a hamzah sign. Paths 0 to 7 are as in the shared fixture.
pub(crate) fn letters_variant_page_data() -> PageData {
    let mut data = letters_page_data();
    let (_, kasrah) = push_path(
        &mut data.ops,
        &mut data.paths,
        &square(1400, 6200, 1600, 6400),
        PathKind::Mark,
        Mark::Kasrah,
        Family::Diacritic,
        PF_EVENODD,
    );
    let (_, hamzah) = push_path(
        &mut data.ops,
        &mut data.paths,
        &square(1400, 4200, 1600, 4400),
        PathKind::Mark,
        Mark::Hamzah,
        Family::Diacritic,
        PF_EVENODD,
    );
    data.strings[2] = "ؤِ".to_string();
    let w = &mut data.words[2];
    w.n_paths = 4;
    w.bbox.union(&kasrah);
    w.bbox.union(&hamzah);
    let bbox = w.bbox;
    data.lines[1].bbox = bbox;
    data.ayahs[1].bbox = bbox;
    data
}

/// The layer of [`letters_variant_page_data`]. As [`letters_layer`], but the letter of word
/// 2 is `ؤ` and stores its members as: body path 6 at 0, the omitted alif (path 7) with no
/// offset, the kasrah (path 8) at 1, and the hamzah sign (path 9) at 0. Its marks are out of
/// text order, one of them is at the letter's own offset, and one has no offset. The pieces
/// are paths 10 and 11.
pub(crate) fn letters_variant_layer() -> LetterData {
    let mut layer = letters_layer();
    layer.n_page_paths = 10;
    layer.letters[4].text = 'ؤ';
    layer.letters[4].n_members = 4;
    layer.members.push(MemberRec { path: MemberPath::Page(8), at: 1, is_body: false });
    layer.members.push(MemberRec { path: MemberPath::Page(9), at: 0, is_body: false });
    layer
}

/// The variant page with its layer attached.
pub(crate) fn letters_variant_page() -> Page {
    let data = letters_variant_page_data();
    let layer = encode_letters(&letters_variant_layer(), &data).unwrap();
    let mut p = Page::from_data(data).unwrap();
    p.attach_letters(&layer).unwrap();
    p
}

/// `data` with the boxes of every word, line and ayah made again from its paths.
pub(crate) fn reboxed(mut data: PageData) -> PageData {
    for w in 0..data.words.len() {
        let r = &data.words[w];
        let mut b = IBox::EMPTY;
        for q in r.first_path..r.first_path + r.n_paths as u32 {
            b.union(&data.paths[q as usize].bbox);
        }
        data.words[w].bbox = b;
    }
    for l in &mut data.lines {
        let mut b = IBox::EMPTY;
        for w in &data.words[l.first_word as usize..(l.first_word + l.n_words) as usize] {
            b.union(&w.bbox);
        }
        l.bbox = b;
    }
    for a in &mut data.ayahs {
        let mut b = IBox::EMPTY;
        for w in &data.words[a.first_word as usize..(a.first_word + a.n_words) as usize] {
            b.union(&w.bbox);
        }
        a.bbox = b;
    }
    data
}

/// Put `cmds` in place of path `i`'s outline, keeping its kind, mark and family.
pub(crate) fn reshape(data: &mut PageData, i: usize, cmds: &[Cmd]) {
    let r = data.paths[i];
    let mut one = Vec::new();
    push_path(&mut data.ops, &mut one, cmds, r.kind, r.mark, r.family, r.flags);
    data.paths[i] = one[0];
}

/// `data` as [`attached`] loads it: reboxed, with its ops in canonical order.
pub(crate) fn settled(data: PageData) -> PageData {
    let mut data = reboxed(data);
    data.canonicalize_ops();
    data
}

/// The page of `data`, settled, with `layer` attached.
pub(crate) fn attached(data: PageData, layer: &LetterData) -> Page {
    let data = settled(data);
    let bytes = encode_letters(layer, &data).unwrap();
    let mut p = Page::from_data(data).unwrap();
    p.attach_letters(&bytes).unwrap();
    p
}

/// The letter fixture with two marks drawn over the ba's piece. Paths 3 and 4 are new in
/// word 0, so every later page path moves up by two and the pieces are paths 10 and 11:
/// - path 3: a sukun of the lam (letter 1, offset 2), `square(5300,1300,5700,1700)`;
/// - path 4: a shaddah of the ba (letter 0, offset 0), `square(5450,1450,5550,1550)`.
///
/// Both lie inside piece 0 (the ba), and path 4 lies inside path 3. Returns the page data and
/// its layer.
pub(crate) fn overlap_fixture() -> (PageData, LetterData) {
    let mut data = letters_page_data();
    let mut extra = Vec::new();
    push_path(
        &mut data.ops,
        &mut extra,
        &square(5300, 1300, 5700, 1700),
        PathKind::Mark,
        Mark::Sukun,
        Family::Diacritic,
        PF_EVENODD,
    );
    push_path(
        &mut data.ops,
        &mut extra,
        &square(5450, 1450, 5550, 1550),
        PathKind::Mark,
        Mark::Shaddah,
        Family::Diacritic,
        PF_EVENODD,
    );
    for (i, rec) in extra.into_iter().enumerate() {
        data.paths.insert(3 + i, rec);
    }
    data.words[0].n_paths = 5;
    data.words[1].first_path = 5;
    data.words[2].first_path = 8;
    data.canonicalize_ops();
    let mut layer = letters_layer();
    layer.n_page_paths = 10;
    let member = |path: u32, at: u16, is_body: bool| MemberRec { path: MemberPath::Page(path), at, is_body };
    let piece = |i: u32| MemberRec { path: MemberPath::Piece(i), at: if i == 0 { 0 } else { 2 }, is_body: true };
    layer.members = vec![
        piece(0),
        member(1, 0, false),
        member(2, 1, false),
        member(4, 0, false),
        piece(1),
        member(3, 2, false),
        member(5, 0, true),
        member(6, 2, true),
        member(8, 0, true),
        member(9, NO_AT, false),
    ];
    for (l, (first, n)) in layer.letters.iter_mut().zip([(0, 4), (4, 2), (6, 1), (7, 1), (8, 2)]) {
        l.first_member = first;
        l.n_members = n;
    }
    layer.word_marks[0].path = 7;
    (data, layer)
}

/// The page of [`overlap_fixture`] with its layer attached.
pub(crate) fn overlap_page() -> Page {
    let (data, layer) = overlap_fixture();
    let bytes = encode_letters(&layer, &data).unwrap();
    let mut p = Page::from_data(data).unwrap();
    p.attach_letters(&bytes).unwrap();
    p
}

/// The layer of the letter fixture with a seam: the lam's piece ends at 48, so from 48 to 50
/// the cut path holds the point and no piece does.
pub(crate) fn seam_layer() -> LetterData {
    let mut layer = letters_layer();
    layer.pieces[1].cmds = square(3900, 1000, 4800, 2000);
    layer
}

/// The letter fixture with the seam of [`seam_layer`].
pub(crate) fn seam_page() -> Page {
    attached(letters_page_data(), &seam_layer())
}

/// The layer of the letter fixture where the omitted alif (path 7, a mark of the page) is a
/// second body of word 2's letter, at offset 0, as a dagger alif can be.
pub(crate) fn body_mark_layer() -> LetterData {
    let mut layer = letters_layer();
    layer.members[7] = MemberRec { path: MemberPath::Page(7), at: 0, is_body: true };
    layer
}

/// The letter fixture with the layer of [`body_mark_layer`].
pub(crate) fn body_mark_page() -> Page {
    attached(letters_page_data(), &body_mark_layer())
}

/// The letter fixture with a counter in the lam, under another word's box: the cut path and
/// the lam's piece have a hole `square(4200,1300,4700,1700)`, and word 1's waqf sign (path 5)
/// is moved over the hole to `square(4400,400,4600,600)`. Once the page is settled, word 1's
/// box reaches over the hole, and the word hit test picks word 1 there. Returns the page data,
/// before [`settled`], and its layer.
pub(crate) fn counter_under_a_box_fixture() -> (PageData, LetterData) {
    let hole = square(4200, 1300, 4700, 1700);
    let mut data = letters_page_data();
    let mut joined = square(4000, 1000, 6000, 2000);
    joined.extend(hole.iter().copied());
    reshape(&mut data, 0, &joined);
    reshape(&mut data, 5, &square(4400, 400, 4600, 600));
    let mut layer = letters_layer();
    let mut lam = square(3900, 1000, 5000, 2000);
    lam.extend(hole.iter().copied());
    layer.pieces[1].cmds = lam;
    (data, layer)
}

/// The letter fixture with a decoration on line 2: path 8, an ayah mark ornament at
/// `square(7000,5000,8000,6000)`. Returns the page data and its layer, whose pieces are paths
/// 9 and 10.
pub(crate) fn letters_decoration_fixture() -> (PageData, LetterData) {
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
    (data, LetterData { n_page_paths: 9, ..letters_layer() })
}
