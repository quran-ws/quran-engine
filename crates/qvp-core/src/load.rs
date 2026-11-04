//! Loading a page: the steps that turn decoded page data into what a `Page` reads.
use crate::reflow::line_under_stroke;
use crate::style::PathCtx;
use crate::{Geometry, PathGeom, NONE};
use qvp_format::*;
use std::collections::HashMap;

/// The word and the decoration each path belongs to, indexed by path. `NONE` where a path
/// belongs to none.
pub(crate) fn path_owners(data: &PageData) -> (Vec<u32>, Vec<u32>) {
    let mut path_word = vec![NONE; data.paths.len()];
    for (wi, w) in data.words.iter().enumerate() {
        for p in w.first_path..w.first_path + w.n_paths as u32 {
            path_word[p as usize] = wi as u32;
        }
    }
    let mut path_deco = vec![NONE; data.paths.len()];
    for (di, d) in data.decorations.iter().enumerate() {
        for p in d.first_path..d.first_path + d.n_paths as u32 {
            path_deco[p as usize] = di as u32;
        }
    }
    (path_word, path_deco)
}

/// The reference centre of every line, in page units.
///
/// Line reference centres come from body ink. Diacritics would wobble them. A sajdah sign or
/// a rosette on the line is taller than the ink and would pull the centre off the text. A
/// header line (surah name, basmalah) has no words, so it uses its decoration.
pub(crate) fn line_centres(data: &PageData) -> Vec<f32> {
    let q = data.header.quant as f32;
    let mut line_centre = vec![0f32; data.lines.len()];
    for (li, l) in data.lines.iter().enumerate() {
        let mut bb = IBox::EMPTY;
        for wi in l.first_word..l.first_word + l.n_words {
            let w = &data.words[wi as usize];
            for p in w.first_path..w.first_path + w.n_paths as u32 {
                let pr = &data.paths[p as usize];
                if pr.kind == PathKind::Body {
                    bb.union(&pr.bbox);
                }
            }
        }
        if bb.is_empty() {
            for d in data
                .decorations
                .iter()
                .filter(|d| d.line as usize == li && matches!(d.kind, DecoKind::SurahName | DecoKind::Basmalah))
            {
                for pi in d.first_path..d.first_path + d.n_paths as u32 {
                    let path = &data.paths[pi as usize];
                    if path.kind == PathKind::HeaderInk {
                        bb.union(&path.bbox);
                    }
                }
            }
            // Old page data can carry an unclassified header path. Keep its established
            // centre rather than turning the line into an empty one.
            if bb.is_empty() {
                for d in data
                    .decorations
                    .iter()
                    .filter(|d| d.line as usize == li && matches!(d.kind, DecoKind::SurahName | DecoKind::Basmalah))
                {
                    bb.union(&d.bbox);
                }
            }
        }
        if bb.is_empty() {
            bb = l.bbox;
        }
        line_centre[li] = if bb.is_empty() { 0.0 } else { (bb.y0 + bb.y1) as f32 / 2.0 / q };
    }
    line_centre
}

/// The printed line spacing: the median distance between neighbouring line centres, or a
/// fifteenth of the page height when the page has too few lines to measure.
pub(crate) fn line_spacing(data: &PageData, line_centre: &[f32]) -> f32 {
    let mut d: Vec<f32> = line_centre.windows(2).map(|w| (w[1] - w[0]).abs()).filter(|v| *v > 1.0).collect();
    d.sort_by(f32::total_cmp);
    if d.is_empty() {
        data.header.height / 15.0
    } else {
        d[d.len() / 2]
    }
}

/// The line every path is laid out with, indexed by path.
pub(crate) fn path_lines(data: &PageData, path_word: &[u32], path_deco: &[u32], line_centre: &[f32]) -> Vec<u32> {
    let q = data.header.quant as f32;
    let nearest_line = |y: f32| -> u32 {
        let mut best = 0usize;
        let mut bd = f32::MAX;
        for (i, c) in line_centre.iter().enumerate() {
            let dd = (c - y).abs();
            if dd < bd {
                bd = dd;
                best = i;
            }
        }
        best as u32
    };
    // An ayah mark is stored without a line: the exporter keeps the marks outside the
    // line groups. It takes the line of the last word of the ayah it closes, so it
    // moves with that line under layout. Anything else without a line snaps to the
    // nearest line centre.
    let mut deco_line = vec![NONE; data.decorations.len()];
    for a in data.ayahs.iter().filter(|a| a.fragment == a.fragments && a.n_words > 0) {
        let di = a.ayah_mark_decoration as usize;
        if di < data.decorations.len() {
            deco_line[di] = data.words[(a.first_word + a.n_words - 1) as usize].line_index as u32;
        }
    }
    let mut path_line = vec![0u32; data.paths.len()];
    for i in 0..data.paths.len() {
        let wi = path_word[i];
        path_line[i] = if wi != NONE {
            data.words[wi as usize].line_index as u32
        } else if path_deco[i] != NONE {
            let d = &data.decorations[path_deco[i] as usize];
            // The sajdah line is drawn over the sajdah word, which can sit lines above the
            // sign that closes the ayah: it takes the line of the word directly below it.
            let under = if data.paths[i].mark == Mark::SajdahLine {
                line_under_stroke(data, &data.paths[i].bbox)
            } else {
                None
            };
            if let Some(li) = under {
                li as u32
            } else if d.line != NONE_U16 && (d.line as usize) < data.lines.len() {
                d.line as u32
            } else if deco_line[path_deco[i] as usize] != NONE {
                deco_line[path_deco[i] as usize]
            } else {
                nearest_line((d.bbox.y0 + d.bbox.y1) as f32 / 2.0 / q)
            }
        } else {
            0
        };
    }
    path_line
}

