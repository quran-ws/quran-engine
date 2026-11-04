//! Reflow: keep the page width, break the words onto other lines.
//!
//! The printed layout ([`crate::layout`]) fits the page to the viewport width and moves
//! whole lines. Reflow answers a different question: the reader wants bigger ink and the
//! same page width, so fewer words fit on a row and the rest move down. The page keeps its
//! width and grows taller; the host scrolls it.
//!
//! What reflow never does: reshape a word (every outline is placed as printed, whole),
//! split an ayah from its medallion, move a word to another page, or leave a sajdah line
//! or header title behind. A surah name or basmalah keeps a row of its own, and text never
//! flows across it. The printed frame around a surah name belongs to the printed layout only.
use crate::Page;
use qvp_format::{DecoKind, Mark};
use std::collections::HashMap;
use std::sync::Arc;

mod atoms;
mod banner;
mod breaks;
mod horizontal;
mod sajdah;
mod vertical;

use atoms::{Atom, Neighbour};
use sajdah::SajdahLine;

pub(crate) use sajdah::line_under_stroke;

const NONE: u32 = u32::MAX;

/// How the gap between two words on a reflowed row is picked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum GapMode {
    /// The gap the print puts between those two words, and the page's median where the print
    /// never set them together. The print justifies each line on its own, so a row drawn from
    /// two printed lines carries two different spacings.
    Printed = 0,
    /// The page's median gap between every pair, measured between the words' letters, so the
    /// spacing reads the same across a row wherever its words came from.
    Uniform = 1,
}

/// How the words are broken onto rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Breaks {
    /// Fill each row until the next word does not fit. What the print does, and what leaves one
    /// row full and the next half empty when a long word falls at a boundary.
    Greedy = 0,
    /// Choose the breaks that leave the rows of a block as even as they can be, by weighing how
    /// much every row of the block is left short rather than only the row in hand.
    Even = 1,
    /// Weigh a row by the width it can reach once its gaps have opened as far as they may,
    /// not by the width its words happen to take, and charge for the difference between one
    /// row and the row above it. A row whose shortfall its gaps can close is cheap; a row of
    /// three long words, whose two gaps can close almost nothing, is not, and the breaks move
    /// instead.
    Fitted = 2,
    /// Keep each printed line on one row and clamp the zoom so the widest row fits.
    Printed = 3,
}

/// How a reflowed row fills the width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Fill {
    /// Words keep their gap and the row starts at the right margin, so it ends where it ends.
    Ragged = 0,
    /// Gaps stretch so the row spans the full width. A row that cannot, the last of a block or
    /// one its gaps would have to stretch past [`ReflowSpec::max_stretch`], is centred.
    Justified = 1,
    /// Words keep their gap and the row is centred: what is left over is split between the
    /// two margins. This is what a reflowed page does unless a host asks otherwise.
    Centred = 2,
}

/// Ask [`crate::LayoutSpec`] for a reflowed page.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ReflowSpec {
    /// How much bigger the ink is than fit-to-width. 1.0 reflows at the printed size
    /// (which reproduces the print's rows on a page whose lines already fill the width);
    /// 2.0 gives ink twice as tall and rows that hold about half as much.
    pub zoom: f32,
    pub fill: Fill,
    pub breaks: Breaks,
    pub gaps: GapMode,
    /// Multiplier on every gap (1.0 = the gap `gaps` picked).
    pub word_gap: f32,
    /// How far a row is opened towards the row beside it when it comes out much shorter, from
    /// 0 (left as it is) to 1 (opened all the way to its neighbour's width). The row is never
    /// justified by this, and [`ReflowSpec::max_stretch`] still caps how far its gaps may go.
    pub relax: f32,
    /// How far a gap may stretch under [`Fill::Justified`], as a multiple of the gap the row
    /// started with. A row that would need more than this opens as far as the cap allows and is
    /// centred, so a short row is never gapped out to the margins. 0 or below means no cap.
    pub max_stretch: f32,
}

