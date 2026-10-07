//! Passages: a contiguous range of complete ayahs, laid out on rows of a given width from the
//! outlines of their pages.
//!
//! A passage shows ayahs away from their page: a card, a tooltip, a list row. The words keep
//! their original outlines, dots and reading signs; the layout only moves them onto rows. Each
//! word carries the signs that go with it: an ayah medallion and a sajdah sign go with the word
//! that closes their ayah, and a division mark goes with the word that it opens. A sajdah line
//! goes with the words under it, and it is stretched over each row that holds them.
//!
//! This is the reference for `web/lite-passage.mjs`. The two measure ink the same way: in bands
//! of one fixed height, so that one row can hold words from two pages, and in `f64`, so that the
//! two agree to the last digit on the same page data.

use crate::reflow::Placement;
use crate::{defaults, Geometry, Page};
use qvp_format::{DecoKind, IBox, Mark, NONE_U16, OP_CLOSE, OP_CUBIC, OP_LINE, OP_MOVE, OP_QUAD};
use std::collections::{BTreeMap, HashMap};

// ───────────────────────────── constants ─────────────────────────────

/// The height of a measuring band, in page units. Every page uses the same height, so the bands
/// of words from two pages line up on one row.
const BAND: f64 = 0.375;
/// The most straight pieces one segment of an outline is walked as.
const SEGMENT_STEPS: f64 = 512.0;
/// The measuring budget of one page: sample points, and bands over all of its traces. The
/// largest page of the mushaf needs under 447,000 points and 13,000 bands. The budget keeps a
/// malformed page from turning bounded input into unbounded work.
const MAX_POINTS: i64 = 2_000_000;
const MAX_BANDS: i64 = 32_768;
/// The most words a passage holds.
const MAX_WORDS: usize = 4096;
/// The highest surah number, and the most ayahs a surah has.
const MAX_SURAH: u16 = 114;
const MAX_AYAH: u16 = 286;
/// How far above a sajdah line, in page units, the top of a word it marks may reach.
const SAJDAH_LINE_REACH: f64 = 2.0;
/// The air kept between the ink of two rows, in page units.
const ROW_AIR: f64 = 1.0;
/// How far a row may overrun its width before it counts as too wide, in page units.
const OVERRUN: f64 = 0.001;
/// The widest layout, the largest scale and the largest line spacing a passage accepts.
const MAX_WIDTH: f32 = 32_768.0;
const MAX_SCALE: f32 = 32.0;
const MAX_LINE_SPACING: f32 = 4.0;

// ───────────────────────────── types ─────────────────────────────

/// One band of a silhouette: its index from the baseline, and the leftmost and rightmost ink.
type Band = (i32, f64, f64);
/// A rectangle as `[x0, y0, x1, y1]`.
type Rect = [f64; 4];

/// Why a passage cannot be loaded.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PassageError {
    /// The surah is not 1–114, or the ayahs are not a range within 1–286.
    Range,
    /// Two of the pages have the same page number.
    RepeatedPage,
    /// An ayah of the range is not on the pages, or not all of it is.
    IncompleteAyah {
        /// The surah of the ayah.
        surah: u16,
        /// The ayah.
        ayah: u16,
    },
    /// The range has more than 4,096 words.
    TooManyWords,
    /// A page needs more measuring than its budget allows.
    MeasureLimit,
    /// A page has a sign or a sajdah line that no word goes with.
    UnattachedSign,
}

impl std::fmt::Display for PassageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Range => write!(f, "invalid passage ayah range"),
            Self::RepeatedPage => write!(f, "repeated QVP page"),
            Self::IncompleteAyah { surah, ayah } => write!(f, "incomplete passage ayah {surah}:{ayah}"),
            Self::TooManyWords => write!(f, "a passage may contain at most {MAX_WORDS} words"),
            Self::MeasureLimit => write!(f, "QVP passage measurement limit exceeded"),
            Self::UnattachedSign => write!(f, "a passage sign has no word"),
        }
    }
}

impl std::error::Error for PassageError {}

/// Where the rows of a passage start.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
#[repr(u8)]
pub enum Align {
    /// At the right edge, as Arabic text starts.
    #[default]
    Right = 0,
    /// In the middle of the width.
    Center = 1,
}

/// What a layout of a passage asks for. Lengths are layout pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PassageSpec {
    /// The width of the rows. Zero or less puts the whole passage on one row, as wide as it is.
    pub width: f32,
    /// Layout pixels per page unit. It shrinks only when one word and its signs are wider than
    /// the rows.
    pub scale: f32,
    /// The distance between the baselines of two rows, as a multiple of the printed one. At
    /// least 1: a row moves further down when its ink needs the room.
    pub line_spacing: f32,
    /// The empty margin on every side.
    pub padding: f32,
    /// Where each row starts.
    pub align: Align,
    /// The most rows to show. Zero shows every row.
    pub max_rows: u32,
    /// After a cut, place the medallion of the passage's last ayah after the ellipsis, so that
    /// the reader still sees which ayah it is.
    pub keep_ayah_mark: bool,
    /// The width that the host needs for its ellipsis after a cut.
    pub ellipsis_width: f32,
}

impl Default for PassageSpec {
    fn default() -> Self {
        Self {
            width: 0.0,
            scale: 1.0,
            line_spacing: 1.0,
            padding: 2.0,
            align: Align::Right,
            max_rows: 0,
            keep_ayah_mark: false,
            ellipsis_width: 0.0,
        }
    }
}

/// One row of a laid-out passage, in layout pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PassageRow {
    /// The box of the row's ink.
    pub x0: f32,
    /// The top of the row's ink.
    pub y0: f32,
    /// The right edge of the row's ink.
    pub x1: f32,
    /// The bottom of the row's ink.
    pub y1: f32,
    /// The height the row's words sit on.
    pub baseline: f32,
}

/// One word of a laid-out passage, with the signs that go with it, in layout pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PassageWord {
    /// The surah of the word.
    pub surah: u16,
    /// The ayah of the word.
    pub ayah: u16,
    /// The word's number in its ayah, from 1.
    pub word: u16,
    /// The page the word is printed on.
    pub page: u16,
    /// The row the word is on, from 0.
    pub row: u32,
    /// The left edge of the word and its signs.
    pub x0: f32,
    /// The top of the word and its signs.
    pub y0: f32,
    /// The right edge of the word and its signs.
    pub x1: f32,
    /// The bottom of the word and its signs.
    pub y1: f32,
}

/// The bounds of one ayah that a layout shows, in layout pixels.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PassageAyah {
    /// The surah of the ayah.
    pub surah: u16,
    /// The ayah.
    pub ayah: u16,
    /// The left edge of the ayah's shown words.
    pub x0: f32,
    /// The top of the ayah's shown words.
    pub y0: f32,
    /// The right edge of the ayah's shown words.
    pub x1: f32,
    /// The bottom of the ayah's shown words.
    pub y1: f32,
}