/// What the style matcher knows of every path, indexed by path.
pub(crate) fn path_contexts(data: &PageData, path_word: &[u32], path_deco: &[u32], path_line: &[u32]) -> Vec<PathCtx> {
    let mut path_ctx = Vec::with_capacity(data.paths.len());
    let mut nth_in_word = 0u16;
    let mut nth_mark = 0u16;
    let mut named: HashMap<Mark, u16> = HashMap::new();
    let mut cur_word = NONE;
    for (i, p) in data.paths.iter().enumerate() {
        let wi = path_word[i];
        if wi != cur_word {
            cur_word = wi;
            nth_in_word = 0;
            nth_mark = 0;
            named.clear();
        }
        let di = path_deco[i];
        let (surah, ayah, line_number) = if wi != NONE {
            let w = &data.words[wi as usize];
            (w.surah, w.ayah, data.lines[w.line_index as usize].line_number)
        } else if di != NONE {
            let d = &data.decorations[di as usize];
            (d.surah, d.ayah, data.lines[path_line[i] as usize].line_number)
        } else {
            (0, 0, 0)
        };
        let nm = *named.entry(p.mark).and_modify(|v| *v += 1).or_insert(0);
        path_ctx.push(PathCtx {
            word: wi,
            decoration: di,
            line_number,
            surah,
            ayah,
            kind: p.kind,
            mark: p.mark,
            family: p.family,
            category: p.mark.category(),
            nth_in_word,
            nth_mark: if p.kind == PathKind::Mark { nth_mark } else { u16::MAX },
            nth_mark_named: nm,
            deco_kind: if di != NONE { data.decorations[di as usize].kind } else { DecoKind::Other },
        });
        if wi != NONE {
            nth_in_word += 1;
            if p.kind == PathKind::Mark {
                nth_mark += 1;
            }
        }
    }
    path_ctx
}

/// Flatten every path's outline into page units, with the table that indexes it.
///
/// A page's viewBox is what the page is. The artwork may carry ink beyond it — p17 of the
/// Hafs KFGQPC mushaf draws its page number and its running head outside the box, and it is
/// the only page that does — and the format keeps that ink rather than drop it, so a crop or
/// an SVG export still has every stroke the artwork drew. Drawing is the other question: an
/// SVG viewport clips to the viewBox, and ink outside it is not part of the page, so the
/// geometry gives that ink no outline to draw. Its record stays, and so does its box, its
/// kind and its place in the path numbering every other call is indexed by.
pub(crate) fn flatten(data: &PageData, path_word: &[u32], path_line: &[u32], path_ctx: &[PathCtx]) -> Geometry {
    let q = data.header.quant as f32;
    let mut geom = Geometry::default();
    let on_page = |bb: &IBox| -> bool {
        let (w, h) = (data.header.width, data.header.height);
        (bb.x1 as f32 / q) > 0.0 && (bb.y1 as f32 / q) > 0.0 && (bb.x0 as f32 / q) < w && (bb.y0 as f32 / q) < h
    };
    for (i, p) in data.paths.iter().enumerate() {
        let op_start = geom.ops.len() as u32;
        let pt_start = geom.pts.len() as u32;
        let mut push = |x: f32, y: f32| {
            geom.pts.push(x);
            geom.pts.push(y);
        };
        if !on_page(&p.bbox) {
            // no outline: every host draws it as nothing, without knowing to ask
        } else if let Some(inst) = data.path_inst(i) {
            let m = |x: i32, y: i32| -> (f32, f32) {
                let (gx, gy) = (x as f64 / q as f64, y as f64 / q as f64);
                (
                    (inst.a as f64 * gx + inst.c as f64 * gy + inst.e as f64) as f32,
                    (inst.b as f64 * gx + inst.d as f64 * gy + inst.f as f64) as f32,
                )
            };
            for c in data.glyph_cmds(inst.glyph as usize).unwrap_or_default() {
                emit(&mut geom.ops, &mut push, c, m);
            }
        } else {
            let m = |x: i32, y: i32| -> (f32, f32) { (x as f32 / q, y as f32 / q) };
            for c in data.path_cmds(i).unwrap_or_default() {
                emit(&mut geom.ops, &mut push, c, m);
            }
        }
        let evenodd = (p.flags & PF_EVENODD != 0) as u32;
        let inst = (p.flags & PF_GLYPH != 0) as u32;
        let c = &path_ctx[i];
        geom.table.push(PathGeom {
            op_start,
            op_count: geom.ops.len() as u32 - op_start,
            pt_start,
            pt_count: geom.pts.len() as u32 - pt_start,
            flags: p.kind as u32 | (p.mark as u32) << 8 | (p.family as u32) << 16 | (evenodd | inst << 1) << 24,
            word: path_word[i],
            line: path_line[i],
            extra: c.category as u32
                | (c.nth_in_word as u32 & 0xff) << 8
                | ((if c.nth_mark == u16::MAX { 0xff } else { c.nth_mark as u32 }) & 0xff) << 16,
        });
    }
    geom
}

