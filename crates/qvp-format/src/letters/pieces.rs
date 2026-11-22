//! The geometry of the pieces in a QVL1 file.
//!
//! A piece is cut from one page path, its source, and most of what it draws is already in
//! that path: runs of the source's segments, parts of one source segment, and cut lines
//! between points on the source. The file codes each drawing command as an operation on
//! the source, and keeps the rare command that is none of these as plain deltas.
//!
//! The decoder computes a part of a segment in integers only: a fixed-point de Casteljau
//! split, then a per-coordinate correction from the file. So every platform rebuilds the
//! same commands. `docs/FORMAT.md` specifies every operation, the order of the state
//! changes, and the arithmetic.

use super::PieceRec;
use crate::{read_varint, unzigzag, write_varint, zigzag, Cmd, Error, PageData, COORD_LIMIT, PF_GLYPH};
// The opcodes of a page path's run in a decoded page (`PageData::ops`).
use crate::{OP_CLOSE as V1_CLOSE, OP_CUBIC as V1_CUBIC, OP_LINE as V1_LINE, OP_MOVE as V1_MOVE, OP_QUAD as V1_QUAD};
use std::collections::HashMap;

/// The geometry sections, in file order after the tables.
pub(super) const N_STREAMS: usize = 9;
const OPS: usize = 0;
const LENS: usize = 1;
const SEG: usize = 2;
const BACK: usize = 3;
const TS: usize = 4;
const CORR: usize = 5;
const CORR_ESC: usize = 6;
const XS: usize = 7;
const YS: usize = 8;

// Operation codes, four bits each.
const OP_ESCAPE: u8 = 0;
const OP_RUN: u8 = 1;
const OP_PART_TO_END: u8 = 2;
const OP_PART_TO_NEW: u8 = 3;
const OP_PART_TO_REG: u8 = 4;
const OP_REVERSE: u8 = 5;
const OP_CUBIC: u8 = 6;
const OP_MOVE_SEG: u8 = 7;
const OP_MOVE_REG: u8 = 8;
const OP_MOVE_ON: u8 = 9;
const OP_LINE_SEG: u8 = 10;
const OP_LINE_REG: u8 = 11;
const OP_LINE_ON: u8 = 12;
// The three parts again, from a stored start.
const OP_PART_X_TO_END: u8 = 13;
const OP_PART_X_TO_NEW: u8 = 14;
const OP_PART_X_TO_REG: u8 = 15;

// The sub-codes of an escape, in the segment stream.
const ESC_MOVE: u32 = 0;
const ESC_NO_CLOSE: u32 = 1;
const ESC_LINE: u32 = 2;
const ESC_QUAD: u32 = 3;
const ESC_CLOSE: u32 = 4;

/// Fractional bits of the fixed-point coordinates of a split.
const FRAC: u32 = 14;
/// Bits of the split parameter `u`: `u` runs from 0 to `1 << U_BITS`.
const U_BITS: u32 = 16;
/// The fewest and the most bits of a stored split parameter `v`.
const V_BITS_MIN: u32 = 2;
const V_BITS_MAX: u32 = 16;

type P = (i32, i32);

/// A drawing command without its start point: `deg` is 1 for a line, 2 for a quad and 3
/// for a cubic. `pts` holds the control points, then the end point. Unused points are 0.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct Draw {
    deg: u8,
    pts: [P; 3],
}

impl Draw {
    fn line(e: P) -> Draw {
        Draw { deg: 1, pts: [e, (0, 0), (0, 0)] }
    }
    fn of(c: &Cmd) -> Option<Draw> {
        Some(match *c {
            Cmd::LineTo(x, y) => Draw::line((x, y)),
            Cmd::QuadTo(a, b, x, y) => Draw { deg: 2, pts: [(a, b), (x, y), (0, 0)] },
            Cmd::CubicTo(a, b, c, d, x, y) => Draw { deg: 3, pts: [(a, b), (c, d), (x, y)] },
            _ => return None,
        })
    }
    fn end(&self) -> P {
        self.pts[self.deg as usize - 1]
    }
    fn cmd(&self) -> Cmd {
        let p = self.pts;
        match self.deg {
            1 => Cmd::LineTo(p[0].0, p[0].1),
            2 => Cmd::QuadTo(p[0].0, p[0].1, p[1].0, p[1].1),
            _ => Cmd::CubicTo(p[0].0, p[0].1, p[1].0, p[1].1, p[2].0, p[2].1),
        }
    }
    /// The same curve drawn from its end back to `start`.
    fn reversed(&self, start: P) -> Draw {
        let p = self.pts;
        match self.deg {
            1 => Draw::line(start),
            2 => Draw { deg: 2, pts: [p[0], start, (0, 0)] },
            _ => Draw { deg: 3, pts: [p[1], p[0], start] },
        }
    }
}

/// One segment of a source path. A subpath that does not end at its start has one more
/// segment, the line that closes it.
#[derive(Clone, Copy, Debug)]
struct Seg {
    start: P,
    draw: Draw,
    /// The subpath's first segment and its segment count: the segment after the last one
    /// of a subpath is its first.
    sub0: u32,
    sub_len: u32,
    /// The bits of a stored split parameter on this segment.
    bits: u32,
}

fn in_envelope(p: P) -> bool {
    p.0.unsigned_abs() < COORD_LIMIT && p.1.unsigned_abs() < COORD_LIMIT
}

/// The bits of `v` on a segment: one more than the bit length of the Chebyshev length
/// of its control polygon, from 2 to 16.
fn v_bits(start: P, d: &Draw) -> u32 {
    let mut len: u64 = 0;
    let mut c = start;
    for &p in &d.pts[..d.deg as usize] {
        let dx = (p.0 as i64 - c.0 as i64).unsigned_abs();
        let dy = (p.1 as i64 - c.1 as i64).unsigned_abs();
        len += dx.max(dy);
        c = p;
    }
    (64 - len.max(1).leading_zeros() + 1).clamp(V_BITS_MIN, V_BITS_MAX)
}

/// `v` on a segment's grid as a split parameter `u`.
fn u_of(v: u32, bits: u32) -> i64 {
    (v as i64) << (U_BITS - bits)
}

