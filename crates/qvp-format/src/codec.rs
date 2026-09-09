//! The QVP page codec: `PageData` ⇄ bytes.
//!
//! The file stores only what cannot be worked out again, and groups like bytes together:
//!
//! * **Tables are struct-of-arrays, varint/zigzag-delta coded.** Their fields are
//!   sorted or near-sorted, so deltas are small; fixed 2/4-byte fields were not.
//! * **Bboxes are not stored.** A path's is the bbox of its own decoded points
//!   (control points included); a word's, ayah's, line's and deco's is the union of
//!   its members'. They would be 16 bytes of near-random `i32` per record — also the
//!   part a compressor can do least with.
//! * **`PathRec::ox/oy` and `op_off` are not stored.** The origin is the owning group's
//!   bbox minimum, and runs are contiguous, so both fall out of the same pass that
//!   recomputes the bboxes.
//! * **The opcode stream is four streams.** Two bits of opcode per command packed four
//!   to a byte, an explicit-`Z` index list (only 5,031 of 805,655 contours carry one),
//!   and x and y deltas apart — which costs nothing raw but hands a compressor two
//!   homogeneous streams instead of one interleaved one.
//!
//! Together that is 27% smaller raw and 19.6% smaller brotli-11 than storing the
//! records as they sit in memory: over all 604 pages, 111.61 → 81.51 MB raw and
//! 46.30 → 37.25 MB brotli. The cost is paid at load, rebuilding what was dropped:
//! about 0.28 ms per page once decompression is counted.
//!
//! Two boxes are stored rather than derived: a glyph instance's (it was measured on the
//! original coordinates under the instance transform, so recomputing it from the
//! quantised outline can round a unit out) and a glyph outline's.
//!
//! Ops runs are written in **canonical order** — inline paths by path index, then glyph
//! outlines by glyph index. [`PageData::canonicalize_ops`] puts a page in that order and
//! the converter calls it, so `op_off` survives the round trip unchanged.

use crate::*;

/// magic(4) + version..pad(36) + six u32 section lengths(24).
const HEADER_LEN: usize = 64;

// ───────────────────────────── helpers ─────────────────────────────

fn vi(o: &mut Vec<u8>, v: u32) {
    write_varint(o, v);
}
fn zv(o: &mut Vec<u8>, v: i32) {
    write_varint(o, zigzag(v));
}
/// `NONE_U16` ↔ 0, everything else biased by one, so the common "absent" case is one byte.
fn opt(v: u16) -> u32 {
    if v == NONE_U16 {
        0
    } else {
        v as u32 + 1
    }
}
fn unopt(v: u32) -> u16 {
    if v == 0 {
        NONE_U16
    } else {
        (v - 1) as u16
    }
}

struct Rd<'a> {
    b: &'a [u8],
    pos: usize,
}
impl<'a> Rd<'a> {
    fn vi(&mut self, what: &'static str) -> Result<u32, Error> {
        read_varint(self.b, &mut self.pos).map_err(|_| Error::Truncated(what))
    }
    fn zv(&mut self, what: &'static str) -> Result<i32, Error> {
        Ok(unzigzag(self.vi(what)?))
    }
    fn u8(&mut self, what: &'static str) -> Result<u8, Error> {
        let v = *self.b.get(self.pos).ok_or(Error::Truncated(what))?;
        self.pos += 1;
        Ok(v)
    }
    fn f32(&mut self, what: &'static str) -> Result<f32, Error> {
        let s = self.b.get(self.pos..self.pos + 4).ok_or(Error::Truncated(what))?;
        self.pos += 4;
        Ok(f32::from_bits(u32::from_le_bytes(s.try_into().unwrap())))
    }
}

/// The commands of one run, and how many of them are not `Close`.
struct Run {
    cmds: Vec<Cmd>,
    n_ops: u32,
}

