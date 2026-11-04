//! The sajdah line: the stroke the print draws over the words of a sajdah, and the sign beside it.
use crate::Page;
use qvp_format::{IBox, Mark};

/// A sajdah line stroke the reflow draws over its words: the decoration that holds it, the
/// stroke's own paths, its printed horizontal extent in page units, and the words under it.
pub(super) struct SajdahLine {
    pub(super) deco: u32,
    pub(super) paths: Vec<u32>,
    pub(super) x0: f32,
    pub(super) x1: f32,
    pub(super) words: Vec<u32>,
}

impl Page {
    /// The words a sajdah line stroke is drawn over: the ones it spans horizontally on the
    /// line directly below it. `b` is the box of the stroke's own ink, not the decoration's,
    /// which can also hold the sign printed in the margin.
    pub(crate) fn words_under_stroke(&self, b: &IBox) -> Vec<u32> {
        let q = self.quant();
        let d = self.data();
        let tol = (2.0 * q) as i32;
        // the line under the stroke: the nearest words below it that it overlaps
        let under = d
            .words
            .iter()
            .filter(|w| w.bbox.x0 < b.x1 && w.bbox.x1 > b.x0 && w.bbox.y0 >= b.y0 - tol)
            .min_by_key(|w| w.bbox.y0 - b.y1)
            .map(|w| w.line_index);
        let Some(li) = under else { return vec![] };
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
    pub(crate) fn deco_mark_bbox(&self, di: usize) -> qvp_format::IBox {
        let d = self.data();
        let mut acc: Option<qvp_format::IBox> = None;
        for pi in self.deco_other_paths(di) {
            let b = d.paths[pi as usize].bbox;
            acc = Some(match acc {
                None => b,
                Some(mut a) => {
                    a.union(&b);
                    a
                }
            });
        }
        acc.unwrap_or(d.decorations[di].bbox)
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
}