/// The words of each line sorted by `bbox.x0` ascending, as (x0, word index).
pub(crate) fn line_words(data: &PageData) -> Vec<Vec<(i32, u32)>> {
    let mut line_words = Vec::with_capacity(data.lines.len());
    for l in &data.lines {
        let mut v: Vec<(i32, u32)> =
            (l.first_word..l.first_word + l.n_words).map(|wi| (data.words[wi as usize].bbox.x0, wi as u32)).collect();
        v.sort_unstable();
        line_words.push(v);
    }
    line_words
}

/// The horizontal extent of each word's letters in page units, without its marks: 2 words in 3
/// carry ink outside them.
pub(crate) fn word_bodies(data: &PageData) -> Vec<(f32, f32)> {
    let q = data.header.quant as f32;
    data.words
        .iter()
        .map(|w| {
            let (mut x0, mut x1) = (f32::INFINITY, f32::NEG_INFINITY);
            for pi in w.first_path..w.first_path + w.n_paths as u32 {
                if data.paths[pi as usize].kind != PathKind::Body {
                    continue;
                }
                let b = &data.paths[pi as usize].bbox;
                x0 = x0.min(b.x0 as f32 / q);
                x1 = x1.max(b.x1 as f32 / q);
            }
            if x0 < x1 {
                (x0, x1)
            } else {
                (w.bbox.x0 as f32 / q, w.bbox.x1 as f32 / q)
            }
        })
        .collect()
}

/// The baseline of every line: the height most of its words end on. A line without words
/// takes its centre.
pub(crate) fn line_baselines(data: &PageData, line_words: &[Vec<(i32, u32)>], line_centre: &[f32]) -> Vec<f32> {
    let q = data.header.quant as f32;
    (0..data.lines.len())
        .map(|li| {
            let mut bottoms: Vec<f32> =
                line_words[li].iter().map(|&(_, wi)| data.words[wi as usize].bbox.y1 as f32 / q).collect();
            if bottoms.is_empty() {
                return line_centre[li];
            }
            bottoms.sort_by(f32::total_cmp);
            bottoms[bottoms.len() / 2]
        })
        .collect()
}

fn emit(ops: &mut Vec<u8>, push: &mut impl FnMut(f32, f32), c: Cmd, m: impl Fn(i32, i32) -> (f32, f32)) {
    match c {
        Cmd::MoveTo(x, y) => {
            ops.push(OP_MOVE);
            let (x, y) = m(x, y);
            push(x, y);
        }
        Cmd::LineTo(x, y) => {
            ops.push(OP_LINE);
            let (x, y) = m(x, y);
            push(x, y);
        }
        Cmd::QuadTo(x1, y1, x, y) => {
            ops.push(OP_QUAD);
            let (a, b) = m(x1, y1);
            push(a, b);
            let (x, y) = m(x, y);
            push(x, y);
        }
        Cmd::CubicTo(x1, y1, x2, y2, x, y) => {
            ops.push(OP_CUBIC);
            let (a, b) = m(x1, y1);
            push(a, b);
            let (a, b) = m(x2, y2);
            push(a, b);
            let (x, y) = m(x, y);
            push(x, y);
        }
        Cmd::Close => ops.push(OP_CLOSE),
    }
}
