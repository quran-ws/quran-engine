//! The sajdah line: the stroke the print draws over the words of a sajdah, and the sign beside it.
use super::{Placement, Reflowed, Repeat, NONE};
use crate::Page;
use qvp_format::{IBox, Mark, PageData};

/// A sajdah line stroke the reflow draws over its words: the decoration that holds it, the
/// stroke's own paths, its printed horizontal extent in page units, and the words under it.
pub(super) struct SajdahLine {
    pub(super) deco: u32,
    pub(super) paths: Vec<u32>,
    pub(super) x0: f32,
    pub(super) x1: f32,
    pub(super) words: Vec<u32>,
}

/// The line a sajdah line stroke is drawn over: the line of the nearest words below it that it
/// overlaps. `b` is the box of the stroke's own ink. `None` when no word sits under it.
pub(crate) fn line_under_stroke(d: &PageData, b: &IBox) -> Option<u16> {
    let tol = (2.0 * d.header.quant as f32) as i32;
    d.words
        .iter()
        .filter(|w| w.bbox.x0 < b.x1 && w.bbox.x1 > b.x0 && w.bbox.y0 >= b.y0 - tol)
        .min_by_key(|w| w.bbox.y0 - b.y1)
        .map(|w| w.line_index)
}

impl Page {
    /// The words a sajdah line stroke is drawn over: the ones it spans horizontally on the
    /// line directly below it. `b` is the box of the stroke's own ink, not the decoration's,
    /// which can also hold the sign printed in the margin.
    pub(crate) fn words_under_stroke(&self, b: &IBox) -> Vec<u32> {
        let d = self.data();
        let Some(li) = line_under_stroke(d, b) else { return vec![] };
        d.words
            .iter()
            .enumerate()
            .filter(|(_, w)| w.line_index == li && w.bbox.x0 < b.x1 && w.bbox.x1 > b.x0)
            .map(|(i, _)| i as u32)
            .collect()
    }

    /// Which decorations hold a sajdah line, the stroke drawn over a span of words.
    ///
    /// A decoration can hold more than one job: the sajdah of 16:50 is one decoration holding
    /// the stroke over its words *and* the sign printed in the margin. They are placed apart,
    /// so the flags are read per path, not per decoration.
    pub(crate) fn sajdah_line_decos(&self) -> Vec<bool> {
        let d = self.data();
        d.decorations
            .iter()
            .map(|deco| {
                (deco.first_path..deco.first_path + deco.n_paths as u32)
                    .any(|pi| d.paths[pi as usize].mark == Mark::SajdahLine)
            })
            .collect()
    }

    /// The ink of a decoration apart from its sajdah line: the sign in the margin, and
    /// anything else drawn with it. The decoration's own box holds both and is no use for
    /// placing either.
    pub(crate) fn deco_mark_bbox(&self, di: usize) -> IBox {
        self.paths_bbox(self.deco_other_paths(di)).unwrap_or(self.data().decorations[di].bbox)
    }

    /// The box around the ink of a set of paths, or `None` for no paths.
    pub(super) fn paths_bbox(&self, paths: impl IntoIterator<Item = u32>) -> Option<IBox> {
        let d = self.data();
        let mut paths = paths.into_iter().peekable();
        paths.peek()?;
        let mut bbox = IBox::EMPTY;
        for pi in paths {
            bbox.union(&d.paths[pi as usize].bbox);
        }
        Some(bbox)
    }

    /// The paths of a decoration that are not its sajdah line: the sign in the margin, and
    /// anything else drawn with it.
    pub(crate) fn deco_other_paths(&self, di: usize) -> Vec<u32> {
        let d = self.data();
        let deco = &d.decorations[di];
        (deco.first_path..deco.first_path + deco.n_paths as u32)
            .filter(|&pi| d.paths[pi as usize].mark != Mark::SajdahLine)
            .collect()
    }

    /// Place every sajdah line over the words it marks, wherever the rows put them. The span
    /// can break across rows, so the stroke is drawn once per row, stretched along x to cover
    /// that row's part of it. Only this stroke is stretched; the words are not.
    pub(super) fn place_sajdah_lines(&self, out: &mut Reflowed) {
        let q = self.quant();
        let d = self.data();
        for line in &self.reflow_prepared().sajdah_lines {
            let (di, words) = (line.deco as usize, &line.words);
            let (bx0, bx1) = (line.x0, line.x1);
            let mut by_row: Vec<(u32, f32, f32)> = Vec::new(); // (row, x0, x1) of the placed span
            for &wi in words {
                let r = out.word_row[wi as usize];
                if r == NONE {
                    continue;
                }
                let p = out.word_place[wi as usize];
                let w = &d.words[wi as usize];
                let (x0, x1) = (p.apply(w.bbox.x0 as f32 / q, 0.0).0, p.apply(w.bbox.x1 as f32 / q, 0.0).0);
                match by_row.iter_mut().find(|(rr, _, _)| *rr == r) {
                    Some(e) => {
                        e.1 = e.1.min(x0);
                        e.2 = e.2.max(x1);
                    }
                    None => by_row.push((r, x0, x1)),
                }
            }
            for (n, &(r, sx0, sx1)) in by_row.iter().enumerate() {
                let kx = if sx1 > sx0 { (sx1 - sx0) / (bx1 - bx0) } else { 1.0 };
                // the stroke takes the same shift as the words it is drawn over, so it keeps
                // the height above them the print gave it. They all came from one printed line,
                // so they all carry the same shift.
                let dy = words
                    .iter()
                    .find(|&&wi| out.word_row[wi as usize] == r)
                    .map(|&wi| out.word_place[wi as usize].dy)
                    .unwrap_or(0.0);
                let p = Placement { dx: sx0 - kx * bx0, dy, kx, ky: 1.0 };
                if n == 0 {
                    out.sajdah_line_place[di] = p;
                } else {
                    // only the stroke is drawn again; the sign beside it is printed once
                    for &pi in &line.paths {
                        out.repeats.push(Repeat { decoration: di as u32, first_path: pi, n_paths: 1, placement: p });
                    }
                }
            }
        }
    }
}