/// One step of de Casteljau in fixed point: `a + floor((b - a) * u / 2^16)`. With `a` and
/// `b` below 2^44 in absolute value and `u` from 0 to 2^16, the product is below 2^61, and
/// the result lies between `a` and `b`.
fn lerp(a: i64, b: i64, u: i64) -> i64 {
    a + (((b - a) * u) >> U_BITS)
}

/// The blossom of a segment at the parameters `w` (one per degree), rounded to the
/// quantised grid: level `i` of de Casteljau runs at `w[i]`. A coordinate below 2^30 is
/// below 2^44 in fixed point.
fn blossom(seg: &Seg, w: &[i64]) -> P {
    let d = seg.draw.deg as usize;
    let mut x = [0i64; 4];
    let mut y = [0i64; 4];
    x[0] = seg.start.0 as i64 * (1 << FRAC);
    y[0] = seg.start.1 as i64 * (1 << FRAC);
    for j in 0..d {
        x[j + 1] = seg.draw.pts[j].0 as i64 * (1 << FRAC);
        y[j + 1] = seg.draw.pts[j].1 as i64 * (1 << FRAC);
    }
    for (level, &u) in w.iter().enumerate().take(d) {
        for j in 0..d - level {
            x[j] = lerp(x[j], x[j + 1], u);
            y[j] = lerp(y[j], y[j + 1], u);
        }
    }
    let round = |v: i64| ((v + (1 << (FRAC - 1))) >> FRAC) as i32;
    (round(x[0]), round(y[0]))
}

/// Point `i` (1 to the degree) of the part of `seg` from `u0` to `u1`, before correction.
fn part_point(seg: &Seg, i: usize, u0: i64, u1: i64) -> P {
    let d = seg.draw.deg as usize;
    let mut w = [u1; 3];
    for x in w.iter_mut().take(d - i) {
        *x = u0;
    }
    blossom(seg, &w[..d])
}

/// The point of `seg` at `u`, before correction.
fn point_at(seg: &Seg, u: i64) -> P {
    blossom(seg, &[u; 3][..seg.draw.deg as usize])
}

/// The state of one group, the pieces of one source. It is built again for each group.
#[derive(Default)]
struct Group {
    segs: Vec<Seg>,
    /// `(start, segment)` for every segment, sorted: the segments that start at a point
    /// are next to each other, in segment order.
    starts: Vec<(u64, u32)>,
    /// The registry: `(point, segment, v)`, in the order the operations add them.
    reg: Vec<(P, u32, u32)>,
    /// The latest registry entry of each point.
    reg_at: HashMap<P, u32>,
    /// The commands a later `REVERSE` can draw backwards: `(start, command)`.
    rev: Vec<(P, Draw)>,
    /// How many more source segments the runs of this group may draw.
    budget: u32,
}

/// A point as one sort key: equal points have equal keys.
fn key(p: P) -> u64 {
    ((p.0 as u32 as u64) << 32) | p.1 as u32 as u64
}

impl Group {
    /// Build the segments of `source`. The source must be an inline path of the page, and
    /// each of its points inside the envelope.
    fn load(&mut self, page: &PageData, source: u32) -> Result<(), Error> {
        self.segs.clear();
        self.starts.clear();
        self.reg.clear();
        self.reg_at.clear();
        self.rev.clear();
        let p = page.paths.get(source as usize).ok_or(Error::Bounds("piece source"))?;
        if p.flags & PF_GLYPH != 0 {
            return Err(Error::Corrupt("piece source glyph"));
        }
        let end = p.op_off.checked_add(p.op_len).ok_or(Error::Bounds("path ops"))?;
        let ops = page.ops.get(p.op_off as usize..end as usize).ok_or(Error::Bounds("path ops"))?;
        // The opcode run of a page path, as `decode_cmds` reads it, into segments.
        let (mut cur, mut first) = ((0, 0), (0, 0));
        let mut sub0 = 0usize;
        let mut open = false;
        let finish = |segs: &mut Vec<Seg>, sub0: usize, cur: P, first: P| {
            if cur != first {
                segs.push(Seg { start: cur, draw: Draw::line(first), sub0: 0, sub_len: 0, bits: 0 });
            }
            let n = (segs.len() - sub0) as u32;
            for s in &mut segs[sub0..] {
                s.sub0 = sub0 as u32;
                s.sub_len = n;
            }
        };
        let (mut pos, mut at) = (0usize, (p.ox, p.oy));
        let mut pt = |pos: &mut usize| -> Result<P, Error> {
            let dx = unzigzag(read_varint(ops, pos)?);
            let dy = unzigzag(read_varint(ops, pos)?);
            at.0 = at.0.checked_add(dx).ok_or(Error::Corrupt("coordinate overflow"))?;
            at.1 = at.1.checked_add(dy).ok_or(Error::Corrupt("coordinate overflow"))?;
            if !in_envelope(at) {
                return Err(Error::Corrupt("source range"));
            }
            Ok(at)
        };
        while pos < ops.len() {
            let op = ops[pos];
            pos += 1;
            if op == V1_MOVE {
                if open {
                    finish(&mut self.segs, sub0, cur, first);
                }
                cur = pt(&mut pos)?;
                sub0 = self.segs.len();
                first = cur;
                open = true;
                continue;
            }
            if op == V1_CLOSE {
                if open {
                    finish(&mut self.segs, sub0, cur, first);
                }
                open = false;
                cur = first;
                continue;
            }
            let deg = match op {
                V1_LINE => 1,
                V1_QUAD => 2,
                V1_CUBIC => 3,
                _ => return Err(Error::Corrupt("bad opcode")),
            };
            let mut d = Draw { deg, pts: [(0, 0); 3] };
            for q in d.pts.iter_mut().take(deg as usize) {
                *q = pt(&mut pos)?;
            }
            if !open {
                sub0 = self.segs.len();
                open = true;
            }
            self.segs.push(Seg { start: cur, draw: d, sub0: 0, sub_len: 0, bits: 0 });
            cur = d.end();
        }
        if open {
            finish(&mut self.segs, sub0, cur, first);
        }
        if self.segs.len() > u32::MAX as usize {
            return Err(Error::Bounds("source segments"));
        }
        for s in &mut self.segs {
            s.bits = v_bits(s.start, &s.draw);
        }
        self.starts.extend(self.segs.iter().enumerate().map(|(i, s)| (key(s.start), i as u32)));
        self.starts.sort_unstable();
        self.budget = self.segs.len() as u32;
        Ok(())
    }

