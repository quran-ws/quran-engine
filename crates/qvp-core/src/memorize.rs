//! Memorisation: masking words (hide / block / blur) with progressive reveal, and
//! the greyed-page reading position.
use crate::{Page, Rgba, Target, NONE};
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum MaskMode {
    /// ink alpha 0 — the page keeps its shape and spacing
    Hide = 0,
    /// host draws an opaque box over each hidden word (see `mask_boxes_view`)
    Block = 1,
    /// host blurs each hidden word's box
    Blur = 2,
}

#[derive(Clone, Debug)]
pub struct MaskState {
    /// every word under the mask, reading order
    pub words: Vec<u32>,
    pub hidden: BTreeSet<u32>,
    pub mode: MaskMode,
    pub block_color: Rgba,
    pub pad_x: f32,
    pub pad_y: f32,
    pub radius: f32,
    pub reverse: bool,
}

impl Default for MaskState {
    fn default() -> Self {
        MaskState { words: vec![], hidden: BTreeSet::new(), mode: MaskMode::Hide, block_color: 0xd9d4c8ff, pad_x: 0.6, pad_y: 0.6, radius: 0.8, reverse: false }
    }
}

/// The page greyed except a window of `lit` steps ending at `at`.
#[derive(Clone, Debug)]
pub struct Reveal {
    pub at: i64,
    pub lit: u32,
    pub by_ayah: bool,
    pub grey: Rgba,
    pub ink: Rgba,
    pub markers: bool,
    pub transition_ms: u32,
    /// step index per word
    pub(crate) step_of_word: Vec<u32>,
    pub steps: u32,
}

impl Reveal {
    pub(crate) fn color_for(&self, page: &Page, pi: u32) -> Option<Rgba> {
        let c = &page.path_ctx[pi as usize];
        let step = if c.word != NONE {
            self.step_of_word[c.word as usize] as i64
        } else if self.markers && c.deco != NONE && c.ayah != 0 && page.data().decos[c.deco as usize].kind == qvp_format::DecoKind::AyahMarker {
            // a medallion lights with the ayah it closes: its last word's step
            match page.data().words.iter().rposition(|w| w.sura == c.sura && w.ayah == c.ayah) {
                Some(wi) => self.step_of_word[wi] as i64,
                None => return Some(self.grey),
            }
        } else {
            return Some(self.grey);
        };
        let lo = self.at - self.lit as i64;
        Some(if step > lo && step <= self.at { self.ink } else { self.grey })
    }
}

impl Page {
    /// Mask a target. Everything in it is hidden; reveal progressively.
    pub fn mask(&mut self, target: &Target, mode: MaskMode) {
        let words = self.resolve(target);
        self.mask.words = words.clone();
        self.mask.hidden = words.into_iter().collect();
        self.mask.mode = mode;
        self.state_dirty = true;
    }
    /// Mask everything from a word onward.
    pub fn mask_from(&mut self, wi: u32, mode: MaskMode) {
        let n = self.data().words.len() as u32;
        self.mask(&Target::Range(wi, n.saturating_sub(1)), mode);
    }
    pub fn mask_words(&self) -> &[u32] {
        &self.mask.words
    }
    pub fn mask_hidden(&self) -> Vec<u32> {
        self.mask.hidden.iter().copied().collect()
    }
    pub fn mask_hidden_count(&self) -> usize {
        self.mask.hidden.len()
    }
    /// Reveal the next `n` words in order (reverse order when `reverse`). Returns how many changed.
    pub fn reveal_next(&mut self, n: usize) -> usize {
        let order: Vec<u32> = if self.mask.reverse { self.mask.words.iter().rev().copied().collect() } else { self.mask.words.clone() };
        let mut done = 0;
        for w in order {
            if done >= n {
                break;
            }
            if self.mask.hidden.remove(&w) {
                done += 1;
            }
        }
        if done > 0 {
            self.state_dirty = true;
        }
        done
    }
    /// Re-hide the last `n` revealed words.
    pub fn hide_back(&mut self, n: usize) -> usize {
        let order: Vec<u32> = if self.mask.reverse { self.mask.words.clone() } else { self.mask.words.iter().rev().copied().collect() };
        let mut done = 0;
        for w in order {
            if done >= n {
                break;
            }
            if !self.mask.hidden.contains(&w) {
                self.mask.hidden.insert(w);
                done += 1;
            }
        }
        if done > 0 {
            self.state_dirty = true;
        }
        done
    }
    pub fn reveal_word(&mut self, wi: u32) -> bool {
        let r = self.mask.hidden.remove(&wi);
        self.state_dirty |= r;
        r
    }
    pub fn hide_word(&mut self, wi: u32) -> bool {
        if !self.mask.words.contains(&wi) {
            self.mask.words.push(wi);
            self.mask.words.sort_unstable();
        }
        let r = self.mask.hidden.insert(wi);
        self.state_dirty |= r;
        r
    }
    pub fn reveal_all(&mut self) {
        self.mask.hidden.clear();
        self.state_dirty = true;
    }
    pub fn hide_all(&mut self) {
        self.mask.hidden = self.mask.words.iter().copied().collect();
        self.state_dirty = true;
    }
    pub fn unmask(&mut self) {
        self.mask = MaskState::default();
        self.state_dirty = true;
    }
    pub fn set_mask_options(&mut self, block_color: Rgba, pad_x: f32, pad_y: f32, radius: f32, reverse: bool) {
        self.mask.block_color = block_color;
        self.mask.pad_x = pad_x;
        self.mask.pad_y = pad_y;
        self.mask.radius = radius;
        self.mask.reverse = reverse;
    }
    pub fn mask_mode(&self) -> MaskMode {
        self.mask.mode
    }

