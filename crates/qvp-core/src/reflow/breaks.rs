//! Breaking a page into rows: the blocks text flows through, and where each block is cut.
use super::{Atom, Breaks, ReflowSpec};
use crate::Page;
use std::collections::{HashMap, HashSet};

/// The atoms of the page in reading order, cut into blocks by the banners that text never
/// flows across.
enum Block<'a> {
    Banner(u32),
    Text(Vec<&'a Atom>),
}

impl Page {
    /// Break the page's words into rows `row_w` page units wide: the row units of every row in
    /// reading order, and the printed banner line each banner row holds, keyed by row. A banner
    /// row holds no row unit.
    pub(super) fn break_rows(
        &self,
        spec: &ReflowSpec,
        row_w: f32,
        block: (f32, f32),
    ) -> (Vec<Vec<&Atom>>, HashMap<usize, u32>) {
        let mut rows: Vec<Vec<&Atom>> = Vec::new();
        let mut header_rows: Vec<(usize, u32)> = Vec::new(); // (row index, printed line)
        for item in self.text_blocks(spec, row_w, block) {
            match item {
                Block::Banner(li) => {
                    header_rows.push((rows.len(), li));
                    rows.push(Vec::new());
                }
                Block::Text(atoms) => {
                    let cuts: HashSet<usize> = self.cut_points(&atoms, spec, row_w).into_iter().collect();
                    let mut row: Vec<&Atom> = Vec::new();
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
        (rows, header_rows.into_iter().collect())
    }

    /// The atoms of the page in reading order, and the banner lines that interrupt them.
    fn text_blocks(&self, spec: &ReflowSpec, row_w: f32, block: (f32, f32)) -> Vec<Block<'_>> {
        let d = self.data();
        let prep = self.reflow_prepared();
        let mut cur: Vec<&Atom> = Vec::new();
        // Text flows from one printed line into the next only when the reader has zoomed in and
        // the rows are narrower than the printed lines. At the printed size the page is the
        // printed page: every line keeps its own row, including the short ones a surah ends on.
        let flows_on = spec.breaks != Breaks::Printed && row_w + 0.01 < block.1 - block.0;
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
                cur.push(&prep.atoms[wi as usize]);
            }
        }
        if !cur.is_empty() {
            blocks.push(Block::Text(cur));
        }
        blocks
    }

    /// Where to break a block into rows. `width(i..j)` is a run of atoms with the gaps that
    /// fall between them, so every strategy reads it off two prefix sums.
    fn cut_points(&self, atoms: &[&Atom], spec: &ReflowSpec, row_w: f32) -> Vec<usize> {
        let median = self.reflow_prepared().median;
        // between two atoms, not two words: an atom's printed extent already holds the
        // medallion that closes its ayah, so the print's word-to-word distance would count
        // that medallion twice and tear the row open
        let gap_of = |prev: &Atom, next: &Atom| self.atom_gap(prev, next, spec, median);
        let n = atoms.len();
        if n == 0 {
            return vec![];
        }
        let (mut w, mut g) = (vec![0.0f32; n + 1], vec![0.0f32; n + 1]);
        for i in 0..n {
            w[i + 1] = w[i] + atoms[i].width;
            g[i + 1] = g[i] + if i > 0 { gap_of(atoms[i - 1], atoms[i]) } else { 0.0 };
        }
        let width = |i: usize, j: usize| (w[j] - w[i]) + (g[j] - g[i + 1]);
        match spec.breaks {
            Breaks::Printed => vec![],
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
            // Rows weighed by the width they can reach, against one another. A row's
            // shortfall is measured after its own gaps have opened as far as the cap
            // allows, so a shortfall the spacing can absorb costs little and one it cannot
            // moves the break. Two further terms keep the page steady: the step between a
            // row and the row above it, and the spacing actually spent to get there, which
            // makes a better break worth more than a wider gap.
            Breaks::Fitted => {
                // the gaps free to stretch in a run, and how far each may go
                let mut o = vec![0u32; n + 1];
                for i in 1..n {
                    o[i + 1] = o[i] + u32::from(!self.interlocked(atoms[i - 1], atoms[i]));
                }
                let per_gap = (median * spec.word_gap.max(0.0) * (spec.max_stretch - 1.0)).max(0.0);
                let per_gap = if spec.max_stretch > 0.0 { per_gap } else { row_w };
                // fill from 0 to 1, natural and after opening
                let reach = |i: usize, j: usize| -> (f32, f32) {
                    let used = width(i, j);
                    let room = (o[j] - o[i + 1]) as f32 * per_gap;
                    (used / row_w, (used + room).min(row_w) / row_w)
                };
                // The cost of a row against the row before it. The state has to carry that
                // row, so the table is over pairs: `best[at(i, j)]` is the block broken up
                // to `j` with its last row running from `i`. One flat vector a side, not a
                // vector of vectors, so a block costs two allocations and not `2n`.
                const SHORT: f32 = 1.0;
                const STEP: f32 = 0.6;
                const SPEND: f32 = 0.25;
                let fits = |i: usize, j: usize| width(i, j) <= row_w + 0.01 || i + 1 == j;
                let at = |i: usize, j: usize| i * (n + 1) + j;
                let mut best: Vec<f32> = vec![f32::INFINITY; (n + 1) * (n + 1)];
                let mut from: Vec<u32> = vec![0; (n + 1) * (n + 1)];
                for j in 1..=n {
                    for i in (0..j).rev() {
                        if !fits(i, j) {
                            break;
                        }
                        let (natural, reached) = reach(i, j);
                        // a block ends where its text ends, so its last row is not short
                        let short = if j == n { 0.0 } else { (1.0 - reached).max(0.0) };
                        let own = SHORT * short * short + SPEND * (reached - natural) * (reached - natural);
                        if i == 0 {
                            best[at(i, j)] = own;
                            continue;
                        }
                        for h in (0..i).rev() {
                            if !fits(h, i) {
                                break;
                            }
                            if best[at(h, i)].is_infinite() {
                                continue;
                            }
                            let step = reach(h, i).1 - reached;
                            let cost = best[at(h, i)] + own + STEP * step * step;
                            if cost < best[at(i, j)] {
                                best[at(i, j)] = cost;
                                from[at(i, j)] = h as u32;
                            }
                        }
                    }
                }
                let mut i = (0..n)
                    .rev()
                    .filter(|&i| best[at(i, n)].is_finite())
                    .min_by(|&a, &b| best[at(a, n)].partial_cmp(&best[at(b, n)]).unwrap_or(std::cmp::Ordering::Equal));
                let (mut cuts, mut j) = (vec![], n);
                while let Some(start) = i {
                    if start == 0 {
                        break;
                    }
                    cuts.push(start);
                    let h = from[at(start, j)] as usize;
                    j = start;
                    i = Some(h);
                }
                cuts.reverse();
                cuts
            }
        }
    }
}