impl Default for ReflowSpec {
    fn default() -> Self {
        ReflowSpec {
            zoom: 1.0,
            fill: Fill::Centred,
            breaks: Breaks::Fitted,
            gaps: GapMode::Uniform,
            word_gap: 1.0,
            relax: crate::defaults::REFLOW_RELAX,
            max_stretch: crate::defaults::REFLOW_MAX_STRETCH,
        }
    }
}

/// Where one group of paths is placed: a point `p` in page units is drawn at
/// `(kx·p.x + dx, ky·p.y + dy)`, still in page units, before the layout's scale and offset.
///
/// A word is always placed with `kx == ky == 1`: it moves, it is never resized or reshaped.
/// The two scales exist for the page's furniture and for the sajdah line, which is stretched
/// along x to cover the words it marks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Placement {
    pub dx: f32,
    pub dy: f32,
    pub kx: f32,
    pub ky: f32,
}

impl Placement {
    pub const IDENTITY: Placement = Placement { dx: 0.0, dy: 0.0, kx: 1.0, ky: 1.0 };
    pub fn shifted(dy: f32) -> Placement {
        Placement { dx: 0.0, dy, kx: 1.0, ky: 1.0 }
    }
    /// A move with no resizing.
    pub fn moved(dx: f32, dy: f32) -> Placement {
        Placement { dx, dy, kx: 1.0, ky: 1.0 }
    }
    /// Uniformly scaled by `k`, then moved.
    pub fn scaled(dx: f32, dy: f32, k: f32) -> Placement {
        Placement { dx, dy, kx: k, ky: k }
    }
    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (self.kx * x + self.dx, self.ky * y + self.dy)
    }
    /// The page-unit point that lands on `(x, y)`.
    pub fn invert(&self, x: f32, y: f32) -> (f32, f32) {
        ((x - self.dx) / self.kx, (y - self.dy) / self.ky)
    }
}

/// One more drawing of a decoration's paths, at another placement. The sajdah line uses it:
/// the words it marks can end up on two rows, and the stroke is then drawn over each of them.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Repeat {
    pub decoration: u32,
    pub first_path: u32,
    pub n_paths: u32,
    pub placement: Placement,
}

/// The rows a reflow produced, and where every word landed.
#[derive(Clone, Debug, PartialEq)]
pub struct Reflowed {
    /// Words of each row, in reading order (right to left).
    pub row_words: Vec<Vec<u32>>,
    /// The baseline every word of the row sits on, in page units.
    pub row_baseline: Vec<f32>,
    /// Vertical band of each row in page units: (top, bottom), meeting its neighbours.
    pub row_band: Vec<(f32, f32)>,
    /// Placement of every word of the page, indexed by word.
    pub word_place: Vec<Placement>,
    /// Row each word landed on, indexed by word.
    pub word_row: Vec<u32>,
    /// Placement of every decoration, indexed by decoration. A decoration's sajdah line is not
    /// placed by this: the stroke goes over the words it marks, wherever they are, so it has
    /// [`Reflowed::sajdah_line_place`] of its own.
    pub deco_place: Vec<Placement>,
    /// Placement of a decoration's sajdah line stroke, indexed by decoration.
    pub sajdah_line_place: Vec<Placement>,
    /// Extra drawings of a decoration, one per further row its words occupy.
    pub repeats: Vec<Repeat>,
    /// Width of a row in page units.
    /// True when the rows came out as the printed lines, so the page was laid out as printed.
    pub as_printed: bool,
    /// Decorations this layout does not draw: the sheet's own furniture, printed outside the
    /// page box (running head, page number).
    pub omitted: Vec<u32>,
    pub row_w: f32,
    /// The floor on the distance between two baselines, in page units: the printed line
    /// spacing. A row whose ink needs more room gets more.
    pub pitch: f32,
    /// Vertical extent of everything placed, page units: the rows plus the running head and
    /// the page number, which sit on rows of their own above and below the text.
    pub y0: f32,
    pub y1: f32,
}