/// One path that a layout draws: its page, its index on that page, and its placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PassageDraw {
    /// The page number. Draw the path with that page's geometry.
    pub page: u16,
    /// The path's index on its page.
    pub path: u32,
    /// The index of the path's placement in [`PassageLayout::placements`].
    pub placement: u32,
}

/// A laid-out passage. Lengths are layout pixels; a placement maps page units to layout pixels
/// together with `scale`: a point `(x, y)` of a path draws at
/// `scale · (kx · x + dx), scale · (ky · y + dy)`.
#[derive(Clone, Debug, PartialEq)]
pub struct PassageLayout {
    /// The width of the layout: the asked width, or the width of the one row.
    pub width: f32,
    /// The height of the layout, padding included.
    pub height: f32,
    /// Layout pixels per page unit: the asked scale, or less when a word did not fit.
    pub scale: f32,
    /// The distance between the baselines of two rows, in page units, before any row moves
    /// down for its ink.
    pub line_spacing: f32,
    /// The rows, top to bottom.
    pub rows: Vec<PassageRow>,
    /// The words shown, in reading order.
    pub words: Vec<PassageWord>,
    /// The ayahs with at least one word shown, in reading order.
    pub ayahs: Vec<PassageAyah>,
    /// Every path to draw, in drawing order.
    pub draws: Vec<PassageDraw>,
    /// What [`PassageDraw::placement`] indexes.
    pub placements: Vec<Placement>,
    /// The layout shows fewer words than the passage has.
    pub is_truncated: bool,
    /// Where the host draws its ellipsis after a cut, as `[x0, y0, x1, y1]`: from the top of
    /// the last row's ink down to its baseline.
    pub ellipsis: Option<[f32; 4]>,
}

/// A contiguous range of complete ayahs, ready to lay out at any width. It copies what it needs
/// from its pages, so the pages can be freed after it is loaded.
pub struct Passage {
    atoms: Vec<PassageAtom>,
    /// The gap before each word, page units. The first is zero.
    gaps: Vec<f64>,
    /// The sajdah lines of every page given, with the page number.
    strokes: Vec<(u16, Stroke)>,
    ayahs: Vec<(u16, u16, String)>,
    line_spacing: f64,
    layout: Option<PassageLayout>,
}

/// A word of one page, measured for passages, with the signs that go with it.
#[derive(Clone, Debug)]
struct Atom {
    word: u32,
    line: u16,
    surah: u16,
    ayah: u16,
    number: u16,
    /// The word's own paths, then the paths of its signs.
    paths: Vec<u32>,
    signs: Vec<Sign>,
    /// The word with its signs.
    rect: Rect,
    /// The word alone.
    word_rect: Rect,
    /// The word with its signs, band by band.
    bands: Vec<Band>,
    /// The word alone, band by band.
    word_bands: Vec<Band>,
    baseline: f64,
    /// The page's printed line spacing and its usual gap between words.
    pitch: f64,
    gap: f64,
}

/// A sign that goes with a word: an ayah medallion, a division mark or a sajdah sign.
#[derive(Clone, Debug)]
struct Sign {
    kind: DecoKind,
    paths: Vec<u32>,
    rect: Rect,
    bands: Vec<Band>,
}

/// A sajdah line, and the words it is over.
#[derive(Clone, Debug)]
struct Stroke {
    path: u32,
    words: Vec<u32>,
    rect: Rect,
}

/// A word of a passage: the page it is from, and its measurements.
#[derive(Clone, Debug)]
struct PassageAtom {
    page: u16,
    atom: Atom,
}

/// What a page knows about passages, measured once.
pub(crate) struct Prepared {
    atoms: Vec<Atom>,
    strokes: Vec<Stroke>,
}

/// The measuring budget left on one page.
struct Budget {
    points: i64,
    bands: i64,
}

// ───────────────────────────── passage ─────────────────────────────

impl Passage {
    /// Load the ayahs `from` to `to` of `surah` from the pages that print them, in any order.
    ///
    /// A page that holds none of the range is allowed: its sajdah lines are ignored. Missing or
    /// incomplete ayahs fail, rather than show part of an ayah.
    pub fn load(pages: &[&Page], surah: u16, from: u16, to: u16) -> Result<Passage, PassageError> {
        if !(1..=MAX_SURAH).contains(&surah) || from < 1 || to < from || to > MAX_AYAH {
            return Err(PassageError::Range);
        }
        let mut numbers: Vec<u16> = pages.iter().map(|p| p.page_number()).collect();
        numbers.sort_unstable();
        if numbers.windows(2).any(|w| w[0] == w[1]) {
            return Err(PassageError::RepeatedPage);
        }
        let prepared: Vec<&Prepared> = pages.iter().map(|p| p.passage_prepared()).collect::<Result<_, _>>()?;

        let mut atoms: Vec<PassageAtom> = Vec::new();
        let mut ayahs = Vec::new();
        for ayah in from..=to {
            let incomplete = PassageError::IncompleteAyah { surah, ayah };
            let mut words: Vec<(usize, u32)> = Vec::new();
            let mut fragments = Vec::new();
            for (pi, page) in pages.iter().enumerate() {
                let data = page.data();
                for (wi, w) in data.words.iter().enumerate() {
                    if w.surah == surah && w.ayah == ayah {
                        words.push((pi, wi as u32));
                    }
                }
                fragments.extend(data.ayahs.iter().filter(|a| a.surah == surah && a.ayah == ayah).copied());
            }
            words.sort_by_key(|&(pi, wi)| pages[pi].data().words[wi as usize].word);
            fragments.sort_by_key(|a| a.fragment);
            let complete = !words.is_empty()
                && !fragments.is_empty()
                && fragments.len() == fragments[0].fragments as usize
                && fragments
                    .iter()
                    .enumerate()
                    .all(|(i, a)| a.fragment as usize == i + 1 && a.fragments as usize == fragments.len())
                && words.len() == fragments.iter().map(|a| a.n_words as usize).sum::<usize>()
                && words
                    .iter()
                    .enumerate()
                    .all(|(i, &(pi, wi))| pages[pi].data().words[wi as usize].word as usize == i + 1);
            if !complete {
                return Err(incomplete);
            }
            if atoms.len() + words.len() > MAX_WORDS {
                return Err(PassageError::TooManyWords);
            }
            let mut text = Vec::with_capacity(words.len());
            for &(pi, wi) in &words {
                atoms
                    .push(PassageAtom { page: pages[pi].page_number(), atom: prepared[pi].atoms[wi as usize].clone() });
                text.push(pages[pi].word_text(wi));
            }
            ayahs.push((surah, ayah, text.join(" ")));
        }

        // Two words the print draws as one piece of calligraphy keep their printed distance;
        // any other two are set the usual gap apart, measured on their ink.
        let gaps = (0..atoms.len())
            .map(|i| {
                if i == 0 {
                    return 0.0;
                }
                let (previous, atom) = (&atoms[i - 1], &atoms[i]);
                let (a, b) = (&previous.atom, &atom.atom);
                let natural = a.rect[0] - b.rect[2];
                let joined = previous.page == atom.page
                    && a.line == b.line
                    && a.word + 1 == b.word
                    && clearance(&a.word_bands, &b.word_bands)
                        .is_some_and(|air| -air >= b.pitch * defaults::INTERLOCK_DEPTH as f64);
                if joined {
                    return natural;
                }
                let wanted = (a.gap + b.gap) / 2.0;
                match clearance(&a.bands, &b.bands) {
                    Some(air) => natural + wanted - air,
                    None => wanted,
                }
            })
            .collect();
        let strokes = pages
            .iter()
            .zip(&prepared)
            .flat_map(|(p, prep)| prep.strokes.iter().map(|s| (p.page_number(), s.clone())))
            .collect();
        let line_spacing = median(atoms.iter().map(|a| a.atom.pitch).collect(), 0.0);
        Ok(Passage { atoms, gaps, strokes, ayahs, line_spacing, layout: None })
    }

