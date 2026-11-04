//! Banner lines: a surah name or a basmalah, which keeps a row of its own on a reflowed page.
use super::{Placement, Reflowed};
use crate::Page;
use qvp_format::{DecoKind, IBox, PathKind, NONE_U16};

impl Page {
    /// Native frame paths of a surah name. The printed page draws them; a reflowed page omits
    /// them so the title can grow with the reader instead of being fitted inside the frame.
    pub(crate) fn surah_name_ornament_paths(&self) -> Vec<u32> {
        let d = self.data();
        let mut paths: Vec<u32> = d
            .decorations
            .iter()
            .filter(|deco| deco.kind == DecoKind::SurahName)
            .flat_map(|deco| deco.first_path..deco.first_path + deco.n_paths as u32)
            .filter(|&pi| d.paths[pi as usize].kind == PathKind::Ornament)
            .collect();
        paths.sort_unstable();
        paths.dedup();
        paths
    }

    /// A decoration's box after paths this layout does not draw are removed.
    pub(crate) fn decoration_bbox_without(&self, di: usize, omitted: &[u32]) -> IBox {
        let d = self.data();
        let deco = &d.decorations[di];
        if omitted.is_empty() {
            return deco.bbox;
        }
        let kept =
            (deco.first_path..deco.first_path + deco.n_paths as u32).filter(|pi| omitted.binary_search(pi).is_err());
        self.paths_bbox(kept).unwrap_or(IBox::EMPTY)
    }

    /// The box of a banner line's own ink: a banner line holds no words, so its ink is the
    /// decorations drawn on it.
    pub(super) fn header_bbox(&self, li: usize, omitted: &[u32]) -> IBox {
        let mut bbox = IBox::EMPTY;
        for (di, _) in self
            .data()
            .decorations
            .iter()
            .enumerate()
            .filter(|(_, deco)| deco.line != NONE_U16 && deco.line as usize == li)
        {
            bbox.union(&self.decoration_bbox_without(di, omitted));
        }
        bbox
    }

    /// The horizontal extent of a banner line's own ink, in page units.
    pub(super) fn header_ink(&self, li: usize, omitted: &[u32]) -> (f32, f32) {
        let bbox = self.header_bbox(li, omitted);
        let q = self.quant();
        if bbox.is_empty() {
            (0.0, 0.0)
        } else {
            (bbox.x0 as f32 / q, bbox.x1 as f32 / q)
        }
    }

    /// How far a banner is shrunk to sit on a row: its own ink against the row, never enlarged.
    /// A title or basmalah stays whole: it is fitted, not broken.
    ///
    /// The ink is what is measured, not the text block the banner sits in. A surah name is a
    /// fifth of the block wide and a basmalah about half, so both keep their printed size as
    /// the reader zooms in — growing with the words around them, which is what the reader
    /// asked for — and start to shrink only once the row itself is narrower than they are.
    ///
    /// `max_k` is [`crate::LayoutSpec::banner_zoom`] against the zoom, already worked out; it is
    /// infinite when the host left the growth uncapped.
    pub(super) fn header_scale(&self, li: usize, row_w: f32, max_k: f32, omitted: &[u32]) -> f32 {
        let ink = self.header_ink(li, omitted);
        (row_w / (ink.1 - ink.0).max(1.0)).min(1.0).min(max_k)
    }

    /// The air the print keeps above and below a banner line, in page units: from its own ink
    /// to the nearest ink on the line above and the line below. A banner line holds no words,
    /// so its ink is the decorations drawn on it.
    pub(super) fn header_air(&self, li: usize, omitted: &[u32]) -> (f32, f32) {
        let d = self.data();
        let q = self.quant();
        let ink = |line: usize| -> Option<(f32, f32)> {
            let mut bbox = if d.lines[line].n_words > 0 { d.lines[line].bbox } else { IBox::EMPTY };
            bbox.union(&self.header_bbox(line, omitted));
            (!bbox.is_empty()).then_some((bbox.y0 as f32 / q, bbox.y1 as f32 / q))
        };
        let Some(mine) = ink(li) else { return (0.0, 0.0) };
        let above = li.checked_sub(1).and_then(ink).map(|(_, y1)| (mine.0 - y1).max(0.0)).unwrap_or(0.0);
        let below = ink(li + 1).map(|(y0, _)| (y0 - mine.1).max(0.0)).unwrap_or(0.0);
        (above, below)
    }

    /// A banner line keeps its printed drawing, shrunk to the row width and centred on it.
    pub(super) fn place_header(
        &self,
        out: &mut Reflowed,
        li: u32,
        centre: f32,
        row_w: f32,
        max_k: f32,
        omitted: &[u32],
    ) {
        let d = self.data();
        let k = self.header_scale(li as usize, row_w, max_k, omitted);
        let ink = self.header_ink(li as usize, omitted);
        let cx = (ink.0 + ink.1) / 2.0;
        for (di, deco) in d.decorations.iter().enumerate() {
            if deco.line != NONE_U16 && deco.line as u32 == li {
                let c = self.line_centre(li as usize);
                out.deco_place[di] = Placement::scaled(row_w / 2.0 - k * cx, centre - k * c, k);
            }
        }
    }
}