/// The shape of the rows a reflow fills: the page's text block and the margins either side of
/// it, the width and pitch of a row, where the first one starts, and whether the reader asked
/// for no more leading than the print has.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct RowSpec {
    pub block: (f32, f32),
    pub margins: (f32, f32),
    pub row_w: f32,
    pub pitch: f32,
    pub top: f32,
    pub printed_spacing: bool,
    /// How many rows the reader sees at once. Rows are evened out against the others of their
    /// own screenful, because that is what a reader can compare. 0 weighs the whole page.
    pub rows_per_view: usize,
    /// The most a banner may be drawn at, as a share of its printed drawing — the layout's
    /// own [`crate::LayoutSpec::banner_zoom`] against the zoom it is reflowing at, worked out
    /// once. Infinite leaves the banner growing with the page.
    pub banner_k: f32,
    /// Whether a row that still reproduces the printed page keeps a native surah frame.
    pub surah_frames: bool,
}

/// What a reflow reads off the page alone, measured once per page. A continuous pinch lays
/// the page out again on every frame, and nothing here changes between two frames: the gap
/// the print keeps between words, the row unit of every word, how each word meets the one
/// before it, which strokes are sajdah lines, the native surah frames, the text block and the
/// group every path is drawn under.
pub(crate) struct Prepared {
    /// The page's median word gap, [`Page::median_word_gap`].
    median: f32,
    /// The printed text block, [`Page::text_block`].
    pub block: (f32, f32),
    /// The row unit of every word, indexed by word.
    atoms: Vec<Atom>,
    /// Every word against the word before it in reading order, indexed by word.
    before: Vec<Neighbour>,
    /// The widest row unit once the decorations that carry a sajdah line are left out: what
    /// bounds the zoom under every break strategy but [`Breaks::Printed`].
    widest_without_sajdah_line: f32,
    /// The sajdah lines, in decoration order.
    sajdah_lines: Vec<SajdahLine>,
    /// The sheet's own furniture, which a reflowed page leaves out: the running head and the
    /// page number, as decoration indices.
    omitted: Vec<u32>,
    /// The native frame paths of every surah name, sorted. [`Page::surah_name_ornament_paths`].
    pub surah_frame_paths: Vec<u32>,
    /// The group each path is drawn under on a reflowed page: its word, or its decoration,
    /// or its decoration's sajdah line. Shared with every [`crate::Layout`] made from it.
    pub path_group: Arc<[u32]>,
}

