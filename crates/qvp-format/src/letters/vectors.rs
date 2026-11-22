//! Golden vectors: fixed QVL1 files and the exact commands they decode to. The checked tests
//! do not call the encoder, so a change to the arithmetic that the encoder and the decoder
//! share fails here, where a round trip cannot see it.
//!
//! Each vector isolates one rule of `docs/FORMAT.md`. `print_vectors`, an ignored test,
//! builds the files from `definitions` with the encoder and prints the block at the end of
//! this file: `cargo test -p qvp-format --release print_vectors -- --ignored --nocapture`.
//! Regenerate only when the format changes.

use super::*;
use crate::{encode_cmds, Family, Header, Mark, PathRec, WordRec, VERSION};

type P = (i32, i32);

/// The cubic, line and quad of path 0, at odd and negative coordinates, so that splits
/// are inexact and round both ways.
const A_C: [P; 4] = [(-1000, -1000), (-700, -1301), (-299, -1299), (1, -999)];
const A_Q: [P; 3] = [(1, -2001), (-500, -2303), (-1000, -2001)];

/// The page of the vectors: path 0 is source A, path 1 source B (two subpaths, the second
/// with a cubic), path 2 a mark. One word holds the three paths. Page 9, `quant` 100.
pub(super) fn vector_page() -> PageData {
    let a = vec![
        Cmd::MoveTo(-1000, -1000),
        Cmd::CubicTo(-700, -1301, -299, -1299, 1, -999),
        Cmd::LineTo(1, -2001),
        Cmd::QuadTo(-500, -2303, -1000, -2001),
        Cmd::Close,
    ];
    let b = vec![
        Cmd::MoveTo(5000, 0),
        Cmd::LineTo(6000, 0),
        Cmd::LineTo(6000, 1000),
        Cmd::Close,
        Cmd::MoveTo(7000, 0),
        Cmd::CubicTo(7300, 0, 7700, 400, 8000, 1000),
        Cmd::LineTo(7000, 1000),
        Cmd::Close,
    ];
    let mark = vec![Cmd::MoveTo(0, 500), Cmd::LineTo(100, 500), Cmd::LineTo(100, 600), Cmd::Close];
    let mut ops = Vec::new();
    let mut paths = Vec::new();
    for (cmds, kind) in [(a, PathKind::Body), (b, PathKind::Body), (mark, PathKind::Mark)] {
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
    PageData {
        header: Header { version: VERSION, quant: 100, page: 9, flags: 0, width: 100.0, height: 100.0 },
        lines: vec![],
        ayahs: vec![],
        words: vec![WordRec { first_path: 0, n_paths: 3, ..WordRec::default() }],
        paths,
        decorations: vec![],
        glyphs: vec![],
        insts: vec![],
        ops,
        strings: vec![],
    }
}

/// The layer of a vector: one letter, whose members are the pieces.
fn layer(pieces: &[(u32, &[Cmd])]) -> LetterData {
    let n = pieces.len() as u32;
    LetterData {
        page: 9,
        quant: 100,
        n_page_paths: 3,
        n_page_words: 1,
        letters: vec![LetterRec { word: 0, at: 0, text: 'ب', more_at: vec![], first_member: 0, n_members: n as u16 }],
        members: (0..n).map(|i| MemberRec { path: MemberPath::Piece(i), at: 0, is_body: true }).collect(),
        word_marks: vec![],
        pieces: pieces.iter().map(|(s, c)| PieceRec { source: *s, cmds: c.to_vec() }).collect(),
    }
}

struct Vector {
    name: &'static str,
    /// The operations the vector is about: codes 0 to 15, escapes as 16 + sub-code.
    ops: &'static [usize],
    hex: &'static str,
    pieces: &'static [(u32, &'static [Cmd])],
}