    /// Get the ayahs of the passage in reading order, with the page's own word texts.
    pub fn ayahs(&self) -> impl Iterator<Item = (u16, u16, &str)> {
        self.ayahs.iter().map(|(s, a, t)| (*s, *a, t.as_str()))
    }

    /// Get how many words the passage has.
    pub fn word_count(&self) -> usize {
        self.atoms.len()
    }

    /// Get the printed line spacing of the passage's pages, in page units: the median over its
    /// words. A host that sizes a passage by its printed line divides by it to get a scale.
    pub fn line_spacing(&self) -> f32 {
        self.line_spacing as f32
    }

    /// Get the last layout, if there is one.
    pub fn current_layout(&self) -> Option<&PassageLayout> {
        self.layout.as_ref()
    }

    /// Lay the passage out and keep the layout. `None` when the spec is out of range.
    pub fn layout(&mut self, spec: &PassageSpec) -> Option<&PassageLayout> {
        let layout = self.compute(spec)?;
        self.layout = Some(layout);
        self.layout.as_ref()
    }

    fn compute(&self, spec: &PassageSpec) -> Option<PassageLayout> {
        let valid = spec.width.is_finite()
            && spec.width <= MAX_WIDTH
            && spec.scale.is_finite()
            && spec.scale > 0.0
            && spec.scale <= MAX_SCALE
            && spec.padding.is_finite()
            && spec.padding >= 0.0
            && (spec.width <= 0.0 || 2.0 * spec.padding < spec.width)
            && spec.line_spacing.is_finite()
            && (1.0..=MAX_LINE_SPACING).contains(&spec.line_spacing)
            && spec.ellipsis_width.is_finite()
            && spec.ellipsis_width >= 0.0;
        if !valid || self.atoms.is_empty() {
            return None;
        }
        let atoms: Vec<&Atom> = self.atoms.iter().map(|a| &a.atom).collect();
        let width_of = |i: usize| atoms[i].rect[2] - atoms[i].rect[0];
        let used = |row: &[usize]| -> f64 {
            row.iter().enumerate().map(|(k, &i)| width_of(i) + if k > 0 { self.gaps[i] } else { 0.0 }).sum()
        };
        let padding = spec.padding as f64;
        let one_row = spec.width <= 0.0;
        let mut scale = spec.scale as f64;
        let all: Vec<usize> = (0..atoms.len()).collect();
        let row_width = if one_row {
            used(&all)
        } else {
            // A word and its signs cannot be divided: at an unusually narrow width, everything
            // shrinks so that the widest one fits.
            let widest = (0..atoms.len()).map(width_of).fold(0.0, f64::max);
            if widest > 0.0 {
                scale = scale.min((spec.width as f64 - 2.0 * padding) / widest);
            }
            (spec.width as f64 - 2.0 * padding) / scale
        };
        let mut rows = if one_row { vec![all] } else { break_rows(&atoms, &self.gaps, row_width) };

        // Past the row limit the rows are filled in turn, as text is: balanced rows can need one
        // more row than full ones. When even full rows do not fit, the last one is cut early
        // enough for the ellipsis and, when asked, the medallion of the last ayah.
        let max_rows = spec.max_rows as usize;
        if max_rows > 0 && rows.len() > max_rows {
            rows = fill_rows(&atoms, &self.gaps, row_width);
        }
        let is_truncated = max_rows > 0 && rows.len() > max_rows;
        let ellipsis_width = spec.ellipsis_width as f64 / scale;
        let tail = if is_truncated && spec.keep_ayah_mark {
            let last = atoms.len() - 1;
            atoms[last].signs.iter().find(|s| s.kind == DecoKind::AyahMark).map(|s| (last, s))
        } else {
            None
        };
        let reserve = |row: &[usize]| -> f64 {
            let gap = row.last().map_or(0.0, |&i| atoms[i].gap);
            gap + ellipsis_width + tail.map_or(0.0, |(_, s)| gap + s.rect[2] - s.rect[0])
        };
        if is_truncated {
            rows.truncate(max_rows);
            let last = rows.last_mut().expect("a cut keeps at least one row");
            while last.len() > 1 && used(last) + reserve(last) > row_width + OVERRUN {
                last.pop();
            }
        }

        let pitch = median(atoms.iter().map(|a| a.pitch).collect(), 0.0) * spec.line_spacing as f64;
        let mut out_rows: Vec<(Rect, f64)> = Vec::with_capacity(rows.len());
        let mut words: Vec<(usize, u32, Rect)> = Vec::new();
        let mut draws: Vec<(u16, u32, u32)> = Vec::new();
        let mut placements: Vec<(f64, f64, f64, u32)> = Vec::new();
        let mut placed: HashMap<(u16, u32), (u32, f64, f64, Rect)> = HashMap::new();
        let mut ellipsis: Option<Rect> = None;
        let (mut last_baseline, mut last_bottom) = (0.0, 0.0);
        for (r, row) in rows.iter().enumerate() {
            let is_last_cut = is_truncated && r == rows.len() - 1;
            let mut row_used = used(row);
            let mut ascent = row.iter().map(|&i| atoms[i].baseline - atoms[i].rect[1]).fold(f64::MIN, f64::max);
            let mut descent = row.iter().map(|&i| atoms[i].rect[3] - atoms[i].baseline).fold(f64::MIN, f64::max);
            if is_last_cut {
                row_used += reserve(row);
                if let Some((owner, sign)) = tail {
                    ascent = ascent.max(atoms[owner].baseline - sign.rect[1]);
                    descent = descent.max(sign.rect[3] - atoms[owner].baseline);
                }
            }
            // Signs and tall marks have the same right to vertical space as the letters.
            let baseline =
                if r == 0 { ascent } else { f64::max(last_baseline + pitch, last_bottom + ascent + ROW_AIR) };
            let mut cursor = match spec.align {
                Align::Center => (row_width + row_used) / 2.0,
                Align::Right => row_width,
            };
            let mut boxes = Vec::with_capacity(row.len() + 1);
            for (k, &i) in row.iter().enumerate() {
                let atom = atoms[i];
                if k > 0 {
                    cursor -= self.gaps[i];
                }
                let (dx, dy) = (cursor - atom.rect[2], baseline - atom.baseline);
                let placement = placements.len() as u32;
                placements.push((dx, dy, 1.0, r as u32));
                let page = self.atoms[i].page;
                draws.extend(atom.paths.iter().map(|&p| (page, p, placement)));
                placed.insert((page, atom.word), (r as u32, dx, dy, atom.word_rect));
                let rect = moved(atom.rect, dx, dy);
                boxes.push(rect);
                words.push((i, r as u32, rect));
                cursor -= width_of(i);
            }
            if is_last_cut {
                cursor -= row.last().map_or(0.0, |&i| atoms[i].gap);
                let x1 = cursor;
                cursor -= ellipsis_width;
                ellipsis = Some([cursor, 0.0, x1, baseline]);
                if let Some((owner, sign)) = tail {
                    cursor -= atoms[owner].gap;
                    let (dx, dy) = (cursor - sign.rect[2], baseline - atoms[owner].baseline);
                    let placement = placements.len() as u32;
                    placements.push((dx, dy, 1.0, r as u32));
                    let page = self.atoms[owner].page;
                    draws.extend(sign.paths.iter().map(|&p| (page, p, placement)));
                    boxes.push(moved(sign.rect, dx, dy));
                }
            }
            out_rows.push((bounds(boxes.iter().copied()).expect("a row holds at least one word"), baseline));
            last_baseline = baseline;
            last_bottom = baseline + descent;
        }

        // A sajdah line belongs to the words under it, which can be in an earlier ayah than the
        // sign. When those words wrap, the stroke is drawn on each row that holds them, and only
        // the stroke is stretched.
        for (page, stroke) in &self.strokes {
            let mut spans: Vec<(u32, Rect, f64)> = Vec::new();
            for &w in &stroke.words {
                let Some(&(row, dx, dy, word_rect)) = placed.get(&(*page, w)) else { continue };
                let rect = moved(word_rect, dx, dy);
                match spans.iter_mut().find(|s| s.0 == row) {
                    Some(span) => {
                        span.1 = union(span.1, rect);
                        span.2 = dy;
                    }
                    None => spans.push((row, rect, dy)),
                }
            }
            for (row, rect, dy) in spans {
                let kx = (rect[2] - rect[0]) / (stroke.rect[2] - stroke.rect[0]);
                let dx = rect[0] - kx * stroke.rect[0];
                let placement = placements.len() as u32;
                placements.push((dx, dy, kx, row));
                draws.push((*page, stroke.path, placement));
                let r = &mut out_rows[row as usize].0;
                *r = union(*r, [rect[0], stroke.rect[1] + dy, rect[2], stroke.rect[3] + dy]);
            }
        }

        // The strokes take room too. A row that now touches the one above moves down, with every
        // row after it: moving whole rows never reshapes ink.
        for r in 1..out_rows.len() {
            let needed = out_rows[r - 1].0[3] + ROW_AIR - out_rows[r].0[1];
            if needed <= 0.0 {
                continue;
            }
            for row in &mut out_rows[r..] {
                row.0[1] += needed;
                row.0[3] += needed;
                row.1 += needed;
            }
            for word in words.iter_mut().filter(|w| w.1 as usize >= r) {
                word.2[1] += needed;
                word.2[3] += needed;
            }
            for placement in placements.iter_mut().filter(|p| p.3 as usize >= r) {
                placement.1 += needed;
            }
        }
        if let Some(e) = ellipsis.as_mut() {
            let (rect, baseline) = out_rows[out_rows.len() - 1];
            e[1] = rect[1];
            e[3] = baseline;
        }

        let ink = bounds(out_rows.iter().map(|r| r.0)).expect("a layout holds at least one row");
        let offset = padding / scale - ink[1];
        let pixels = |r: Rect| -> [f32; 4] {
            [
                (r[0] * scale + padding) as f32,
                ((r[1] + offset) * scale) as f32,
                (r[2] * scale + padding) as f32,
                ((r[3] + offset) * scale) as f32,
            ]
        };
        let words: Vec<PassageWord> = words
            .into_iter()
            .map(|(i, row, rect)| {
                let (a, [x0, y0, x1, y1]) = (&self.atoms[i], pixels(rect));
                PassageWord {
                    surah: a.atom.surah,
                    ayah: a.atom.ayah,
                    word: a.atom.number,
                    page: a.page,
                    row,
                    x0,
                    y0,
                    x1,
                    y1,
                }
            })
            .collect();
        let mut ayahs: Vec<PassageAyah> = Vec::new();
        for w in &words {
            match ayahs.last_mut() {
                Some(a) if a.surah == w.surah && a.ayah == w.ayah => {
                    a.x0 = a.x0.min(w.x0);
                    a.y0 = a.y0.min(w.y0);
                    a.x1 = a.x1.max(w.x1);
                    a.y1 = a.y1.max(w.y1);
                }
                _ => ayahs.push(PassageAyah { surah: w.surah, ayah: w.ayah, x0: w.x0, y0: w.y0, x1: w.x1, y1: w.y1 }),
            }
        }
        let width = if one_row { row_width * scale + 2.0 * padding } else { spec.width as f64 };
        Some(PassageLayout {
            width: width as f32,
            height: ((ink[3] - ink[1]) * scale + 2.0 * padding) as f32,
            scale: scale as f32,
            line_spacing: pitch as f32,
            rows: out_rows
                .iter()
                .map(|&(rect, baseline)| {
                    let [x0, y0, x1, y1] = pixels(rect);
                    PassageRow { x0, y0, x1, y1, baseline: ((baseline + offset) * scale) as f32 }
                })
                .collect(),
            words,
            ayahs,
            draws: draws.into_iter().map(|(page, path, placement)| PassageDraw { page, path, placement }).collect(),
            placements: placements
                .into_iter()
                .map(|(dx, dy, kx, _)| Placement {
                    dx: (dx + padding / scale) as f32,
                    dy: (dy + offset) as f32,
                    kx: kx as f32,
                    ky: 1.0,
                })
                .collect(),
            is_truncated,
            ellipsis: ellipsis.map(pixels),
        })
    }
}

