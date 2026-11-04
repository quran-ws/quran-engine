//! Horizontal placement: where each row unit sits along its row, the marks set in the margin
//! beside it, and the medallion set between two ayahs.
use super::{Atom, Fill, Placement, ReflowSpec, Reflowed, RowSpec};
use crate::defaults;
use crate::Page;
use qvp_format::DecoKind;
use std::collections::HashMap;

/// What a row holds before anything is opened up: its ink, the gaps the print or the page's
/// own spacing give it, and which of those gaps may stretch.
struct Measured {
    ink: f32,
    gaps: Vec<f32>,
    open: Vec<bool>,
}

impl Page {
    /// The x shift of every row unit from its printed position, by row and then by unit in
    /// reading order. A banner row has none. `reproduced` is true when the rows came out as
    /// the printed lines, which keeps every word where the page has it.
    pub(super) fn place_across(
        &self,
        rows: &[Vec<&Atom>],
        headers: &HashMap<usize, u32>,
        spec: &ReflowSpec,
        shape: &RowSpec,
        reproduced: bool,
    ) -> Vec<Vec<f32>> {
        let (block, row_w) = (shape.block, shape.row_w);
        let median = self.reflow_prepared().median;
        let gap_of = |prev: &Atom, next: &Atom| self.atom_gap(prev, next, spec, median);
        let last_row = rows.len().saturating_sub(1);
        let measured: Vec<Measured> = rows
            .iter()
            .map(|atoms| Measured {
                ink: atoms.iter().map(|a| a.width).sum(),
                gaps: atoms.windows(2).map(|p| gap_of(p[0], p[1])).collect(),
                open: atoms.windows(2).map(|p| !self.interlocked(p[0], p[1])).collect(),
            })
            .collect();
        let used = |r: usize| measured[r].ink + measured[r].gaps.iter().sum::<f32>();
        // What a row is evened against: the widest row near it. The window slides, so a row is
        // measured against its own neighbours rather than against whichever group of rows it
        // happened to fall in, and it holds a fixed number of rows, so a taller screen does not
        // change how a page is set.
        let reach = defaults::RELAX_NEIGHBOURS;
        let widest: Vec<f32> = (0..rows.len())
            .map(|r| {
                let (lo, hi) = (r.saturating_sub(reach), (r + reach).min(rows.len().saturating_sub(1)));
                (lo..=hi)
                    .filter(|&k| !headers.contains_key(&k) && !rows[k].is_empty())
                    .fold(0.0f32, |w, k| w.max(used(k)))
            })
            .collect();
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
                    // A row is opened towards the widest row near it, because those are the
                    // rows a reader sees beside it and compares it with. It is a share of the
                    // way and not the whole of it: the row is never justified, and
                    // `max_stretch` still caps how far its gaps may go.
                    _ if spec.relax > 0.0 => {
                        let target = widest.get(r).copied().unwrap_or(row_w).min(row_w);
                        let mine = ink + natural;
                        if target > mine {
                            widen(&mut gaps, (target - mine) * spec.relax.min(1.0));
                        }
                    }
                    _ => {}
                }
            }
            if reproduced {
                // the print reproduced: the words stay exactly where the page has them
                dxs.extend(atoms.iter().map(|_| -block.0));
                row_dx.push(dxs);
                continue;
            }
            // A centred row starts half its leftover space in from the right margin, and so does
            // a justified row that could not reach both: the last of a block, a single word, or
            // one its gaps could not fill. Hung off the right margin, it reads as a ragged column
            // under a justified block.
            let mut cursor = row_w;
            if matches!(spec.fill, Fill::Centred | Fill::Justified) {
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
        row_dx
    }

    /// Where a margin mark goes once its word is on a row `dy` below its printed line: `w` is
    /// the mark's ink width.
    ///
    /// A margin mark keeps the side of the sheet the print puts it on and the height it was
    /// printed at; it only follows its word down to the new row. The margin narrows as the
    /// reader zooms in, so the mark is held inside it.
    pub(super) fn margin_mark_place(&self, di: u32, w: f32, dy: f32, shape: &RowSpec) -> Placement {
        let (block, margins, row_w) = (shape.block, shape.margins, shape.row_w);
        let q = self.quant();
        let b = self.deco_mark_bbox(di as usize);
        let (bx0, bx1) = (b.x0 as f32 / q, b.x1 as f32 / q);
        let left = (bx0 + bx1) / 2.0 < (block.0 + block.1) / 2.0;
        let x1 = if left {
            // outside the row's left edge, and never past the margin the page has
            (bx1 - block.0).min(0.0).max(w - margins.0)
        } else {
            (bx0 - block.0).max(row_w).min(row_w + margins.1 - w) + w
        };
        Placement::moved(x1 - bx1, dy)
    }

    /// A medallion stands between the ayah it closes and the one that follows. Where both
    /// words share a row it is set midway between them, rather than at the distance the
    /// print happened to give it on a line it is no longer on. Moving it changes nothing
    /// else: the words either side stay where the row put them.
    ///
    /// Each side is the whole unit the row placed, the word and the signs that travel with
    /// it, such as the quarter star (۞) that opens the next ayah. Measured on the letters
    /// alone, the medallion was centred on a space the star half filled, and sat against it.
    pub(super) fn centre_medallions(&self, out: &mut Reflowed, rows: &[Vec<&Atom>]) {
        let d = self.data();
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
                    let others: Vec<u32> = right_word.inline_decos.iter().copied().filter(|&o| o != di).collect();
                    let right_ink = self.slices_with(right_word.word, &others);
                    let before = Page::slice_clearance(&right_ink, self.deco_slices(di), dx_m - dx_a);
                    let after = Page::slice_clearance(self.deco_slices(di), &left_word.slices, dx_b - dx_m);
                    if let (Some(before), Some(after)) = (before, after) {
                        out.deco_place[di as usize].dx += (before - after) / 2.0;
                    }
                }
            }
        }
    }
}