fn split_runs(p: &PageData) -> Result<Vec<Run>, Error> {
    let mut runs = Vec::with_capacity(p.paths.len() + p.glyphs.len());
    for (i, r) in p.paths.iter().enumerate() {
        if r.flags & PF_GLYPH != 0 {
            continue;
        }
        let cmds = p.path_cmds(i)?;
        let n_ops = cmds.iter().filter(|c| !matches!(c, Cmd::Close)).count() as u32;
        runs.push(Run { cmds, n_ops });
    }
    for g in &p.glyphs {
        let ops = p.ops.get(g.op_off as usize..(g.op_off + g.op_len) as usize).ok_or(Error::Truncated("glyph ops"))?;
        let cmds = decode_cmds(ops, 0, 0)?;
        let n_ops = cmds.iter().filter(|c| !matches!(c, Cmd::Close)).count() as u32;
        runs.push(Run { cmds, n_ops });
    }
    Ok(runs)
}

// ───────────────────────────── encode ─────────────────────────────

pub fn encode(p: &PageData) -> Vec<u8> {
    let runs = split_runs(p).expect("page geometry must be walkable to encode it");

    // ── ops: 2-bit opcodes, explicit-Z indices, x and y deltas ──
    let (mut opbits, mut zs, mut xs, mut ys) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut z_at: Vec<u32> = Vec::new();
    let mut bit = 0u32;
    let mut acc = 0u8;
    let mut n_ops_total: u32 = 0;
    let mut push_op = |code: u8, opbits: &mut Vec<u8>| {
        acc |= code << (bit * 2);
        bit += 1;
        if bit == 4 {
            opbits.push(acc);
            acc = 0;
            bit = 0;
        }
    };
    let (mut cx, mut cy) = (0i32, 0i32);
    for run in &runs {
        // Each run starts from the previous run's first point; glyph runs start at 0,0.
        let put = |x: i32, y: i32, cx: &mut i32, cy: &mut i32, xs: &mut Vec<u8>, ys: &mut Vec<u8>| {
            zv(xs, x - *cx);
            zv(ys, y - *cy);
            *cx = x;
            *cy = y;
        };
        for c in &run.cmds {
            match *c {
                Cmd::MoveTo(x, y) => {
                    push_op(0, &mut opbits);
                    n_ops_total += 1;
                    put(x, y, &mut cx, &mut cy, &mut xs, &mut ys);
                }
                Cmd::LineTo(x, y) => {
                    push_op(1, &mut opbits);
                    n_ops_total += 1;
                    put(x, y, &mut cx, &mut cy, &mut xs, &mut ys);
                }
                Cmd::QuadTo(x1, y1, x, y) => {
                    push_op(2, &mut opbits);
                    n_ops_total += 1;
                    put(x1, y1, &mut cx, &mut cy, &mut xs, &mut ys);
                    put(x, y, &mut cx, &mut cy, &mut xs, &mut ys);
                }
                Cmd::CubicTo(x1, y1, x2, y2, x, y) => {
                    push_op(3, &mut opbits);
                    n_ops_total += 1;
                    put(x1, y1, &mut cx, &mut cy, &mut xs, &mut ys);
                    put(x2, y2, &mut cx, &mut cy, &mut xs, &mut ys);
                    put(x, y, &mut cx, &mut cy, &mut xs, &mut ys);
                }
                // A Close carries no point; record where it sat so it can be put back.
                Cmd::Close => z_at.push(n_ops_total),
            }
        }
        // The next run's deltas are measured from this one's first point, not its last.
        if let Some(&Cmd::MoveTo(x, y)) = run.cmds.first() {
            cx = x;
            cy = y;
        }
    }
    if bit != 0 {
        opbits.push(acc);
    }
    vi(&mut zs, z_at.len() as u32);
    let mut prev = 0u32;
    for z in &z_at {
        vi(&mut zs, z - prev);
        prev = *z;
    }

    // ── tables ──
    let mut t: Vec<u8> = Vec::with_capacity(p.paths.len() * 8 + p.words.len() * 16 + 512);
    for l in &p.lines {
        t.push(l.line_no);
        vi(&mut t, l.n_words as u32);
    }
    let (mut ps, mut pa) = (0i32, 0i32);
    for a in &p.ayahs {
        zv(&mut t, a.surah as i32 - ps);
        zv(&mut t, a.ayah as i32 - pa);
        ps = a.surah as i32;
        pa = a.ayah as i32;
        t.push(a.fragment);
        t.push(a.fragments);
        t.push(a.flags);
        vi(&mut t, a.n_words as u32);
        vi(&mut t, opt(a.ayah_mark_deco));
        vi(&mut t, a.rubu_al_hizb as u32);
    }
    let (mut ws, mut wa, mut ww, mut wl, mut wai) = (0i32, 0i32, 0i32, 0i32, 0i32);
    let mut wend = 0i64;
    for x in &p.words {
        zv(&mut t, (x.first_path as i64 - wend) as i32);
        wend = x.first_path as i64 + x.n_paths as i64;
        zv(&mut t, x.surah as i32 - ws);
        zv(&mut t, x.ayah as i32 - wa);
        zv(&mut t, x.word as i32 - ww);
        zv(&mut t, x.line_idx as i32 - wl);
        zv(&mut t, x.ayah_idx as i32 - wai);
        ws = x.surah as i32;
        wa = x.ayah as i32;
        ww = x.word as i32;
        wl = x.line_idx as i32;
        wai = x.ayah_idx as i32;
        for v in [x.text, x.rasm_imlai, x.qpc, x.rasm, x.search] {
            vi(&mut t, opt(v));
        }
        vi(&mut t, x.n_paths as u32);
    }
    // paths, column by column: like bytes next to like bytes
    for x in &p.paths {
        t.push(x.kind as u8);
    }
    for x in &p.paths {
        t.push(x.mark as u8);
    }
    for x in &p.paths {
        t.push(x.family as u8);
    }
    for x in &p.paths {
        t.push(x.flags);
    }
    let mut ri = 0usize;
    for x in &p.paths {
        if x.flags & PF_GLYPH != 0 {
            vi(&mut t, x.op_off as u32); // instance index
        } else {
            vi(&mut t, runs[ri].n_ops as u32);
            ri += 1;
        }
    }
    // A glyph instance's box was measured on the ORIGINAL coordinates under the instance
    // transform. Recomputing it from the already-quantised outline rounds twice and can
    // land a unit out (page 144, 6:128), so these few thousand boxes are stored.
    for x in p.paths.iter().filter(|x| x.flags & PF_GLYPH != 0) {
        zv(&mut t, x.bbox.x0);
        zv(&mut t, x.bbox.y0);
        zv(&mut t, x.bbox.x1 - x.bbox.x0);
        zv(&mut t, x.bbox.y1 - x.bbox.y0);
    }
    let mut dend = 0i64;
    let mut ds = 0i32;
    for d in &p.decos {
        zv(&mut t, (d.first_path as i64 - dend) as i32);
        dend = d.first_path as i64 + d.n_paths as i64;
        t.push(d.kind as u8);
        zv(&mut t, d.surah as i32 - ds);
        ds = d.surah as i32;
        vi(&mut t, d.ayah as u32);
        vi(&mut t, opt(d.text));
        vi(&mut t, d.n_paths as u32);
        vi(&mut t, opt(d.line));
    }
    for (g, r) in p.glyphs.iter().zip(&runs[p.paths.iter().filter(|x| x.flags & PF_GLYPH == 0).count()..]) {
        vi(&mut t, r.n_ops);
        zv(&mut t, g.bbox.x0);
        zv(&mut t, g.bbox.y0);
        zv(&mut t, g.bbox.x1 - g.bbox.x0);
        zv(&mut t, g.bbox.y1 - g.bbox.y0);
    }
    for i in &p.insts {
        t.extend_from_slice(&i.glyph.to_le_bytes());
        for v in [i.a, i.b, i.c, i.d, i.e, i.f] {
            t.extend_from_slice(&v.to_bits().to_le_bytes());
        }
    }

    let mut strings: Vec<u8> = Vec::new();
    for s in &p.strings {
        let b = s.as_bytes();
        vi(&mut strings, b.len() as u32);
        strings.extend_from_slice(b);
    }

    // ── header ──
    let mut o = Vec::with_capacity(HEADER_LEN + t.len() + opbits.len() + zs.len() + xs.len() + ys.len() + strings.len());
    o.extend_from_slice(MAGIC);
    let u16v = |o: &mut Vec<u8>, v: u16| o.extend_from_slice(&v.to_le_bytes());
    u16v(&mut o, VERSION);
    u16v(&mut o, p.header.quant);
    u16v(&mut o, p.header.page);
    u16v(&mut o, p.header.flags);
    o.extend_from_slice(&p.header.width.to_bits().to_le_bytes());
    o.extend_from_slice(&p.header.height.to_bits().to_le_bytes());
    u16v(&mut o, p.lines.len() as u16);
    u16v(&mut o, p.ayahs.len() as u16);
    u16v(&mut o, p.words.len() as u16);
    u16v(&mut o, p.decos.len() as u16);
    o.extend_from_slice(&(p.paths.len() as u32).to_le_bytes());
    u16v(&mut o, p.strings.len() as u16);
    u16v(&mut o, p.glyphs.len() as u16);
    u16v(&mut o, p.insts.len() as u16);
    u16v(&mut o, 0);
    for len in [t.len(), opbits.len(), zs.len(), xs.len(), ys.len(), strings.len()] {
        o.extend_from_slice(&(len as u32).to_le_bytes());
    }
    assert_eq!(o.len(), HEADER_LEN, "v2 header length");
    for s in [&t, &opbits, &zs, &xs, &ys, &strings] {
        o.extend_from_slice(s);
    }
    o
}