/// The byte offset and the length of each section.
fn sections_of(b: &[u8]) -> Vec<(usize, usize)> {
    let mut at = HEADER_LEN;
    (0..N_SECTIONS)
        .map(|i| {
            let f = LENGTHS_AT + 4 * i;
            let len = u32::from_le_bytes(b[f..f + 4].try_into().unwrap()) as usize;
            at += len;
            (at - len, len)
        })
        .collect()
}

fn unhex(s: &str) -> Vec<u8> {
    (0..s.len()).step_by(2).map(|i| u8::from_str_radix(&s[i..i + 2], 16).unwrap()).collect()
}

fn decoded(v: &Vector) -> (Result<LetterData, Error>, [u32; 21]) {
    pieces::SEEN.with(|s| *s.borrow_mut() = [0; 21]);
    let r = decode_letters(&unhex(v.hex), &vector_page());
    (r, pieces::SEEN.with(|s| *s.borrow()))
}

#[test]
fn every_vector_decodes_to_its_commands() {
    for v in VECTORS {
        let (r, seen) = decoded(v);
        assert_eq!(r, Ok(layer(v.pieces)), "vector {}", v.name);
        for &op in v.ops {
            assert!(seen[op] > 0, "vector {} has no operation {op}: {seen:?}", v.name);
        }
    }
}

#[test]
fn vectors_reject_unused_correction_bits() {
    // `split` has six corrections: two codes of its last correction byte are unused
    let v = VECTORS.iter().find(|v| v.name == "split").unwrap();
    let b = unhex(v.hex);
    let (corr, len) = sections_of(&b)[1 + 5];
    assert_eq!(len, 2);
    assert_eq!(b[corr + 1] >> 4, 0);
    let mut bad = b.clone();
    bad[corr + 1] |= 0x40;
    assert_eq!(decode_letters(&super::tests::seal(bad), &vector_page()), Err(Error::Damaged("corrections")));
}

#[test]
fn vectors_reject_a_large_correction_that_a_short_code_gives() {
    // `corrections` stores -2 and +2 as large corrections: zigzag 3 and 4
    let v = VECTORS.iter().find(|v| v.name == "corrections").unwrap();
    let b = unhex(v.hex);
    let (esc, len) = sections_of(&b)[1 + 6];
    assert_eq!(&b[esc..esc + len], &[3, 4]);
    let mut bad = b.clone();
    bad[esc] = 1;
    assert_eq!(decode_letters(&super::tests::seal(bad), &vector_page()), Err(Error::Damaged("corrections")));
}

/// The page of the worked example in `docs/FORMAT.md`: page 1, `quant` 100, one word, one
/// path, the lens.
fn example_page() -> PageData {
    let lens = [
        Cmd::MoveTo(0, 0),
        Cmd::CubicTo(1000, 3000, 4000, 3000, 5000, 0),
        Cmd::LineTo(5000, -2000),
        Cmd::QuadTo(2500, -4000, 0, -2000),
        Cmd::Close,
    ];
    let mut p = vector_page();
    p.header.page = 1;
    p.ops.clear();
    p.paths.truncate(1);
    p.paths[0].bbox = encode_cmds(&lens, 0, 0, &mut p.ops);
    p.paths[0].op_len = p.ops.len() as u32;
    p.words[0].n_paths = 1;
    p
}

/// The layer of the worked example: two letters, a piece each.
fn example() -> LetterData {
    let left = vec![
        Cmd::MoveTo(0, 0),
        Cmd::CubicTo(500, 1500, 1500, 2250, 2500, 2250),
        Cmd::LineTo(2500, -3000),
        Cmd::QuadTo(1250, -3000, 0, -2000),
        Cmd::Close,
    ];
    let right = vec![
        Cmd::MoveTo(2500, 2250),
        Cmd::CubicTo(3500, 2250, 4500, 1500, 5000, 0),
        Cmd::LineTo(5000, -2000),
        Cmd::QuadTo(3750, -3000, 2500, -3000),
        Cmd::LineTo(2500, 2250),
        Cmd::Close,
    ];
    let letter = |at, text, first_member| LetterRec { word: 0, at, text, more_at: vec![], first_member, n_members: 1 };
    LetterData {
        page: 1,
        quant: 100,
        n_page_paths: 1,
        n_page_words: 1,
        letters: vec![letter(0, 'ل', 0), letter(1, 'ا', 1)],
        members: vec![
            MemberRec { path: MemberPath::Piece(0), at: 0, is_body: true },
            MemberRec { path: MemberPath::Piece(1), at: 1, is_body: true },
        ],
        word_marks: vec![],
        pieces: vec![PieceRec { source: 0, cmds: left }, PieceRec { source: 0, cmds: right }],
    }
}