    /// Boxes the host must draw over hidden words in Block/Blur mode (viewport px).
    pub fn mask_boxes_view(&self) -> Vec<crate::highlight::ViewBox> {
        if self.mask.mode == MaskMode::Hide || self.mask.hidden.is_empty() {
            return vec![];
        }
        let q = self.quant();
        let (scale, ox, oy, dy): (f32, f32, f32, Vec<f32>) = match self.current_layout() {
            Some(l) => (l.scale, l.ox, l.oy, l.line_dy.clone()),
            None => (1.0, 0.0, 0.0, vec![0.0; self.data().lines.len()]),
        };
        self.mask
            .hidden
            .iter()
            .map(|&wi| {
                let w = &self.data().words[wi as usize];
                let d = dy[w.line_idx as usize];
                crate::highlight::ViewBox {
                    highlight: 0,
                    line: w.line_idx as u32,
                    x0: ox + (w.bbox.x0 as f32 / q - self.mask.pad_x) * scale,
                    y0: oy + (w.bbox.y0 as f32 / q - self.mask.pad_y + d) * scale,
                    x1: ox + (w.bbox.x1 as f32 / q + self.mask.pad_x) * scale,
                    y1: oy + (w.bbox.y1 as f32 / q + self.mask.pad_y + d) * scale,
                    color: self.mask.block_color,
                    radius: self.mask.radius * scale,
                }
            })
            .collect()
    }

    /// Start the greyed-page reveal. Steps are words (or ayahs) in reading order.
    pub fn reveal_start(&mut self, lit: u32, by_ayah: bool, grey: Rgba, ink: Rgba, markers: bool, transition_ms: u32) -> u32 {
        let d = self.data();
        let mut step_of_word = Vec::with_capacity(d.words.len());
        let mut steps = 0u32;
        let mut last = (u16::MAX, u16::MAX);
        for (i, w) in d.words.iter().enumerate() {
            if by_ayah {
                if (w.sura, w.ayah) != last {
                    if i > 0 {
                        steps += 1;
                    }
                    last = (w.sura, w.ayah);
                }
                step_of_word.push(steps);
            } else {
                step_of_word.push(i as u32);
            }
        }
        steps = if by_ayah { steps + 1 } else { d.words.len() as u32 };
        self.reveal = Some(Reveal { at: -1, lit: lit.max(1), by_ayah, grey, ink, markers, transition_ms, step_of_word, steps });
        self.state_dirty = true;
        steps
    }
    pub fn reveal_goto(&mut self, at: i64) -> bool {
        match &mut self.reveal {
            Some(r) => {
                r.at = at.clamp(-1, r.steps as i64 - 1);
                self.state_dirty = true;
                true
            }
            None => false,
        }
    }
    pub fn reveal_at(&self) -> Option<i64> {
        self.reveal.as_ref().map(|r| r.at)
    }
    pub fn reveal_steps(&self) -> u32 {
        self.reveal.as_ref().map(|r| r.steps).unwrap_or(0)
    }
    /// Step index of a word under the current reveal (word index when by word).
    pub fn reveal_step_of(&self, wi: u32) -> Option<u32> {
        self.reveal.as_ref().map(|r| r.step_of_word[wi as usize])
    }
    pub fn reveal_stop(&mut self) {
        if self.reveal.take().is_some() {
            self.state_dirty = true;
        }
    }
}
