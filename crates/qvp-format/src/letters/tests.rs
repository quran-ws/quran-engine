use super::*;
use crate::testing::Lcg;
use crate::{encode_cmds, Family, Header, IBox, Mark, PageData, PathKind, PathRec, WordRec, VERSION};

type P = (i32, i32);

const PIECE_A: [Cmd; 4] = [Cmd::MoveTo(1000, 2000), Cmd::LineTo(1100, 2050), Cmd::LineTo(1050, 2100), Cmd::Close];
const PIECE_B: [Cmd; 4] = [
    Cmd::MoveTo(1200, 2000),
    Cmd::QuadTo(1250, 1950, 1300, 2000),
    Cmd::CubicTo(1310, 2010, 1290, 2040, 1200, 2050),
    Cmd::Close,
];

/// The lens that `sample_page` has as path 2: a cubic, a line, a quad, and the line that
/// closes it.
const LENS_C: [P; 4] = [(0, 0), (1000, 3000), (4000, 3000), (5000, 0)];
const LENS_Q: [P; 3] = [(5000, -2000), (2500, -4000), (0, -2000)];

fn square(x0: i32, y0: i32, x1: i32, y1: i32) -> Vec<Cmd> {
    vec![Cmd::MoveTo(x0, y0), Cmd::LineTo(x1, y0), Cmd::LineTo(x1, y1), Cmd::LineTo(x0, y1), Cmd::Close]
}

/// A page of 2 words and 6 paths. Word 0 holds paths 0 to 2, word 1 paths 3 to 5. Path 2
/// is the lens, path 3 two triangles that start at the same point.
fn sample_page() -> PageData {
    let lens = vec![
        Cmd::MoveTo(0, 0),
        Cmd::CubicTo(1000, 3000, 4000, 3000, 5000, 0),
        Cmd::LineTo(5000, -2000),
        Cmd::QuadTo(2500, -4000, 0, -2000),
        Cmd::Close,
    ];
    let pair = vec![
        Cmd::MoveTo(10000, 0),
        Cmd::LineTo(12000, 0),
        Cmd::LineTo(12000, 2000),
        Cmd::Close,
        Cmd::MoveTo(10000, 0),
        Cmd::LineTo(8000, 0),
        Cmd::LineTo(8000, -2000),
        Cmd::Close,
    ];
    let geometry = [
        (square(-9000, 0, -8000, 1000), PathKind::Body),
        (square(-9000, 2000, -8800, 2200), PathKind::Mark),
        (lens, PathKind::Body),
        (pair, PathKind::Body),
        (square(9000, 3000, 9200, 3200), PathKind::Mark),
        (square(9400, 3000, 9600, 3200), PathKind::Mark),
    ];
    let mut ops = Vec::new();
    let mut paths = Vec::new();
    for (cmds, kind) in geometry {
        let off = ops.len() as u32;
        let bbox = encode_cmds(&cmds, 0, 0, &mut ops);
        paths.push(PathRec {
            kind,
            mark: Mark::None,
            family: Family::None,
            flags: 0,
            ox: 0,
            oy: 0,
            op_off: off,
            op_len: ops.len() as u32 - off,
            bbox,
        });
    }
    let word = |first_path, n_paths| WordRec { first_path, n_paths, ..WordRec::default() };
    PageData {
        header: Header { version: VERSION, quant: 100, page: 7, flags: 0, width: 345.0, height: 550.0 },
        lines: vec![],
        ayahs: vec![],
        words: vec![word(0, 3), word(3, 3)],
        paths,
        decorations: vec![],
        glyphs: vec![],
        insts: vec![],
        ops,
        strings: vec![],
    }
}

/// A layer of `sample_page` that passes `check_against`.
fn sample() -> LetterData {
    let body = |path, at| MemberRec { path, at, is_body: true };
    let mark = |path, at| MemberRec { path, at, is_body: false };
    LetterData {
        page: 7,
        quant: 100,
        n_page_paths: 6,
        n_page_words: 2,
        letters: vec![
            LetterRec { word: 0, at: 0, text: 'ب', more_at: vec![], first_member: 0, n_members: 2 },
            LetterRec { word: 0, at: 2, text: 'ل', more_at: vec![], first_member: 2, n_members: 2 },
            LetterRec { word: 1, at: 0, text: 'ل', more_at: vec![3], first_member: 4, n_members: 1 },
        ],
        members: vec![
            body(MemberPath::Page(0), 0),
            mark(MemberPath::Page(1), 1),
            body(MemberPath::Piece(0), 2),
            body(MemberPath::Piece(1), 2),
            body(MemberPath::Page(3), 0),
        ],
        word_marks: vec![WordMarkRec { word: 1, path: 4, at: NO_AT }, WordMarkRec { word: 1, path: 5, at: 5 }],
        pieces: vec![PieceRec { source: 2, cmds: PIECE_A.to_vec() }, PieceRec { source: 2, cmds: PIECE_B.to_vec() }],
    }
}

/// The point of a Bézier curve at `t`, rounded.
fn at(pts: &[P], t: f64) -> P {
    let mut v: Vec<(f64, f64)> = pts.iter().map(|p| (p.0 as f64, p.1 as f64)).collect();
    while v.len() > 1 {
        v = v.windows(2).map(|w| (w[0].0 + (w[1].0 - w[0].0) * t, w[0].1 + (w[1].1 - w[0].1) * t)).collect();
    }
    (v[0].0.round() as i32, v[0].1.round() as i32)
}