/// Measure a page for reflow. See [`Prepared`].
pub(crate) fn prepare(page: &Page) -> Prepared {
    let d = page.data();
    let median = page.median_word_gap();
    let block = page.text_block();
    let anchors = page.deco_anchor_words(median);
    let sajdah_line_decos = page.sajdah_line_decos();
    let mut word_decos: Vec<Vec<u32>> = vec![Vec::new(); d.words.len()];
    for (di, &wi) in anchors.iter().enumerate() {
        if wi != NONE {
            word_decos[wi as usize].push(di as u32);
        }
    }
    let atoms: Vec<Atom> =
        (0..d.words.len() as u32).map(|wi| page.atom(wi, &word_decos[wi as usize], median, block)).collect();
    // reading order: each line right to left, line after line
    let mut before = vec![Neighbour { prev: NONE, interlock: None, clearance: None }; d.words.len()];
    let mut prev = NONE;
    for words in page.line_words.iter() {
        for &(_, wi) in words.iter().rev() {
            if prev != NONE {
                let (a, b) = (&atoms[prev as usize], &atoms[wi as usize]);
                before[wi as usize] = Neighbour {
                    prev,
                    interlock: page.words_interlock(prev, wi),
                    clearance: Page::slice_clearance(&a.slices, &b.slices, 0.0),
                };
            }
            prev = wi;
        }
    }
    // The sajdah line is drawn over a span of words, not over one, so it is not what bounds
    // the zoom. A word whose decorations all stay keeps the unit measured above; one that
    // loses a decoration is measured again, extent only, since no gap reads its bands.
    let widest_without_sajdah_line = (0..d.words.len())
        .map(|wi| {
            let kept: Vec<u32> = word_decos[wi].iter().copied().filter(|&di| !sajdah_line_decos[di as usize]).collect();
            if kept.len() == word_decos[wi].len() {
                atoms[wi].width
            } else {
                let (x0, x1, _, _) = page.atom_extent(wi as u32, &kept, median, block);
                x1 - x0
            }
        })
        .fold(0.0f32, f32::max);
    let sajdah_lines = (0..d.decorations.len())
        .filter(|&di| sajdah_line_decos[di])
        .filter_map(|di| {
            let deco = &d.decorations[di];
            // the stroke's own ink, not the decoration's, which also holds the sign in the margin
            let paths: Vec<u32> = (deco.first_path..deco.first_path + deco.n_paths as u32)
                .filter(|&pi| d.paths[pi as usize].mark == Mark::SajdahLine)
                .collect();
            let b = page.paths_bbox(paths.iter().copied())?;
            let q = page.quant();
            let (x0, x1) = (b.x0 as f32 / q, b.x1 as f32 / q);
            (x1 > x0).then(|| SajdahLine { deco: di as u32, paths, x0, x1, words: page.words_under_stroke(&b) })
        })
        .collect();
    let omitted = d
        .decorations
        .iter()
        .enumerate()
        .filter(|(_, deco)| matches!(deco.kind, DecoKind::PageNumber | DecoKind::RunningHead))
        .map(|(di, _)| di as u32)
        .collect();
    // one group per word, then one per decoration, then one per decoration's sajdah line:
    // a path's group follows the job it does, not only the record it belongs to
    let (n_words, n_decos) = (d.words.len() as u32, d.decorations.len() as u32);
    let path_group: Arc<[u32]> = (0..d.paths.len())
        .map(|i| {
            let g = &page.geometry().table[i];
            if g.word != NONE {
                g.word
            } else if page.path_deco(i as u32) != NONE {
                let di = page.path_deco(i as u32);
                if d.paths[i].mark == Mark::SajdahLine {
                    n_words + n_decos + di
                } else {
                    n_words + di
                }
            } else {
                0
            }
        })
        .collect();
    Prepared {
        median,
        block,
        atoms,
        before,
        widest_without_sajdah_line,
        sajdah_lines,
        omitted,
        surah_frame_paths: page.surah_name_ornament_paths(),
        path_group,
    }
}

impl Page {
    /// The largest `zoom` at which every word of this page still fits a row, so the reflow
    /// never has to place ink wider than the row it sits on. A host clamps its pinch to this.
    ///
    /// The sajdah line is left out of the measure: it is drawn over a span of words, not over
    /// one, so it is not what bounds the zoom.
    ///
    /// The atoms are measured the way [`Page::layout`] builds them for a reflowed page, with the page's own median
    /// gap as the tolerance that sorts a mark into the row or the margin. `word_gap` is a
    /// multiplier on the gaps *between* atoms; it does not change what an atom holds.
    /// With [`Breaks::Printed`], the widest printed row, including the requested gaps, bounds
    /// the zoom against the page width instead of the text block.
    pub fn reflow_max_zoom(&self, spec: &ReflowSpec) -> f32 {
        let prep = self.reflow_prepared();
        let (median, block) = (prep.median, prep.block);
        if spec.breaks == Breaks::Printed {
            let widest = self
                .line_words
                .iter()
                .map(|words| {
                    let atoms: Vec<&Atom> = words.iter().rev().map(|&(_, wi)| &prep.atoms[wi as usize]).collect();
                    atoms.iter().map(|a| a.width).sum::<f32>()
                        + atoms.windows(2).map(|p| self.atom_gap(p[0], p[1], spec, median)).sum::<f32>()
                })
                .fold(0.0f32, f32::max);
            return if widest > 0.0 { self.width() / widest } else { f32::INFINITY };
        }
        let widest = prep.widest_without_sajdah_line;
        if widest > 0.0 {
            ((block.1 - block.0) / widest).max(1.0)
        } else {
            f32::INFINITY
        }
    }