// ───────────────────────────── decode ─────────────────────────────

pub fn decode(b: &[u8]) -> Result<PageData, Error> {
    if b.len() < HEADER_LEN {
        return Err(Error::Truncated("header"));
    }
    let u16a = |at: usize| u16::from_le_bytes([b[at], b[at + 1]]);
    let u32a = |at: usize| u32::from_le_bytes(b[at..at + 4].try_into().unwrap());
    let header = Header {
        version: u16a(4),
        quant: u16a(6),
        page: u16a(8),
        flags: u16a(10),
        width: f32::from_bits(u32a(12)),
        height: f32::from_bits(u32a(16)),
    };
    let (n_lines, n_ayahs, n_words, n_decos) = (u16a(20) as usize, u16a(22) as usize, u16a(24) as usize, u16a(26) as usize);
    let n_paths = u32a(28) as usize;
    let (n_strings, n_glyphs, n_insts) = (u16a(32) as usize, u16a(34) as usize, u16a(36) as usize);
    let mut at = HEADER_LEN;
    let mut sec = [0usize; 6];
    for (i, s) in sec.iter_mut().enumerate() {
        let len = u32a(40 + i * 4) as usize;
        *s = at;
        at += len;
        if at > b.len() {
            return Err(Error::Truncated("section"));
        }
    }
    let (o_t, o_op, o_zs, o_xs, o_ys, o_str) = (sec[0], sec[1], sec[2], sec[3], sec[4], sec[5]);
    let mut t = Rd { b, pos: o_t };

    // ── tables (bboxes and origins are filled in after the geometry is decoded) ──
    let mut lines = Vec::with_capacity(n_lines);
    let mut fw = 0u32;
    for _ in 0..n_lines {
        let line_no = t.u8("line")?;
        let n_words = t.vi("line")? as u16;
        lines.push(LineRec { line_no, first_word: fw as u16, n_words, bbox: IBox::EMPTY });
        fw += n_words as u32;
    }
    let mut ayahs = Vec::with_capacity(n_ayahs);
    let (mut ps, mut pa) = (0i32, 0i32);
    let mut afw = 0u32;
    for _ in 0..n_ayahs {
        ps += t.zv("ayah")?;
        pa += t.zv("ayah")?;
        let fragment = t.u8("ayah")?;
        let fragments = t.u8("ayah")?;
        let flags = t.u8("ayah")?;
        let n_words = t.vi("ayah")? as u16;
        let ayah_mark_deco = unopt(t.vi("ayah")?);
        let rubu_al_hizb = t.vi("ayah")? as u16;
        ayahs.push(AyahRec {
            surah: ps as u16,
            ayah: pa as u16,
            fragment,
            fragments,
            flags,
            first_word: afw as u16,
            n_words,
            ayah_mark_deco,
            rubu_al_hizb,
            bbox: IBox::EMPTY,
        });
        afw += n_words as u32;
    }
    let mut words = Vec::with_capacity(n_words);
    let (mut ws, mut wa, mut ww, mut wl, mut wai) = (0i32, 0i32, 0i32, 0i32, 0i32);
    let mut wend = 0i64;
    for _ in 0..n_words {
        let first_path = (wend + t.zv("word")? as i64) as u32;
        ws += t.zv("word")?;
        wa += t.zv("word")?;
        ww += t.zv("word")?;
        wl += t.zv("word")?;
        wai += t.zv("word")?;
        let text = unopt(t.vi("word")?);
        let rasm_imlai = unopt(t.vi("word")?);
        let qpc = unopt(t.vi("word")?);
        let rasm = unopt(t.vi("word")?);
        let search = unopt(t.vi("word")?);
        let n_paths = t.vi("word")? as u16;
        wend = first_path as i64 + n_paths as i64;
        words.push(WordRec {
            surah: ws as u16,
            ayah: wa as u16,
            word: ww as u16,
            line_idx: wl as u16,
            ayah_idx: wai as u16,
            text,
            rasm_imlai,
            qpc,
            rasm,
            search,
            first_path,
            n_paths,
            bbox: IBox::EMPTY,
        });
    }
    if t.pos + n_paths * 4 > b.len() {
        return Err(Error::Truncated("paths"));
    }
    let (c_kind, c_mark, c_family, c_flags) = (t.pos, t.pos + n_paths, t.pos + n_paths * 2, t.pos + n_paths * 3);
    t.pos += n_paths * 4;
    let mut paths = Vec::with_capacity(n_paths);
    let mut n_ops_of = Vec::with_capacity(n_paths);
    for i in 0..n_paths {
        let flags = b[c_flags + i];
        let v = t.vi("path")?;
        let (op_off, n_ops) = if flags & PF_GLYPH != 0 { (v as u32, 0) } else { (0, v as u32) };
        n_ops_of.push(n_ops);
        paths.push(PathRec {
            kind: PathKind::from_u8(b[c_kind + i]),
            mark: Mark::from_u8(b[c_mark + i]),
            family: Family::from_u8(b[c_family + i]),
            flags,
            ox: 0,
            oy: 0,
            op_off,
            op_len: 0,
            bbox: IBox::EMPTY,
        });
    }
    let mut inst_bbox = Vec::new();
    for _ in 0..paths.iter().filter(|x| x.flags & PF_GLYPH != 0).count() {
        let x0 = t.zv("inst bbox")?;
        let y0 = t.zv("inst bbox")?;
        let w = t.zv("inst bbox")?;
        let h = t.zv("inst bbox")?;
        inst_bbox.push(IBox { x0, y0, x1: x0 + w, y1: y0 + h });
    }
    let mut decos = Vec::with_capacity(n_decos);
    let mut dend = 0i64;
    let mut ds = 0i32;
    for _ in 0..n_decos {
        let first_path = (dend + t.zv("deco")? as i64) as u32;
        let kind = DecoKind::from_u8(t.u8("deco")?);
        ds += t.zv("deco")?;
        let ayah = t.vi("deco")? as u16;
        let text = unopt(t.vi("deco")?);
        let n_paths = t.vi("deco")? as u16;
        let line = unopt(t.vi("deco")?);
        dend = first_path as i64 + n_paths as i64;
        decos.push(DecoRec { kind, surah: ds as u16, ayah, text, first_path, n_paths, line, bbox: IBox::EMPTY });
    }
    let mut glyph_ops = Vec::with_capacity(n_glyphs);
    let mut glyph_bbox = Vec::with_capacity(n_glyphs);
    for _ in 0..n_glyphs {
        glyph_ops.push(t.vi("glyph")?);
        let x0 = t.zv("glyph bbox")?;
        let y0 = t.zv("glyph bbox")?;
        let w = t.zv("glyph bbox")?;
        let h = t.zv("glyph bbox")?;
        glyph_bbox.push(IBox { x0, y0, x1: x0 + w, y1: y0 + h });
    }
    let mut insts = Vec::with_capacity(n_insts);
    for _ in 0..n_insts {
        let glyph = u16::from_le_bytes([t.u8("inst")?, t.u8("inst")?]);
        let (a, bb, c, d, e, f) =
            (t.f32("inst")?, t.f32("inst")?, t.f32("inst")?, t.f32("inst")?, t.f32("inst")?, t.f32("inst")?);
        insts.push(InstRec { glyph, a, b: bb, c, d, e, f });
    }

    // The tables are one varint run: if the reader did not land exactly on the next
    // section, the file disagrees with this reader and every value after the first
    // mismatch is noise. Fail here rather than hand back a plausible-looking page.
    if t.pos != o_op {
        return Err(Error::Corrupt("tables"));
    }

    // ── geometry: walk the four streams once, rebuilding absolute commands ──
    let mut zr = Rd { b, pos: o_zs };
    let n_z = zr.vi("z")? as usize;
    let mut z_next: Vec<u32> = Vec::with_capacity(n_z);
    let mut acc = 0u32;
    for _ in 0..n_z {
        acc += zr.vi("z")?;
        z_next.push(acc);
    }
    let mut z_i = 0usize;
    let mut xr = Rd { b, pos: o_xs };
    let mut yr = Rd { b, pos: o_ys };
    let opbits = &b[o_op..o_op + (o_zs - o_op)];
    let mut op_i = 0usize;
    let mut seen: u32 = 0;
    let (mut cx, mut cy) = (0i32, 0i32);

    let n_inline = paths.iter().filter(|x| x.flags & PF_GLYPH == 0).count();
    let mut run_lens: Vec<u32> = Vec::with_capacity(n_inline + n_glyphs);
    run_lens.extend(n_ops_of.iter().zip(&paths).filter(|(_, p)| p.flags & PF_GLYPH == 0).map(|(n, _)| *n));
    run_lens.extend(glyph_ops.iter().copied());

    // One buffer for the whole page, sliced per run: a Vec per run costs more in
    // allocator time than the whole varint pass.
    let total: usize = run_lens.iter().map(|n| *n as usize).sum();
    let mut cmds: Vec<Cmd> = Vec::with_capacity(total + n_z);
    let mut run_at: Vec<u32> = Vec::with_capacity(run_lens.len() + 1);

    for n_ops in &run_lens {
        run_at.push(cmds.len() as u32);
        for _ in 0..*n_ops {
            let byte = *opbits.get(op_i / 4).ok_or(Error::Truncated("opcodes"))?;
            let code = (byte >> ((op_i % 4) * 2)) & 3;
            op_i += 1;
            macro_rules! pt {
                () => {{
                    cx += xr.zv("x")?;
                    cy += yr.zv("y")?;
                    (cx, cy)
                }};
            }
            cmds.push(match code {
                0 => {
                    let (x, y) = pt!();
                    Cmd::MoveTo(x, y)
                }
                1 => {
                    let (x, y) = pt!();
                    Cmd::LineTo(x, y)
                }
                2 => {
                    let (x1, y1) = pt!();
                    let (x, y) = pt!();
                    Cmd::QuadTo(x1, y1, x, y)
                }
                _ => {
                    let (x1, y1) = pt!();
                    let (x2, y2) = pt!();
                    let (x, y) = pt!();
                    Cmd::CubicTo(x1, y1, x2, y2, x, y)
                }
            });
            seen += 1;
            while z_i < z_next.len() && z_next[z_i] == seen {
                cmds.push(Cmd::Close);
                z_i += 1;
            }
        }
        if let Some(&Cmd::MoveTo(x, y)) = cmds.get(*run_at.last().unwrap() as usize) {
            cx = x;
            cy = y;
        }
    }
    run_at.push(cmds.len() as u32);
    let run = |i: usize| &cmds[run_at[i] as usize..run_at[i + 1] as usize];

    // ── derive: path bbox from its own points, group origin from the group's union ──
    let mut run_of = vec![usize::MAX; paths.len()];
    let mut k = 0usize;
    for (i, r) in paths.iter().enumerate() {
        if r.flags & PF_GLYPH == 0 {
            run_of[i] = k;
            k += 1;
        }
    }
    let mut gi = 0usize;
    for (i, r) in paths.iter_mut().enumerate() {
        r.bbox = if r.flags & PF_GLYPH != 0 {
            gi += 1;
            inst_bbox[gi - 1]
        } else {
            cmds_bbox(run(run_of[i]))
        };
    }
    let group_bbox = |paths: &[PathRec], first: u32, n: u16| {
        let mut bb = IBox::EMPTY;
        for r in paths.iter().skip(first as usize).take(n as usize) {
            bb.union(&r.bbox);
        }
        bb
    };
    let set_origin = |paths: &mut Vec<PathRec>, first: u32, n: u16| -> IBox {
        let bb = group_bbox(paths, first, n);
        let (ox, oy) = if bb.is_empty() { (0, 0) } else { (bb.x0, bb.y0) };
        for r in paths.iter_mut().skip(first as usize).take(n as usize) {
            if r.flags & PF_GLYPH == 0 {
                r.ox = ox;
                r.oy = oy;
            }
        }
        bb
    };
    for i in 0..words.len() {
        words[i].bbox = set_origin(&mut paths, words[i].first_path, words[i].n_paths);
    }
    for i in 0..decos.len() {
        decos[i].bbox = set_origin(&mut paths, decos[i].first_path, decos[i].n_paths);
    }
    let wbox = |first: u16, n: u16| {
        let mut bb = IBox::EMPTY;
        for w in words.iter().skip(first as usize).take(n as usize) {
            bb.union(&w.bbox);
        }
        bb
    };
    for a in &mut ayahs {
        a.bbox = wbox(a.first_word, a.n_words);
    }
    for l in &mut lines {
        l.bbox = wbox(l.first_word, l.n_words);
    }

    // ── re-emit the v1 opcode stream in canonical order ──
    let mut ops: Vec<u8> = Vec::with_capacity(o_str - o_xs);
    for (i, r) in paths.iter_mut().enumerate() {
        if r.flags & PF_GLYPH != 0 {
            r.op_len = 0;
            continue;
        }
        let off = ops.len() as u32;
        encode_cmds(run(run_of[i]), r.ox, r.oy, &mut ops);
        r.op_off = off;
        r.op_len = ops.len() as u32 - off;
    }
    let mut glyphs = Vec::with_capacity(n_glyphs);
    for gi in 0..n_glyphs {
        let off = ops.len() as u32;
        encode_cmds(run(n_inline + gi), 0, 0, &mut ops);
        glyphs.push(GlyphRec { op_off: off, op_len: ops.len() as u32 - off, bbox: glyph_bbox[gi] });
    }

    if xr.pos != o_ys || yr.pos != o_str {
        return Err(Error::Corrupt("coordinates"));
    }
    let mut sr = Rd { b, pos: o_str };
    let mut strings = Vec::with_capacity(n_strings);
    for _ in 0..n_strings {
        let len = sr.vi("string")? as usize;
        let s = b.get(sr.pos..sr.pos + len).ok_or(Error::Truncated("string"))?;
        sr.pos += len;
        strings.push(String::from_utf8_lossy(s).into_owned());
    }

    Ok(PageData { header, lines, ayahs, words, paths, decos, glyphs, insts, ops, strings })
}