    /// The segments that start at `p`, in segment order.
    fn starting_at(&self, p: P) -> &[(u64, u32)] {
        let k = key(p);
        let a = self.starts.partition_point(|e| e.0 < k);
        let b = a + self.starts[a..].partition_point(|e| e.0 == k);
        &self.starts[a..b]
    }

    fn next(&self, k: u32) -> u32 {
        let s = &self.segs[k as usize];
        s.sub0 + (k - s.sub0 + 1) % s.sub_len
    }

    fn push_reg(&mut self, p: P, seg: u32, v: u32) {
        self.reg_at.insert(p, self.reg.len() as u32);
        self.reg.push((p, seg, v));
    }

    /// The registry entry `back` places before the latest.
    fn reg_back(&self, back: u32) -> Result<(P, u32, u32), Error> {
        let n = self.reg.len();
        if back as usize >= n {
            return Err(Error::Bounds("registry"));
        }
        Ok(self.reg[n - 1 - back as usize])
    }
}

// ───────────────────────────── decode ─────────────────────────────

#[cfg(test)]
thread_local! {
    /// The operations this thread decoded: codes 0 to 15, then the escapes 16 to 20.
    pub(super) static SEEN: std::cell::RefCell<[u32; 21]> = const { std::cell::RefCell::new([0; 21]) };
}

/// A reader over the geometry sections.
pub(super) struct Reader<'a> {
    s: [&'a [u8]; N_STREAMS],
    pos: [usize; N_STREAMS],
    op_i: usize,
    corr_i: usize,
}

