//! QVL1: the letter layer of one page.
//!
//! A page file (QVP1) knows words and paths. This file adds the letters of each word: the
//! character a letter is, the paths that draw it, and the marks that belong to a word but
//! to no letter of it. Where one outline in the page joins several letters, the converter
//! cuts it, and this file holds the pieces. The page file does not change.
//!
//! The file follows the conventions of `codec.rs`: a fixed header with section lengths,
//! struct-of-arrays tables in varints and zigzag deltas, and the drawing operations of the
//! pieces in the same four streams as a page's runs. `docs/FORMAT.md` specifies it.

use crate::{read_varint, unzigzag, write_varint, zigzag, Cmd, Error, PageData, PathKind, COORD_LIMIT};

pub const LETTERS_MAGIC: &[u8; 4] = b"QVL1";
pub const LETTERS_VERSION: u16 = 1;
/// "No offset" for a member or a word-level mark.
pub const NO_AT: u16 = 0xFFFF;

/// magic(4) + version, quant, page, word count(8) + path count(4) + four section lengths(16).
const HEADER_LEN: usize = 32;

/// Member flags: bit 0 body ink, bit 1 the path is a piece, bit 2 `at` is stored.
const MF_BODY: u8 = 1;
const MF_PIECE: u8 = 2;
const MF_AT: u8 = 4;

/// The fewest bytes one record of each table takes, in table order: letters (five
/// varints), members (a flags byte and a varint), word marks (three varints), pieces (two
/// varints). A count whose records cannot fit in the tables section is rejected before
/// anything is allocated for it.
const MIN_RECORD_BYTES: [u64; 4] = [5, 2, 3, 2];