// ───────────────────────────── preparation ─────────────────────────────

/// Measure a page for passages: each word with the signs that go with it, and its sajdah lines.
pub(crate) fn prepare(page: &Page) -> Result<Prepared, PassageError> {
    let data = page.data();
    let geometry = page.geometry();
    let q = page.quant();
    let rect =
        |b: &IBox| -> Rect { [b.x0 as f32 / q, b.y0 as f32 / q, b.x1 as f32 / q, b.y1 as f32 / q].map(f64::from) };
    let mut budget = Budget { points: MAX_POINTS, bands: MAX_BANDS };
    let pitch = page.line_spacing() as f64;
    let baselines: Vec<f64> = (0..data.lines.len()).map(|li| page.line_baseline(li) as f64).collect();
    let line_words = |li: usize| -> std::ops::Range<u32> {
        let l = &data.lines[li];
        l.first_word as u32..(l.first_word + l.n_words) as u32
    };
    let word_paths = |wi: u32| -> std::ops::Range<u32> {
        let w = &data.words[wi as usize];
        w.first_path..w.first_path + w.n_paths as u32
    };
    let word_bands: Vec<Vec<Band>> = (0..data.words.len() as u32)
        .map(|wi| trace(geometry, word_paths(wi), baselines[data.words[wi as usize].line_index as usize], &mut budget))
        .collect::<Result<_, _>>()?;

    // The page's usual gap: the median air between two words of one ayah on one printed line.
    let mut airs = Vec::new();
    for li in 0..data.lines.len() {
        for wi in line_words(li).skip(1) {
            let (a, b) = (&data.words[wi as usize - 1], &data.words[wi as usize]);
            if a.surah != b.surah || a.ayah != b.ayah {
                continue;
            }
            if let Some(air) = clearance(&word_bands[wi as usize - 1], &word_bands[wi as usize]) {
                if air > 0.0 {
                    airs.push(air);
                }
            }
        }
    }
    let gap = median(airs, pitch / 4.0);

    let mut signs: Vec<Vec<Sign>> = vec![Vec::new(); data.words.len()];
    let mut strokes = Vec::new();
    for deco in &data.decorations {
        if !matches!(deco.kind, DecoKind::AyahMark | DecoKind::DivisionMark | DecoKind::SajdahMark) {
            continue;
        }
        let all = deco.first_path..deco.first_path + deco.n_paths as u32;
        for pi in all.clone().filter(|&pi| data.paths[pi as usize].mark == Mark::SajdahLine) {
            let line = rect(&data.paths[pi as usize].bbox);
            let mut under: Option<(u32, f64)> = None;
            for (wi, w) in data.words.iter().enumerate() {
                let r = rect(&w.bbox);
                if r[0] < line[2]
                    && r[2] > line[0]
                    && r[1] >= line[1] - SAJDAH_LINE_REACH
                    && under.is_none_or(|u| r[1] < u.1)
                {
                    under = Some((wi as u32, r[1]));
                }
            }
            let (under, _) = under.ok_or(PassageError::UnattachedSign)?;
            let words = line_words(data.words[under as usize].line_index as usize)
                .filter(|&wi| {
                    let r = rect(&data.words[wi as usize].bbox);
                    r[0] < line[2] && r[2] > line[0]
                })
                .collect();
            strokes.push(Stroke { path: pi, words, rect: line });
        }
        let paths: Vec<u32> = all.filter(|&pi| data.paths[pi as usize].mark != Mark::SajdahLine).collect();
        let Some(sign_rect) = bounds(paths.iter().map(|&pi| rect(&data.paths[pi as usize].bbox))) else { continue };
        let owner = if deco.kind == DecoKind::DivisionMark {
            let middle = (sign_rect[1] + sign_rect[3]) / 2.0;
            let li = if deco.line != NONE_U16 && (deco.line as usize) < data.lines.len() {
                deco.line as usize
            } else {
                let mut best = 0;
                for li in 1..data.lines.len() {
                    if (page.line_centre(li) as f64 - middle).abs() < (page.line_centre(best) as f64 - middle).abs() {
                        best = li;
                    }
                }
                best
            };
            // The word it opens: the nearest word that ends before it, else the nearest edge.
            let mut owner: Option<(u32, f64)> = None;
            for wi in line_words(li) {
                let r = rect(&data.words[wi as usize].bbox);
                if r[2] <= sign_rect[0] + gap && owner.is_none_or(|o| r[2] > o.1) {
                    owner = Some((wi, r[2]));
                }
            }
            owner.map(|o| o.0).or_else(|| {
                let distance = |wi: u32| {
                    let r = rect(&data.words[wi as usize].bbox);
                    f64::min((r[0] - sign_rect[0]).abs(), (r[2] - sign_rect[2]).abs())
                };
                let mut best: Option<(u32, f64)> = None;
                for wi in line_words(li) {
                    let d = distance(wi);
                    if best.is_none_or(|b| d < b.1) {
                        best = Some((wi, d));
                    }
                }
                best.map(|b| b.0)
            })
        } else {
            // A medallion or a sajdah sign: the last word of its ayah.
            data.words.iter().rposition(|w| w.surah == deco.surah && w.ayah == deco.ayah).map(|wi| wi as u32)
        };
        let owner = owner.ok_or(PassageError::UnattachedSign)?;
        let baseline = baselines[data.words[owner as usize].line_index as usize];
        let bands = trace(geometry, paths.iter().copied(), baseline, &mut budget)?;
        signs[owner as usize].push(Sign { kind: deco.kind, paths, rect: sign_rect, bands });
    }

    let atoms = data
        .words
        .iter()
        .enumerate()
        .map(|(wi, w)| {
            let own = &signs[wi];
            let word_rect = rect(&w.bbox);
            let mut paths: Vec<u32> = word_paths(wi as u32).collect();
            for sign in own {
                paths.extend(&sign.paths);
            }
            let mut bands = word_bands[wi].clone();
            for sign in own {
                merge(&mut bands, &sign.bands);
            }
            Atom {
                word: wi as u32,
                line: w.line_index,
                surah: w.surah,
                ayah: w.ayah,
                number: w.word,
                paths,
                signs: own.clone(),
                rect: own.iter().fold(word_rect, |r, s| union(r, s.rect)),
                word_rect,
                bands,
                word_bands: word_bands[wi].clone(),
                baseline: baselines[w.line_index as usize],
                pitch,
                gap,
            }
        })
        .collect();
    Ok(Prepared { atoms, strokes })
}