impl<'a> Reader<'a> {
    pub(super) fn new(s: [&'a [u8]; N_STREAMS]) -> Self {
        Reader { s, pos: [0; N_STREAMS], op_i: 0, corr_i: 0 }
    }
    fn op(&mut self) -> Result<u8, Error> {
        let byte = *self.s[OPS].get(self.op_i / 2).ok_or(Error::Truncated("operations"))?;
        let code = (byte >> (4 * (self.op_i % 2))) & 15;
        self.op_i += 1;
        Ok(code)
    }
    fn vi(&mut self, i: usize, what: &'static str) -> Result<u32, Error> {
        read_varint(self.s[i], &mut self.pos[i]).map_err(|e| match e {
            Error::Truncated(_) => Error::Truncated(what),
            e => e,
        })
    }
    fn zv(&mut self, i: usize, what: &'static str) -> Result<i32, Error> {
        Ok(unzigzag(self.vi(i, what)?))
    }
    fn corr(&mut self) -> Result<i32, Error> {
        let byte = *self.s[CORR].get(self.corr_i / 4).ok_or(Error::Truncated("corrections"))?;
        let code = (byte >> (2 * (self.corr_i % 4))) & 3;
        self.corr_i += 1;
        Ok(match code {
            0 => 0,
            1 => 1,
            2 => -1,
            _ => self.zv(CORR_ESC, "corrections")?,
        })
    }
    /// A point from the delta streams, relative to `from`.
    fn delta(&mut self, from: P) -> Result<P, Error> {
        let x = from.0.checked_add(self.zv(XS, "x")?).ok_or(Error::Corrupt("coordinate overflow"))?;
        let y = from.1.checked_add(self.zv(YS, "y")?).ok_or(Error::Corrupt("coordinate overflow"))?;
        checked((x, y))
    }
    /// A segment index of the group.
    fn seg(&mut self, g: &Group) -> Result<u32, Error> {
        let k = self.vi(SEG, "segment")?;
        if k as usize >= g.segs.len() {
            return Err(Error::Bounds("segment"));
        }
        Ok(k)
    }
    /// One of the segments that start at the current point: the only one, or the one at
    /// the stored position.
    fn seg_at(&mut self, list: &[(u64, u32)]) -> Result<u32, Error> {
        if list.len() == 1 {
            return Ok(list[0].1);
        }
        let i = self.vi(SEG, "segment position")? as usize;
        list.get(i).map(|e| e.1).ok_or(Error::Bounds("segment position"))
    }
    /// A split parameter on a segment's grid: 0 to `1 << bits`.
    fn v(&mut self, seg: &Seg) -> Result<u32, Error> {
        let v = self.vi(TS, "split")?;
        if v as u64 > 1u64 << seg.bits {
            return Err(Error::Corrupt("split"));
        }
        Ok(v)
    }
    /// A computed point plus its correction.
    fn corrected(&mut self, p: P) -> Result<P, Error> {
        let x = p.0.checked_add(self.corr()?).ok_or(Error::Corrupt("coordinate overflow"))?;
        let y = p.1.checked_add(self.corr()?).ok_or(Error::Corrupt("coordinate overflow"))?;
        checked((x, y))
    }
    /// Every section was read to its end, and the unused bits of the last operation byte
    /// and the last correction byte are 0.
    pub(super) fn finish(&self) -> Result<(), Error> {
        for i in 0..N_STREAMS {
            if i != OPS && i != CORR && self.pos[i] != self.s[i].len() {
                return Err(Error::Corrupt("geometry streams"));
            }
        }
        if self.s[OPS].len() != self.op_i.div_ceil(2) || (self.op_i % 2 == 1 && self.s[OPS][self.op_i / 2] >> 4 != 0) {
            return Err(Error::Corrupt("operations"));
        }
        let c = self.corr_i % 4;
        if self.s[CORR].len() != self.corr_i.div_ceil(4) || (c != 0 && self.s[CORR][self.corr_i / 4] >> (2 * c) != 0) {
            return Err(Error::Corrupt("corrections"));
        }
        Ok(())
    }
}

fn checked(p: P) -> Result<P, Error> {
    if in_envelope(p) {
        Ok(p)
    } else {
        Err(Error::Corrupt("coordinate range"))
    }
}

/// Decode the pieces. `groups` gives each group's source and piece count, `runs` each
/// piece's operation count. The caller checked that the runs add up to the operations
/// that the operations section holds.
pub(super) fn decode_pieces(
    page: &PageData,
    groups: &[(u32, u32)],
    runs: &[u32],
    r: &mut Reader,
) -> Result<Vec<PieceRec>, Error> {
    let mut out = Vec::with_capacity(runs.len());
    let mut g = Group::default();
    let mut cur: P = (0, 0);
    let mut run_i = 0usize;
    for &(source, n_pieces) in groups {
        g.load(page, source)?;
        for _ in 0..n_pieces {
            let n_ops = runs[run_i];
            run_i += 1;
            let cmds = decode_piece(&mut g, n_ops, &mut cur, r)?;
            out.push(PieceRec { source, cmds });
        }
    }
    Ok(out)
}

fn decode_piece(g: &mut Group, n_ops: u32, cur: &mut P, r: &mut Reader) -> Result<Vec<Cmd>, Error> {
    let mut cmds: Vec<Cmd> = Vec::with_capacity(n_ops as usize + 1);
    let mut first = *cur;
    let mut open = false;
    let close = |cmds: &mut Vec<Cmd>, g: &mut Group, cur: &mut P, first: P| {
        cmds.push(Cmd::Close);
        if *cur != first {
            g.rev.push((*cur, Draw::line(first)));
        }
        *cur = first;
    };
    for _ in 0..n_ops {
        let code = r.op()?;
        let sub = if code == OP_ESCAPE { Some(r.vi(SEG, "escape")?) } else { None };
        #[cfg(test)]
        SEEN.with(|s| s.borrow_mut()[sub.map_or(code as usize, |e| 16 + e.min(4) as usize)] += 1);
        match sub {
            Some(ESC_CLOSE) => {
                close(&mut cmds, g, cur, first);
                open = false;
                continue;
            }
            Some(ESC_NO_CLOSE) => {
                open = false;
                continue;
            }
            _ => {}
        }
        // A move closes the open subpath first, then reads its point.
        if matches!(code, OP_MOVE_SEG | OP_MOVE_REG | OP_MOVE_ON) || sub == Some(ESC_MOVE) {
            if open {
                close(&mut cmds, g, cur, first);
            }
            let p = match code {
                OP_MOVE_SEG => g.segs[r.seg(g)? as usize].start,
                OP_MOVE_REG => g.reg_back(r.vi(BACK, "registry")?)?.0,
                OP_MOVE_ON => on_segment(g, r)?,
                _ => r.delta(*cur)?,
            };
            cmds.push(Cmd::MoveTo(p.0, p.1));
            *cur = p;
            first = p;
            open = true;
            continue;
        }
        open = true;
        // A command from deltas: each point from the one before, the first from the
        // current point.
        let deltas = |deg: u8, r: &mut Reader, g: &mut Group, cur: P| -> Result<Draw, Error> {
            let mut d = Draw { deg, pts: [(0, 0); 3] };
            let mut from = cur;
            for i in 0..deg as usize {
                d.pts[i] = r.delta(from)?;
                from = d.pts[i];
            }
            g.rev.push((cur, d));
            Ok(d)
        };
        let d = match (code, sub) {
            (OP_ESCAPE, Some(ESC_LINE)) => deltas(1, r, g, *cur)?,
            (OP_ESCAPE, Some(ESC_QUAD)) => deltas(2, r, g, *cur)?,
            (OP_ESCAPE, _) => return Err(Error::Corrupt("escape")),
            (OP_CUBIC, _) => deltas(3, r, g, *cur)?,
            (OP_LINE_SEG | OP_LINE_REG | OP_LINE_ON, _) => {
                let e = match code {
                    OP_LINE_SEG => g.segs[r.seg(g)? as usize].start,
                    OP_LINE_REG => g.reg_back(r.vi(BACK, "registry")?)?.0,
                    _ => on_segment(g, r)?,
                };
                let d = Draw::line(e);
                g.rev.push((*cur, d));
                d
            }
            (OP_RUN, _) => {
                let list = g.starting_at(*cur);
                if list.is_empty() {
                    return Err(Error::Corrupt("run start"));
                }
                let mut s = r.seg_at(list)?;
                let len = r.vi(LENS, "run length")?.checked_add(1).ok_or(Error::Corrupt("run length"))?;
                if len > g.budget {
                    return Err(Error::Corrupt("run length"));
                }
                g.budget -= len;
                for j in 0..len {
                    if j > 0 {
                        s = g.next(s);
                    }
                    let d = g.segs[s as usize].draw;
                    cmds.push(d.cmd());
                    *cur = d.end();
                }
                continue;
            }
            (OP_REVERSE, _) => {
                let back = r.vi(BACK, "reverse")? as usize;
                let n = g.rev.len();
                if back >= n {
                    return Err(Error::Bounds("reverse"));
                }
                let (start, d) = g.rev[n - 1 - back];
                d.reversed(start)
            }
            (OP_PART_TO_END..=OP_PART_TO_REG | OP_PART_X_TO_END..=OP_PART_X_TO_REG, _) => part(g, code, *cur, r)?,
            _ => return Err(Error::Corrupt("operation")),
        };
        cmds.push(d.cmd());
        *cur = d.end();
    }
    if open {
        close(&mut cmds, g, cur, first);
    }
    // A piece is ink: it starts with a move, which the next piece's deltas start from.
    match cmds.first() {
        Some(&Cmd::MoveTo(x, y)) => *cur = (x, y),
        _ => return Err(Error::Corrupt("piece start")),
    }
    Ok(cmds)
}

/// `MOVE_ON` and `LINE_ON`: a point on a segment, corrected, added to the registry.
fn on_segment(g: &mut Group, r: &mut Reader) -> Result<P, Error> {
    let s = r.seg(g)?;
    let seg = g.segs[s as usize];
    let v = r.v(&seg)?;
    let p = r.corrected(point_at(&seg, u_of(v, seg.bits)))?;
    g.push_reg(p, s, v);
    Ok(p)
}

/// The six `PART` operations: a part of one segment, from the current point.
fn part(g: &mut Group, code: u8, cur: P, r: &mut Reader) -> Result<Draw, Error> {
    // Where the part starts on its segment: a stored segment and split, or the current
    // point as a segment start or a registry point.
    let mut start_v = None;
    let list = g.starting_at(cur);
    let (s, u0) = if code >= OP_PART_X_TO_END {
        let s = r.seg(g)?;
        let v = r.v(&g.segs[s as usize])?;
        start_v = Some(v);
        (s, u_of(v, g.segs[s as usize].bits))
    } else if !list.is_empty() {
        (r.seg_at(list)?, 0)
    } else if let Some(&i) = g.reg_at.get(&cur) {
        let (_, s, v) = g.reg[i as usize];
        (s, u_of(v, g.segs[s as usize].bits))
    } else {
        return Err(Error::Corrupt("part start"));
    };
    let seg = g.segs[s as usize];
    let mut end_v = None;
    let (u1, fixed_end) = match code {
        OP_PART_TO_END | OP_PART_X_TO_END => (1 << U_BITS, Some(seg.draw.end())),
        OP_PART_TO_NEW | OP_PART_X_TO_NEW => {
            let v = r.v(&seg)?;
            end_v = Some(v);
            (u_of(v, seg.bits), None)
        }
        _ => {
            let (p, s2, v) = g.reg_back(r.vi(BACK, "registry")?)?;
            if s2 != s {
                return Err(Error::Corrupt("part end"));
            }
            (u_of(v, seg.bits), Some(p))
        }
    };
    let deg = seg.draw.deg as usize;
    let mut d = Draw { deg: seg.draw.deg, pts: [(0, 0); 3] };
    for i in 1..=deg {
        d.pts[i - 1] = match fixed_end {
            Some(e) if i == deg => e,
            _ => r.corrected(part_point(&seg, i, u0, u1))?,
        };
    }
    if let Some(v) = start_v {
        g.push_reg(cur, s, v);
    }
    if let Some(v) = end_v {
        g.push_reg(d.end(), s, v);
    }
    Ok(d)
}

// ───────────────────────────── encode ─────────────────────────────

/// The geometry sections being written.
#[derive(Default)]
pub(super) struct Streams {
    pub(super) s: [Vec<u8>; N_STREAMS],
    n_ops: usize,
    n_corr: usize,
}

impl Streams {
    fn op(&mut self, code: u8) {
        if self.n_ops.is_multiple_of(2) {
            self.s[OPS].push(code);
        } else {
            *self.s[OPS].last_mut().expect("an operation byte") |= code << 4;
        }
        self.n_ops += 1;
    }
    fn escape(&mut self, sub: u32) {
        self.op(OP_ESCAPE);
        self.vi(SEG, sub);
    }
    fn vi(&mut self, i: usize, v: u32) {
        write_varint(&mut self.s[i], v);
    }
    fn zv(&mut self, i: usize, v: i32) {
        write_varint(&mut self.s[i], zigzag(v));
    }
    fn corr(&mut self, v: i32) {
        let c = match v {
            0 => 0,
            1 => 1,
            -1 => 2,
            _ => {
                self.zv(CORR_ESC, v);
                3
            }
        };
        if self.n_corr.is_multiple_of(4) {
            self.s[CORR].push(c);
        } else {
            *self.s[CORR].last_mut().expect("a correction byte") |= c << (2 * (self.n_corr % 4));
        }
        self.n_corr += 1;
    }
    fn delta(&mut self, from: P, to: P) {
        self.zv(XS, to.0.wrapping_sub(from.0));
        self.zv(YS, to.1.wrapping_sub(from.1));
    }
}

/// The encoder's view of a group: the decoder's state, and the maps it searches.
#[derive(Default)]
struct Enc {
    g: Group,
    /// For each command in `g.rev`, the command drawn backwards and its start, to its index.
    rev_at: HashMap<(P, Draw), u32>,
}

impl Enc {
    fn push_rev(&mut self, start: P, d: Draw) {
        self.rev_at.insert((d.end(), d.reversed(start)), self.g.rev.len() as u32);
        self.g.rev.push((start, d));
    }
}

/// The largest correction the encoder accepts in a part or a point on a segment. A
/// larger one is valid in the file, but costs more than the plain deltas.
const MAX_CORR: i32 = 2;
/// The grid steps the encoder searches on each side of its estimate of a split.
const SEARCH: i64 = 6;
/// How far, in quantised units, a point may lie from a segment for the encoder to try it.
const NEAR: f64 = 2.0;

/// Encode the pieces into `w`. Pieces must be in source order. Returns the operation
/// count of each piece.
pub(super) fn encode_pieces(page: &PageData, pieces: &[PieceRec], w: &mut Streams) -> Result<Vec<u32>, Error> {
    let mut runs = Vec::with_capacity(pieces.len());
    let mut e = Enc::default();
    let mut cur: P = (0, 0);
    let mut source: Option<u32> = None;
    for pc in pieces {
        if source != Some(pc.source) {
            if source.is_some_and(|s| s > pc.source) {
                return Err(Error::Corrupt("piece order"));
            }
            e.g.load(page, pc.source)?;
            e.rev_at.clear();
            source = Some(pc.source);
        }
        let before = w.n_ops;
        encode_piece(&mut e, &pc.cmds, &mut cur, w)?;
        runs.push((w.n_ops - before) as u32);
    }
    Ok(runs)
}

fn encode_piece(e: &mut Enc, cmds: &[Cmd], cur: &mut P, w: &mut Streams) -> Result<(), Error> {
    let start = match cmds.first() {
        Some(&Cmd::MoveTo(x, y)) => (x, y),
        _ => return Err(Error::Corrupt("piece start")),
    };
    for c in cmds {
        let d = Draw::of(c);
        let pts: &[P] = match (c, &d) {
            (Cmd::MoveTo(x, y), _) => &[(*x, *y)],
            (_, Some(d)) => &d.pts[..d.deg as usize],
            _ => &[],
        };
        if !pts.iter().all(|p| in_envelope(*p)) {
            return Err(Error::Corrupt("coordinate range"));
        }
    }
    // `open` is the decoder's: a subpath it will close at the next move or at the end.
    // `closed`: the encoder met that close already, so the decoder's close is the one
    // the piece has, and the state it changes is changed.
    let (mut open, mut closed) = (false, false);
    let mut first = *cur;
    let mut i = 0;
    while i < cmds.len() {
        match cmds[i] {
            Cmd::MoveTo(x, y) => {
                if open && !closed {
                    w.escape(ESC_NO_CLOSE);
                }
                closed = false;
                let p = (x, y);
                encode_move(e, *cur, p, w);
                *cur = p;
                first = p;
                open = true;
                i += 1;
            }
            Cmd::Close => {
                let implied = open && !closed && cmds.get(i + 1).is_none_or(|c| matches!(c, Cmd::MoveTo(..)));
                if implied {
                    closed = true;
                } else {
                    w.escape(ESC_CLOSE);
                    open = false;
                }
                if *cur != first {
                    e.push_rev(*cur, Draw::line(first));
                }
                *cur = first;
                i += 1;
            }
            ref c => {
                let d = Draw::of(c).expect("a drawing command");
                open = true;
                i += encode_draw(e, cmds, i, *cur, d, w);
                *cur = Draw::of(&cmds[i - 1]).expect("a drawing command").end();
            }
        }
    }
    if open && !closed {
        w.escape(ESC_NO_CLOSE);
    }
    *cur = start;
    Ok(())
}

fn encode_move(e: &mut Enc, cur: P, p: P, w: &mut Streams) {
    let g = &mut e.g;
    if let Some(&r) = g.reg_at.get(&p) {
        w.op(OP_MOVE_REG);
        w.vi(BACK, (g.reg.len() - 1 - r as usize) as u32);
    } else if let Some(&(_, k)) = g.starting_at(p).first() {
        w.op(OP_MOVE_SEG);
        w.vi(SEG, k);
    } else if let Some(on) = find_on(g, p) {
        w.op(OP_MOVE_ON);
        write_on(g, p, on, w);
    } else {
        w.escape(ESC_MOVE);
        w.delta(cur, p);
    }
}

/// A point on a segment: the segment, `v`, and the correction.
struct On {
    seg: u32,
    v: u32,
    corr: [i32; 2],
}

fn write_on(g: &mut Group, p: P, on: On, w: &mut Streams) {
    w.vi(SEG, on.seg);
    w.vi(TS, on.v);
    w.corr(on.corr[0]);
    w.corr(on.corr[1]);
    g.push_reg(p, on.seg, on.v);
}

/// Code the drawing command at `cmds[i]`, and the run of source segments it starts.
/// Returns how many commands it coded.
fn encode_draw(e: &mut Enc, cmds: &[Cmd], i: usize, cur: P, d: Draw, w: &mut Streams) -> usize {
    // a run of source segments
    if e.g.budget > 0 {
        let list = e.g.starting_at(cur);
        if let Some(pos) = list.iter().position(|&(_, s)| e.g.segs[s as usize].draw == d) {
            let mut s = list[pos].1;
            w.op(OP_RUN);
            if list.len() > 1 {
                w.vi(SEG, pos as u32);
            }
            let mut len = 1u32;
            while len < e.g.budget {
                let nx = e.g.next(s);
                match cmds.get(i + len as usize).and_then(Draw::of) {
                    Some(d2) if e.g.segs[nx as usize].draw == d2 => {
                        s = nx;
                        len += 1;
                    }
                    _ => break,
                }
            }
            w.vi(LENS, len - 1);
            e.g.budget -= len;
            return len as usize;
        }
    }
    // a command that an earlier one drew backwards
    if let Some(j) = e.rev_at.remove(&(cur, d)) {
        w.op(OP_REVERSE);
        w.vi(BACK, (e.g.rev.len() - 1 - j as usize) as u32);
        return 1;
    }
    // a part of one segment
    if let Some(f) = find_part(&e.g, cur, &d) {
        w.op(f.code);
        if let Some(pos) = f.pos {
            w.vi(SEG, pos);
        }
        if let Some(v0) = f.start_v {
            w.vi(SEG, f.seg);
            w.vi(TS, v0);
        }
        match f.code {
            OP_PART_TO_NEW | OP_PART_X_TO_NEW => w.vi(TS, f.end_v),
            OP_PART_TO_REG | OP_PART_X_TO_REG => w.vi(BACK, f.back),
            _ => {}
        }
        for c in &f.corr[..f.n_corr] {
            w.corr(*c);
        }
        if let Some(v0) = f.start_v {
            e.g.push_reg(cur, f.seg, v0);
        }
        if matches!(f.code, OP_PART_TO_NEW | OP_PART_X_TO_NEW) {
            e.g.push_reg(d.end(), f.seg, f.end_v);
        }
        return 1;
    }
    if d.deg == 1 {
        let p = d.end();
        let g = &mut e.g;
        if let Some(&r) = g.reg_at.get(&p) {
            w.op(OP_LINE_REG);
            w.vi(BACK, (g.reg.len() - 1 - r as usize) as u32);
        } else if let Some(&(_, k)) = g.starting_at(p).first() {
            w.op(OP_LINE_SEG);
            w.vi(SEG, k);
        } else if let Some(on) = find_on(g, p) {
            w.op(OP_LINE_ON);
            write_on(g, p, on, w);
        } else {
            w.escape(ESC_LINE);
            w.delta(cur, p);
        }
    } else {
        if d.deg == 2 {
            w.escape(ESC_QUAD);
        } else {
            w.op(OP_CUBIC);
        }
        let mut from = cur;
        for &p in &d.pts[..d.deg as usize] {
            w.delta(from, p);
            from = p;
        }
    }
    e.push_rev(cur, d);
    1
}

// ── the encoder's search: floats estimate a split, integers decide it ──

fn eval_f(seg: &Seg, t: f64) -> (f64, f64) {
    let mut x = [0f64; 4];
    let mut y = [0f64; 4];
    x[0] = seg.start.0 as f64;
    y[0] = seg.start.1 as f64;
    let d = seg.draw.deg as usize;
    for j in 0..d {
        x[j + 1] = seg.draw.pts[j].0 as f64;
        y[j + 1] = seg.draw.pts[j].1 as f64;
    }
    for level in 0..d {
        for j in 0..d - level {
            x[j] += (x[j + 1] - x[j]) * t;
            y[j] += (y[j + 1] - y[j]) * t;
        }
    }
    (x[0], y[0])
}

/// The bounding box of a segment's control polygon holds a point, with a margin.
fn near_box(seg: &Seg, p: P) -> bool {
    let m = NEAR as i64 + 1;
    let (mut x0, mut y0, mut x1, mut y1) =
        (seg.start.0 as i64, seg.start.1 as i64, seg.start.0 as i64, seg.start.1 as i64);
    for q in &seg.draw.pts[..seg.draw.deg as usize] {
        x0 = x0.min(q.0 as i64);
        y0 = y0.min(q.1 as i64);
        x1 = x1.max(q.0 as i64);
        y1 = y1.max(q.1 as i64);
    }
    (p.0 as i64) >= x0 - m && (p.0 as i64) <= x1 + m && (p.1 as i64) >= y0 - m && (p.1 as i64) <= y1 + m
}

/// The distance from a point to a segment, and the `t` of the nearest point, from a
/// polyline of the segment.
fn project(seg: &Seg, p: P) -> (f64, f64) {
    let mut len = 0.0;
    let mut c = (seg.start.0 as f64, seg.start.1 as f64);
    for q in &seg.draw.pts[..seg.draw.deg as usize] {
        len += ((q.0 as f64 - c.0).powi(2) + (q.1 as f64 - c.1).powi(2)).sqrt();
        c = (q.0 as f64, q.1 as f64);
    }
    let n = if seg.draw.deg == 1 { 1 } else { ((len / 0.5).ceil() as usize).clamp(8, 4000) };
    let pf = (p.0 as f64, p.1 as f64);
    let mut best = (f64::MAX, 0.0);
    let mut prev = eval_f(seg, 0.0);
    for i in 1..=n {
        let q = eval_f(seg, i as f64 / n as f64);
        let (dx, dy) = (q.0 - prev.0, q.1 - prev.1);
        let l2 = dx * dx + dy * dy;
        let u = if l2 > 0.0 { (((pf.0 - prev.0) * dx + (pf.1 - prev.1) * dy) / l2).clamp(0.0, 1.0) } else { 0.0 };
        let d = ((pf.0 - prev.0 - u * dx).powi(2) + (pf.1 - prev.1 - u * dy).powi(2)).sqrt();
        if d < best.0 {
            best = (d, (i as f64 - 1.0 + u) / n as f64);
        }
        prev = q;
    }
    best
}

/// The cost of a set of corrections: their sum of magnitudes, then the largest.
fn cost(c: &[i32]) -> (i64, i64) {
    (c.iter().map(|v| v.unsigned_abs() as i64).sum(), c.iter().map(|v| v.unsigned_abs() as i64).max().unwrap_or(0))
}

/// The best grid value near `estimate` for a free `v` from 1 to `one - 1`, under `f`, which
/// gives the corrections for a `v`. Ties go to the value nearest the estimate, then to
/// the smaller one.
fn search(one: u32, estimate: f64, mut f: impl FnMut(u32) -> Corr) -> Option<(u32, Corr)> {
    let v0 = ((estimate * one as f64).round() as i64).clamp(1, one as i64 - 1);
    let mut best: Option<(u32, Corr, (i64, i64))> = None;
    // v0, then v0 - 1, v0 + 1, v0 - 2, ...: only a lower cost replaces the best
    for k in 0..=2 * SEARCH {
        let v = if k % 2 == 1 { v0 - (k + 1) / 2 } else { v0 + k / 2 };
        if v < 1 || v >= one as i64 {
            continue;
        }
        let c = f(v as u32);
        let cc = cost(c.as_slice());
        if best.as_ref().is_none_or(|b| cc < b.2) {
            best = Some((v as u32, c, cc));
        }
    }
    best.map(|b| (b.0, b.1))
}

/// Up to six corrections.
#[derive(Clone, Copy, Default)]
struct Corr {
    v: [i32; 6],
    n: usize,
}
impl Corr {
    fn as_slice(&self) -> &[i32] {
        &self.v[..self.n]
    }
}

fn find_on(g: &Group, p: P) -> Option<On> {
    let mut best: Option<(i64, On)> = None;
    for (s, seg) in g.segs.iter().enumerate() {
        if !near_box(seg, p) {
            continue;
        }
        let (dist, t) = project(seg, p);
        if dist > NEAR {
            continue;
        }
        let one = 1u32 << seg.bits;
        let Some((v, c)) = search(one, t, |v| {
            let q = point_at(seg, u_of(v, seg.bits));
            Corr { v: [p.0.wrapping_sub(q.0), p.1.wrapping_sub(q.1), 0, 0, 0, 0], n: 2 }
        }) else {
            continue;
        };
        let k = cost(c.as_slice());
        if k.1 <= MAX_CORR as i64 && best.as_ref().is_none_or(|b| k.0 < b.0) {
            best = Some((k.0, On { seg: s as u32, v, corr: [c.v[0], c.v[1]] }));
        }
    }
    best.map(|b| b.1)
}

/// A part of a segment, as the encoder chose it.
struct Fit {
    code: u8,
    seg: u32,
    /// The position among the segments that start at the current point, when there are
    /// several.
    pos: Option<u32>,
    /// The stored start, when the current point is neither a segment start nor in the
    /// registry.
    start_v: Option<u32>,
    end_v: u32,
    back: u32,
    corr: [i32; 6],
    n_corr: usize,
}

/// The corrections of the part `[v0, v1]` of `seg` that draws `d`: every point but a
/// fixed end, x then y.
fn part_corr(seg: &Seg, u0: i64, u1: i64, d: &Draw, end_fixed: bool) -> Corr {
    let deg = seg.draw.deg as usize;
    let mut c = Corr::default();
    for i in 1..=deg {
        if i == deg && end_fixed {
            break;
        }
        let q = part_point(seg, i, u0, u1);
        c.v[c.n] = d.pts[i - 1].0.wrapping_sub(q.0);
        c.v[c.n + 1] = d.pts[i - 1].1.wrapping_sub(q.1);
        c.n += 2;
    }
    c
}

/// The best part of a segment that draws `d` from `cur`: from the start that the decoder
/// infers when it can, else from a stored start.
fn find_part(g: &Group, cur: P, d: &Draw) -> Option<Fit> {
    let list = g.starting_at(cur);
    // (segment, position, start v, the start is stored)
    let mut cands: Vec<(u32, Option<u32>, u32, bool)> = Vec::new();
    if !list.is_empty() {
        for (pos, &(_, s)) in list.iter().enumerate() {
            cands.push((s, (list.len() > 1).then_some(pos as u32), 0, false));
        }
    } else if let Some(&r) = g.reg_at.get(&cur) {
        let (_, s, v) = g.reg[r as usize];
        cands.push((s, None, v, false));
    }
    if let Some(f) = best_part(g, cur, d, &cands) {
        return Some(f);
    }
    cands.clear();
    for (s, seg) in g.segs.iter().enumerate() {
        if seg.draw.deg == d.deg && near_box(seg, cur) {
            cands.push((s as u32, None, 0, true));
        }
    }
    best_part(g, cur, d, &cands)
}

fn best_part(g: &Group, cur: P, d: &Draw, cands: &[(u32, Option<u32>, u32, bool)]) -> Option<Fit> {
    let end = d.end();
    let mut best: Option<(Fit, (i64, i64))> = None;
    for &(s, pos, v0_fixed, free) in cands {
        let seg = &g.segs[s as usize];
        if seg.draw.deg != d.deg {
            continue;
        }
        let bits = seg.bits;
        let one = 1u32 << bits;
        let mut v0 = v0_fixed;
        if free {
            let (dist, t) = project(seg, cur);
            if dist > NEAR {
                continue;
            }
            v0 = ((t * one as f64).round() as i64).clamp(1, one as i64 - 1) as u32;
        }
        // where the part ends
        let (code, mut v1, back, end_fixed) = if seg.draw.end() == end {
            (OP_PART_TO_END, one, 0, true)
        } else if let Some(&r) = g.reg_at.get(&end).filter(|&&r| g.reg[r as usize].1 == s) {
            (OP_PART_TO_REG, g.reg[r as usize].2, (g.reg.len() - 1 - r as usize) as u32, true)
        } else {
            let (dist, t1) = project(seg, end);
            if dist > NEAR {
                continue;
            }
            (OP_PART_TO_NEW, ((t1 * one as f64).round() as i64).clamp(1, one as i64 - 1) as u32, 0, false)
        };
        let corr_of = |v0: u32, v1: u32| part_corr(seg, u_of(v0, bits), u_of(v1, bits), d, end_fixed);
        let mut c = corr_of(v0, v1);
        if !end_fixed {
            match search(one, v1 as f64 / one as f64, |v| corr_of(v0, v)) {
                Some((v, cc)) => {
                    v1 = v;
                    c = cc;
                }
                None => continue,
            }
        }
        if free {
            // the start is free too: search each in turn
            for _ in 0..2 {
                if let Some((v, cc)) = search(one, v0 as f64 / one as f64, |v| corr_of(v, v1)) {
                    if cost(cc.as_slice()) < cost(c.as_slice()) {
                        v0 = v;
                        c = cc;
                    }
                }
                if !end_fixed {
                    if let Some((v, cc)) = search(one, v1 as f64 / one as f64, |v| corr_of(v0, v)) {
                        if cost(cc.as_slice()) < cost(c.as_slice()) {
                            v1 = v;
                            c = cc;
                        }
                    }
                }
            }
        }
        if u_of(v1, bits) <= u_of(v0, bits) {
            continue;
        }
        let k = cost(c.as_slice());
        if k.1 > MAX_CORR as i64 {
            continue;
        }
        if best.as_ref().is_none_or(|b| k < b.1) {
            let fit = Fit {
                code: if free { code - OP_PART_TO_END + OP_PART_X_TO_END } else { code },
                seg: s,
                pos,
                start_v: free.then_some(v0),
                end_v: v1,
                back,
                corr: c.v,
                n_corr: c.n,
            };
            best = Some((fit, k));
        }
    }
    best.map(|b| b.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seg(start: P, d: Draw) -> Seg {
        Seg { start, draw: d, sub0: 0, sub_len: 1, bits: v_bits(start, &d) }
    }

    #[test]
    fn a_split_keeps_the_ends() {
        let s = seg((0, 0), Draw { deg: 3, pts: [(100, 300), (400, 300), (500, 0)] });
        assert_eq!(point_at(&s, 0), (0, 0));
        assert_eq!(point_at(&s, 1 << U_BITS), (500, 0));
        // the middle of a symmetric cubic: (0 + 3·100 + 3·400 + 500) / 8, (3·300 + 3·300) / 8
        assert_eq!(point_at(&s, 1 << (U_BITS - 1)), (250, 225));
        // the whole segment as a part gives its own control points back
        for i in 1..=3 {
            assert_eq!(part_point(&s, i, 0, 1 << U_BITS), s.draw.pts[i - 1]);
        }
    }

    #[test]
    fn a_split_at_the_envelope_does_not_overflow() {
        let m = COORD_LIMIT as i32 - 1;
        let s = seg((-m, -m), Draw { deg: 3, pts: [(m, -m), (-m, m), (m, m)] });
        for u in [0, 1, 12345, 1 << 15, (1 << U_BITS) - 1, 1 << U_BITS] {
            assert!(in_envelope(point_at(&s, u)), "{u}");
            for i in 1..=3 {
                part_point(&s, i, u / 3, u);
            }
        }
    }

    #[test]
    fn the_split_rounds_by_its_rule() {
        // a line from 0 to 3 at u = 1/2: 1.5 rounds up, -1.5 rounds up to -1
        let s = seg((0, 0), Draw::line((3, -3)));
        assert_eq!(point_at(&s, 1 << (U_BITS - 1)), (2, -1));
        // each step floors: 2^14 · 1 / 2^16 is 0.25, which floors to 0
        let s = seg((0, 0), Draw::line((1, 0)));
        assert_eq!(point_at(&s, 1), (0, 0));
        assert_eq!(point_at(&s, 1 << (U_BITS - 1)), (1, 0));
    }

    #[test]
    fn v_bits_follow_the_polygon_length() {
        assert_eq!(v_bits((0, 0), &Draw::line((0, 0))), 2);
        assert_eq!(v_bits((0, 0), &Draw::line((3, 1))), 3);
        assert_eq!(v_bits((0, 0), &Draw::line((100, -40))), 8);
        let m = COORD_LIMIT as i32 - 1;
        assert_eq!(v_bits((-m, -m), &Draw { deg: 3, pts: [(m, m), (-m, -m), (m, m)] }), 16);
    }
}
