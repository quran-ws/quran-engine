//! Reflow: keep the page width, break the words onto other lines.
//!
//! The printed layout ([`crate::layout`]) fits the page to the viewport width and moves
//! whole lines. Reflow answers a different question: the reader wants bigger ink and the
//! same page width, so fewer words fit on a row and the rest move down. The page keeps its
//! width and grows taller; the host scrolls it.
//!
//! What reflow never does: reshape a word (every outline is placed as printed, whole),
//! split an ayah from its medallion, move a word to another page, or leave a sajdah line
//! or a surah banner behind. A surah name or basmalah keeps a row of its own, and text
//! never flows across it.
use crate::Page;
use qvp_format::{DecoKind, Mark, NONE_U16};

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
}

/// How a reflowed row fills the width.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Fill {
    /// Words keep their gap and the row starts at the right margin, so it ends where it ends.
    Ragged = 0,
    /// Gaps stretch so the row spans the full width. The last row of a block stays ragged.
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
    /// started with. A row that would need more than this stays as it is, right-aligned, so a
    /// short row is never gapped out to the margins. 0 means no cap.
    pub max_stretch: f32,
}

impl Default for ReflowSpec {
    fn default() -> Self {
        ReflowSpec {
            zoom: 1.0,
            fill: Fill::Centred,
            breaks: Breaks::Even,
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
    /// [`Reflowed::sajdah_place`] of its own.
    pub deco_place: Vec<Placement>,
    /// Placement of a decoration's sajdah line stroke, indexed by decoration.
    pub sajdah_place: Vec<Placement>,
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
}

/// One unit that a row holds whole: a word, the ink printed with it (the medallion that
/// closes its ayah, the sajdah line drawn over it) and the marks the print puts in the
/// margin, which a reflowed row has no margin for.
struct Atom {
    word: u32,
    /// decorations that keep their printed offset from the word
    inline_decos: Vec<u32>,
    /// margin marks, placed after the word in reading order: (decoration, ink width)
    margin_decos: Vec<(u32, f32)>,
    /// printed extent of the word and its inline ink, in page units
    x0: f32,
    x1: f32,
    /// the atom's letters and inline marks sliced into bands: what the air is measured on
    slices: Vec<(i16, f32, f32)>,
    /// what the atom takes off a row: its printed extent plus room for the margin marks
    width: f32,
}

impl Page {
    /// The air the print keeps between the letters of two neighbouring words, taken over the
    /// whole page: what a reflowed row leaves between any two words.
    ///
    /// Measured as the narrowest distance between the two words' strokes
    /// ([`Page::words_clearance`]), not between their boxes. The calligraphy interlocks a
    /// word's opening stroke with the one before it — the boxes of `سَاحِرٌ` and `كَذَّابٌ` overlap
    /// by 12 page units while their strokes stay 5 apart — so a box measure would pull such a
    /// pair apart and leave a hole where the strokes used to interleave.
    pub(crate) fn median_word_gap(&self) -> f32 {
        let mut gaps: Vec<f32> = Vec::new();
        let d = self.data();
        for ws in self.line_words.iter() {
            for pair in ws.windows(2) {
                // line_words runs left to right, so the second of the pair is the earlier word
                let (a, b) = (pair[1].1, pair[0].1);
                // a pair with a medallion standing between them is not a word gap
                if d.words[a as usize].ayah_index != d.words[b as usize].ayah_index {
                    continue;
                }
                if let Some(g) = self.words_clearance(a, b, 0.0) {
                    if g > 0.0 {
                        gaps.push(g);
                    }
                }
            }
        }
        if gaps.is_empty() {
            return self.line_spacing() * 0.25;
        }
        gaps.sort_by(|a, b| a.partial_cmp(b).unwrap());
        gaps[gaps.len() / 2]
    }

    /// The decoration that closes each ayah, resolved to the word it must travel with.
    /// `NONE` for a decoration that belongs to no single word: a surah banner, a basmalah,
    /// the page number, the running head, or a sajdah line, which spans several words.
    pub(crate) fn deco_anchor_words(&self) -> Vec<u32> {
        let d = self.data();
        let median = self.median_word_gap();
        let mut out = vec![NONE; d.decorations.len()];
        // the medallion an ayah record names closes that record's last word, which is the
        // one the reflow must keep it with (an ayah split across lines has one record a line)
        for a in d.ayahs.iter() {
            if a.ayah_mark_decoration != NONE_U16 && a.n_words > 0 {
                let di = a.ayah_mark_decoration as usize;
                if di < out.len() {
                    out[di] = (a.first_word + a.n_words - 1) as u32;
                }
            }
        }
        for (di, deco) in d.decorations.iter().enumerate() {
            if out[di] == NONE {
                out[di] = match deco.kind {
                    // a sajdah mark closes the word it is printed after, so it travels with
                    // that word and no row can open with it
                    DecoKind::SajdahMark => self
                        .deco_other_paths(di)
                        .first()
                        .and_then(|&pi| self.word_before_path(pi, median))
                        .unwrap_or(NONE),
                    // a rub' al-hizb opens a division, so it travels with the word it opens
                    // and no row can close with it
                    DecoKind::DivisionMark => self
                        .deco_other_paths(di)
                        .first()
                        .and_then(|&pi| self.word_after_path(pi, median))
                        .unwrap_or(NONE),
                    // a medallion closes its ayah, so it travels with that ayah's last word on
                    // the page; the record usually names it, and this is the fallback
                    DecoKind::AyahMark => d
                        .ayahs
                        .iter()
                        .filter(|a| a.surah == deco.surah && a.ayah == deco.ayah && a.n_words > 0)
                        .map(|a| (a.first_word + a.n_words - 1) as u32)
                        .next_back()
                        .unwrap_or(NONE),
                    _ => NONE,
                };
            }
        }
        // A sajdah mark is followed by the medallion that closes its ayah, and the two are read
        // as one sign. Whatever the geometry says, they travel with the same word.
        for di in 0..d.decorations.len() {
            if d.decorations[di].kind != DecoKind::SajdahMark {
                continue;
            }
            let (surah, ayah) = (d.decorations[di].surah, d.decorations[di].ayah);
            let medallion =
                d.decorations.iter().position(|x| x.kind == DecoKind::AyahMark && x.surah == surah && x.ayah == ayah);
            if let Some(m) = medallion {
                if out[m] != NONE {
                    out[di] = out[m];
                }
            }
        }
        out
    }

    /// The word that follows a mark in reading order: the one to its left on the line the mark
    /// was resolved to. A rub' al-hizb stands where its division starts, so it travels with the
    /// word it opens and can never be left at the end of a row.
    pub(crate) fn word_after_path(&self, pi: u32, tolerance: f32) -> Option<u32> {
        let d = self.data();
        let q = self.quant();
        let li = self.geometry().table.get(pi as usize)?.line as usize;
        let b = &d.paths[pi as usize].bbox;
        let reach = b.x0 + (tolerance * q) as i32;
        let ws = self.line_words.get(li)?;
        ws.iter()
            .filter(|&&(_, wi)| d.words[wi as usize].bbox.x1 <= reach)
            .min_by_key(|&&(_, wi)| reach - d.words[wi as usize].bbox.x1)
            .or_else(|| {
                let centre = (b.x0 + b.x1) / 2;
                ws.iter().min_by_key(|(_, wi)| {
                    let w = &d.words[*wi as usize];
                    (w.bbox.x0 - centre).abs().min((w.bbox.x1 - centre).abs())
                })
            })
            .map(|&(_, wi)| wi)
    }

    /// The word a mark belongs with: the one before it in reading order on the line the mark
    /// was resolved to, which is the word to its right. A mark with nothing to its right takes
    /// the nearest word on that line instead.
    pub(crate) fn word_before_path(&self, pi: u32, tolerance: f32) -> Option<u32> {
        let d = self.data();
        let q = self.quant();
        let li = self.geometry().table.get(pi as usize)?.line as usize;
        let b = &d.paths[pi as usize].bbox;
        // the print sets a mark tight against the word it follows, and sometimes a hair into
        // it, so a word that overlaps the mark by less than a word gap still counts as before it
        let reach = b.x1 - (tolerance * q) as i32;
        let ws = self.line_words.get(li)?;
        ws.iter()
            .filter(|(x0, _)| *x0 >= reach)
            .min_by_key(|(x0, _)| x0 - reach)
            .or_else(|| {
                let centre = (b.x0 + b.x1) / 2;
                ws.iter().min_by_key(|(_, wi)| {
                    let w = &d.words[*wi as usize];
                    (w.bbox.x0 - centre).abs().min((w.bbox.x1 - centre).abs())
                })
            })
            .map(|&(_, wi)| wi)
    }

    /// The words a sajdah line is drawn over: the ones it spans horizontally on the line
    /// directly below it. Empty for any other decoration.
    pub(crate) fn sajdah_line_words(&self, di: usize) -> Vec<u32> {
        let q = self.quant();
        let d = self.data();
        let deco = &d.decorations[di];
        // the stroke's own ink: a decoration can also hold the sign printed in the margin, and
        // the words under that are not the words the stroke marks
        let mut stroke: Option<qvp_format::IBox> = None;
        for pi in deco.first_path..deco.first_path + deco.n_paths as u32 {
            if d.paths[pi as usize].mark == Mark::SajdahLine {
                let bb = d.paths[pi as usize].bbox;
                stroke = Some(match stroke {
                    None => bb,
                    Some(mut acc) => {
                        acc.union(&bb);
                        acc
                    }
                });
            }
        }
        let Some(b) = stroke else { return vec![] };
        let b = &b;
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

    /// The largest `zoom` at which every word of this page still fits a row, so the reflow
    /// never has to place ink wider than the row it sits on. A host clamps its pinch to this.
    ///
    /// The sajdah line is left out of the measure: it is drawn over a span of words, not over
    /// one, so it is not what bounds the zoom.
    pub fn reflow_max_zoom(&self, spec: &ReflowSpec) -> f32 {
        let d = self.data();
        let median = self.median_word_gap();
        let anchors = self.deco_anchor_words();
        let sajdah_line = self.sajdah_line_decos();
        let mut word_decos: Vec<Vec<u32>> = vec![Vec::new(); d.words.len()];
        for (di, &wi) in anchors.iter().enumerate() {
            if wi != NONE && !sajdah_line[di] {
                word_decos[wi as usize].push(di as u32);
            }
        }
        let block = self.text_block();
        let widest = (0..d.words.len() as u32)
            .map(|wi| self.atom(wi, &word_decos[wi as usize], median * spec.word_gap.max(0.0), block).width)
            .fold(0.0f32, f32::max);
        if widest > 0.0 {
            ((block.1 - block.0) / widest).max(1.0)
        } else {
            f32::INFINITY
        }
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
    /// The paths that are a sajdah line stroke.
    pub(crate) fn sajdah_line_paths(&self) -> Vec<bool> {
        self.data().paths.iter().map(|p| p.mark == Mark::SajdahLine).collect()
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

    pub(crate) fn reflow(&self, spec: &ReflowSpec, rows: &RowSpec) -> Reflowed {
        let RowSpec { block, margins, row_w, pitch, top, printed_spacing, rows_per_view } = *rows;
        let q = self.quant();
        let d = self.data();
        let median = self.median_word_gap();
        let anchors = self.deco_anchor_words();
        let mut word_decos: Vec<Vec<u32>> = vec![Vec::new(); d.words.len()];
        for (di, &wi) in anchors.iter().enumerate() {
            if wi != NONE {
                word_decos[wi as usize].push(di as u32);
            }
        }

        // atoms in reading order, and the banner lines that interrupt them
        let mut rows: Vec<Vec<Atom>> = Vec::new();
        let mut header_rows: Vec<(usize, u32)> = Vec::new(); // (row index, printed line)
        let mut cur: Vec<Atom> = Vec::new();
        // between two atoms, not two words: an atom's printed extent already holds the
        // medallion that closes its ayah, so the print's word-to-word distance would count
        // that medallion twice and tear the row open
        let carried = |prev: &Atom, next: &Atom| self.words_interlock(prev.word, next.word);
        let interlocked = |prev: &Atom, next: &Atom| carried(prev, next).is_some();
        let gap_of = |prev: &Atom, next: &Atom| -> f32 {
            // A word the print carries under the one before it keeps the distance the print
            // gave it, whatever the rest of the row is set at: the pair is one piece of
            // calligraphy. Only a row break between them sets them as two ordinary words.
            if carried(prev, next).is_some() {
                return prev.x0 - next.x1;
            }
            let air = median * spec.word_gap.max(0.0);
            match spec.gaps {
                // Leave the same air between the strokes of every pair. Shifting the second
                // word by `t` changes the air and the box gap alike, so the box gap that
                // leaves the wanted air is the printed one moved by the difference.
                GapMode::Uniform => match Page::slice_clearance(&prev.slices, &next.slices, 0.0) {
                    Some(now) => (prev.x0 - next.x1) + (air - now),
                    // no band in common: neither word has ink at any height the other does, so
                    // nothing can meet and the boxes may sit the wanted air apart
                    None => air,
                },
                GapMode::Printed => {
                    let (a, b) = (&d.words[prev.word as usize], &d.words[next.word as usize]);
                    // the print's own distance, where the print set the two together
                    if a.line_index == b.line_index {
                        prev.x0 - next.x1
                    } else {
                        match Page::slice_clearance(&prev.slices, &next.slices, 0.0) {
                            Some(now) => (prev.x0 - next.x1) + (air - now),
                            None => air,
                        }
                    }
                }
            }
        };
        // Text flows from one printed line into the next only when the reader has zoomed in and
        // the rows are narrower than the printed lines. At the printed size the page is the
        // printed page: every line keeps its own row, including the short ones a surah ends on.
        let flows_on = row_w + 0.01 < block.1 - block.0;
        // The atoms of the page in reading order, cut into blocks by the banners that text
        // never flows across.
        enum Block {
            Banner(u32),
            Text(Vec<Atom>),
        }
        let mut blocks: Vec<Block> = Vec::new();
        for li in 0..d.lines.len() {
            if !flows_on && !cur.is_empty() {
                blocks.push(Block::Text(std::mem::take(&mut cur)));
            }
            if self.line_is_header(li) {
                if !cur.is_empty() {
                    blocks.push(Block::Text(std::mem::take(&mut cur)));
                }
                blocks.push(Block::Banner(li as u32));
                continue;
            }
            // words of the line right to left
            for &(_, wi) in self.line_words[li].iter().rev() {
                cur.push(self.atom(wi, &word_decos[wi as usize], median, block));
            }
        }
        if !cur.is_empty() {
            blocks.push(Block::Text(cur));
        }

        // Where to break a block into rows. `width(i..j)` is a run of atoms with the gaps that
        // fall between them, so both strategies read it off two prefix sums.
        let cut_points = |atoms: &[Atom]| -> Vec<usize> {
            let n = atoms.len();
            if n == 0 {
                return vec![];
            }
            let (mut w, mut g) = (vec![0.0f32; n + 1], vec![0.0f32; n + 1]);
            for i in 0..n {
                w[i + 1] = w[i] + atoms[i].width;
                g[i + 1] = g[i] + if i > 0 { gap_of(&atoms[i - 1], &atoms[i]) } else { 0.0 };
            }
            let width = |i: usize, j: usize| (w[j] - w[i]) + (g[j] - g[i + 1]);
            match spec.breaks {
                Breaks::Greedy => {
                    let (mut cuts, mut start) = (vec![], 0usize);
                    for j in 1..n {
                        if width(start, j + 1) > row_w + 0.01 {
                            cuts.push(j);
                            start = j;
                        }
                    }
                    cuts
                }
                // the breaks that leave the block's rows as even as they can be: the cost of a
                // row is the square of what it is left short, so one row far shorter than the
                // rest costs more than several a little short. The block's last row is free,
                // because a block ends where its text ends.
                Breaks::Even => {
                    let (mut best, mut from) = (vec![f32::INFINITY; n + 1], vec![0usize; n + 1]);
                    best[0] = 0.0;
                    for j in 1..=n {
                        for i in (0..j).rev() {
                            let used = width(i, j);
                            // a row must hold at least one atom, however wide that atom is
                            if used > row_w + 0.01 && i + 1 < j {
                                break;
                            }
                            let slack = (row_w - used).max(0.0);
                            let cost = if j == n { 0.0 } else { slack * slack };
                            if best[i] + cost < best[j] {
                                best[j] = best[i] + cost;
                                from[j] = i;
                            }
                        }
                    }
                    let (mut cuts, mut j) = (vec![], n);
                    while j > 0 {
                        let i = from[j];
                        if i > 0 {
                            cuts.push(i);
                        }
                        j = i;
                    }
                    cuts.reverse();
                    cuts
                }
            }
        };
        for item in blocks {
            match item {
                Block::Banner(li) => {
                    header_rows.push((rows.len(), li));
                    rows.push(Vec::new());
                }
                Block::Text(atoms) => {
                    let cuts: std::collections::HashSet<usize> = cut_points(&atoms).into_iter().collect();
                    let mut row: Vec<Atom> = Vec::new();
                    for (i, a) in atoms.into_iter().enumerate() {
                        if i > 0 && cuts.contains(&i) {
                            rows.push(std::mem::take(&mut row));
                        }
                        row.push(a);
                    }
                    if !row.is_empty() {
                        rows.push(row);
                    }
                }
            }
        }

        // place every row right to left
        let headers: std::collections::HashMap<usize, u32> = header_rows.into_iter().collect();
        let mut out = Reflowed {
            row_words: Vec::with_capacity(rows.len()),
            row_baseline: Vec::with_capacity(rows.len()),
            row_band: Vec::with_capacity(rows.len()),
            word_place: vec![Placement::IDENTITY; d.words.len()],
            word_row: vec![NONE; d.words.len()],
            deco_place: vec![Placement::IDENTITY; d.decorations.len()],
            sajdah_place: vec![Placement::IDENTITY; d.decorations.len()],
            repeats: Vec::new(),
            as_printed: false,
            omitted: Vec::new(),
            row_w,
            pitch,
            y0: 0.0,
            y1: 0.0,
        };
        // When a row holds exactly the words of one printed line, and every row does, the
        // reflow has reproduced the print: the page then goes back to its printed line
        // positions, irregular spacing and all, instead of the even rhythm the rows would
        // otherwise take. This is what returning to zoom 1 gives back.
        let as_printed: Option<Vec<usize>> = {
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
            (ok && lines.len() == rows.len()).then_some(lines)
        };
        // The printed line positions come back only when the reader asked for no more leading
        // than the print has. A reader opening the lines up wants that on a reproduced page
        // too, so the rows then take the even rhythm with the spacing asked for.
        let printed_y = printed_spacing.then_some(()).and(as_printed.clone());
        // Horizontal placement first: it does not depend on the heights, and the heights need
        // to know which ink of two rows ends up over which.
        let last_row = rows.len().saturating_sub(1);
        // What each row holds before anything is opened up: its ink, the gaps the print or the
        // page's own spacing give it, and which of those gaps may stretch.
        struct Measured {
            ink: f32,
            gaps: Vec<f32>,
            open: Vec<bool>,
        }
        let measured: Vec<Measured> = rows
            .iter()
            .map(|atoms| Measured {
                ink: atoms.iter().map(|a| a.width).sum(),
                gaps: atoms.windows(2).map(|p| gap_of(&p[0], &p[1])).collect(),
                open: atoms.windows(2).map(|p| !interlocked(&p[0], &p[1])).collect(),
            })
            .collect();
        let used = |r: usize| measured[r].ink + measured[r].gaps.iter().sum::<f32>();
        // The widest row of each screenful: what the rows a reader sees together are evened
        // against, since those are the rows that can be compared.
        let view_of = |r: usize| r.checked_div(rows_per_view).unwrap_or(0);
        let mut widest: Vec<f32> = Vec::new();
        for (r, atoms) in rows.iter().enumerate() {
            if headers.contains_key(&r) || atoms.is_empty() {
                continue;
            }
            let v = view_of(r);
            if widest.len() <= v {
                widest.resize(v + 1, 0.0);
            }
            widest[v] = widest[v].max(used(r));
        }
        let mut row_dx: Vec<Vec<f32>> = Vec::with_capacity(rows.len());
        for (r, atoms) in rows.iter().enumerate() {
            let mut dxs = Vec::with_capacity(atoms.len());
            if headers.contains_key(&r) {
                row_dx.push(dxs);
                continue;
            }
            let Measured { ink, ref gaps, ref open } = measured[r];
            let mut gaps = gaps.clone();
            let natural: f32 = gaps.iter().sum();
            let n_open = open.iter().filter(|o| **o).count();
            // A row that ends a block — the page's last row, or the row before a banner — is
            // short because the text ran out, so it is left as it is.
            let ends_block = r == last_row || headers.contains_key(&(r + 1));
            // How much a row may be opened up, over the gaps that are free to stretch: never
            // past the cap, because gapped-out words read worse than a short row.
            let widen = |gaps: &mut Vec<f32>, extra: f32| {
                // measured on the air a gap holds, not on the distance between the two boxes:
                // a box gap goes negative wherever a word's stroke reaches over its neighbour,
                // and a cap read off that would pin every such row shut
                let room = if spec.max_stretch > 0.0 {
                    (n_open as f32 * median * spec.word_gap.max(0.0) * (spec.max_stretch - 1.0)).max(0.0)
                } else {
                    f32::MAX
                };
                let extra = extra.min(room);
                if extra <= 0.0 || n_open == 0 {
                    return;
                }
                let share = extra / n_open as f32;
                for (g, o) in gaps.iter_mut().zip(open) {
                    if *o {
                        *g += share;
                    }
                }
            };
            if !ends_block {
                match spec.fill {
                    Fill::Justified => widen(&mut gaps, row_w - ink - natural),
                    // A row is opened towards the widest row of its own screenful, because
                    // those are the rows a reader sees together and compares. It is a share of
                    // the way and not the whole of it: the row is never justified, and
                    // `max_stretch` still caps how far its gaps may go.
                    _ if spec.relax > 0.0 => {
                        let target = widest.get(view_of(r)).copied().unwrap_or(row_w).min(row_w);
                        let mine = ink + natural;
                        if target > mine {
                            widen(&mut gaps, (target - mine) * spec.relax.min(1.0));
                        }
                    }
                    _ => {}
                }
            }
            if as_printed.is_some() {
                // the print reproduced: the words stay exactly where the page has them
                dxs.extend(atoms.iter().map(|_| -block.0));
                row_dx.push(dxs);
                continue;
            }
            // a centred row starts half its leftover space in from the right margin
            let mut cursor = row_w;
            if spec.fill == Fill::Centred {
                let slack = row_w - ink - gaps.iter().sum::<f32>();
                if slack > 0.0 {
                    cursor -= slack / 2.0;
                }
            }
            for (i, a) in atoms.iter().enumerate() {
                if i > 0 {
                    cursor -= gaps[i - 1];
                }
                dxs.push(cursor - a.x1);
                cursor -= a.x1 - a.x0;
            }
            row_dx.push(dxs);
        }

        // Every word sits on its line's baseline, so a row places its words on one baseline of
        // its own, the printed line spacing apart. Two rows are pushed further apart only where
        // their ink would actually meet: a deep descender over a tall mark in the same column.
        // The print allows ink to cross into a neighbouring line, and so does this.
        let baselines = &self.line_baseline;
        let ink_of = |a: &Atom, dx: f32| -> (f32, f32, f32, f32) {
            let w = &d.words[a.word as usize];
            let b = baselines[w.line_index as usize];
            (
                w.bbox.x0 as f32 / q + dx,
                w.bbox.x1 as f32 / q + dx,
                b - w.bbox.y0 as f32 / q, // ascent
                w.bbox.y1 as f32 / q - b, // descent
            )
        };
        let mut row_y: Vec<f32> = Vec::with_capacity(rows.len());
        let (mut ascents, mut descents) = (Vec::with_capacity(rows.len()), Vec::with_capacity(rows.len()));
        for (r, atoms) in rows.iter().enumerate() {
            let (mut asc, mut desc) = (0.0f32, 0.0f32);
            for (i, a) in atoms.iter().enumerate() {
                let (_, _, aa, dd) = ink_of(a, row_dx[r].get(i).copied().unwrap_or(0.0));
                asc = asc.max(aa);
                desc = desc.max(dd);
            }
            // a banner row is as tall as its own ink
            if let Some(&li) = headers.get(&r) {
                let bb = d.lines[li as usize].bbox;
                let c = self.line_centre(li as usize);
                asc = asc.max(c - bb.y0 as f32 / q);
                desc = desc.max(bb.y1 as f32 / q - c);
            }
            ascents.push(asc.max(0.0));
            descents.push(desc.max(0.0));
            // the printed page reproduced: keep the printed line positions
            if let Some(lines) = &printed_y {
                let li = lines[r];
                let y = if headers.contains_key(&r) { self.line_centre(li) } else { baselines[li] };
                row_y.push(y);
                continue;
            }
            let y = match r.checked_sub(1) {
                None => top + ascents[r],
                Some(prev) => {
                    // the room the two rows' ink needs where it shares a column
                    let mut need: f32 = 0.0;
                    for (i, a) in rows[prev].iter().enumerate() {
                        let (ax0, ax1, _, adesc) = ink_of(a, row_dx[prev].get(i).copied().unwrap_or(0.0));
                        for (j, b) in atoms.iter().enumerate() {
                            let (bx0, bx1, basc, _) = ink_of(b, row_dx[r].get(j).copied().unwrap_or(0.0));
                            if ax0 < bx1 && bx0 < ax1 {
                                need = need.max(adesc + basc);
                            }
                        }
                    }
                    // a banner keeps clear of the text on both sides
                    if headers.contains_key(&prev) || headers.contains_key(&r) {
                        need = need.max(descents[prev] + ascents[r]);
                    }
                    row_y[prev] + pitch.max(need)
                }
            };
            row_y.push(y);
        }
        // Bands meet halfway between neighbouring rows, as the printed slots do — measured
        // between the rows' ink, not their baselines. A baseline sits low in its row (the
        // ascenders and marks are all above it), so a band built around one would hang below
        // the words it belongs to.
        // the row's ink centre; a page that came out as printed uses the printed line's own
        // centre, so its slots are the printed slots
        let centres: Vec<f32> = (0..rows.len())
            .map(|r| match &printed_y {
                Some(lines) => self.line_centre(lines[r]),
                None => row_y[r] - (ascents[r] - descents[r]) / 2.0,
            })
            .collect();
        let half = pitch / 2.0;
        for r in 0..rows.len() {
            // beside a banner the printed gap is wide (the opening pages set the banner pitches
            // above the text) and it is not the text row's to claim, so the boundary stops half
            // a line spacing out. The printed slots do the same.
            let banner = |i: usize| headers.contains_key(&i);
            let top_b = match r.checked_sub(1) {
                Some(prev) => {
                    let mid = (centres[prev] + centres[r]) / 2.0;
                    if banner(r) || banner(prev) {
                        mid.max(centres[r] - half)
                    } else {
                        mid
                    }
                }
                None => centres[r] - half,
            };
            let bottom_b = match centres.get(r + 1) {
                Some(&next) => {
                    let mid = (centres[r] + next) / 2.0;
                    if banner(r) || banner(r + 1) {
                        mid.min(centres[r] + half)
                    } else {
                        mid
                    }
                }
                None => centres[r] + half,
            };
            out.row_band.push((top_b, bottom_b));
        }
        out.as_printed = printed_y.is_some();
        out.row_baseline = row_y.clone();
        for (r, atoms) in rows.iter().enumerate() {
            let base = row_y[r];
            out.row_words.push(atoms.iter().map(|a| a.word).collect());
            if let Some(&li) = headers.get(&r) {
                let mid = (out.row_band[r].0 + out.row_band[r].1) / 2.0;
                self.place_header(&mut out, li, mid, block, row_w);
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
                // a margin mark keeps the side of the sheet the print puts it on and the
                // height it was printed at; it only follows its word down to the new row. The
                // margin narrows as the reader zooms in, so the mark is held inside it.
                for &(di, w) in &a.margin_decos {
                    let b = self.deco_mark_bbox(di as usize);
                    let (bx0, bx1) = (b.x0 as f32 / q, b.x1 as f32 / q);
                    let left = (bx0 + bx1) / 2.0 < (block.0 + block.1) / 2.0;
                    let x1 = if left {
                        // outside the row's left edge, and never past the margin the page has
                        (bx1 - block.0).min(0.0).max(w - margins.0)
                    } else {
                        (bx0 - block.0).max(row_w).min(row_w + margins.1 - w) + w
                    };
                    out.deco_place[di as usize] = Placement::moved(x1 - bx1, dy);
                }
            }
        }
        // A medallion stands between the ayah it closes and the one that follows. Where both
        // words share a row it is set midway between them, rather than at the distance the
        // print happened to give it on a line it is no longer on. Moving it changes nothing
        // else: the words either side stay where the row put them.
        for atoms in rows.iter() {
            for pair in atoms.windows(2) {
                let (left_word, right_word) = (&pair[1], &pair[0]);
                if out.word_row[right_word.word as usize] != out.word_row[left_word.word as usize] {
                    continue;
                }
                for &di in &right_word.inline_decos {
                    if d.decorations[di as usize].kind != DecoKind::AyahMark {
                        continue;
                    }
                    let (dx_a, dx_b) =
                        (out.word_place[right_word.word as usize].dx, out.word_place[left_word.word as usize].dx);
                    let dx_m = out.deco_place[di as usize].dx;
                    let before =
                        Page::slice_clearance(self.word_slices(right_word.word), self.deco_slices(di), dx_m - dx_a);
                    let after =
                        Page::slice_clearance(self.deco_slices(di), self.word_slices(left_word.word), dx_b - dx_m);
                    if let (Some(before), Some(after)) = (before, after) {
                        out.deco_place[di as usize].dx += (before - after) / 2.0;
                    }
                }
            }
        }

        // the sajdah line: drawn over the words it marks, wherever they now are. The span can
        // break across rows, so the stroke is drawn once per row, stretched along x to cover
        // that row's part of it. Only this stroke is stretched; the words are not.
        for (di, carries_line) in self.sajdah_line_decos().into_iter().enumerate() {
            if !carries_line {
                continue;
            }
            let deco = &d.decorations[di];
            // the stroke's own ink, not the decoration's, which also holds the sign in the margin
            let mut stroke: Option<qvp_format::IBox> = None;
            for pi in deco.first_path..deco.first_path + deco.n_paths as u32 {
                if d.paths[pi as usize].mark == Mark::SajdahLine {
                    let b = d.paths[pi as usize].bbox;
                    stroke = Some(match stroke {
                        None => b,
                        Some(mut acc) => {
                            acc.union(&b);
                            acc
                        }
                    });
                }
            }
            let Some(b) = stroke else { continue };
            let (bx0, bx1) = (b.x0 as f32 / q, b.x1 as f32 / q);
            if bx1 <= bx0 {
                continue;
            }
            let words = self.sajdah_line_words(di);
            let mut by_row: Vec<(u32, f32, f32)> = Vec::new(); // (row, x0, x1) of the placed span
            for &wi in &words {
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
                    out.sajdah_place[di] = p;
                } else {
                    // only the stroke is drawn again; the sign beside it is printed once
                    for pi in deco.first_path..deco.first_path + deco.n_paths as u32 {
                        if d.paths[pi as usize].mark == Mark::SajdahLine {
                            out.repeats.push(Repeat {
                                decoration: di as u32,
                                first_path: pi,
                                n_paths: 1,
                                placement: p,
                            });
                        }
                    }
                }
            }
        }
        // The running head (surah name and juz) and the page number are printed outside the
        // page's own box: they belong to the sheet, not to the text, and a reflowed page has no
        // sheet. They are left out of the layout and listed in `omitted` so a renderer skips
        // them; a host that wants them draws its own, from `page_number` and the atlas.
        for (di, deco) in d.decorations.iter().enumerate() {
            if matches!(deco.kind, DecoKind::PageNumber | DecoKind::RunningHead) {
                out.omitted.push(di as u32);
            }
        }
        // everything is placed: measure what was laid out, and move it so the topmost ink
        // sits where the caller asked the first row to start
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
                let b = &d.decorations[di].bbox;
                grow(p, b.y0 as f32 / q, b.y1 as f32 / q);
            }
        }
        if y0.is_finite() {
            // the print reproduced: the page keeps its own origin, so its top margin survives
            let shift = if printed_y.is_some() { top - pitch / 2.0 } else { top - pitch / 2.0 - y0 };
            for p in out.word_place.iter_mut().chain(out.deco_place.iter_mut()).chain(out.sajdah_place.iter_mut()) {
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
        out
    }

    /// Build the row unit for a word: the ink printed with it keeps its printed offset, and
    /// a mark the print puts in the margin is re-placed beside the word, where a reflowed
    /// row has room for it.
    fn atom(&self, wi: u32, decos: &[u32], median: f32, block: (f32, f32)) -> Atom {
        let q = self.quant();
        let d = self.data();
        let w = &d.words[wi as usize];
        let (mut x0, mut x1) = (w.bbox.x0 as f32 / q, w.bbox.x1 as f32 / q);
        let (wy0, wy1) = (w.bbox.y0 as f32 / q, w.bbox.y1 as f32 / q);
        let (mut inline_decos, mut margin_decos) = (Vec::new(), Vec::new());
        for &di in decos {
            let deco = &d.decorations[di as usize];
            let b = self.deco_mark_bbox(di as usize);
            let (bx0, bx1) = (b.x0 as f32 / q, b.x1 as f32 / q);
            let (by0, by1) = (b.y0 as f32 / q, b.y1 as f32 / q);
            // Where the print puts a mark decides how it travels. A mark printed inside the
            // text block runs with the words and keeps its place among them; one printed out
            // in the sheet's margin, where a reflowed row has no room, is placed in the margin
            // beside its word. A medallion always runs with the words that it closes.
            let in_block = bx0 >= block.0 - median && bx1 <= block.1 + median;
            let beside = (x0 - bx1).max(bx0 - x1) <= 2.0 * median && by0 < wy1 + median && by1 > wy0 - median;
            let inline = deco.kind == DecoKind::AyahMark || in_block || beside;
            if inline {
                x0 = x0.min(bx0);
                x1 = x1.max(bx1);
                inline_decos.push(di);
            } else {
                margin_decos.push((di, bx1 - bx0));
            }
        }
        let slices = self.slices_with(wi, &inline_decos);
        Atom { word: wi, inline_decos, margin_decos, x0, x1, width: x1 - x0, slices }
    }

    /// A banner line keeps its printed drawing, shrunk to the row width and centred on it.
    fn place_header(&self, out: &mut Reflowed, li: u32, centre: f32, block: (f32, f32), row_w: f32) {
        let d = self.data();
        let bw = (block.1 - block.0).max(1.0);
        let k = (row_w / bw).min(1.0);
        let cx = (block.0 + block.1) / 2.0;
        for (di, deco) in d.decorations.iter().enumerate() {
            if deco.line != NONE_U16 && deco.line as u32 == li {
                let c = self.line_centre(li as usize);
                out.deco_place[di] = Placement::scaled(row_w / 2.0 - k * cx, centre - k * c, k);
            }
        }
    }
}