/// Trace paths into bands of the shared height: where the ink starts and ends at each height.
/// It walks the outline, not its points: a curve's control points are far apart, so a band
/// between two of them would read as empty.
fn trace(
    geometry: &Geometry,
    paths: impl Iterator<Item = u32>,
    baseline: f64,
    budget: &mut Budget,
) -> Result<Vec<Band>, PassageError> {
    let mut bands = Silhouette { rows: BTreeMap::new(), baseline, budget };
    let (ops, pts) = (&geometry.ops, &geometry.pts);
    let steps = defaults::CURVE_STEPS;
    for pi in paths {
        let g = &geometry.table[pi as usize];
        let at = |k: usize| (pts[k] as f64, pts[k + 1] as f64);
        let (mut k, mut here, mut start) = (g.pt_start as usize, (0.0, 0.0), (0.0, 0.0));
        for oi in g.op_start..g.op_start + g.op_count {
            match ops[oi as usize] {
                OP_MOVE => {
                    here = at(k);
                    start = here;
                    k += 2;
                    bands.mark(here.0, here.1)?;
                }
                OP_LINE => {
                    let to = at(k);
                    k += 2;
                    bands.segment(here, to)?;
                    here = to;
                }
                OP_CLOSE => {
                    bands.segment(here, start)?;
                    here = start;
                }
                op @ (OP_QUAD | OP_CUBIC) => {
                    let (c1, c2) = (at(k), at(k + 2));
                    let to = if op == OP_QUAD { c2 } else { at(k + 4) };
                    k += if op == OP_QUAD { 4 } else { 6 };
                    // A curve is walked as a run of short straight pieces: close enough to its
                    // outline for a band to know where the ink is.
                    let mut previous = here;
                    for i in 1..=steps {
                        let t = i as f64 / steps as f64;
                        let u = 1.0 - t;
                        let next = if op == OP_QUAD {
                            (
                                u * u * here.0 + 2.0 * u * t * c1.0 + t * t * to.0,
                                u * u * here.1 + 2.0 * u * t * c1.1 + t * t * to.1,
                            )
                        } else {
                            (
                                u.powi(3) * here.0 + 3.0 * u * u * t * c1.0 + 3.0 * u * t * t * c2.0 + t.powi(3) * to.0,
                                u.powi(3) * here.1 + 3.0 * u * u * t * c1.1 + 3.0 * u * t * t * c2.1 + t.powi(3) * to.1,
                            )
                        };
                        bands.segment(previous, next)?;
                        previous = next;
                    }
                    here = to;
                }
                _ => {}
            }
        }
    }
    Ok(bands.rows.into_iter().map(|(band, (x0, x1))| (band, x0, x1)).collect())
}