/// The part `[t0, t1]` of a Bézier curve, rounded, as the command that draws it.
fn part(pts: &[P], t0: f64, t1: f64) -> Cmd {
    // the blossom with i copies of t1 and the rest t0
    let d = pts.len() - 1;
    let q: Vec<P> = (1..=d)
        .map(|i| {
            let mut v: Vec<(f64, f64)> = pts.iter().map(|p| (p.0 as f64, p.1 as f64)).collect();
            for level in 0..d {
                let t = if level < d - i { t0 } else { t1 };
                v = v.windows(2).map(|w| (w[0].0 + (w[1].0 - w[0].0) * t, w[0].1 + (w[1].1 - w[0].1) * t)).collect();
            }
            (v[0].0.round() as i32, v[0].1.round() as i32)
        })
        .collect();
    match d {
        1 => Cmd::LineTo(q[0].0, q[0].1),
        2 => Cmd::QuadTo(q[0].0, q[0].1, q[1].0, q[1].1),
        _ => Cmd::CubicTo(q[0].0, q[0].1, q[1].0, q[1].1, q[2].0, q[2].1),
    }
}

fn mv(p: P) -> Cmd {
    Cmd::MoveTo(p.0, p.1)
}
fn line(p: P) -> Cmd {
    Cmd::LineTo(p.0, p.1)
}

/// A layer of `sample_page` whose pieces use every operation and every escape. The codec
/// tests use it; it does not pass `check_against`, which the codec does not need.
fn rich() -> LetterData {
    let c_half = at(&LENS_C, 0.5);
    let q_half = at(&LENS_Q, 0.5);
    let pieces = vec![
        // the square cut into one piece: a run of three segments and the close
        square(-9000, 0, -8000, 1000),
        // the lens cut in two at x = 2500: the left piece starts at the cubic's start,
        // draws its first half, a cut line to the middle of the quad, and the quad's
        // second half
        vec![mv((0, 0)), part(&LENS_C, 0.0, 0.5), line(q_half), part(&LENS_Q, 0.5, 1.0), Cmd::Close],
        // the right piece: its start and the quad's end come from the registry, and the
        // cut runs back along the left piece's cut
        vec![
            mv(c_half),
            part(&LENS_C, 0.5, 1.0),
            line((5000, -2000)),
            part(&LENS_Q, 0.0, 0.5),
            line(c_half),
            Cmd::Close,
        ],
        // parts from points that plain cubics reached: to the cubic's end, and to the
        // registry point the left piece's cut line added
        vec![
            mv((-3000, -3000)),
            Cmd::CubicTo(-2000, -4000, 6000, 4000, at(&LENS_C, 0.9).0, at(&LENS_C, 0.9).1),
            part(&LENS_C, 0.9, 1.0),
            Cmd::Close,
        ],
        vec![
            mv((7000, -5000)),
            Cmd::CubicTo(6000, -5000, 5000, -4000, at(&LENS_Q, 0.3).0, at(&LENS_Q, 0.3).1),
            part(&LENS_Q, 0.3, 0.5),
            Cmd::Close,
        ],
        // two subpaths, the second a hole, far from the source: plain deltas
        vec![
            mv((100000, 100000)),
            line((100500, 100000)),
            Cmd::QuadTo(100700, 100300, 100500, 100600),
            Cmd::CubicTo(100300, 100900, 100100, 100500, 100000, 100500),
            Cmd::Close,
            mv((100200, 100200)),
            line((100200, 100300)),
            line((100300, 100300)),
            Cmd::Close,
        ],
        // a close followed by a command, and two closes in a row
        vec![
            mv((200000, 0)),
            line((200100, 0)),
            line((200100, 100)),
            Cmd::Close,
            line((200000, -100)),
            line((200050, -50)),
            Cmd::Close,
            Cmd::Close,
        ],
        // subpaths with no close
        vec![mv((300000, 0)), line((300100, 0)), line((300100, 100)), mv((300200, 0)), line((300300, 0))],
        // the middle of the cubic, from a point that a plain cubic reached, then a line to
        // a registry point
        vec![
            mv((-3000, 3000)),
            Cmd::CubicTo(-2000, 4000, -1000, 4000, at(&LENS_C, 0.25).0, at(&LENS_C, 0.25).1),
            part(&LENS_C, 0.25, 0.75),
            line(q_half),
            Cmd::Close,
        ],
        // a part of the line from (5000, 0), to a new point
        vec![mv((5000, 0)), line((5000, -1000)), line((6000, -1000)), Cmd::Close],
        // a move to a point on the quad, and a part from it
        vec![mv(at(&LENS_Q, 0.25)), part(&LENS_Q, 0.25, 0.4), line((3000, -6000)), Cmd::Close],
        // the second triangle of path 3: a run from the second segment that starts at
        // (10000, 0)
        vec![mv((10000, 0)), line((8000, 0)), line((8000, -2000)), Cmd::Close],
        // a line to a segment start
        vec![mv((12000, 2000)), line((9000, 500)), line((12000, 0)), Cmd::Close],
    ];
    let sources = [0, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 3, 3];
    let mut d = sample();
    d.pieces = sources.iter().zip(pieces).map(|(&source, cmds)| PieceRec { source, cmds }).collect();
    // a member for each piece
    let n = d.pieces.len() as u32;
    d.members = (0..n).map(|i| MemberRec { path: MemberPath::Piece(i), at: 0, is_body: true }).collect();
    d.letters = vec![LetterRec { word: 0, at: 0, text: 'ب', more_at: vec![], first_member: 0, n_members: n as u16 }];
    d.word_marks = vec![];
    d
}