#[test]
fn the_worked_example_decodes_as_documented() {
    assert_eq!(decode_letters(&unhex(EXAMPLE_HEX), &example_page()), Ok(example()));
}

// ── the definitions, and the printer that freezes them ──

fn at(pts: &[P], t: f64) -> P {
    let mut v: Vec<(f64, f64)> = pts.iter().map(|p| (p.0 as f64, p.1 as f64)).collect();
    while v.len() > 1 {
        v = v.windows(2).map(|w| (w[0].0 + (w[1].0 - w[0].0) * t, w[0].1 + (w[1].1 - w[0].1) * t)).collect();
    }
    (v[0].0.round() as i32, v[0].1.round() as i32)
}

fn part(pts: &[P], t0: f64, t1: f64) -> Cmd {
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
fn ln(p: P) -> Cmd {
    Cmd::LineTo(p.0, p.1)
}
fn cubic_to(c1: P, c2: P, e: P) -> Cmd {
    Cmd::CubicTo(c1.0, c1.1, c2.0, c2.1, e.0, e.1)
}

/// A vector's name, the operations it must use, and its pieces as (source, commands).
type Definition = (&'static str, Vec<usize>, Vec<(u32, Vec<Cmd>)>);

fn definitions() -> Vec<Definition> {
    let c_half = at(&A_C, 0.5);
    let q_half = at(&A_Q, 0.5);
    let inner = (-500, -1500);
    vec![
        // an inexact split of a cubic from its start, at negative coordinates
        ("split", vec![7, 3], vec![(0, vec![mv((-1000, -1000)), part(&A_C, 0.0, 0.37), Cmd::Close])]),
        // parts from stored starts whose first control point rounds differently when a
        // de Casteljau step rounds towards zero instead of down: v0 40 and v1 601, then v0
        // 139 and v1 1300, on the cubic's grid of 2^11
        (
            "rounding",
            vec![14],
            vec![
                (
                    0,
                    vec![
                        mv((-3000, -2000)),
                        cubic_to((-3000, -1500), (-2000, -1000), (-982, -1017)),
                        cubic_to((-899, -1097), (-809, -1153), (-715, -1187)),
                        Cmd::Close,
                    ],
                ),
                (
                    0,
                    vec![
                        mv((-3000, -2500)),
                        cubic_to((-3000, -1800), (-2000, -1300), (-938, -1057)),
                        cubic_to((-760, -1204), (-555, -1255), (-358, -1208)),
                        Cmd::Close,
                    ],
                ),
            ],
        ),
        // a part of a line, to a new point
        ("line part", vec![7, 3, 18], vec![(0, vec![mv((1, -999)), ln((1, -1500)), ln((-400, -1500)), Cmd::Close])]),
        // a part of a quad
        ("quad part", vec![7, 3], vec![(0, vec![mv((1, -2001)), part(&A_Q, 0.0, 0.6), Cmd::Close])]),
        // a cubic from deltas, then the same cubic backwards
        (
            "reversed cubic",
            vec![6, 5, 16],
            vec![
                (
                    0,
                    vec![
                        mv((-5000, -5000)),
                        cubic_to((-5100, -4800), (-4800, -4600), (-4700, -4900)),
                        ln((-5000, -4900)),
                        Cmd::Close,
                    ],
                ),
                (
                    0,
                    vec![
                        mv((-4700, -4900)),
                        cubic_to((-4800, -4600), (-5100, -4800), (-5000, -5000)),
                        ln((-4700, -5000)),
                        Cmd::Close,
                    ],
                ),
            ],
        ),
        // source A cut along two lines: the second piece draws the cut backwards, both lines
        (
            "reversed cut",
            vec![7, 3, 12, 2, 8, 1, 4, 5],
            vec![
                (
                    0,
                    vec![
                        mv((-1000, -1000)),
                        part(&A_C, 0.0, 0.5),
                        ln(inner),
                        ln(q_half),
                        part(&A_Q, 0.5, 1.0),
                        Cmd::Close,
                    ],
                ),
                (
                    0,
                    vec![
                        mv(c_half),
                        part(&A_C, 0.5, 1.0),
                        ln((1, -2001)),
                        part(&A_Q, 0.0, 0.5),
                        ln(inner),
                        ln(c_half),
                        Cmd::Close,
                    ],
                ),
            ],
        ),
        // a run from the line that ends the second subpath of source B, through its closing
        // line, to the subpath's first segment
        (
            "run wraps",
            vec![7, 1],
            vec![(
                1,
                vec![
                    mv((8000, 1000)),
                    ln((7000, 1000)),
                    ln((7000, 0)),
                    cubic_to((7300, 0), (7700, 400), (8000, 1000)),
                    Cmd::Close,
                ],
            )],
        ),
        // the three stored starts, from points that plain cubics reached; the first piece
        // adds the middle of the cubic to the registry
        (
            "stored starts",
            vec![13, 14, 15, 3],
            vec![
                (0, vec![mv((-1000, -1000)), part(&A_C, 0.0, 0.5), Cmd::Close]),
                (
                    0,
                    vec![
                        mv((-3000, -500)),
                        cubic_to((-3000, -900), (-2000, -1300), at(&A_C, 0.25)),
                        part(&A_C, 0.25, 0.5),
                        Cmd::Close,
                    ],
                ),
                (
                    0,
                    vec![
                        mv((-3000, -600)),
                        cubic_to((-3000, -900), (-2000, -1300), at(&A_C, 0.6)),
                        part(&A_C, 0.6, 0.8),
                        Cmd::Close,
                    ],
                ),
                (
                    0,
                    vec![
                        mv((-3000, -700)),
                        cubic_to((-3000, -900), (-2000, -1300), at(&A_C, 0.9)),
                        part(&A_C, 0.9, 1.0),
                        Cmd::Close,
                    ],
                ),
            ],
        ),
        // the cubic of source B with its control points moved by +1, -1, -2 and +2
        (
            "corrections",
            vec![7, 2, 1],
            vec![(
                1,
                vec![
                    mv((7000, 0)),
                    cubic_to((7301, -1), (7698, 402), (8000, 1000)),
                    ln((7000, 1000)),
                    ln((7000, 0)),
                    Cmd::Close,
                ],
            )],
        ),
        // the escapes: a close with a command after it, two closes, no close, a quad
        (
            "escapes",
            vec![16, 17, 18, 19, 20],
            vec![
                (
                    0,
                    vec![
                        mv((20000, 0)),
                        ln((20100, 0)),
                        ln((20100, 100)),
                        Cmd::Close,
                        ln((20000, -100)),
                        ln((20050, -50)),
                        Cmd::Close,
                        Cmd::Close,
                    ],
                ),
                (0, vec![mv((30000, 0)), ln((30100, 0)), mv((30200, 0)), ln((30300, 0))]),
                (0, vec![mv((40000, 0)), Cmd::QuadTo(40100, 100, 40200, 0), Cmd::Close]),
            ],
        ),
        // source B as one piece: two subpaths, each a move to a segment start and a run
        (
            "two subpaths",
            vec![7, 1],
            vec![(
                1,
                vec![
                    mv((5000, 0)),
                    ln((6000, 0)),
                    ln((6000, 1000)),
                    Cmd::Close,
                    mv((7000, 0)),
                    cubic_to((7300, 0), (7700, 400), (8000, 1000)),
                    ln((7000, 1000)),
                    Cmd::Close,
                ],
            )],
        ),
    ]
}

fn cmd_lit(c: &Cmd) -> String {
    match *c {
        Cmd::MoveTo(x, y) => format!("Cmd::MoveTo({x}, {y})"),
        Cmd::LineTo(x, y) => format!("Cmd::LineTo({x}, {y})"),
        Cmd::QuadTo(a, b, x, y) => format!("Cmd::QuadTo({a}, {b}, {x}, {y})"),
        Cmd::CubicTo(a, b, c, d, x, y) => format!("Cmd::CubicTo({a}, {b}, {c}, {d}, {x}, {y})"),
        Cmd::Close => "Cmd::Close".into(),
    }
}

#[test]
#[ignore]
fn print_vectors() {
    let page = vector_page();
    println!("// ── generated by print_vectors ──\n");
    let b = encode_letters(&example(), &example_page()).unwrap();
    let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
    println!("/// The worked example of `docs/FORMAT.md`, {} bytes.\nconst EXAMPLE_HEX: &str = \"{hex}\";\n", b.len());
    println!("const VECTORS: &[Vector] = &[");
    for (name, ops, pcs) in definitions() {
        let refs: Vec<(u32, &[Cmd])> = pcs.iter().map(|(s, c)| (*s, c.as_slice())).collect();
        let d = layer(&refs);
        let b = encode_letters(&d, &page).unwrap();
        pieces::SEEN.with(|s| *s.borrow_mut() = [0; 21]);
        assert_eq!(decode_letters(&b, &page), Ok(d), "{name}");
        let seen = pieces::SEEN.with(|s| *s.borrow());
        for &op in &ops {
            assert!(seen[op] > 0, "{name}: no operation {op}: {seen:?}");
        }
        let hex: String = b.iter().map(|x| format!("{x:02x}")).collect();
        println!("    Vector {{\n        name: {name:?},\n        ops: &{ops:?},\n        hex: \"{hex}\",\n        pieces: &[");
        for (s, c) in &pcs {
            let cs: Vec<String> = c.iter().map(cmd_lit).collect();
            println!("            ({s}, &[{}]),", cs.join(", "));
        }
        println!("        ],\n    }},");
    }
    println!("];");
}

// ── generated by print_vectors ──

/// The worked example of `docs/FORMAT.md`, 113 bytes.
const EXAMPLE_HEX: &str = "51564c310100640001000100010000008c266e358c0d058054fa49673a37ac28150000000500000001000000020000000300000005000000040000000000000000000000000000000202000202a70c1d00010000030300000100010405372c284105000002010001808001804000000000";

const VECTORS: &[Vector] = &[
    Vector {
        name: "split",
        ops: &[7, 3],
        hex: "51564c3101006400090001000300000094fdac7c3f3af28640728a0486694f5f0f0000000100000000000000010000000000000002000000020000000000000000000000000000000101000101a80c00000300010000023700f5050000",
        pieces: &[
            (0, &[Cmd::MoveTo(-1000, -1000), Cmd::CubicTo(-889, -1111, -764, -1181, -636, -1210), Cmd::Close]),
        ],
    },
    Vector {
        name: "rounding",
        ops: &[14],
        hex: "51564c3101006400090001000300000094fdac7c3f3af28662ab6d9257e31e4612000000030000000000000004000000000000000700000003000000000000000d0000000f0000000102000201a80c0100030300000100010303600ee60000000028d9048a01950a000000ef2e00d00ff40f0000d00fcc109f1fe807e80721e707f80ae807e603",
        pieces: &[
            (0, &[Cmd::MoveTo(-3000, -2000), Cmd::CubicTo(-3000, -1500, -2000, -1000, -982, -1017), Cmd::CubicTo(-899, -1097, -809, -1153, -715, -1187), Cmd::Close]),
            (0, &[Cmd::MoveTo(-3000, -2500), Cmd::CubicTo(-3000, -1800, -2000, -1300, -938, -1057), Cmd::CubicTo(-760, -1204, -555, -1255, -358, -1208), Cmd::Close]),
        ],
    },
    Vector {
        name: "line part",
        ops: &[7, 3, 18],
        hex: "51564c3101006400090001000300000094fdac7c3f3af286c2f99582d7caec300f0000000200000000000000020000000000000002000000010000000000000002000000010000000101000101a80c000003000100000337000102800800a10600",
        pieces: &[
            (0, &[Cmd::MoveTo(1, -999), Cmd::LineTo(1, -1500), Cmd::LineTo(-400, -1500), Cmd::Close]),
        ],
    },
    Vector {
        name: "quad part",
        ops: &[7, 3],
        hex: "51564c3101006400090001000300000094fdac7c3f3af2865a4cffa65d5d17b00f0000000100000000000000010000000000000002000000010000000000000000000000000000000101000101a80c00000300010000023702cd0900",
        pieces: &[
            (0, &[Cmd::MoveTo(1, -2001), Cmd::QuadTo(-300, -2182, -600, -2146), Cmd::Close]),
        ],
    },
    Vector {
        name: "reversed cubic",
        ops: &[6, 5, 16],
        hex: "51564c3101006400090001000300000094fdac7c3f3af2863cfb0dcffa27311412000000030000000000000004000000010000000000000000000000000000000e0000000c0000000102000201a80c010003030000010001030360000500020002028f4ec701d804c801d704d804d8048f4e90039003d70400c80100",
        pieces: &[
            (0, &[Cmd::MoveTo(-5000, -5000), Cmd::CubicTo(-5100, -4800, -4800, -4600, -4700, -4900), Cmd::LineTo(-5000, -4900), Cmd::Close]),
            (0, &[Cmd::MoveTo(-4700, -4900), Cmd::CubicTo(-4800, -4600, -5100, -4800, -5000, -5000), Cmd::LineTo(-4700, -5000), Cmd::Close]),
        ],
    },
    Vector {
        name: "reversed cut",
        ops: &[7, 3, 12, 2, 8, 1, 4, 5],
        hex: "51564c3101006400090001000300000094fdac7c3f3af28613c5b0ec81c15309120000000600000001000000030000000400000004000000040000000000000001000000020000000102000201a80c010003030000010001050637c0821254050000020201000102ff0781080800100000a504",
        pieces: &[
            (0, &[Cmd::MoveTo(-1000, -1000), Cmd::CubicTo(-850, -1151, -675, -1225, -500, -1225), Cmd::LineTo(-500, -1500), Cmd::LineTo(-500, -2152), Cmd::QuadTo(-750, -2152, -1000, -2001), Cmd::Close]),
            (0, &[Cmd::MoveTo(-500, -1225), Cmd::CubicTo(-324, -1225, -149, -1149, 1, -999), Cmd::LineTo(1, -2001), Cmd::QuadTo(-250, -2152, -500, -2152), Cmd::LineTo(-500, -1500), Cmd::LineTo(-500, -1225), Cmd::Close]),
        ],
    },
    Vector {
        name: "run wraps",
        ops: &[7, 1],
        hex: "51564c3101006400090001000300000094fdac7c3f3af286718536064fee83fd0f0000000100000001000000010000000000000000000000000000000000000000000000000000000101000101a80c0000030001010002170204",
        pieces: &[
            (1, &[Cmd::MoveTo(8000, 1000), Cmd::LineTo(7000, 1000), Cmd::LineTo(7000, 0), Cmd::CubicTo(7300, 0, 7700, 400, 8000, 1000), Cmd::Close]),
        ],
    },
    Vector {
        name: "stored starts",
        ops: &[13, 14, 15, 3],
        hex: "51564c3101006400090001000300000094fdac7c3f3af286f5811e7cd959168618000000060000000000000007000000010000000a000000050000000000000013000000180000000104000401a80c030003030303000000000100030203030337600fe6600d0000000000000000ff078004cd09e60cb30e08000000009f1f00d00fb2130000d00f8a190000d00fe81de8079f069f068602c701d7049f06a801c7018f039f06b803",
        pieces: &[
            (0, &[Cmd::MoveTo(-1000, -1000), Cmd::CubicTo(-850, -1151, -675, -1225, -500, -1225), Cmd::Close]),
            (0, &[Cmd::MoveTo(-3000, -500), Cmd::CubicTo(-3000, -900, -2000, -1300, -759, -1169), Cmd::CubicTo(-675, -1206, -587, -1225, -500, -1225), Cmd::Close]),
            (0, &[Cmd::MoveTo(-3000, -600), Cmd::CubicTo(-3000, -900, -2000, -1300, -395, -1216), Cmd::CubicTo(-325, -1203, -256, -1179, -190, -1143), Cmd::Close]),
            (0, &[Cmd::MoveTo(-3000, -700), Cmd::CubicTo(-3000, -900, -2000, -1300, -92, -1080), Cmd::CubicTo(-60, -1056, -29, -1029, 1, -999), Cmd::Close]),
        ],
    },
    Vector {
        name: "corrections",
        ops: &[7, 2, 1],
        hex: "51564c3101006400090001000300000094fdac7c3f3af2864a34c3b5acb81d6e0f0000000200000001000000010000000000000000000000010000000200000000000000000000000101000101a80c000003000101000327010103f90304",
        pieces: &[
            (1, &[Cmd::MoveTo(7000, 0), Cmd::CubicTo(7301, -1, 7698, 402, 8000, 1000), Cmd::LineTo(7000, 1000), Cmd::LineTo(7000, 0), Cmd::Close]),
        ],
    },
    Vector {
        name: "escapes",
        ops: &[16, 17, 18, 19, 20],
        hex: "51564c3101006400090001000300000094fdac7c3f3af286e20d66996da43c04150000000800000000000000100000000000000000000000000000000000000018000000100000000103000301a80c0200030303000000010002080602000000000000000000020204020204040002010002010003c0b802c801000064a09c01c801c801c801a09c01c801c8010000c801c701640000000000c801c701",
        pieces: &[
            (0, &[Cmd::MoveTo(20000, 0), Cmd::LineTo(20100, 0), Cmd::LineTo(20100, 100), Cmd::Close, Cmd::LineTo(20000, -100), Cmd::LineTo(20050, -50), Cmd::Close, Cmd::Close]),
            (0, &[Cmd::MoveTo(30000, 0), Cmd::LineTo(30100, 0), Cmd::MoveTo(30200, 0), Cmd::LineTo(30300, 0)]),
            (0, &[Cmd::MoveTo(40000, 0), Cmd::QuadTo(40100, 100, 40200, 0), Cmd::Close]),
        ],
    },
    Vector {
        name: "two subpaths",
        ops: &[7, 1],
        hex: "51564c3101006400090001000300000094fdac7c3f3af28656d2de6f611421950f0000000200000002000000020000000000000000000000000000000000000000000000000000000101000101a80c0000030001010004171701010003",
        pieces: &[
            (1, &[Cmd::MoveTo(5000, 0), Cmd::LineTo(6000, 0), Cmd::LineTo(6000, 1000), Cmd::Close, Cmd::MoveTo(7000, 0), Cmd::CubicTo(7300, 0, 7700, 400, 8000, 1000), Cmd::LineTo(7000, 1000), Cmd::Close]),
        ],
    },
];