/// A silhouette being traced, and the page budget it spends.
struct Silhouette<'a> {
    rows: BTreeMap<i32, (f64, f64)>,
    baseline: f64,
    budget: &'a mut Budget,
}

impl Silhouette<'_> {
    /// Mark one point of ink into its band.
    fn mark(&mut self, x: f64, y: f64) -> Result<(), PassageError> {
        self.budget.points -= 1;
        if self.budget.points < 0 {
            return Err(PassageError::MeasureLimit);
        }
        let band = ((y - self.baseline) / BAND).floor().clamp(i32::MIN as f64, i32::MAX as f64) as i32;
        match self.rows.get_mut(&band) {
            Some(span) => {
                span.0 = span.0.min(x);
                span.1 = span.1.max(x);
            }
            None => {
                self.budget.bands -= 1;
                if self.budget.bands < 0 {
                    return Err(PassageError::MeasureLimit);
                }
                self.rows.insert(band, (x, x));
            }
        }
        Ok(())
    }

    /// Mark a straight run of the outline into every band it crosses.
    fn segment(&mut self, a: (f64, f64), b: (f64, f64)) -> Result<(), PassageError> {
        let steps = ((b.1 - a.1).abs() / BAND).ceil().clamp(1.0, SEGMENT_STEPS) as u32;
        for i in 0..=steps {
            let t = i as f64 / steps as f64;
            self.mark(a.0 + (b.0 - a.0) * t, a.1 + (b.1 - a.1) * t)?;
        }
        Ok(())
    }
}

// ───────────────────────────── helpers ─────────────────────────────

/// Break a passage into rows of `width` without stretching its gaps: each row costs the square
/// of its slack, plus one, and the last row's slack is free, so the rows come out balanced.
fn break_rows(atoms: &[&Atom], gaps: &[f64], width: f64) -> Vec<Vec<usize>> {
    let n = atoms.len();
    let mut best = vec![f64::INFINITY; n + 1];
    let mut previous = vec![0usize; n + 1];
    best[0] = 0.0;
    for end in 1..=n {
        let mut used = 0.0;
        for start in (0..end).rev() {
            used += atoms[start].rect[2] - atoms[start].rect[0] + if start < end - 1 { gaps[start + 1] } else { 0.0 };
            if used > width + OVERRUN {
                break;
            }
            let slack = width - used;
            let cost = best[start] + if end == n { 0.0 } else { slack * slack } + 1.0;
            if cost < best[end] {
                best[end] = cost;
                previous[end] = start;
            }
        }
    }
    let mut rows = Vec::new();
    let mut end = n;
    while end > 0 {
        let start = previous[end];
        rows.push((start..end).collect());
        end = start;
    }
    rows.reverse();
    rows
}

/// Fill rows of `width` in turn, each with as many words as it holds.
fn fill_rows(atoms: &[&Atom], gaps: &[f64], width: f64) -> Vec<Vec<usize>> {
    let mut rows: Vec<Vec<usize>> = Vec::new();
    let mut row: Vec<usize> = Vec::new();
    let mut used = 0.0;
    for (i, atom) in atoms.iter().enumerate() {
        let w = atom.rect[2] - atom.rect[0];
        let with = used + w + if row.is_empty() { 0.0 } else { gaps[i] };
        if row.is_empty() || with <= width + OVERRUN {
            row.push(i);
            used = with;
        } else {
            rows.push(std::mem::take(&mut row));
            row.push(i);
            used = w;
        }
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}

/// The air between two silhouettes side by side: the narrowest distance from `a`'s leftmost ink
/// to `b`'s rightmost over the bands they share. `a` is the right-hand, earlier word. `None`
/// when they share no band.
fn clearance(a: &[Band], b: &[Band]) -> Option<f64> {
    let mut air = f64::INFINITY;
    let (mut i, mut j) = (0, 0);
    while i < a.len() && j < b.len() {
        match a[i].0.cmp(&b[j].0) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                air = air.min(a[i].1 - b[j].2);
                i += 1;
                j += 1;
            }
        }
    }
    air.is_finite().then_some(air)
}

/// Widen the bands of `into` by those of `other`.
fn merge(into: &mut Vec<Band>, other: &[Band]) {
    for &(band, x0, x1) in other {
        match into.binary_search_by_key(&band, |b| b.0) {
            Ok(i) => {
                into[i].1 = into[i].1.min(x0);
                into[i].2 = into[i].2.max(x1);
            }
            Err(i) => into.insert(i, (band, x0, x1)),
        }
    }
}

/// The upper middle value, or `fallback` for none: the lite module's median.
fn median(mut values: Vec<f64>, fallback: f64) -> f64 {
    if values.is_empty() {
        return fallback;
    }
    values.sort_by(f64::total_cmp);
    values[values.len() / 2]
}

fn bounds(rects: impl Iterator<Item = Rect>) -> Option<Rect> {
    rects.reduce(union)
}

fn union(a: Rect, b: Rect) -> Rect {
    [a[0].min(b[0]), a[1].min(b[1]), a[2].max(b[2]), a[3].max(b[3])]
}