fn enc(d: &LetterData) -> Vec<u8> {
    encode_letters(d, &sample_page()).unwrap()
}
fn dec(b: &[u8]) -> Result<LetterData, Error> {
    decode_letters(b, &sample_page())
}

/// Set every letter's `first_member` to the running total of `n_members`.
fn renumber(d: &mut LetterData) {
    let mut first = 0u32;
    for l in &mut d.letters {
        l.first_member = first;
        first += l.n_members as u32;
    }
}

fn header_u32(b: &[u8], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

/// The byte offset of each section, and its length.
fn sections(b: &[u8]) -> Vec<(usize, usize)> {
    let mut at = HEADER_LEN;
    (0..N_SECTIONS)
        .map(|i| {
            let len = header_u32(b, 24 + 4 * i) as usize;
            at += len;
            (at - len, len)
        })
        .collect()
}

/// Replace the varint at `at` with `v`, and fix the length of section `sec`.
fn patch_varint(b: &[u8], at: usize, old_len: usize, v: u32, sec: usize) -> Vec<u8> {
    let mut out = b[..at].to_vec();
    write_varint(&mut out, v);
    let grown = out.len() - at;
    out.extend_from_slice(&b[at + old_len..]);
    let len = header_u32(b, 24 + 4 * sec) as usize + grown - old_len;
    out[24 + 4 * sec..28 + 4 * sec].copy_from_slice(&(len as u32).to_le_bytes());
    out
}

// ── round trips ──

#[test]
fn round_trip() {
    let d = sample();
    assert_eq!(dec(&enc(&d)), Ok(d));
}

#[test]
fn every_operation_round_trips() {
    let d = rich();
    let b = enc(&d);
    pieces::SEEN.with(|s| *s.borrow_mut() = [0; 21]);
    assert_eq!(dec(&b), Ok(d));
    let seen = pieces::SEEN.with(|s| *s.borrow());
    let names = [
        "escape",
        "run",
        "part to end",
        "part to new",
        "part to registry",
        "reverse",
        "cubic",
        "move to segment",
        "move to registry",
        "move on segment",
        "line to segment",
        "line to registry",
        "line on segment",
        "stored part to end",
        "stored part to new",
        "stored part to registry",
        "escape: move",
        "escape: no close",
        "escape: line",
        "escape: quad",
        "escape: close",
    ];
    for (i, n) in names.iter().enumerate() {
        // code 0 is counted under its sub-codes
        if i != 0 {
            assert!(seen[i] > 0, "the rich layer has no {n}: {seen:?}");
        }
    }
}

#[test]
fn pieces_round_trip_bit_for_bit_at_any_offset() {
    // the same cuts, moved across the envelope: the integer split gives the same parts
    let page = sample_page();
    for (dx, dy) in [(0, 0), (1, -1), (-7, 13), (123_456, -654_321)] {
        let mut p = page.clone();
        let mut d = rich();
        let shift = |c: &Cmd| match *c {
            Cmd::MoveTo(x, y) => Cmd::MoveTo(x + dx, y + dy),
            Cmd::LineTo(x, y) => Cmd::LineTo(x + dx, y + dy),
            Cmd::QuadTo(a, b, x, y) => Cmd::QuadTo(a + dx, b + dy, x + dx, y + dy),
            Cmd::CubicTo(a, b, c, e, x, y) => Cmd::CubicTo(a + dx, b + dy, c + dx, e + dy, x + dx, y + dy),
            Cmd::Close => Cmd::Close,
        };
        let mut ops = Vec::new();
        for i in 0..p.paths.len() {
            let cmds: Vec<Cmd> = page.path_cmds(i).unwrap().iter().map(shift).collect();
            let off = ops.len() as u32;
            encode_cmds(&cmds, 0, 0, &mut ops);
            p.paths[i].op_off = off;
            p.paths[i].op_len = ops.len() as u32 - off;
        }
        p.ops = ops;
        for pc in &mut d.pieces {
            pc.cmds = pc.cmds.iter().map(shift).collect();
        }
        assert_eq!(decode_letters(&encode_letters(&d, &p).unwrap(), &p), Ok(d), "{dx}, {dy}");
    }
}

#[test]
fn a_body_member_with_its_own_offset_round_trips() {
    let mut d = sample();
    d.members[0].at = 9;
    d.members[2].at = NO_AT;
    d.members[1].at = 4;
    assert_eq!(dec(&enc(&d)), Ok(d.clone()));
    d.members[1].at = NO_AT;
    assert_eq!(dec(&enc(&d)), Ok(d));
}

#[test]
fn letter_fields_out_of_the_short_codes_round_trip() {
    let mut d = sample();
    // a word that jumps, a gap in the spelling, eight members, two `more_at`
    d.n_page_words = 2;
    d.letters[1].at = 40;
    d.letters[1].more_at = vec![41, 42];
    d.letters[0].n_members = 8;
    d.members.splice(1..1, (0..6).map(|i| MemberRec { path: MemberPath::Page(i), at: NO_AT, is_body: false }));
    renumber(&mut d);
    assert_eq!(dec(&enc(&d)), Ok(d));
}

#[test]
fn empty_layer_round_trips() {
    let d = LetterData { page: 7, quant: 100, n_page_paths: 6, n_page_words: 2, ..LetterData::default() };
    let bytes = enc(&d);
    // four counts, the alphabet count and the group count
    assert_eq!(bytes.len(), HEADER_LEN + 6);
    assert_eq!(dec(&bytes), Ok(d));
}

#[test]
fn the_encoder_is_deterministic() {
    let d = rich();
    let a = enc(&d);
    for _ in 0..3 {
        assert_eq!(enc(&d), a);
    }
    let b = std::thread::spawn(move || encode_letters(&d, &sample_page()).unwrap()).join().unwrap();
    assert_eq!(a, b);
}

// ── the page digest ──

#[test]
fn page_digest_is_fnv_1a_64() {
    assert_eq!(page_digest(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(page_digest(b"a"), 0xaf63_dc4c_8601_ec8c);
    assert_eq!(page_digest(b"foobar"), 0x8594_4171_f739_67e8);
}

#[test]
fn the_header_names_the_page_file() {
    let page = sample_page();
    let b = enc(&sample());
    let file = crate::encode(&page);
    assert_eq!(letters_page_digest(&b), Ok(page_digest(&file)));
    assert_eq!(page_data_digest(&page), Ok(page_digest(&file)));
    // other bytes of the same page give another digest
    let mut other = page.clone();
    other.header.width = 346.0;
    assert_ne!(letters_page_digest(&encode_letters(&sample(), &other).unwrap()), letters_page_digest(&b));
    assert_eq!(letters_page_digest(b"QVL1"), Err(Error::BadMagic));
    assert_eq!(letters_page_digest(&b[..HEADER_LEN - 1]), Err(Error::Truncated("header")));
    assert_eq!(letters_page_digest(b"QVP1\x01\x00"), Err(Error::BadMagic));
}

#[test]
fn a_page_whose_commands_do_not_decode_has_no_digest() {
    let mut page = sample_page();
    page.ops[page.paths[3].op_off as usize] = 9;
    assert_eq!(page_data_digest(&page), Err(Error::Corrupt("bad opcode")));
    assert!(encode_letters(&sample(), &page).is_err());
}

// ── what the encoder refuses ──

#[test]
fn the_encoder_refuses_a_piece_it_cannot_code() {
    let page = sample_page();
    let refused = |f: fn(&mut LetterData)| {
        let mut d = sample();
        f(&mut d);
        encode_letters(&d, &page)
    };
    assert_eq!(refused(|d| d.pieces[1].source = 6), Err(Error::Bounds("piece source")));
    assert_eq!(refused(|d| d.pieces[0].source = 3), Err(Error::Corrupt("piece order")));
    assert_eq!(refused(|d| d.pieces[1].cmds[0] = Cmd::LineTo(1200, 2000)), Err(Error::Corrupt("piece start")));
    assert_eq!(refused(|d| d.pieces[1].cmds.clear()), Err(Error::Corrupt("piece start")));
    assert_eq!(
        refused(|d| d.pieces[0].cmds[1] = Cmd::LineTo(COORD_LIMIT as i32, 2050)),
        Err(Error::Corrupt("coordinate range"))
    );
    let mut glyph = page.clone();
    glyph.paths[2].flags = PF_GLYPH;
    glyph.paths[2].op_off = 0;
    glyph.insts.push(crate::InstRec { glyph: 0, a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 });
    glyph.glyphs.push(crate::GlyphRec { op_off: 0, op_len: page.paths[0].op_len, bbox: IBox::EMPTY });
    assert_eq!(encode_letters(&sample(), &glyph), Err(Error::Corrupt("piece source glyph")));
}

// ── what the decoder rejects ──

#[test]
fn rejects_bad_magic() {
    let mut bytes = enc(&sample());
    bytes[3] = b'2';
    assert_eq!(dec(&bytes), Err(Error::BadMagic));
    assert_eq!(dec(b"QV"), Err(Error::BadMagic));
}

#[test]
fn rejects_bad_version() {
    let mut bytes = enc(&sample());
    bytes[4] = 2;
    assert_eq!(dec(&bytes), Err(Error::BadVersion(2)));
}

#[test]
fn rejects_a_zero_quant() {
    let mut bytes = enc(&sample());
    bytes[6] = 0;
    bytes[7] = 0;
    assert_eq!(dec(&bytes), Err(Error::Corrupt("quant")));
}

#[test]
fn rejects_trailing_bytes() {
    let mut bytes = enc(&sample());
    bytes.push(0);
    assert_eq!(dec(&bytes), Err(Error::Corrupt("trailing bytes")));
}

#[test]
fn rejects_another_page() {
    let bytes = enc(&rich());
    let refused = |f: fn(&mut PageData)| {
        let mut p = sample_page();
        f(&mut p);
        decode_letters(&bytes, &p)
    };
    assert_eq!(refused(|p| p.header.page = 8), Err(Error::Corrupt("layer page")));
    assert_eq!(refused(|p| p.header.quant = 10), Err(Error::Corrupt("layer quant")));
    assert_eq!(refused(|p| p.paths.truncate(5)), Err(Error::Corrupt("layer path count")));
    assert_eq!(refused(|p| p.words.truncate(1)), Err(Error::Corrupt("layer word count")));
}

/// A page with the layer's counts but other paths: the decoder gives an error or some
/// layer, and never panics. The digest tells the two pages apart.
#[test]
fn another_page_with_the_same_counts_does_not_panic() {
    let d = rich();
    let bytes = enc(&d);
    let page = sample_page();
    let mut pages = Vec::new();
    // the paths in another order
    for (a, b) in [(2, 3), (0, 2), (2, 5), (3, 4)] {
        let mut p = page.clone();
        p.paths.swap(a, b);
        pages.push(p);
    }
    // a source path with fewer segments, or none
    for cmds in [square(0, 0, 10, 10), vec![Cmd::MoveTo(0, 0)], vec![]] {
        let mut p = page.clone();
        let off = p.ops.len() as u32;
        encode_cmds(&cmds, 0, 0, &mut p.ops);
        p.paths[2].op_off = off;
        p.paths[2].op_len = p.ops.len() as u32 - off;
        pages.push(p);
    }
    // a source run past the opcode stream, and a source that is a glyph instance
    let mut p = page.clone();
    p.paths[2].op_len = u32::MAX;
    pages.push(p);
    let mut p = page.clone();
    p.paths[3].flags = PF_GLYPH;
    pages.push(p);
    for (i, p) in pages.iter().enumerate() {
        if let Ok(other) = decode_letters(&bytes, p) {
            assert_ne!(other, d, "page {i}");
        }
        assert_ne!(page_data_digest(p).ok(), letters_page_digest(&bytes).ok(), "page {i}");
    }
}

#[test]
fn every_truncation_is_an_error() {
    for d in [sample(), rich()] {
        let bytes = enc(&d);
        for n in 0..bytes.len() {
            assert!(dec(&bytes[..n]).is_err(), "{n} of {} bytes", bytes.len());
        }
    }
}

#[test]
fn a_huge_count_does_not_allocate() {
    let bytes = enc(&sample());
    // The four counts open the tables section, one byte each here.
    assert_eq!(bytes[HEADER_LEN..HEADER_LEN + 4], [3, 5, 2, 2]);
    for i in 0..4 {
        let patched = patch_varint(&bytes, HEADER_LEN + i, 1, u32::MAX, 0);
        assert_eq!(dec(&patched), Err(Error::Truncated("tables")), "count {i}");
    }
    // the alphabet count follows the four counts
    let patched = patch_varint(&bytes, HEADER_LEN + 4, 1, u32::MAX, 0);
    assert_eq!(dec(&patched), Err(Error::Truncated("alphabet")));
}

#[test]
fn a_huge_more_at_count_does_not_allocate() {
    let mut d = sample();
    d.letters[2].more_at = vec![0x7F; 1];
    let bytes = enc(&d);
    let tables_end = HEADER_LEN + header_u32(&bytes, 24) as usize;
    // The more_at count of the last letter is 1, then its one offset 0x7F.
    let at = (HEADER_LEN..tables_end).find(|&i| bytes[i] == 1 && bytes[i + 1] == 0x7F).unwrap();
    let patched = patch_varint(&bytes, at, 1, u32::MAX, 0);
    assert_eq!(dec(&patched), Err(Error::Truncated("more_at")));
}

#[test]
fn a_huge_operation_count_does_not_allocate() {
    let bytes = enc(&sample());
    // the last varint of the tables is the operation count of the last piece
    let (t, len) = sections(&bytes)[0];
    let patched = patch_varint(&bytes, t + len - 1, 1, u32::MAX, 0);
    assert_eq!(dec(&patched), Err(Error::Truncated("operations")));
}

#[test]
fn rejects_a_member_count_that_differs_from_the_members_table() {
    let mut d = sample();
    d.letters[2].n_members = 2;
    assert_eq!(dec(&enc(&d)), Err(Error::Corrupt("member count")));
}

#[test]
fn rejects_out_of_range_indices() {
    let decoded = |f: fn(&mut LetterData)| {
        let mut d = sample();
        f(&mut d);
        dec(&enc(&d))
    };
    assert_eq!(decoded(|d| d.members[0].path = MemberPath::Page(6)), Err(Error::Bounds("member page path")));
    assert_eq!(decoded(|d| d.members[2].path = MemberPath::Piece(2)), Err(Error::Bounds("member piece")));
    assert_eq!(decoded(|d| d.letters[2].word = 2), Err(Error::Bounds("letter word")));
    assert_eq!(decoded(|d| d.word_marks[1].word = 2), Err(Error::Bounds("word mark word")));
    assert_eq!(decoded(|d| d.word_marks[1].path = 6), Err(Error::Bounds("word mark path")));
    // a source past the page: the group's source is the first varint after the word marks
    let bytes = enc(&sample());
    let (t, len) = sections(&bytes)[0];
    // ... then the group count 1, the source 2, the piece count less one 1, two runs
    let at = (t..t + len - 4).rev().find(|&i| bytes[i..i + 3] == [1, 2, 1]).unwrap();
    let mut patched = bytes.clone();
    patched[at + 1] = 6;
    assert_eq!(dec(&patched), Err(Error::Bounds("piece source")));
}

#[test]
fn rejects_a_surrogate_as_text() {
    let mut d = sample();
    d.letters[1].text = '\u{D7FF}';
    d.letters[2].text = '\u{D7FF}';
    let bytes = enc(&d);
    // the alphabet: 'ب' (0x628), then the delta 0xD7FF - 0x628 = 0xD1D7, the varint D7 A3
    // 03. One more gives 0xD800, a surrogate, in the same three bytes.
    let at = (HEADER_LEN..bytes.len() - 2).find(|&i| bytes[i..i + 3] == [0xD7, 0xA3, 0x03]).unwrap();
    let mut patched = bytes.clone();
    patched[at] = 0xD8;
    assert_eq!(dec(&patched), Err(Error::Corrupt("alphabet")));
    let mut patched = bytes.clone();
    // the alphabet has two characters: an index of 2 is past it
    let (t, len) = sections(&bytes)[0];
    let tables = &bytes[t..t + len];
    let last_letter_text = tables.iter().rposition(|&b| b == 1).unwrap();
    patched[t + last_letter_text] = 2;
    assert!(dec(&patched).is_err());
}

#[test]
fn rejects_unknown_member_flags() {
    let bytes = enc(&sample());
    // The flags column of the sample: a body, a mark at its letter's offset plus one, two
    // piece bodies, a body.
    let tables_end = HEADER_LEN + header_u32(&bytes, 24) as usize;
    let flags_at = (HEADER_LEN..tables_end - 4)
        .find(|&i| bytes[i..i + 5] == [1, 8, 3, 3, 1])
        .expect("the flags column of the sample");
    for bad in [0x20, 0x80, 6 << 2] {
        let mut patched = bytes.clone();
        patched[flags_at] |= bad;
        assert_eq!(dec(&patched), Err(Error::Corrupt("member flags")), "{bad:#x}");
    }
}

#[test]
fn rejects_a_coordinate_out_of_range() {
    // A piece that starts at x = 2^30 - 10, coded as a delta of that size from 0. A delta
    // that the same five bytes code takes it past the envelope.
    let mut d = sample();
    let x = COORD_LIMIT as i32 - 10;
    d.pieces[0].cmds = vec![Cmd::MoveTo(x, 0), Cmd::LineTo(x, 10), Cmd::LineTo(x - 10, 10), Cmd::Close];
    let bytes = enc(&d);
    let (xs, len) = sections(&bytes)[1 + 7];
    let mut want = Vec::new();
    write_varint(&mut want, zigzag(x));
    assert_eq!(&bytes[xs..xs + want.len()], &want[..]);
    let mut over = Vec::new();
    write_varint(&mut over, zigzag(x + 20));
    assert_eq!(over.len(), want.len());
    assert!(len >= want.len());
    let mut patched = bytes.clone();
    patched[xs..xs + over.len()].copy_from_slice(&over);
    assert_eq!(dec(&patched), Err(Error::Corrupt("coordinate range")));
}

#[test]
fn rejects_a_piece_that_does_not_start_with_a_move() {
    // the first piece starts with a move from plain deltas: escape 0. Escape 1, no close,
    // leaves it without a move.
    let bytes = enc(&sample());
    let (seg, _) = sections(&bytes)[1 + 2];
    assert_eq!(bytes[seg], 0);
    let mut patched = bytes.clone();
    patched[seg] = 1;
    assert_eq!(dec(&patched), Err(Error::Corrupt("piece start")));
}

#[test]
fn rejects_unused_operation_bits() {
    let mut d = sample();
    // an odd count of operations leaves the high half of the last operation byte unused
    d.pieces.pop();
    d.members[3].path = MemberPath::Piece(0);
    let bytes = enc(&d);
    let (ops, len) = sections(&bytes)[1];
    assert_eq!(bytes[ops + len - 1] >> 4, 0, "an odd count of operations");
    let mut patched = bytes.clone();
    patched[ops + len - 1] |= 0x10;
    assert_eq!(dec(&patched), Err(Error::Corrupt("operations")));
    // and the unused bits of the last correction byte
    let d = rich();
    let bytes = enc(&d);
    let (corr, len) = sections(&bytes)[1 + 5];
    let last = bytes[corr + len - 1];
    let used = (0..4).rev().find(|i| last >> (2 * i) & 3 != 0).unwrap_or(0);
    if used < 3 {
        let mut patched = bytes.clone();
        patched[corr + len - 1] |= 3 << 6;
        assert!(dec(&patched).is_err());
    }
}

#[test]
fn a_letter_with_no_offset_is_refused() {
    let mut d = sample();
    d.letters[0].at = NO_AT;
    d.members[0].at = 0;
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("letter at")));
    assert_eq!(dec(&enc(&d)), Err(Error::Corrupt("letter at")));
}

// ── malformed files ──

/// What decodes, encodes again to bytes that decode to the same layer.
fn stable(b: &[u8], page: &PageData, what: &dyn Fn() -> String) {
    if let Ok(d) = decode_letters(b, page) {
        let again = encode_letters(&d, page).unwrap_or_else(|e| panic!("{}: {e:?}", what()));
        assert_eq!(decode_letters(&again, page), Ok(d), "{}", what());
    }
}

#[test]
fn every_single_byte_mutation_is_an_error_or_a_valid_layer() {
    let page = sample_page();
    for d in [sample(), rich()] {
        let bytes = enc(&d);
        let mut b = bytes.clone();
        for at in 0..bytes.len() {
            for v in 0..=255u8 {
                b[at] = v;
                stable(&b, &page, &|| format!("byte {at} set to {v}"));
            }
            b[at] = bytes[at];
        }
    }
}

#[test]
fn random_mutations_never_panic() {
    let page = sample_page();
    let bytes = enc(&rich());
    crate::testing::each_mutation(&bytes, 4000, |at, value, b| {
        stable(b, &page, &|| format!("byte {at} set to {value}"));
    });
}

/// A deterministic fuzz of the decoder: byte sets, inserts, removals, section lengths
/// that grow or shrink, and decoding against another page. `QVL_FUZZ_CASES` sets the
/// count; `cargo test` runs 20,000.
#[test]
fn a_mutation_fuzz_never_panics() {
    let cases: usize = std::env::var("QVL_FUZZ_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(20_000);
    let page = sample_page();
    let mut swapped = page.clone();
    swapped.paths.swap(2, 3);
    let clean = [enc(&rich()), enc(&sample())];
    let mut rng = Lcg(crate::testing::SEED);
    let mut decoded = 0usize;
    for case in 0..cases {
        let mut b = clean[case % 2].clone();
        for _ in 0..1 + rng.step() % 3 {
            let at = rng.step() as usize % b.len();
            match rng.step() % 6 {
                0 | 1 => b[at] = rng.step() as u8,
                2 => b.insert(at, rng.step() as u8),
                3 => {
                    b.remove(at);
                }
                4 => {
                    // a section length, grown or shrunk by a little
                    let f = 24 + 4 * (rng.step() as usize % N_SECTIONS);
                    if f + 4 <= b.len() {
                        let v = header_u32(&b, f).wrapping_add(rng.step() % 9).wrapping_sub(4);
                        b[f..f + 4].copy_from_slice(&v.to_le_bytes());
                    }
                }
                _ => b[at] ^= 1 << (rng.step() % 8),
            }
            if b.is_empty() {
                break;
            }
        }
        let p = if case % 7 == 0 { &swapped } else { &page };
        stable(&b, p, &|| format!("case {case}"));
        if decode_letters(&b, p).is_ok() {
            decoded += 1;
        }
    }
    eprintln!("{cases} mutated files: {decoded} decode, the rest are errors");
}

// ── the rules that tie a layer to its page ──

#[test]
fn check_against_accepts_the_sample() {
    assert_eq!(sample().check_against(&sample_page()), Ok(()));
}

#[test]
fn check_against_rejects_a_different_page() {
    let checked = |f: fn(&mut PageData)| {
        let mut p = sample_page();
        f(&mut p);
        sample().check_against(&p)
    };
    assert_eq!(checked(|p| p.header.page = 8), Err(Error::Corrupt("layer page")));
    assert_eq!(checked(|p| p.header.quant = 10), Err(Error::Corrupt("layer quant")));
    assert_eq!(checked(|p| p.paths.truncate(5)), Err(Error::Corrupt("layer path count")));
    assert_eq!(checked(|p| p.words.truncate(1)), Err(Error::Corrupt("layer word count")));
}

#[test]
fn check_against_rejects_a_member_outside_its_word() {
    let mut d = sample();
    d.members[1].path = MemberPath::Page(4);
    assert_eq!(d.check_against(&sample_page()), Err(Error::Bounds("letter member")));
}

#[test]
fn check_against_rejects_a_word_mark_outside_its_word() {
    let mut d = sample();
    d.word_marks[0].path = 1;
    assert_eq!(d.check_against(&sample_page()), Err(Error::Bounds("word mark path")));
}

#[test]
fn check_against_rejects_a_piece_cut_from_another_word() {
    let mut d = sample();
    d.pieces[0].source = 3;
    d.pieces[1].source = 3;
    d.members[4].path = MemberPath::Page(5);
    d.word_marks.pop();
    assert_eq!(d.check_against(&sample_page()), Err(Error::Bounds("piece source")));
}

#[test]
fn check_against_rejects_a_piece_cut_from_a_mark() {
    let mut p = sample_page();
    p.paths[2].kind = PathKind::Mark;
    assert_eq!(sample().check_against(&p), Err(Error::Corrupt("piece source kind")));
}

#[test]
fn check_against_rejects_a_path_in_two_letters() {
    let mut d = sample();
    d.letters.insert(2, LetterRec { word: 0, at: 3, text: 'ا', more_at: vec![], first_member: 0, n_members: 1 });
    d.members.insert(4, MemberRec { path: MemberPath::Page(0), at: 3, is_body: true });
    renumber(&mut d);
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("path in two letters")));
}