    /// Break the page's words into rows `row_w` page units wide and place them.
    ///
    /// `pitch` is the distance between row centres and `top` the centre of the first row,
    /// both in page units. Rows are filled right to left in reading order; a banner line
    /// (surah name, basmalah) keeps a row to itself and text never flows across it.
    /// The printed text block: the leftmost and rightmost ink of the page's lines, in page
    /// units. The sheet's side margins are what is left over.
    pub(crate) fn text_block(&self) -> (f32, f32) {
        let q = self.quant();
        let d = self.data();
        let (mut x0, mut x1) = (f32::INFINITY, f32::NEG_INFINITY);
        for l in d.lines.iter().filter(|l| l.n_words > 0) {
            x0 = x0.min(l.bbox.x0 as f32 / q);
            x1 = x1.max(l.bbox.x1 as f32 / q);
        }
        if x0.is_finite() && x1 > x0 {
            (x0, x1)
        } else {
            (0.0, self.width())
        }
    }

    pub(crate) fn reflow(&self, spec: &ReflowSpec, shape: &RowSpec) -> Reflowed {
        let RowSpec { block, row_w, pitch, printed_spacing, rows_per_view, surah_frames, .. } = *shape;
        let d = self.data();
        let prep = self.reflow_prepared();
        let (rows, headers) = self.break_rows(spec, row_w, block);

        // place every row right to left
        let mut out = Reflowed {
            row_words: Vec::with_capacity(rows.len()),
            row_baseline: Vec::with_capacity(rows.len()),
            row_band: Vec::with_capacity(rows.len()),
            word_place: vec![Placement::IDENTITY; d.words.len()],
            word_row: vec![NONE; d.words.len()],
            deco_place: vec![Placement::IDENTITY; d.decorations.len()],
            sajdah_line_place: vec![Placement::IDENTITY; d.decorations.len()],
            repeats: Vec::new(),
            as_printed: false,
            omitted: Vec::new(),
            row_w,
            pitch,
            y0: 0.0,
            y1: 0.0,
        };
        let as_printed = self.rows_as_printed(&rows, &headers, spec);
        // The printed line positions come back only when the reader asked for no more leading
        // than the print has. A reader opening the lines up wants that on a reproduced page
        // too, so the rows then take the even rhythm with the spacing asked for.
        let printed_y = printed_spacing.then_some(()).and(as_printed.clone());
        // The native surah frame belongs to the printed page. Once the page has truly reflowed,
        // the title grows with the reader on its own; the frame neither draws nor constrains it.
        let omitted_header_paths: &[u32] =
            if printed_y.is_some() && surah_frames { &[] } else { &prep.surah_frame_paths };
        let _ = rows_per_view;
        // Horizontal placement first: it does not depend on the heights, and the heights need
        // to know which ink of two rows ends up over which.
        let row_dx = self.place_across(&rows, &headers, spec, shape, as_printed.is_some());
        let heights = self.place_down(&rows, &headers, &row_dx, shape, printed_y.as_deref(), omitted_header_paths);
        let (centres, bands) = self.row_bands(&headers, &heights, pitch, printed_y.as_deref());
        out.row_band = bands;
        out.as_printed = printed_y.is_some();
        out.row_baseline = heights.baseline.clone();
        let baselines = &self.line_baseline;
        for (r, atoms) in rows.iter().enumerate() {
            let base = heights.baseline[r];
            out.row_words.push(atoms.iter().map(|a| a.word).collect());
            if let Some(&li) = headers.get(&r) {
                // on its own ink centre: a band beside a banner stops half a line spacing out,
                // so it is narrower than the banner and its middle is not where the ink goes
                self.place_header(&mut out, li, centres[r], row_w, shape.banner_k, omitted_header_paths);
                continue;
            }
            for (i, a) in atoms.iter().enumerate() {
                let dx = row_dx[r][i];
                // the word keeps its own height above or below the line it was printed on
                let dy = base - baselines[d.words[a.word as usize].line_index as usize];
                let p = Placement::moved(dx, dy);
                out.word_place[a.word as usize] = p;
                out.word_row[a.word as usize] = r as u32;
                // ink printed with the word keeps its printed offset from it
                for &di in &a.inline_decos {
                    out.deco_place[di as usize] = p;
                }
                for &(di, w) in &a.margin_decos {
                    out.deco_place[di as usize] = self.margin_mark_place(di, w, dy, shape);
                }
            }
        }
        self.centre_medallions(&mut out, &rows);
        self.place_sajdah_lines(&mut out);
        // The running head (surah name and juz) and the page number are printed outside the
        // page's own box: they belong to the sheet, not to the text, and a reflowed page has no
        // sheet. They are left out of the layout and listed in `omitted` so a renderer skips
        // them; a host that wants them draws its own, from `page_number` and the atlas.
        out.omitted.clone_from(&prep.omitted);
        self.fit_extent(&mut out, shape, printed_y.is_some(), omitted_header_paths);
        out
    }