fn moved(r: Rect, dx: f64, dy: f64) -> Rect {
    [r[0] + dx, r[1] + dy, r[2] + dx, r[3] + dy]
}

#[cfg(test)]
mod tests {
    use super::*;
    use qvp_format::*;

    fn rect_cmds(b: [i32; 4]) -> Vec<Cmd> {
        vec![
            Cmd::MoveTo(b[0], b[1]),
            Cmd::LineTo(b[2], b[1]),
            Cmd::LineTo(b[2], b[3]),
            Cmd::LineTo(b[0], b[3]),
            Cmd::Close,
        ]
    }

    /// Three words of one ayah on one line, its medallion and a sajdah line over the middle
    /// word: the lite module's synthetic page, in hundredths of a page unit.
    fn synthetic_data(number: u16, ayah: u16) -> PageData {
        let mut ops = Vec::new();
        let mut paths = Vec::new();
        let boxes = [
            ([8000, 500, 10000, 2000], PathKind::Body, Mark::None),
            ([4000, 500, 6500, 2000], PathKind::Body, Mark::None),
            ([1000, 600, 3000, 2000], PathKind::Body, Mark::None),
            ([0, 600, 800, 2000], PathKind::AyahMarkOrnament, Mark::None),
            ([4200, 200, 6300, 300], PathKind::Mark, Mark::SajdahLine),
        ];
        for (b, kind, mark) in boxes {
            let cmds = rect_cmds(b);
            let bb = cmds_bbox(&cmds);
            let off = ops.len() as u32;
            encode_cmds(&cmds, bb.x0, bb.y0, &mut ops);
            paths.push(PathRec {
                kind,
                mark,
                family: Family::None,
                flags: 0,
                ox: bb.x0,
                oy: bb.y0,
                op_off: off,
                op_len: ops.len() as u32 - off,
                bbox: bb,
            });
        }
        let bbox = |i: usize| paths[i].bbox;
        let word = |i: usize| WordRec {
            surah: 1,
            ayah,
            word: i as u16 + 1,
            line_index: 0,
            ayah_index: 0,
            text: i as u16,
            rasm_imlai: i as u16,
            qpc: i as u16,
            rasm: i as u16,
            search: i as u16,
            first_path: i as u32,
            n_paths: 1,
            bbox: bbox(i),
        };
        let mut line = bbox(0);
        line.union(&bbox(2));
        PageData {
            header: Header { version: VERSION, quant: 100, page: number, flags: 0, width: 110.0, height: 40.0 },
            lines: vec![LineRec { line_number: 1, first_word: 0, n_words: 3, bbox: line }],
            ayahs: vec![AyahRec {
                surah: 1,
                ayah,
                fragment: 1,
                fragments: 1,
                flags: 0,
                first_word: 0,
                n_words: 3,
                ayah_mark_decoration: 0,
                rubu_al_hizb: 0,
                bbox: line,
            }],
            words: (0..3).map(word).collect(),
            decorations: vec![
                DecoRec {
                    kind: DecoKind::AyahMark,
                    surah: 1,
                    ayah,
                    text: NONE_U16,
                    first_path: 3,
                    n_paths: 1,
                    line: NONE_U16,
                    bbox: bbox(3),
                },
                DecoRec {
                    kind: DecoKind::SajdahMark,
                    surah: 1,
                    ayah,
                    text: NONE_U16,
                    first_path: 4,
                    n_paths: 1,
                    line: 0,
                    bbox: bbox(4),
                },
            ],
            paths,
            glyphs: vec![],
            insts: vec![],
            ops,
            strings: vec!["1".into(), "2".into(), "3".into()],
        }
    }

    fn synthetic(number: u16, ayah: u16) -> Page {
        Page::load(&encode(&synthetic_data(number, ayah))).unwrap()
    }

    /// Every box of a layout lies inside it.
    fn bounded(l: &PassageLayout) {
        let inside = |[x0, y0, x1, y1]: [f32; 4]| {
            [x0, y0, x1, y1].iter().all(|v| v.is_finite())
                && x0 >= -0.001
                && y0 >= -0.001
                && x1 <= l.width + 0.001
                && y1 <= l.height + 0.001
        };
        for w in &l.words {
            assert!(inside([w.x0, w.y0, w.x1, w.y1]), "word outside {}×{}", l.width, l.height);
        }
        for r in &l.rows {
            assert!(inside([r.x0, r.y0, r.x1, r.y1]), "row outside {}×{}", l.width, l.height);
        }
        for a in &l.ayahs {
            assert!(inside([a.x0, a.y0, a.x1, a.y1]), "ayah outside {}×{}", l.width, l.height);
        }
        if let Some(e) = l.ellipsis {
            assert!(inside(e), "ellipsis outside {}×{}", l.width, l.height);
        }
    }

    fn spec(width: f32) -> PassageSpec {
        PassageSpec { width, ..Default::default() }
    }

    #[test]
    fn a_range_across_pages_wraps_in_reading_order() {
        let pages = [synthetic(1, 1), synthetic(2, 2)];
        let mut passage = Passage::load(&[&pages[0], &pages[1]], 1, 1, 2).unwrap();
        assert_eq!(passage.ayahs().map(|(_, _, t)| t.to_owned()).collect::<Vec<_>>(), ["1 2 3", "1 2 3"]);
        let wide = passage.layout(&spec(240.0)).unwrap().clone();
        let narrow = passage.layout(&spec(70.0)).unwrap().clone();
        let tiny = passage.layout(&PassageSpec { width: 15.0, scale: 2.0, ..Default::default() }).unwrap().clone();
        assert_eq!(wide.words.len(), 6);
        assert_eq!(wide.ayahs.len(), 2);
        assert!(narrow.rows.len() > wide.rows.len());
        assert!(tiny.scale < 2.0);
        let order: Vec<(u16, u16)> = narrow.words.iter().map(|w| (w.ayah, w.word)).collect();
        assert_eq!(order, [(1, 1), (1, 2), (1, 3), (2, 1), (2, 2), (2, 3)]);
        let centred = passage.layout(&PassageSpec { align: Align::Center, ..spec(70.0) }).unwrap().clone();
        for l in [&wide, &narrow, &tiny, &centred] {
            bounded(l);
            assert!(!l.is_truncated);
            assert!(l.ellipsis.is_none());
        }
        assert_eq!(passage.layout(&spec(70.0)).unwrap(), &narrow);
    }