#[test]
fn check_against_rejects_a_path_that_is_a_member_and_a_source() {
    let mut d = sample();
    d.members[1].path = MemberPath::Page(2);
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("member and source")));
}

#[test]
fn check_against_rejects_a_letter_with_no_body() {
    let mut d = sample();
    d.members[0].is_body = false;
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("letter without body")));
}

#[test]
fn check_against_rejects_unsorted_letters() {
    let mut d = sample();
    d.letters.swap(1, 2);
    renumber(&mut d);
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("letter order")));
}

#[test]
fn check_against_rejects_unsorted_pieces() {
    let mut d = sample();
    d.pieces[1].source = 0;
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("piece order")));
}

#[test]
fn check_against_rejects_a_member_range_that_does_not_add_up() {
    let mut d = sample();
    d.letters[1].first_member = 3;
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("first member")));
    let mut d = sample();
    d.members.pop();
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("member count")));
}

#[test]
fn check_against_rejects_indices_past_the_tables() {
    let mut d = sample();
    d.members[2].path = MemberPath::Piece(9);
    assert_eq!(d.check_against(&sample_page()), Err(Error::Bounds("member piece")));
    let mut d = sample();
    d.letters[2].word = 5;
    assert_eq!(d.check_against(&sample_page()), Err(Error::Bounds("letter word")));
}

