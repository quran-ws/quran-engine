//! Synthesised pages and the path builder the unit tests share.
use crate::{Layout, Page};
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