    #[test]
    fn every_path_is_drawn_once_and_the_sajdah_line_over_its_words() {
        let pages = [synthetic(1, 1), synthetic(2, 2)];
        let mut passage = Passage::load(&[&pages[0], &pages[1]], 1, 1, 2).unwrap();
        let l = passage.layout(&spec(70.0)).unwrap().clone();
        // Three words, a medallion and a sajdah line on each page.
        assert_eq!(l.draws.len(), 10);
        for page in [1u16, 2] {
            for path in 0..5u32 {
                assert_eq!(l.draws.iter().filter(|d| d.page == page && d.path == path).count(), 1, "{page}/{path}");
            }
        }
        // The medallion moves with the word that closes its ayah.
        for page in [1u16, 2] {
            let at = |path: u32| l.draws.iter().find(|d| d.page == page && d.path == path).unwrap().placement;
            assert_eq!(at(3), at(2));
        }
        // The sajdah line spans the middle word wherever it lands, and only along x.
        for page in [1u16, 2] {
            let stroke = l.draws.iter().find(|d| d.page == page && d.path == 4).unwrap();
            let p = l.placements[stroke.placement as usize];
            let word = l.words.iter().find(|w| w.page == page && w.word == 2).unwrap();
            let (x0, x1) = ((p.kx * 42.0 + p.dx) * l.scale, (p.kx * 63.0 + p.dx) * l.scale);
            assert!(
                (x0 - word.x0).abs() < 0.01 && (x1 - word.x1).abs() < 0.01,
                "{x0}..{x1} vs {}..{}",
                word.x0,
                word.x1
            );
            assert_eq!(p.ky, 1.0);
        }
    }

    #[test]
    fn an_incomplete_or_repeated_range_fails() {
        let one = synthetic(1, 1);
        assert_eq!(Passage::load(&[&one], 1, 1, 2).err(), Some(PassageError::IncompleteAyah { surah: 1, ayah: 2 }));
        assert_eq!(Passage::load(&[&one, &one], 1, 1, 1).err(), Some(PassageError::RepeatedPage));
        // A word count that disagrees with the words is a broken page, which the codec refuses.
        for change in [|a: &mut AyahRec| a.fragment = 2, |a: &mut AyahRec| a.fragments = 2] {
            let mut d = synthetic_data(1, 1);
            change(&mut d.ayahs[0]);
            let p = Page::from_data(d);
            assert_eq!(Passage::load(&[&p], 1, 1, 1).err(), Some(PassageError::IncompleteAyah { surah: 1, ayah: 1 }));
        }
        for (surah, from, to) in [(0, 1, 1), (115, 1, 1), (1, 0, 1), (1, 2, 1), (1, 1, 287)] {
            assert_eq!(Passage::load(&[&one], surah, from, to).err(), Some(PassageError::Range));
        }
    }

    #[test]
    fn a_spec_out_of_range_lays_nothing_out() {
        let p = synthetic(1, 1);
        let mut passage = Passage::load(&[&p], 1, 1, 1).unwrap();
        for bad in [
            spec(f32::NAN),
            PassageSpec { padding: 10.0, ..spec(20.0) },
            PassageSpec { scale: -1.0, ..spec(70.0) },
            PassageSpec { line_spacing: 0.5, ..spec(70.0) },
            PassageSpec { ellipsis_width: -1.0, ..spec(70.0) },
            spec(40_000.0),
        ] {
            assert!(passage.layout(&bad).is_none(), "{bad:?}");
        }
        assert!(passage.current_layout().is_none());
    }

    #[test]
    fn no_width_is_one_row_as_wide_as_the_passage() {
        let pages = [synthetic(1, 1), synthetic(2, 2)];
        let mut passage = Passage::load(&[&pages[0], &pages[1]], 1, 1, 2).unwrap();
        let l = passage.layout(&spec(0.0)).unwrap().clone();
        assert_eq!(l.rows.len(), 1);
        assert_eq!(l.words.len(), 6);
        assert_eq!(l.scale, 1.0);
        let row = l.rows[0];
        assert!((row.x0 - 2.0).abs() < 0.01 && (row.x1 - (l.width - 2.0)).abs() < 0.01);
        bounded(&l);
    }

    #[test]
    fn a_row_limit_cuts_after_a_whole_word_and_keeps_the_last_medallion() {
        let pages = [synthetic(1, 1), synthetic(2, 2)];
        let mut passage = Passage::load(&[&pages[0], &pages[1]], 1, 1, 2).unwrap();
        let full = passage.layout(&spec(70.0)).unwrap().clone();
        assert!(full.rows.len() > 1);
        let cut = PassageSpec { max_rows: 1, ellipsis_width: 6.0, ..spec(70.0) };
        let l = passage.layout(&cut).unwrap().clone();
        assert!(l.is_truncated);
        assert_eq!(l.rows.len(), 1);
        assert!(!l.words.is_empty() && l.words.len() < 6);
        let ellipsis = l.ellipsis.unwrap();
        assert!((ellipsis[2] - ellipsis[0] - 6.0).abs() < 0.01);
        let last = l.words.last().unwrap();
        assert!(ellipsis[2] <= last.x0, "the ellipsis follows the last word");
        assert_eq!(ellipsis[3], l.rows[0].baseline);
        bounded(&l);
        // The passage's last medallion is not drawn without asking for it.
        assert!(!l.draws.iter().any(|d| d.page == 2 && d.path == 3));

        let kept = passage.layout(&PassageSpec { keep_ayah_mark: true, ..cut }).unwrap().clone();
        assert!(kept.is_truncated);
        let medallion = kept.draws.iter().find(|d| d.page == 2 && d.path == 3).expect("the medallion is kept");
        let p = kept.placements[medallion.placement as usize];
        let (x0, x1) = ((p.dx) * kept.scale, (8.0 + p.dx) * kept.scale);
        let ellipsis = kept.ellipsis.unwrap();
        assert!(x1 <= ellipsis[0] + 0.001 && x0 >= -0.001, "the medallion follows the ellipsis");
        bounded(&kept);

        // A limit the passage fits in cuts nothing.
        let fits = passage.layout(&PassageSpec { max_rows: full.rows.len() as u32, ..cut }).unwrap().clone();
        assert!(!fits.is_truncated && fits.ellipsis.is_none());
        assert_eq!(fits.words.len(), 6);
    }

    #[test]
    fn outlines_that_would_amplify_hit_the_measuring_budget() {
        for unbounded in [true, false] {
            let mut d = synthetic_data(1, 1);
            // Inside the page's box: ink outside it has no outline to measure.
            let mut cmds = vec![Cmd::MoveTo(500, 0)];
            for i in 1..=4000 {
                cmds.push(Cmd::LineTo(500, if unbounded { i * 6300 } else { (i % 2) * 20000 }));
            }
            let bb = cmds_bbox(&cmds);
            let off = d.ops.len() as u32;
            encode_cmds(&cmds, bb.x0, bb.y0, &mut d.ops);
            d.paths[0] =
                PathRec { ox: bb.x0, oy: bb.y0, op_off: off, op_len: d.ops.len() as u32 - off, bbox: bb, ..d.paths[0] };
            let p = Page::from_data(d);
            assert_eq!(Passage::load(&[&p], 1, 1, 1).err(), Some(PassageError::MeasureLimit));
        }
    }
}