/// Each piece shape that the codec cannot give back unchanged. `check_against` rejects
/// every one, so a layer that it accepts round-trips.
#[test]
fn check_against_rejects_every_piece_that_does_not_round_trip() {
    let big = COORD_LIMIT as i32;
    let cases: [(Vec<Cmd>, &str); 5] = [
        (vec![Cmd::Close, Cmd::MoveTo(1200, 2000), Cmd::LineTo(1300, 2000)], "piece start"),
        (vec![], "piece start"),
        (vec![Cmd::LineTo(1200, 2000), Cmd::LineTo(1300, 2000)], "piece start"),
        (vec![Cmd::Close], "piece start"),
        (vec![Cmd::MoveTo(1200, 2000), Cmd::QuadTo(1250, big, 1300, 2000)], "coordinate range"),
    ];
    let page = sample_page();
    for (cmds, what) in cases {
        let mut d = sample();
        d.pieces[1].cmds = cmds.clone();
        assert_eq!(d.check_against(&page), Err(Error::Corrupt(what)), "{cmds:?}");
        assert_ne!(encode_letters(&d, &page).and_then(|b| decode_letters(&b, &page)), Ok(d), "{cmds:?}");
    }
}

#[test]
fn check_against_rejects_a_piece_no_member_uses() {
    let mut d = sample();
    d.pieces.push(PieceRec { source: 2, cmds: PIECE_A.to_vec() });
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("piece unused")));
}

