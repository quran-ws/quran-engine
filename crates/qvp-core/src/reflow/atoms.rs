//! Row units: a word with the ink that travels with it, and how two of them meet.
use super::{GapMode, ReflowSpec, NONE};
use crate::Page;
use qvp_format::{DecoKind, NONE_U16};

/// One unit that a row holds whole: a word, the ink printed with it (the medallion that
/// closes its ayah, the sajdah line drawn over it) and the marks the print puts in the
/// margin, which a reflowed row has no margin for.
pub(super) struct Atom {
    pub(super) word: u32,
    /// decorations that keep their printed offset from the word
    pub(super) inline_decos: Vec<u32>,
    /// margin marks, placed after the word in reading order: (decoration, ink width)
    pub(super) margin_decos: Vec<(u32, f32)>,
    /// printed extent of the word and its inline ink, in page units
    pub(super) x0: f32,
    pub(super) x1: f32,
    /// the atom's letters and inline marks sliced into bands: what the air is measured on
    pub(super) slices: Vec<(i16, f32, f32)>,
    /// what the atom takes off a row: its printed extent plus room for the margin marks
    pub(super) width: f32,
}

/// A word measured against the word before it in reading order: how deeply the print
/// interlocks the two ([`Page::words_interlock`]) and the air between their row units'
/// strokes ([`Page::slice_clearance`]). Every gap a reflow sets is between two such
/// neighbours, so both are measured once.
#[derive(Clone, Copy)]
pub(super) struct Neighbour {
    /// The word before, or `NONE` for the first word of the page.
    pub(super) prev: u32,
    pub(super) interlock: Option<f32>,
    pub(super) clearance: Option<f32>,
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
        gaps.sort_by(f32::total_cmp);
        gaps[gaps.len() / 2]
    }

    /// The decoration that closes each ayah, resolved to the word it must travel with.
    /// `NONE` for a decoration that belongs to no single word: a surah banner, a basmalah,
    /// the page number, the running head, or a sajdah line, which spans several words.
    /// `median` is the page's [`Page::median_word_gap`], the tolerance a mark is matched
    /// to its word with.
    pub(crate) fn deco_anchor_words(&self, median: f32) -> Vec<u32> {
        let d = self.data();
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
                    // a rubu_al_hizb opens a division, so it travels with the word it opens
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
    /// was resolved to. A rubu_al_hizb stands where its division starts, so it travels with the
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

    /// How two row units meet: the depth the print interlocks them to, and the air between
    /// their strokes. Read off the page's measure when `next` follows `prev` in reading order,
    /// which is every pair a reflow sets a gap between, and measured on the spot otherwise.
    pub(super) fn neighbour(&self, prev: &Atom, next: &Atom) -> (Option<f32>, Option<f32>) {
        let n = &self.reflow_prepared().before[next.word as usize];
        debug_assert_eq!(n.prev, prev.word, "a gap between two words that are not reading-order neighbours");
        if n.prev == prev.word {
            (n.interlock, n.clearance)
        } else {
            (self.words_interlock(prev.word, next.word), Page::slice_clearance(&prev.slices, &next.slices, 0.0))
        }
    }

    /// Whether the print draws two row units as one piece of calligraphy, so the gap between
    /// them keeps its printed width and never stretches.
    pub(super) fn interlocked(&self, prev: &Atom, next: &Atom) -> bool {
        self.neighbour(prev, next).0.is_some()
    }

    pub(super) fn atom_gap(&self, prev: &Atom, next: &Atom, spec: &ReflowSpec, median: f32) -> f32 {
        let d = self.data();
        let (interlock, clearance) = self.neighbour(prev, next);
        // A word the print carries under the one before it keeps the distance the print
        // gave it, whatever the rest of the row is set at: the pair is one piece of
        // calligraphy. Only a row break between them sets them as two ordinary words.
        if interlock.is_some() {
            return prev.x0 - next.x1;
        }
        let air = median * spec.word_gap.max(0.0);
        match spec.gaps {
            // Leave the same air between the strokes of every pair. Shifting the second
            // word by `t` changes the air and the box gap alike, so the box gap that
            // leaves the wanted air is the printed one moved by the difference.
            GapMode::Uniform => match clearance {
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
                    match clearance {
                        Some(now) => (prev.x0 - next.x1) + (air - now),
                        None => air,
                    }
                }
            }
        }
    }

    /// Build the row unit for a word: the ink printed with it keeps its printed offset, and
    /// a mark the print puts in the margin is re-placed beside the word, where a reflowed
    /// row has room for it.
    pub(super) fn atom(&self, wi: u32, decos: &[u32], median: f32, block: (f32, f32)) -> Atom {
        let (x0, x1, inline_decos, margin_decos) = self.atom_extent(wi, decos, median, block);
        let slices = self.slices_with(wi, &inline_decos);
        Atom { word: wi, inline_decos, margin_decos, x0, x1, width: x1 - x0, slices }
    }

    /// The extent of a row unit and how its decorations travel, without its bands: the
    /// printed extent of the word and its inline ink, the decorations that run inline and the
    /// marks placed in the margin with their ink width.
    #[allow(clippy::type_complexity)]
    pub(super) fn atom_extent(
        &self,
        wi: u32,
        decos: &[u32],
        median: f32,
        block: (f32, f32),
    ) -> (f32, f32, Vec<u32>, Vec<(u32, f32)>) {
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
        (x0, x1, inline_decos, margin_decos)
    }
}