/// Which path a member is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MemberPath {
    /// A path the page already has: its index in the page's path table.
    Page(u32),
    /// A cut piece: its index in `LetterData::pieces`.
    Piece(u32),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberRec {
    pub path: MemberPath,
    /// The code-point offset, in the word's `qpc` text, of the character this path draws.
    /// A body member carries its letter's offset. `NO_AT` when upstream gives none.
    pub at: u16,
    /// True for the letter's body ink, false for a mark of the letter.
    pub is_body: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LetterRec {
    /// The word's index on the page.
    pub word: u16,
    /// The code-point offset of the letter in the word's `qpc` text.
    pub at: u16,
    /// The letter's character.
    pub text: char,
    /// Offsets of other characters that the letter's body also draws. Empty for every
    /// letter but one in the Hafs data: the lam of 2:97:5 also draws its kashida.
    pub more_at: Vec<u16>,
    pub first_member: u32,
    pub n_members: u16,
}

/// A mark that belongs to a word and to no letter of it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WordMarkRec {
    pub word: u16,
    /// Index in the page's path table.
    pub path: u32,
    pub at: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PieceRec {
    /// The page path this piece is cut from: an index in the page's path table.
    pub source: u32,
    /// Absolute, quantised page-space commands.
    pub cmds: Vec<Cmd>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LetterData {
    pub page: u16,
    pub quant: u16,
    /// The path and word counts of the page this layer belongs to.
    pub n_page_paths: u32,
    pub n_page_words: u16,
    /// In word order, then in the order of the word's spelling.
    pub letters: Vec<LetterRec>,
    pub members: Vec<MemberRec>,
    pub word_marks: Vec<WordMarkRec>,
    /// Sorted by `source`. The pieces of one source are in letter order.
    pub pieces: Vec<PieceRec>,
}

// ───────────────────────────── helpers ─────────────────────────────

fn vi(o: &mut Vec<u8>, v: u32) {
    write_varint(o, v);
}
fn zv(o: &mut Vec<u8>, v: i32) {
    write_varint(o, zigzag(v));
}

/// `NO_AT` is 0, any other offset is stored plus one.
fn opt(v: u16) -> u32 {
    if v == NO_AT {
        0
    } else {
        v as u32 + 1
    }
}

/// True when a coordinate is inside the envelope every page keeps (`COORD_LIMIT`).
fn in_envelope(v: i32) -> bool {
    v.unsigned_abs() < COORD_LIMIT
}

/// The `at` a member has when the file does not store one: its letter's offset for a
/// body member, `NO_AT` for a mark.
fn default_at(is_body: bool, letter_at: u16) -> u16 {
    if is_body {
        letter_at
    } else {
        NO_AT
    }
}

/// The offset of the letter that holds each member, `NO_AT` for a member no letter holds.
fn letter_at_of_members(letters: &[LetterRec], n_members: usize) -> Vec<u16> {
    let mut out = Vec::with_capacity(n_members);
    for l in letters {
        for _ in 0..l.n_members {
            out.push(l.at);
        }
    }
    out.resize(n_members, NO_AT);
    out
}

/// A reader over one section. It cannot read past the section's end.
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
    /// A varint that must fit a `u16` field.
    fn u16(&mut self, what: &'static str) -> Result<u16, Error> {
        u16::try_from(self.vi(what)?).map_err(|_| Error::Corrupt(what))
    }
    /// A zigzag delta added to a running `u16` value.
    fn step_u16(&mut self, acc: &mut u16, what: &'static str) -> Result<u16, Error> {
        let v = (*acc as i32).checked_add(self.zv(what)?).ok_or(Error::Corrupt(what))?;
        *acc = u16::try_from(v).map_err(|_| Error::Corrupt(what))?;
        Ok(*acc)
    }
    fn left(&self) -> usize {
        self.b.len() - self.pos
    }
}

// ───────────────────────────── encode ─────────────────────────────

/// Encode a letter layer. `decode_letters` gives back the same `LetterData` for every
/// layer that `check_against` accepts.
pub fn encode_letters(d: &LetterData) -> Vec<u8> {
    // ── tables ──
    let mut t: Vec<u8> = Vec::with_capacity(d.letters.len() * 6 + d.members.len() * 3 + 64);
    for n in [d.letters.len(), d.members.len(), d.word_marks.len(), d.pieces.len()] {
        vi(&mut t, n as u32);
    }
    let mut pw = 0u16;
    for l in &d.letters {
        zv(&mut t, l.word as i32 - pw as i32);
        pw = l.word;
        vi(&mut t, l.at as u32);
        vi(&mut t, l.text as u32);
        vi(&mut t, l.n_members as u32);
        vi(&mut t, l.more_at.len() as u32);
        for a in &l.more_at {
            vi(&mut t, *a as u32);
        }
    }
    // members, column by column
    let letter_at = letter_at_of_members(&d.letters, d.members.len());
    let stores_at = |i: usize, m: &MemberRec| m.at != default_at(m.is_body, letter_at[i]);
    for (i, m) in d.members.iter().enumerate() {
        let mut f = 0u8;
        if m.is_body {
            f |= MF_BODY;
        }
        if matches!(m.path, MemberPath::Piece(_)) {
            f |= MF_PIECE;
        }
        if stores_at(i, m) {
            f |= MF_AT;
        }
        t.push(f);
    }
    for m in &d.members {
        match m.path {
            MemberPath::Page(p) | MemberPath::Piece(p) => vi(&mut t, p),
        }
    }
    for (i, m) in d.members.iter().enumerate() {
        if stores_at(i, m) {
            vi(&mut t, m.at as u32);
        }
    }
    let mut pw = 0u16;
    for w in &d.word_marks {
        zv(&mut t, w.word as i32 - pw as i32);
        pw = w.word;
        vi(&mut t, w.path);
        vi(&mut t, opt(w.at));
    }
    // Sources are u32: the delta wraps, so every pair of values round-trips.
    let mut ps = 0u32;
    for p in &d.pieces {
        zv(&mut t, p.source.wrapping_sub(ps) as i32);
        ps = p.source;
        vi(&mut t, p.cmds.iter().filter(|c| !matches!(c, Cmd::Close)).count() as u32);
    }

    // ── drawing operations, coded as `codec.rs` codes the runs of a page ──
    let (mut opbits, mut zs, mut xs, mut ys) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut z_at: Vec<u32> = Vec::new();
    let (mut bit, mut acc) = (0u32, 0u8);
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
    for piece in &d.pieces {
        let mut put = |pts: &[(i32, i32)], cx: &mut i32, cy: &mut i32| {
            for &(x, y) in pts {
                zv(&mut xs, x.wrapping_sub(*cx));
                zv(&mut ys, y.wrapping_sub(*cy));
                *cx = x;
                *cy = y;
            }
        };
        for c in &piece.cmds {
            match *c {
                Cmd::MoveTo(x, y) => {
                    push_op(0, &mut opbits);
                    put(&[(x, y)], &mut cx, &mut cy);
                }
                Cmd::LineTo(x, y) => {
                    push_op(1, &mut opbits);
                    put(&[(x, y)], &mut cx, &mut cy);
                }
                Cmd::QuadTo(x1, y1, x, y) => {
                    push_op(2, &mut opbits);
                    put(&[(x1, y1), (x, y)], &mut cx, &mut cy);
                }
                Cmd::CubicTo(x1, y1, x2, y2, x, y) => {
                    push_op(3, &mut opbits);
                    put(&[(x1, y1), (x2, y2), (x, y)], &mut cx, &mut cy);
                }
                // A close carries no point. Record where it sits so that it can be put back.
                Cmd::Close => z_at.push(n_ops_total),
            }
            if !matches!(c, Cmd::Close) {
                n_ops_total += 1;
            }
        }
        // The next piece's deltas start from this piece's first point, not its last.
        if let Some(&Cmd::MoveTo(x, y)) = piece.cmds.first() {
            cx = x;
            cy = y;
        }
    }
    if bit != 0 {
        opbits.push(acc);
    }
    // The close indices, then the x deltas, in one section.
    vi(&mut zs, z_at.len() as u32);
    let mut prev = 0u32;
    for z in &z_at {
        vi(&mut zs, z - prev);
        prev = *z;
    }
    zs.extend_from_slice(&xs);

    // ── header ──
    let mut o = Vec::with_capacity(HEADER_LEN + t.len() + opbits.len() + zs.len() + ys.len());
    o.extend_from_slice(LETTERS_MAGIC);
    for v in [LETTERS_VERSION, d.quant, d.page, d.n_page_words] {
        o.extend_from_slice(&v.to_le_bytes());
    }
    o.extend_from_slice(&d.n_page_paths.to_le_bytes());
    for len in [t.len(), opbits.len(), zs.len(), ys.len()] {
        o.extend_from_slice(&(len as u32).to_le_bytes());
    }
    debug_assert_eq!(o.len(), HEADER_LEN);
    for s in [&t, &opbits, &zs, &ys] {
        o.extend_from_slice(s);
    }
    o
}

// ───────────────────────────── decode ─────────────────────────────

/// Decode a letter layer. A file that fails any check in `docs/FORMAT.md`'s "What a
/// reader rejects" is an error: `Bounds` for an index past its table, `Truncated` for a
/// count or a run that its section cannot hold, `Corrupt` for a value outside its field.
/// The rules that tie a layer to its page are in [`LetterData::check_against`].
pub fn decode_letters(b: &[u8]) -> Result<LetterData, Error> {
    if b.len() < 6 || &b[0..4] != LETTERS_MAGIC {
        return Err(Error::BadMagic);
    }
    let version = u16::from_le_bytes([b[4], b[5]]);
    if version != LETTERS_VERSION {
        return Err(Error::BadVersion(version));
    }
    if b.len() < HEADER_LEN {
        return Err(Error::Truncated("header"));
    }
    let u16a = |at: usize| u16::from_le_bytes([b[at], b[at + 1]]);
    let u32a = |at: usize| u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
    let (quant, page, n_page_words, n_page_paths) = (u16a(6), u16a(8), u16a(10), u32a(12));
    if quant == 0 {
        return Err(Error::Corrupt("quant"));
    }
    let mut sec: [&[u8]; 4] = [&[]; 4];
    let mut at = HEADER_LEN;
    for (i, s) in sec.iter_mut().enumerate() {
        let end = at.checked_add(u32a(16 + i * 4) as usize).ok_or(Error::Truncated("section"))?;
        *s = b.get(at..end).ok_or(Error::Truncated("section"))?;
        at = end;
    }
    if at != b.len() {
        return Err(Error::Corrupt("trailing bytes"));
    }
    let [s_tables, s_ops, s_zx, s_y] = sec;

    // ── tables ──
    let mut t = Rd { b: s_tables, pos: 0 };
    let mut counts = [0usize; 4];
    for c in counts.iter_mut() {
        *c = t.vi("counts")? as usize;
    }
    let need: u64 = counts.iter().zip(MIN_RECORD_BYTES).map(|(n, m)| *n as u64 * m).sum();
    if need > t.left() as u64 {
        return Err(Error::Truncated("tables"));
    }
    let [n_letters, n_members, n_word_marks, n_pieces] = counts;

    let mut letters = Vec::with_capacity(n_letters);
    let mut pw = 0u16;
    let mut first_member: u64 = 0;
    for _ in 0..n_letters {
        let word = t.step_u16(&mut pw, "letter word")?;
        if word >= n_page_words {
            return Err(Error::Bounds("letter word"));
        }
        let at = t.u16("letter at")?;
        let text = char::from_u32(t.vi("letter text")?).ok_or(Error::Corrupt("letter text"))?;
        let n = t.u16("letter members")?;
        let n_more = t.vi("more_at")? as usize;
        // Each offset takes at least one byte of what is left.
        if n_more > t.left() {
            return Err(Error::Truncated("more_at"));
        }
        let mut more_at = Vec::with_capacity(n_more);
        for _ in 0..n_more {
            more_at.push(t.u16("more_at")?);
        }
        // At most n_letters × 65535: it fits a u64 and, below n_members, a u32.
        let first = u32::try_from(first_member).map_err(|_| Error::Corrupt("member count"))?;
        letters.push(LetterRec { word, at, text, more_at, first_member: first, n_members: n });
        first_member += n as u64;
    }
    if first_member != n_members as u64 {
        return Err(Error::Corrupt("member count"));
    }

    // Members: the flags column, then the path column, then the stored offsets.
    let flags = t.b.get(t.pos..t.pos + n_members).ok_or(Error::Truncated("member flags"))?;
    t.pos += n_members;
    if flags.iter().any(|f| f & !(MF_BODY | MF_PIECE | MF_AT) != 0) {
        return Err(Error::Corrupt("member flags"));
    }
    let mut members = Vec::with_capacity(n_members);
    for f in flags {
        let v = t.vi("member path")?;
        let path = if f & MF_PIECE != 0 {
            if v as usize >= n_pieces {
                return Err(Error::Bounds("member piece"));
            }
            MemberPath::Piece(v)
        } else {
            if v >= n_page_paths {
                return Err(Error::Bounds("member page path"));
            }
            MemberPath::Page(v)
        };
        members.push(MemberRec { path, at: NO_AT, is_body: f & MF_BODY != 0 });
    }
    let letter_at = letter_at_of_members(&letters, n_members);
    for (i, (m, f)) in members.iter_mut().zip(flags).enumerate() {
        m.at = if f & MF_AT != 0 { t.u16("member at")? } else { default_at(m.is_body, letter_at[i]) };
    }

    let mut word_marks = Vec::with_capacity(n_word_marks);
    let mut pw = 0u16;
    for _ in 0..n_word_marks {
        let word = t.step_u16(&mut pw, "word mark word")?;
        if word >= n_page_words {
            return Err(Error::Bounds("word mark word"));
        }
        let path = t.vi("word mark path")?;
        if path >= n_page_paths {
            return Err(Error::Bounds("word mark path"));
        }
        let at = match t.vi("word mark at")? {
            0 => NO_AT,
            v => u16::try_from(v - 1).ok().filter(|a| *a != NO_AT).ok_or(Error::Corrupt("word mark at"))?,
        };
        word_marks.push(WordMarkRec { word, path, at });
    }

    let mut sources = Vec::with_capacity(n_pieces);
    let mut run_lens = Vec::with_capacity(n_pieces);
    let mut ps = 0u32;
    for _ in 0..n_pieces {
        ps = ps.wrapping_add(t.zv("piece source")? as u32);
        if ps >= n_page_paths {
            return Err(Error::Bounds("piece source"));
        }
        sources.push(ps);
        run_lens.push(t.vi("piece run")?);
    }
    // The tables are one varint run: the reader must land exactly on the section's end.
    if t.pos != t.b.len() {
        return Err(Error::Corrupt("tables"));
    }

    // ── drawing operations ──
    let mut zr = Rd { b: s_zx, pos: 0 };
    let n_z = zr.vi("close indices")? as usize;
    // Each close index takes at least one byte of its section.
    if n_z > zr.left() {
        return Err(Error::Truncated("close indices"));
    }
    let mut z_next: Vec<u32> = Vec::with_capacity(n_z);
    let mut acc = 0u32;
    for _ in 0..n_z {
        acc = acc.checked_add(zr.vi("close indices")?).ok_or(Error::Corrupt("close indices"))?;
        z_next.push(acc);
    }
    let s_x = &s_zx[zr.pos..];
    // Every operation takes a quarter of an opcode byte and at least one byte of x and of
    // y, so the sections bound the run lengths before any buffer is sized from them.
    let total: u64 = run_lens.iter().map(|n| *n as u64).sum();
    if total > s_ops.len() as u64 * 4 || total > s_x.len() as u64 || total > s_y.len() as u64 {
        return Err(Error::Truncated("opcodes"));
    }
    if s_ops.len() as u64 != total.div_ceil(4) {
        return Err(Error::Corrupt("opcodes"));
    }
    let mut xr = Rd { b: s_x, pos: 0 };
    let mut yr = Rd { b: s_y, pos: 0 };
    let (mut op_i, mut seen, mut z_i) = (0usize, 0u32, 0usize);
    let (mut cx, mut cy) = (0i32, 0i32);
    let mut pieces = Vec::with_capacity(n_pieces);
    for (source, n_ops) in sources.into_iter().zip(run_lens) {
        let mut cmds = Vec::with_capacity(n_ops as usize);
        for _ in 0..n_ops {
            // The opcode section holds total / 4 bytes, checked above.
            let code = (s_ops[op_i / 4] >> ((op_i % 4) * 2)) & 3;
            op_i += 1;
            let mut pt = || -> Result<(i32, i32), Error> {
                cx = cx.checked_add(xr.zv("x")?).ok_or(Error::Corrupt("coordinate overflow"))?;
                cy = cy.checked_add(yr.zv("y")?).ok_or(Error::Corrupt("coordinate overflow"))?;
                if !in_envelope(cx) || !in_envelope(cy) {
                    return Err(Error::Corrupt("coordinate range"));
                }
                Ok((cx, cy))
            };
            cmds.push(match code {
                0 => {
                    let (x, y) = pt()?;
                    Cmd::MoveTo(x, y)
                }
                1 => {
                    let (x, y) = pt()?;
                    Cmd::LineTo(x, y)
                }
                2 => {
                    let (x1, y1) = pt()?;
                    let (x, y) = pt()?;
                    Cmd::QuadTo(x1, y1, x, y)
                }
                _ => {
                    let (x1, y1) = pt()?;
                    let (x2, y2) = pt()?;
                    let (x, y) = pt()?;
                    Cmd::CubicTo(x1, y1, x2, y2, x, y)
                }
            });
            seen += 1;
            while z_i < z_next.len() && z_next[z_i] == seen {
                cmds.push(Cmd::Close);
                z_i += 1;
            }
        }
        // A piece is ink: it starts with a move, which the next piece's deltas start from.
        match cmds.first() {
            Some(&Cmd::MoveTo(x, y)) => {
                cx = x;
                cy = y;
            }
            _ => return Err(Error::Corrupt("piece start")),
        }
        pieces.push(PieceRec { source, cmds });
    }
    // The streams end together, and every close index named an operation.
    if xr.pos != s_x.len() || yr.pos != s_y.len() || z_i != z_next.len() {
        return Err(Error::Corrupt("coordinates"));
    }

    Ok(LetterData { page, quant, n_page_paths, n_page_words, letters, members, word_marks, pieces })
}

// ───────────────────────────── the page ─────────────────────────────

impl LetterData {
    /// Every rule that ties a layer to its page. Used by the converter and, in phase 3,
    /// by the engine. `Corrupt` names a rule the layer breaks, `Bounds` an index outside
    /// the range the rule allows.
    pub fn check_against(&self, page: &PageData) -> Result<(), Error> {
        if page.header.page != self.page {
            return Err(Error::Corrupt("layer page"));
        }
        if page.header.quant != self.quant {
            return Err(Error::Corrupt("layer quant"));
        }
        if page.paths.len() != self.n_page_paths as usize {
            return Err(Error::Corrupt("layer path count"));
        }
        if page.words.len() != self.n_page_words as usize {
            return Err(Error::Corrupt("layer word count"));
        }
        // The file does not store `first_member`: it is the running total of `n_members`.
        let mut first = 0u64;
        for l in &self.letters {
            if l.first_member as u64 != first {
                return Err(Error::Corrupt("first member"));
            }
            first += l.n_members as u64;
        }
        if first != self.members.len() as u64 {
            return Err(Error::Corrupt("member count"));
        }
        if self.letters.windows(2).any(|w| w[0].word > w[1].word) {
            return Err(Error::Corrupt("letter order"));
        }
        if self.pieces.windows(2).any(|w| w[0].source > w[1].source) {
            return Err(Error::Corrupt("piece order"));
        }
        let word_range = |word: u16, what: &'static str| -> Result<std::ops::Range<u64>, Error> {
            let w = page.words.get(word as usize).ok_or(Error::Bounds(what))?;
            Ok(w.first_path as u64..w.first_path as u64 + w.n_paths as u64)
        };
        let mut member_of = vec![false; page.paths.len()];
        let mut is_source = vec![false; page.paths.len()];
        for p in &self.pieces {
            if let Some(s) = is_source.get_mut(p.source as usize) {
                *s = true;
            }
        }
        for l in &self.letters {
            let range = word_range(l.word, "letter word")?;
            let members = &self.members[l.first_member as usize..l.first_member as usize + l.n_members as usize];
            if !members.iter().any(|m| m.is_body) {
                return Err(Error::Corrupt("letter without body"));
            }
            for m in members {
                match m.path {
                    MemberPath::Page(p) => {
                        if !range.contains(&(p as u64)) || p as usize >= page.paths.len() {
                            return Err(Error::Bounds("letter member"));
                        }
                        if std::mem::replace(&mut member_of[p as usize], true) {
                            return Err(Error::Corrupt("path in two letters"));
                        }
                        if is_source[p as usize] {
                            return Err(Error::Corrupt("member and source"));
                        }
                    }
                    MemberPath::Piece(i) => {
                        let piece = self.pieces.get(i as usize).ok_or(Error::Bounds("member piece"))?;
                        let source = page.paths.get(piece.source as usize).ok_or(Error::Bounds("piece source"))?;
                        if !range.contains(&(piece.source as u64)) {
                            return Err(Error::Bounds("piece source"));
                        }
                        if source.kind != PathKind::Body {
                            return Err(Error::Corrupt("piece source kind"));
                        }
                    }
                }
            }
        }
        for w in &self.word_marks {
            if !word_range(w.word, "word mark word")?.contains(&(w.path as u64)) {
                return Err(Error::Bounds("word mark path"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Family, Header, IBox, Mark, PageData, PathKind, PathRec, WordRec, VERSION};

    const PIECE_A: [Cmd; 4] = [Cmd::MoveTo(1000, 2000), Cmd::LineTo(1100, 2050), Cmd::LineTo(1050, 2100), Cmd::Close];
    const PIECE_B: [Cmd; 4] = [
        Cmd::MoveTo(1200, 2000),
        Cmd::QuadTo(1250, 1950, 1300, 2000),
        Cmd::CubicTo(1310, 2010, 1290, 2040, 1200, 2050),
        Cmd::Close,
    ];

    /// A page of 2 words and 6 paths. Word 0 holds paths 0 to 2, word 1 paths 3 to 5.
    fn sample() -> LetterData {
        let body = |path, at| MemberRec { path, at, is_body: true };
        let mark = |path, at| MemberRec { path, at, is_body: false };
        LetterData {
            page: 7,
            quant: 100,
            n_page_paths: 6,
            n_page_words: 2,
            letters: vec![
                LetterRec { word: 0, at: 0, text: 'ب', more_at: vec![], first_member: 0, n_members: 2 },
                LetterRec { word: 0, at: 2, text: 'ل', more_at: vec![], first_member: 2, n_members: 2 },
                LetterRec { word: 1, at: 0, text: 'ل', more_at: vec![3], first_member: 4, n_members: 1 },
            ],
            members: vec![
                body(MemberPath::Page(0), 0),
                mark(MemberPath::Page(1), 1),
                body(MemberPath::Piece(0), 2),
                body(MemberPath::Piece(1), 2),
                body(MemberPath::Page(3), 0),
            ],
            word_marks: vec![WordMarkRec { word: 1, path: 4, at: NO_AT }, WordMarkRec { word: 1, path: 5, at: 5 }],
            pieces: vec![
                PieceRec { source: 2, cmds: PIECE_A.to_vec() },
                PieceRec { source: 2, cmds: PIECE_B.to_vec() },
            ],
        }
    }

    /// The page `sample` belongs to: only the fields `check_against` reads are set.
    fn sample_page() -> PageData {
        let path = |kind| PathRec {
            kind,
            mark: Mark::None,
            family: Family::None,
            flags: 0,
            ox: 0,
            oy: 0,
            op_off: 0,
            op_len: 0,
            bbox: IBox::EMPTY,
        };
        let word = |first_path, n_paths| WordRec { first_path, n_paths, ..WordRec::default() };
        PageData {
            header: Header { version: VERSION, quant: 100, page: 7, flags: 0, width: 345.0, height: 550.0 },
            lines: vec![],
            ayahs: vec![],
            words: vec![word(0, 3), word(3, 3)],
            paths: vec![
                path(PathKind::Body),
                path(PathKind::Mark),
                path(PathKind::Body),
                path(PathKind::Body),
                path(PathKind::Mark),
                path(PathKind::Mark),
            ],
            decorations: vec![],
            glyphs: vec![],
            insts: vec![],
            ops: vec![],
            strings: vec![],
        }
    }

    /// Set every letter's `first_member` to the running total of `n_members`.
    fn renumber(d: &mut LetterData) {
        let mut first = 0u32;
        for l in &mut d.letters {
            l.first_member = first;
            first += l.n_members as u32;
        }
    }

    fn header_u32(b: &[u8], at: usize) -> u32 {
        u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
    }

    #[test]
    fn round_trip() {
        let d = sample();
        assert_eq!(decode_letters(&encode_letters(&d)), Ok(d));
    }

    #[test]
    fn a_body_member_with_its_own_offset_round_trips() {
        let mut d = sample();
        d.members[0].at = 9;
        d.members[2].at = NO_AT;
        assert_eq!(decode_letters(&encode_letters(&d)), Ok(d));
    }

    #[test]
    fn empty_layer_round_trips() {
        let d = LetterData { page: 1, quant: 100, n_page_paths: 0, n_page_words: 0, ..LetterData::default() };
        let bytes = encode_letters(&d);
        assert_eq!(bytes.len(), HEADER_LEN + 5);
        assert_eq!(decode_letters(&bytes), Ok(d));
    }

    #[test]
    fn rejects_bad_magic() {
        let mut bytes = encode_letters(&sample());
        bytes[3] = b'2';
        assert_eq!(decode_letters(&bytes), Err(Error::BadMagic));
        assert_eq!(decode_letters(b"QV"), Err(Error::BadMagic));
    }

    #[test]
    fn rejects_bad_version() {
        let mut bytes = encode_letters(&sample());
        bytes[4] = 2;
        assert_eq!(decode_letters(&bytes), Err(Error::BadVersion(2)));
    }

    #[test]
    fn rejects_a_zero_quant() {
        let mut bytes = encode_letters(&sample());
        bytes[6] = 0;
        bytes[7] = 0;
        assert_eq!(decode_letters(&bytes), Err(Error::Corrupt("quant")));
    }

    #[test]
    fn rejects_trailing_bytes() {
        let mut bytes = encode_letters(&sample());
        bytes.push(0);
        assert_eq!(decode_letters(&bytes), Err(Error::Corrupt("trailing bytes")));
    }

    #[test]
    fn every_truncation_is_an_error() {
        let bytes = encode_letters(&sample());
        for n in 0..bytes.len() {
            assert!(decode_letters(&bytes[..n]).is_err(), "{n} of {} bytes", bytes.len());
        }
    }

    #[test]
    fn a_huge_count_does_not_allocate() {
        let bytes = encode_letters(&sample());
        // The letters count is the first varint of the tables section, one byte here.
        assert_eq!(bytes[HEADER_LEN], 3);
        let mut patched = bytes[..HEADER_LEN].to_vec();
        write_varint(&mut patched, u32::MAX);
        patched.extend_from_slice(&bytes[HEADER_LEN + 1..]);
        let tables = header_u32(&bytes, 16) + 4;
        patched[16..20].copy_from_slice(&tables.to_le_bytes());
        assert_eq!(decode_letters(&patched), Err(Error::Truncated("tables")));
    }

    #[test]
    fn a_huge_more_at_count_does_not_allocate() {
        let mut d = sample();
        d.letters[2].more_at = vec![0x7F; 1];
        let bytes = encode_letters(&d);
        let tables_end = HEADER_LEN + header_u32(&bytes, 16) as usize;
        // The more_at count of the last letter is 1, then its one offset 0x7F.
        let at = (HEADER_LEN..tables_end).find(|&i| bytes[i] == 1 && bytes[i + 1] == 0x7F).unwrap();
        let mut patched = bytes[..at].to_vec();
        write_varint(&mut patched, u32::MAX);
        patched.extend_from_slice(&bytes[at + 1..]);
        let tables = header_u32(&bytes, 16) + 4;
        patched[16..20].copy_from_slice(&tables.to_le_bytes());
        assert_eq!(decode_letters(&patched), Err(Error::Truncated("more_at")));
    }

    #[test]
    fn rejects_a_member_count_that_differs_from_the_members_table() {
        let mut d = sample();
        d.letters[2].n_members = 2;
        assert_eq!(decode_letters(&encode_letters(&d)), Err(Error::Corrupt("member count")));
    }

    #[test]
    fn rejects_out_of_range_indices() {
        let decoded = |f: fn(&mut LetterData)| {
            let mut d = sample();
            f(&mut d);
            decode_letters(&encode_letters(&d))
        };
        assert_eq!(decoded(|d| d.members[0].path = MemberPath::Page(6)), Err(Error::Bounds("member page path")));
        assert_eq!(decoded(|d| d.members[2].path = MemberPath::Piece(2)), Err(Error::Bounds("member piece")));
        assert_eq!(decoded(|d| d.pieces[1].source = 6), Err(Error::Bounds("piece source")));
        assert_eq!(decoded(|d| d.letters[2].word = 2), Err(Error::Bounds("letter word")));
        assert_eq!(decoded(|d| d.word_marks[1].word = 2), Err(Error::Bounds("word mark word")));
        assert_eq!(decoded(|d| d.word_marks[1].path = 6), Err(Error::Bounds("word mark path")));
    }

    #[test]
    fn rejects_a_surrogate_as_text() {
        let mut d = sample();
        d.letters[1].text = '\u{D7FF}';
        let mut bytes = encode_letters(&d);
        // 0xD7FF is the varint FF AF 03; 0xD800 is 80 B0 03, the same length.
        let at = (HEADER_LEN..bytes.len() - 2).find(|&i| bytes[i..i + 3] == [0xFF, 0xAF, 0x03]).unwrap();
        bytes[at..at + 3].copy_from_slice(&[0x80, 0xB0, 0x03]);
        assert_eq!(decode_letters(&bytes), Err(Error::Corrupt("letter text")));
    }

    #[test]
    fn rejects_unknown_member_flags() {
        let bytes = encode_letters(&sample());
        // The flags column of the sample: body, mark with an offset, two piece bodies, body.
        let tables_end = HEADER_LEN + header_u32(&bytes, 16) as usize;
        let flags_at = (HEADER_LEN..tables_end - 4)
            .find(|&i| bytes[i..i + 5] == [1, 4, 3, 3, 1])
            .expect("the flags column of the sample");
        let mut patched = bytes.clone();
        patched[flags_at] |= 0x08;
        assert_eq!(decode_letters(&patched), Err(Error::Corrupt("member flags")));
    }

    #[test]
    fn rejects_a_coordinate_out_of_range() {
        let mut d = sample();
        d.pieces[0].cmds[1] = Cmd::LineTo(COORD_LIMIT as i32, 2050);
        assert_eq!(decode_letters(&encode_letters(&d)), Err(Error::Corrupt("coordinate range")));
    }

    #[test]
    fn rejects_a_piece_that_does_not_start_with_a_move() {
        let mut d = sample();
        d.pieces[1].cmds[0] = Cmd::LineTo(1200, 2000);
        assert_eq!(decode_letters(&encode_letters(&d)), Err(Error::Corrupt("piece start")));
    }

    #[test]
    fn every_byte_flip_is_an_error_or_a_valid_layer() {
        let bytes = encode_letters(&sample());
        for at in 0..bytes.len() {
            let mut flipped = bytes.clone();
            flipped[at] = 0xFF;
            if let Ok(d) = decode_letters(&flipped) {
                // What decodes, encodes again to bytes that decode to the same layer.
                assert_eq!(decode_letters(&encode_letters(&d)), Ok(d), "byte {at}");
            }
        }
    }

    #[test]
    fn random_mutations_never_panic() {
        let bytes = encode_letters(&sample());
        crate::testing::each_mutation(&bytes, 4000, |at, value, b| {
            if let Ok(d) = decode_letters(b) {
                assert_eq!(decode_letters(&encode_letters(&d)), Ok(d), "byte {at} set to {value}");
            }
        });
    }

    #[test]
    fn check_against_accepts_the_sample() {
        assert_eq!(sample().check_against(&sample_page()), Ok(()));
    }

    #[test]
    fn check_against_rejects_a_different_page() {
        let checked = |f: fn(&mut PageData)| {
            let mut p = sample_page();
            f(&mut p);
            sample().check_against(&p)
        };
        assert_eq!(checked(|p| p.header.page = 8), Err(Error::Corrupt("layer page")));
        assert_eq!(checked(|p| p.header.quant = 10), Err(Error::Corrupt("layer quant")));
        assert_eq!(checked(|p| p.paths.truncate(5)), Err(Error::Corrupt("layer path count")));
        assert_eq!(checked(|p| p.words.truncate(1)), Err(Error::Corrupt("layer word count")));
    }

    #[test]
    fn check_against_rejects_a_member_outside_its_word() {
        let mut d = sample();
        d.members[1].path = MemberPath::Page(4);
        assert_eq!(d.check_against(&sample_page()), Err(Error::Bounds("letter member")));
    }

    #[test]
    fn check_against_rejects_a_word_mark_outside_its_word() {
        let mut d = sample();
        d.word_marks[0].path = 1;
        assert_eq!(d.check_against(&sample_page()), Err(Error::Bounds("word mark path")));
    }

    #[test]
    fn check_against_rejects_a_piece_cut_from_another_word() {
        let mut d = sample();
        d.pieces[0].source = 3;
        d.pieces[1].source = 3;
        d.members[4].path = MemberPath::Page(5);
        d.word_marks.pop();
        assert_eq!(d.check_against(&sample_page()), Err(Error::Bounds("piece source")));
    }

    #[test]
    fn check_against_rejects_a_piece_cut_from_a_mark() {
        let mut p = sample_page();
        p.paths[2].kind = PathKind::Mark;
        assert_eq!(sample().check_against(&p), Err(Error::Corrupt("piece source kind")));
    }

    #[test]
    fn check_against_rejects_a_path_in_two_letters() {
        let mut d = sample();
        d.letters.insert(2, LetterRec { word: 0, at: 3, text: 'ا', more_at: vec![], first_member: 0, n_members: 1 });
        d.members.insert(4, MemberRec { path: MemberPath::Page(0), at: 3, is_body: true });
        renumber(&mut d);
        assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("path in two letters")));
    }

    #[test]
    fn check_against_rejects_a_path_that_is_a_member_and_a_source() {
        let mut d = sample();
        d.members[1].path = MemberPath::Page(2);
        assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("member and source")));
    }

    #[test]
    fn check_against_rejects_a_letter_with_no_body() {
        let mut d = sample();
        d.members[0].is_body = false;
        assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("letter without body")));
    }

    #[test]
    fn check_against_rejects_unsorted_letters() {
        let mut d = sample();
        d.letters.swap(1, 2);
        renumber(&mut d);
        assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("letter order")));
    }

    #[test]
    fn check_against_rejects_unsorted_pieces() {
        let mut d = sample();
        d.pieces[1].source = 0;
        assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("piece order")));
    }

    #[test]
    fn check_against_rejects_a_member_range_that_does_not_add_up() {
        let mut d = sample();
        d.letters[1].first_member = 3;
        assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("first member")));
        let mut d = sample();
        d.members.pop();
        assert_eq!(d.check_against(&sample_page()), Err(Error::Corrupt("member count")));
    }

    #[test]
    fn check_against_rejects_indices_past_the_tables() {
        let mut d = sample();
        d.members[2].path = MemberPath::Piece(9);
        assert_eq!(d.check_against(&sample_page()), Err(Error::Bounds("member piece")));
        let mut d = sample();
        d.letters[2].word = 5;
        assert_eq!(d.check_against(&sample_page()), Err(Error::Bounds("letter word")));
    }
}