#[test]
fn check_against_rejects_a_piece_two_members_use() {
    let mut d = sample();
    d.members[3].path = MemberPath::Piece(0);
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("piece used twice")));
}

#[test]
fn check_against_rejects_a_word_mark_that_is_a_member() {
    let mut d = sample();
    d.word_marks[0].path = 3;
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("word mark and member")));
}

#[test]
fn check_against_rejects_a_word_mark_that_is_a_source() {
    let mut d = sample();
    d.word_marks.insert(0, WordMarkRec { word: 0, path: 2, at: NO_AT });
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("word mark and source")));
}

#[test]
fn check_against_rejects_a_path_with_two_word_marks() {
    let mut d = sample();
    d.word_marks[1].path = 4;
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("word mark twice")));
}

#[test]
fn check_against_rejects_a_word_path_with_no_role() {
    let mut d = sample();
    d.word_marks.pop();
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("path without role")));
    // A page path outside every word needs no role.
    let mut p = sample_page();
    p.words[1].n_paths = 2;
    let mut d = sample();
    d.word_marks.pop();
    assert_eq!(d.check_against(&p), Ok(()));
}

#[test]
fn check_against_rejects_letters_out_of_spelling_order() {
    let mut d = sample();
    d.letters[1].at = 0;
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("letter at order")));
    let mut d = sample();
    d.letters.swap(0, 1);
    renumber(&mut d);
    assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("letter at order")));
}

#[test]
fn check_against_rejects_a_piece_cut_from_a_glyph_instance() {
    let mut p = sample_page();
    p.paths[2].flags = PF_GLYPH;
    assert_eq!(sample().check_against(&p), Err(Error::Corrupt("piece source glyph")));
}

/// A word whose path run ends past the path table: an index inside the run but past
/// the table is still out of bounds.
#[test]
fn check_against_rejects_a_path_past_the_page() {
    let mut p = sample_page();
    p.words[1].n_paths = 4;
    let mut d = sample();
    d.members[4].path = MemberPath::Page(6);
    assert_eq!(d.check_against(&p), Err(Error::Bounds("letter member")));
    let mut d = sample();
    d.word_marks[1].path = 6;
    assert_eq!(d.check_against(&p), Err(Error::Bounds("word mark path")));
    let mut d = sample();
    d.pieces[1].source = 6;
    assert_eq!(d.check_against(&p), Err(Error::Bounds("piece source")));
}