    /// The printed line each row holds, when every row holds exactly the words of one printed
    /// line; `None` otherwise.
    ///
    /// The reflow has then reproduced the print: the page goes back to its printed line
    /// positions, irregular spacing and all, instead of the even rhythm the rows would
    /// otherwise take. This is what returning to zoom 1 gives back.
    fn rows_as_printed(
        &self,
        rows: &[Vec<&Atom>],
        headers: &HashMap<usize, u32>,
        spec: &ReflowSpec,
    ) -> Option<Vec<usize>> {
        let d = self.data();
        let mut lines: Vec<usize> = Vec::with_capacity(rows.len());
        let mut ok = true;
        for (r, atoms) in rows.iter().enumerate() {
            match headers.get(&r) {
                Some(&li) => lines.push(li as usize),
                None => {
                    let li = atoms.first().map(|a| d.words[a.word as usize].line_index as usize);
                    match li {
                        Some(li)
                            if atoms.len() == self.line_words[li].len()
                                && atoms.iter().all(|a| d.words[a.word as usize].line_index as usize == li) =>
                        {
                            lines.push(li)
                        }
                        _ => {
                            ok = false;
                            break;
                        }
                    }
                }
            }
        }
        (spec.breaks != Breaks::Printed && ok && lines.len() == rows.len()).then_some(lines)
    }

    /// Measure what was laid out, and move it so the topmost ink sits where the caller asked
    /// the first row to start. `printed_lines` is true when the rows keep the printed line
    /// positions.
    fn fit_extent(&self, out: &mut Reflowed, shape: &RowSpec, printed_lines: bool, omitted_header_paths: &[u32]) {
        let (top, pitch) = (shape.top, shape.pitch);
        let q = self.quant();
        let d = self.data();
        let (mut y0, mut y1) = (f32::INFINITY, f32::NEG_INFINITY);
        let mut grow = |p: &Placement, a: f32, b: f32| {
            y0 = y0.min(p.ky * a + p.dy);
            y1 = y1.max(p.ky * b + p.dy);
        };
        for (wi, p) in out.word_place.iter().enumerate() {
            if out.word_row[wi] != NONE {
                let b = &d.words[wi].bbox;
                grow(p, b.y0 as f32 / q, b.y1 as f32 / q);
            }
        }
        for (di, p) in out.deco_place.iter().enumerate() {
            if *p != Placement::IDENTITY && !out.omitted.contains(&(di as u32)) {
                let bbox = self.decoration_bbox_without(di, omitted_header_paths);
                if !bbox.is_empty() {
                    grow(p, bbox.y0 as f32 / q, bbox.y1 as f32 / q);
                }
            }
        }
        if y0.is_finite() {
            // the print reproduced: the page keeps its own origin, so its top margin survives
            let shift = if printed_lines { top - pitch / 2.0 } else { top - pitch / 2.0 - y0 };
            for p in out.word_place.iter_mut().chain(out.deco_place.iter_mut()).chain(out.sajdah_line_place.iter_mut())
            {
                p.dy += shift;
            }
            for r in out.repeats.iter_mut() {
                r.placement.dy += shift;
            }
            for b in out.row_baseline.iter_mut() {
                *b += shift;
            }
            for b in out.row_band.iter_mut() {
                b.0 += shift;
                b.1 += shift;
            }
            out.y0 = y0 + shift;
            out.y1 = y1 + shift;
        }
    }
}
