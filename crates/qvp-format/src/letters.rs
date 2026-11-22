//! QVL1: the letter layer of one page.
//!
//! A page file (QVP1) knows words and paths. This file adds the letters of each word: the
//! character a letter is, the paths that draw it, and the marks that belong to a word but
//! to no letter of it. Where one outline in the page joins several letters, the converter
//! cuts it, and this file holds the pieces. The page file does not change.
//!
//! The file follows the conventions of `codec.rs`: a fixed header with section lengths,
//! and tables in varints and zigzag deltas. A piece is coded against its source path in
//! the page (`pieces.rs`), so decoding a layer needs its page. The header holds a digest
//! of what the layer takes from its page, and a digest of the file itself.
//! `docs/FORMAT.md` specifies the file.

mod pieces;

use crate::{unzigzag, write_varint, zigzag, Cmd, Error, PageData, PathKind, COORD_LIMIT, PF_GLYPH};
use pieces::{decode_pieces, encode_pieces, load_sources, Digest, Reader, Streams, N_STREAMS};
pub use pieces::{
    MAX_GROUP_PIECES, MAX_LAYER_COMMANDS, MAX_LAYER_OPERATIONS, MAX_PIECE_COMMANDS, MAX_PIECE_OPERATIONS, MAX_REGISTRY,
};

pub const LETTERS_MAGIC: &[u8; 4] = b"QVL1";
pub const LETTERS_VERSION: u16 = 1;
/// "No offset" for a member or a word-level mark.
pub const NO_AT: u16 = 0xFFFF;

/// magic(4) + version, quant, page, word count(8) + path count(4) + page digest(8) + body
/// digest(8) + ten section lengths(40).
const HEADER_LEN: usize = 72;
/// Where the page digest and the body digest sit in the header.
const PAGE_DIGEST_AT: usize = 16;
const BODY_DIGEST_AT: usize = 24;
/// Where the ten section lengths start in the header.
const LENGTHS_AT: usize = 32;
/// The tables section, then the geometry sections.
const N_SECTIONS: usize = 1 + N_STREAMS;

/// Member flags: bit 0 body ink, bit 1 the path is a piece, bits 2 to 4 how `at` is coded.
const MF_BODY: u8 = 1;
const MF_PIECE: u8 = 2;
const MF_AT_SHIFT: u8 = 2;
/// The `at` code of a member: 0 the default, 1 to 5 the letter's offset plus 0 to 4, 7
/// stored. 6 is not used.
const AT_DEFAULT: u8 = 0;
const AT_STORED: u8 = 7;

/// Letter header bits: 0 to 2 the member count less one (7: stored), 3 the next word, 4
/// `more_at` follows, 5 and 6 how `at` is coded (3: stored), 7 a stored word delta.
const LH_MEMBERS: u8 = 7;
const LH_NEXT_WORD: u8 = 8;
const LH_MORE_AT: u8 = 16;
const LH_AT_SHIFT: u8 = 5;
const LH_WORD: u8 = 128;

/// The fewest bytes one record of each table takes, in table order: letters (a header
/// byte and an alphabet index), members (a flags byte and a path delta), word marks
/// (three varints), pieces (an operation count). A count whose records cannot fit in the
/// tables section is rejected before anything is allocated for it.
const MIN_RECORD_BYTES: [u64; 4] = [2, 2, 3, 1];

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

/// The letter layer of one page. `Default` gives `quant` 0, which the decoder rejects,
/// so a caller that starts from it sets `quant`.
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

/// How a member's `at` is coded: the default, the letter's offset plus 0 to 4 for a mark,
/// or stored.
fn member_at_code(m: &MemberRec, letter_at: u16) -> u8 {
    if m.at == default_at(m.is_body, letter_at) {
        AT_DEFAULT
    } else if !m.is_body && letter_at != NO_AT && m.at >= letter_at && m.at - letter_at <= 4 {
        (m.at - letter_at) as u8 + 1
    } else {
        AT_STORED
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

/// A varint of at most 32 bits: at most five bytes, the fifth at most 0x0F. A longer
/// varint, or a fifth byte with more bits, is an error, so two different byte runs never
/// read as one value by losing bits. A varint with more bytes than it needs is accepted,
/// as QVP1 accepts it.
fn read_varint(b: &[u8], pos: &mut usize) -> Result<u32, Error> {
    let mut v: u32 = 0;
    for i in 0..5 {
        let x = *b.get(*pos).ok_or(Error::Truncated("varint"))?;
        *pos += 1;
        if i == 4 && x > 0x0F {
            break;
        }
        v |= ((x & 0x7F) as u32) << (7 * i);
        if x & 0x80 == 0 {
            return Ok(v);
        }
    }
    Err(Error::Corrupt("varint too long"))
}

/// A reader over one section. It cannot read past the section's end.
struct Rd<'a> {
    b: &'a [u8],
    pos: usize,
}
impl<'a> Rd<'a> {
    fn vi(&mut self, what: &'static str) -> Result<u32, Error> {
        read_varint(self.b, &mut self.pos).map_err(|e| match e {
            Error::Truncated(_) => Error::Truncated(what),
            e => e,
        })
    }
    fn zv(&mut self, what: &'static str) -> Result<i32, Error> {
        Ok(unzigzag(self.vi(what)?))
    }
    fn u8(&mut self, what: &'static str) -> Result<u8, Error> {
        let v = *self.b.get(self.pos).ok_or(Error::Truncated(what))?;
        self.pos += 1;
        Ok(v)
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
    /// A zigzag delta added to a running `u32` value in wrapping arithmetic, so that every
    /// pair of values has a delta.
    fn step_u32(&mut self, acc: &mut u32, what: &'static str) -> Result<u32, Error> {
        *acc = acc.wrapping_add(self.zv(what)? as u32);
        Ok(*acc)
    }
    fn left(&self) -> usize {
        self.b.len() - self.pos
    }
}

// ───────────────────────────── the digests ─────────────────────────────

/// The digest of `bytes` that both digests of a letter layer use (`docs/FORMAT.md`, "The
/// digests"): 64 bits, a word at a time.
pub fn digest64(bytes: &[u8]) -> u64 {
    let mut h = Digest::new();
    h.bytes(bytes);
    h.finish()
}

/// The page digest: the digest of what a layer depends on in its page, in the order that
/// `docs/FORMAT.md` gives: the page number, `quant`, the word and path counts, each word's
/// path run, and each path's kind, mark, family, flags and geometry, then the glyph
/// outlines. Strings, boxes and every other field are not in it, so a words sidecar does
/// not change it. A path's geometry does not depend on its origin: its first point is
/// absolute. It is an error when a path's run leaves the opcode stream, or a glyph
/// instance has no instance record.
pub fn page_digest(page: &PageData) -> Result<u64, Error> {
    let mut h = Digest::new();
    h.bytes(&page.header.page.to_le_bytes());
    h.bytes(&page.header.quant.to_le_bytes());
    h.bytes(&(page.words.len() as u32).to_le_bytes());
    h.bytes(&(page.paths.len() as u32).to_le_bytes());
    for w in &page.words {
        h.bytes(&w.first_path.to_le_bytes());
        h.bytes(&w.n_paths.to_le_bytes());
    }
    for p in &page.paths {
        h.bytes(&[p.kind as u8, p.mark as u8, p.family as u8, p.flags]);
        if p.flags & PF_GLYPH != 0 {
            let i = page.insts.get(p.op_off as usize).ok_or(Error::Bounds("glyph instance"))?;
            h.bytes(&i.glyph.to_le_bytes());
            for v in [i.a, i.b, i.c, i.d, i.e, i.f] {
                h.bytes(&v.to_bits().to_le_bytes());
            }
        } else {
            hash_run(&mut h, page, p.op_off, p.op_len, (p.ox, p.oy))?;
        }
    }
    h.bytes(&(page.glyphs.len() as u32).to_le_bytes());
    for g in &page.glyphs {
        hash_run(&mut h, page, g.op_off, g.op_len, (0, 0))?;
    }
    Ok(h.finish())
}

/// One opcode run of the page into the page digest: its first opcode, its first point as
/// two absolute i32 when that opcode has one, then the length of the rest of the run and
/// the rest as it is: each later point is a delta from the point before it.
fn hash_run(h: &mut Digest, page: &PageData, off: u32, len: u32, origin: (i32, i32)) -> Result<(), Error> {
    let end = off.checked_add(len).ok_or(Error::Bounds("path ops"))?;
    let run = page.ops.get(off as usize..end as usize).ok_or(Error::Bounds("path ops"))?;
    let mut pos = 0;
    if let Some(&op) = run.first() {
        pos = 1;
        h.bytes(&[op]);
        if op <= crate::OP_CUBIC {
            let dx = unzigzag(crate::read_varint(run, &mut pos)?);
            let dy = unzigzag(crate::read_varint(run, &mut pos)?);
            h.bytes(&origin.0.wrapping_add(dx).to_le_bytes());
            h.bytes(&origin.1.wrapping_add(dy).to_le_bytes());
        }
    }
    h.bytes(&((run.len() - pos) as u32).to_le_bytes());
    h.bytes(&run[pos..]);
    Ok(())
}

/// The body digest: the digest of the whole file, with its own eight bytes taken as 0.
fn body_digest(b: &[u8]) -> u64 {
    let mut h = Digest::new();
    h.bytes(&b[..BODY_DIGEST_AT]);
    h.bytes(&[0; 8]);
    h.bytes(&b[BODY_DIGEST_AT + 8..]);
    h.finish()
}

fn check_magic(b: &[u8]) -> Result<(), Error> {
    if b.len() < 6 || &b[0..4] != LETTERS_MAGIC {
        return Err(Error::NotLetterLayer);
    }
    let version = u16::from_le_bytes([b[4], b[5]]);
    if version != LETTERS_VERSION {
        return Err(Error::LetterLayerVersion(version));
    }
    if b.len() < HEADER_LEN {
        return Err(Error::Damaged("header"));
    }
    Ok(())
}

/// The limits that a layer's data must keep: commands per piece and per layer, pieces per
/// source. Pieces of one source are next to each other, as `check_against` requires.
fn check_limits(d: &LetterData) -> Result<(), Error> {
    let mut total = 0u64;
    for p in &d.pieces {
        if p.cmds.len() as u64 > MAX_PIECE_COMMANDS as u64 {
            return Err(Error::Bounds("piece commands"));
        }
        total += p.cmds.len() as u64;
    }
    if total > MAX_LAYER_COMMANDS as u64 {
        return Err(Error::Bounds("layer commands"));
    }
    let mut run = (None, 0u32);
    for p in &d.pieces {
        run = if run.0 == Some(p.source) { (run.0, run.1 + 1) } else { (Some(p.source), 1) };
        if run.1 > MAX_GROUP_PIECES {
            return Err(Error::Bounds("group pieces"));
        }
    }
    Ok(())
}

// ───────────────────────────── encode ─────────────────────────────

/// Encode a letter layer for `page`. The header takes the page digest of `page` and the
/// layer's sources, and the body digest of the file.
///
/// It is an error when a piece cannot be coded against the page: the pieces are not in
/// source order, a source is not an inline path of the page, a source path's commands do
/// not decode, a piece does not start with a move or has a point outside the envelope, or
/// the layer is past a limit. `check_against` rejects each of these, so every layer that it
/// accepts encodes, and `decode_letters` gives that layer back.
pub fn encode_letters(d: &LetterData, page: &PageData) -> Result<Vec<u8>, Error> {
    check_limits(d)?;
    // the pieces as groups of one source, in source order
    let mut groups: Vec<(u32, u32)> = Vec::new();
    for p in &d.pieces {
        match groups.last_mut() {
            Some(g) if g.0 == p.source => g.1 += 1,
            Some(g) if g.0 > p.source => return Err(Error::Corrupt("piece order")),
            _ => groups.push((p.source, 1)),
        }
    }
    let digest = page_digest(page)?;
    let sources_list: Vec<u32> = groups.iter().map(|g| g.0).collect();
    let sources = load_sources(page, &sources_list)?;
    let mut geo = Streams::default();
    let runs = encode_pieces(&sources, &d.pieces, &mut geo)?;
    // two operations per command at most, so a layer inside the command limits is inside these
    let ops: u64 = runs.iter().map(|r| *r as u64).sum();
    if runs.iter().any(|r| *r > MAX_PIECE_OPERATIONS) || ops > MAX_LAYER_OPERATIONS as u64 {
        return Err(Error::Bounds("operations"));
    }

    // ── tables ──
    let mut t: Vec<u8> = Vec::with_capacity(d.letters.len() * 3 + d.members.len() * 3 + 64);
    for n in [d.letters.len(), d.members.len(), d.word_marks.len(), d.pieces.len()] {
        vi(&mut t, n as u32);
    }
    // the page alphabet: each character once, in code-point order
    let mut alphabet: Vec<char> = d.letters.iter().map(|l| l.text).collect();
    alphabet.sort_unstable();
    alphabet.dedup();
    vi(&mut t, alphabet.len() as u32);
    let mut prev = 0u32;
    for c in &alphabet {
        vi(&mut t, *c as u32 - prev);
        prev = *c as u32;
    }
    let (mut pw, mut prev_at) = (0u16, None::<u16>);
    for l in &d.letters {
        let wd = l.word as i32 - pw as i32;
        if wd != 0 {
            prev_at = None;
        }
        pw = l.word;
        let a = match prev_at {
            Some(p) => l.at as i64 - p as i64 - 1,
            None => l.at as i64,
        };
        prev_at = Some(l.at);
        let at_code = if (0..=2).contains(&a) { a as u8 } else { 3 };
        let n = l.n_members;
        let mut h = if (1..=7).contains(&n) { (n - 1) as u8 } else { LH_MEMBERS };
        h |= at_code << LH_AT_SHIFT;
        match wd {
            0 => {}
            1 => h |= LH_NEXT_WORD,
            _ => h |= LH_WORD,
        }
        if !l.more_at.is_empty() {
            h |= LH_MORE_AT;
        }
        t.push(h);
        if h & LH_WORD != 0 {
            zv(&mut t, wd);
        }
        if h & LH_MEMBERS == LH_MEMBERS {
            vi(&mut t, n as u32);
        }
        if at_code == 3 {
            vi(&mut t, l.at as u32);
        }
        if !l.more_at.is_empty() {
            vi(&mut t, l.more_at.len() as u32);
            for a in &l.more_at {
                vi(&mut t, *a as u32);
            }
        }
        vi(&mut t, alphabet.binary_search(&l.text).expect("the letter's character") as u32);
    }
    // members, column by column
    let letter_at = letter_at_of_members(&d.letters, d.members.len());
    let codes: Vec<u8> = d.members.iter().zip(&letter_at).map(|(m, la)| member_at_code(m, *la)).collect();
    for (m, code) in d.members.iter().zip(&codes) {
        let mut f = code << MF_AT_SHIFT;
        if m.is_body {
            f |= MF_BODY;
        }
        if matches!(m.path, MemberPath::Piece(_)) {
            f |= MF_PIECE;
        }
        t.push(f);
    }
    // a page path from the previous page path, a piece from the one after the previous piece
    let (mut prev_page, mut next_piece) = (0u32, 0u32);
    for m in &d.members {
        match m.path {
            MemberPath::Page(p) => {
                zv(&mut t, p.wrapping_sub(prev_page) as i32);
                prev_page = p;
            }
            MemberPath::Piece(p) => {
                zv(&mut t, p.wrapping_sub(next_piece) as i32);
                next_piece = p.wrapping_add(1);
            }
        }
    }
    for (m, code) in d.members.iter().zip(&codes) {
        if *code == AT_STORED {
            vi(&mut t, m.at as u32);
        }
    }
    let (mut pw, mut pp) = (0u16, 0u32);
    for w in &d.word_marks {
        zv(&mut t, w.word as i32 - pw as i32);
        pw = w.word;
        zv(&mut t, w.path.wrapping_sub(pp) as i32);
        pp = w.path;
        vi(&mut t, opt(w.at));
    }
    vi(&mut t, groups.len() as u32);
    for (i, (s, n)) in groups.iter().enumerate() {
        // the groups checked the order above, so each source is above the one before
        let v = if i == 0 { *s } else { s - groups[i - 1].0 - 1 };
        vi(&mut t, v);
        vi(&mut t, n - 1);
    }
    for r in &runs {
        vi(&mut t, *r);
    }

    // ── header ──
    let sections: Vec<&Vec<u8>> = std::iter::once(&t).chain(geo.s.iter()).collect();
    let mut o = Vec::with_capacity(HEADER_LEN + sections.iter().map(|s| s.len()).sum::<usize>());
    o.extend_from_slice(LETTERS_MAGIC);
    for v in [LETTERS_VERSION, d.quant, d.page, d.n_page_words] {
        o.extend_from_slice(&v.to_le_bytes());
    }
    o.extend_from_slice(&d.n_page_paths.to_le_bytes());
    o.extend_from_slice(&digest.to_le_bytes());
    o.extend_from_slice(&[0; 8]);
    for s in &sections {
        o.extend_from_slice(&(s.len() as u32).to_le_bytes());
    }
    debug_assert_eq!(o.len(), HEADER_LEN);
    for s in &sections {
        o.extend_from_slice(s);
    }
    let body = body_digest(&o);
    o[BODY_DIGEST_AT..BODY_DIGEST_AT + 8].copy_from_slice(&body.to_le_bytes());
    Ok(o)
}

// ───────────────────────────── decode ─────────────────────────────

/// Decode a letter layer against its page. Each check of `docs/FORMAT.md`'s "What a
/// reader rejects" is an error of one of four kinds:
/// - `NotLetterLayer`: the file does not start with `QVL1`;
/// - `LetterLayerVersion`: a version this reader does not know;
/// - `OtherPage`: the layer was built for another page, or for other paths of this one.
///   Its page number, `quant`, a count or its page digest differs;
/// - `Damaged`: the file's bytes are not what its writer wrote (the body digest differs),
///   or they break a rule of the format.
///
/// The body digest is checked first, the page before any table, and the page digest
/// before any piece. A reader then calls [`LetterData::check_against`] on the result.
pub fn decode_letters(b: &[u8], page: &PageData) -> Result<LetterData, Error> {
    decode_checked(b, page).map_err(|e| match e {
        Error::Bounds(w) | Error::Truncated(w) | Error::Corrupt(w) => Error::Damaged(w),
        e => e,
    })
}

fn decode_checked(b: &[u8], page: &PageData) -> Result<LetterData, Error> {
    check_magic(b)?;
    let stored = u64::from_le_bytes(b[BODY_DIGEST_AT..BODY_DIGEST_AT + 8].try_into().expect("eight bytes"));
    if body_digest(b) != stored {
        return Err(Error::Damaged("body digest"));
    }
    let u16a = |at: usize| u16::from_le_bytes([b[at], b[at + 1]]);
    let u32a = |at: usize| u32::from_le_bytes([b[at], b[at + 1], b[at + 2], b[at + 3]]);
    let (quant, page_no, n_page_words, n_page_paths) = (u16a(6), u16a(8), u16a(10), u32a(12));
    if quant == 0 {
        return Err(Error::Corrupt("quant"));
    }
    let mut sec: [&[u8]; N_SECTIONS] = [&[]; N_SECTIONS];
    let mut at = HEADER_LEN;
    for (i, s) in sec.iter_mut().enumerate() {
        let end = at.checked_add(u32a(LENGTHS_AT + i * 4) as usize).ok_or(Error::Truncated("section"))?;
        *s = b.get(at..end).ok_or(Error::Truncated("section"))?;
        at = end;
    }
    if at != b.len() {
        return Err(Error::Corrupt("trailing bytes"));
    }
    // The pieces are coded against the page's paths: the page must be the layer's page.
    if page.header.page != page_no {
        return Err(Error::OtherPage("page number"));
    }
    if page.header.quant != quant {
        return Err(Error::OtherPage("quant"));
    }
    if page.paths.len() as u64 != n_page_paths as u64 {
        return Err(Error::OtherPage("path count"));
    }
    if page.words.len() as u64 != n_page_words as u64 {
        return Err(Error::OtherPage("word count"));
    }
    let stored = u64::from_le_bytes(b[PAGE_DIGEST_AT..PAGE_DIGEST_AT + 8].try_into().expect("eight bytes"));
    let digest = page_digest(page).map_err(|e| match e {
        Error::Bounds(w) | Error::Truncated(w) | Error::Corrupt(w) => Error::OtherPage(w),
        e => e,
    })?;
    if digest != stored {
        return Err(Error::OtherPage("page digest"));
    }

    // ── tables ──
    let mut t = Rd { b: sec[0], pos: 0 };
    let mut counts = [0usize; 4];
    for c in counts.iter_mut() {
        *c = t.vi("counts")? as usize;
    }
    let need: u64 = counts.iter().zip(MIN_RECORD_BYTES).map(|(n, m)| *n as u64 * m).sum();
    if need > t.left() as u64 {
        return Err(Error::Truncated("tables"));
    }
    let [n_letters, n_members, n_word_marks, n_pieces] = counts;
    // each piece draws at least one command
    if n_pieces as u64 > MAX_LAYER_COMMANDS as u64 {
        return Err(Error::Bounds("pieces"));
    }

    // Each character takes at least one byte of what is left.
    let n_alpha = t.vi("alphabet")? as usize;
    if n_alpha > t.left() {
        return Err(Error::Truncated("alphabet"));
    }
    let mut alphabet = Vec::with_capacity(n_alpha);
    let mut code = 0u32;
    for i in 0..n_alpha {
        // The characters rise: each after the first is at least one above the one before.
        let d = t.vi("alphabet")?;
        if i > 0 && d == 0 {
            return Err(Error::Corrupt("alphabet"));
        }
        code = code.checked_add(d).ok_or(Error::Corrupt("alphabet"))?;
        alphabet.push(char::from_u32(code).ok_or(Error::Corrupt("alphabet"))?);
    }

    let mut letters = Vec::with_capacity(n_letters);
    let mut used = vec![false; n_alpha];
    let (mut pw, mut prev_at) = (0u16, None::<u16>);
    let mut first_member: u64 = 0;
    for i in 0..n_letters {
        let h = t.u8("letter")?;
        let mut word = pw;
        match (h & LH_WORD != 0, h & LH_NEXT_WORD != 0) {
            (true, true) => return Err(Error::Corrupt("letter header")),
            (true, false) => {
                t.step_u16(&mut word, "letter word")?;
                // a delta of 0 or 1 has its own code
                if word == pw || Some(word) == pw.checked_add(1) {
                    return Err(Error::Corrupt("letter header"));
                }
            }
            (false, true) => word = pw.checked_add(1).ok_or(Error::Corrupt("letter word"))?,
            (false, false) => {}
        }
        if word >= n_page_words {
            return Err(Error::Bounds("letter word"));
        }
        if i == 0 || word != pw {
            prev_at = None;
        }
        pw = word;
        // A stored member count is the count itself, 8 and above: 1 to 7 have their own
        // codes, and a letter with no member has no body.
        let n = match h & LH_MEMBERS {
            LH_MEMBERS => t.u16("letter members").and_then(|n| match n {
                0..=7 => Err(Error::Corrupt("letter members")),
                n => Ok(n),
            })?,
            c => c as u16 + 1,
        };
        // A letter always has an offset: `NO_AT` is outside the field, as it is for a
        // word mark's stored `at + 1`.
        let at = match (h >> LH_AT_SHIFT) & 3 {
            3 => {
                let at = t.u16("letter at")?;
                // an `at` that a code of 0 to 2 gives is not stored
                if (0..=2).contains(&prev_at.map_or(at as i64, |p| at as i64 - p as i64 - 1)) {
                    return Err(Error::Corrupt("letter at"));
                }
                at
            }
            c => match prev_at {
                Some(p) => p.checked_add(1 + c as u16).ok_or(Error::Corrupt("letter at"))?,
                None => c as u16,
            },
        };
        if at == NO_AT {
            return Err(Error::Corrupt("letter at"));
        }
        prev_at = Some(at);
        let mut more_at = Vec::new();
        if h & LH_MORE_AT != 0 {
            let n_more = t.vi("more_at")? as usize;
            // Bit 4 means at least one offset. Each takes at least one byte of what is left.
            if n_more == 0 {
                return Err(Error::Corrupt("more_at"));
            }
            if n_more > t.left() {
                return Err(Error::Truncated("more_at"));
            }
            more_at.reserve_exact(n_more);
            for _ in 0..n_more {
                more_at.push(t.u16("more_at")?);
            }
        }
        let ti = t.vi("letter text")? as usize;
        let text = *alphabet.get(ti).ok_or(Error::Bounds("letter text"))?;
        used[ti] = true;
        // At most n_letters × 65535: it fits a u64 and, below n_members, a u32.
        let first = u32::try_from(first_member).map_err(|_| Error::Corrupt("member count"))?;
        letters.push(LetterRec { word, at, text, more_at, first_member: first, n_members: n });
        first_member += n as u64;
    }
    if first_member != n_members as u64 {
        return Err(Error::Corrupt("member count"));
    }
    // The alphabet holds the characters of the letters and no other.
    if used.iter().any(|u| !u) {
        return Err(Error::Corrupt("alphabet"));
    }

    // Members: the flags column, then the path column, then the stored offsets.
    let end = t.pos.checked_add(n_members).ok_or(Error::Truncated("member flags"))?;
    let flags = t.b.get(t.pos..end).ok_or(Error::Truncated("member flags"))?;
    t.pos = end;
    if flags.iter().any(|f| f >> 5 != 0 || (f >> MF_AT_SHIFT) & 7 == 6) {
        return Err(Error::Corrupt("member flags"));
    }
    let mut members = Vec::with_capacity(n_members);
    let (mut prev_page, mut next_piece) = (0u32, 0u32);
    for f in flags {
        let path = if f & MF_PIECE != 0 {
            let v = t.step_u32(&mut next_piece, "member path")?;
            if v as usize >= n_pieces {
                return Err(Error::Bounds("member piece"));
            }
            next_piece = v + 1;
            MemberPath::Piece(v)
        } else {
            let v = t.step_u32(&mut prev_page, "member path")?;
            if v >= n_page_paths {
                return Err(Error::Bounds("member page path"));
            }
            MemberPath::Page(v)
        };
        members.push(MemberRec { path, at: NO_AT, is_body: f & MF_BODY != 0 });
    }
    let letter_at = letter_at_of_members(&letters, n_members);
    for ((m, f), la) in members.iter_mut().zip(flags).zip(letter_at) {
        let code = (f >> MF_AT_SHIFT) & 7;
        m.at = match code {
            AT_DEFAULT => default_at(m.is_body, la),
            AT_STORED => t.u16("member at")?,
            c => la.checked_add(c as u16 - 1).ok_or(Error::Corrupt("member at"))?,
        };
        // Each `at` has one code: the one the writer gives it.
        if member_at_code(m, la) != code {
            return Err(Error::Corrupt("member at"));
        }
    }

    let mut word_marks = Vec::with_capacity(n_word_marks);
    let (mut pw, mut pp) = (0u16, 0u32);
    for _ in 0..n_word_marks {
        let word = t.step_u16(&mut pw, "word mark word")?;
        if word >= n_page_words {
            return Err(Error::Bounds("word mark word"));
        }
        let path = t.step_u32(&mut pp, "word mark path")?;
        if path >= n_page_paths {
            return Err(Error::Bounds("word mark path"));
        }
        let at = match t.vi("word mark at")? {
            0 => NO_AT,
            v => u16::try_from(v - 1).ok().filter(|a| *a != NO_AT).ok_or(Error::Corrupt("word mark at"))?,
        };
        word_marks.push(WordMarkRec { word, path, at });
    }

    // Groups: each takes at least two bytes, and holds at least one piece.
    let n_groups = t.vi("piece groups")? as usize;
    if n_groups > n_pieces || n_groups as u64 * 2 > t.left() as u64 {
        return Err(Error::Truncated("piece groups"));
    }
    let mut groups: Vec<(u32, u32)> = Vec::with_capacity(n_groups);
    let mut in_groups = 0u64;
    for i in 0..n_groups {
        let v = t.vi("piece source")?;
        let source = match groups.last() {
            None => v,
            Some(g) => g.0.checked_add(1).and_then(|s| s.checked_add(v)).ok_or(Error::Corrupt("piece source"))?,
        };
        if source >= n_page_paths {
            return Err(Error::Bounds("piece source"));
        }
        let n = t.vi("piece group")? as u64 + 1;
        if n > MAX_GROUP_PIECES as u64 {
            return Err(Error::Bounds("group pieces"));
        }
        in_groups += n;
        if in_groups > n_pieces as u64 || (i + 1 == n_groups && in_groups != n_pieces as u64) {
            return Err(Error::Corrupt("piece count"));
        }
        groups.push((source, n as u32));
    }
    if n_groups == 0 && n_pieces != 0 {
        return Err(Error::Corrupt("piece count"));
    }
    let mut runs = Vec::with_capacity(n_pieces);
    for _ in 0..n_pieces {
        let n = t.vi("piece operations")?;
        if n > MAX_PIECE_OPERATIONS {
            return Err(Error::Bounds("piece operations"));
        }
        runs.push(n);
    }
    // The tables are one varint run: the reader must land exactly on the section's end.
    if t.pos != t.b.len() {
        return Err(Error::Corrupt("tables"));
    }

    // ── pieces ──
    // Each operation takes half a byte of the operations section, so the section bounds
    // the runs before any piece is sized from them.
    let ops = sec[1].len() as u64;
    let total: u64 = runs.iter().map(|n| *n as u64).sum();
    if total > MAX_LAYER_OPERATIONS as u64 {
        return Err(Error::Bounds("layer operations"));
    }
    if total > ops * 2 {
        return Err(Error::Truncated("operations"));
    }
    if ops != total.div_ceil(2) {
        return Err(Error::Corrupt("operations"));
    }
    // The page digest matched, so a source that is not an inline path of the page, or
    // that does not decode, is the page's error, not the file's.
    let sources_list: Vec<u32> = groups.iter().map(|g| g.0).collect();
    let sources = load_sources(page, &sources_list).map_err(|e| match e {
        Error::Bounds(w) | Error::Truncated(w) | Error::Corrupt(w) => Error::OtherPage(w),
        e => e,
    })?;
    let mut geo: [&[u8]; N_STREAMS] = [&[]; N_STREAMS];
    geo.copy_from_slice(&sec[1..]);
    let mut r = Reader::new(geo);
    let pieces = decode_pieces(&sources, &groups, &runs, &mut r)?;
    r.finish()?;

    Ok(LetterData { page: page_no, quant, n_page_paths, n_page_words, letters, members, word_marks, pieces })
}

// ───────────────────────────── the page ─────────────────────────────

impl LetterData {
    /// Every rule that ties a layer to its page. A reader calls it before it trusts a
    /// layer for a page. `Corrupt` names a rule the layer breaks, `Bounds` an index
    /// outside the range the rule allows. A layer that passes round-trips through
    /// `encode_letters` and `decode_letters`. The check is linear in the size of the
    /// layer and of the page.
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
        // The decoder rejects this too. The check here keeps the round-trip promise.
        if self.letters.iter().any(|l| l.at == NO_AT) {
            return Err(Error::Corrupt("letter at"));
        }
        // Inside one word, the letters follow the word's spelling.
        if self.letters.windows(2).any(|w| w[0].word == w[1].word && w[0].at >= w[1].at) {
            return Err(Error::Corrupt("letter at order"));
        }
        if self.pieces.windows(2).any(|w| w[0].source > w[1].source) {
            return Err(Error::Corrupt("piece order"));
        }
        check_limits(self)?;
        // What the decoder needs to give a piece back unchanged: it starts with a move,
        // and every point is inside the envelope.
        for p in &self.pieces {
            if !matches!(p.cmds.first(), Some(Cmd::MoveTo(..))) {
                return Err(Error::Corrupt("piece start"));
            }
            let inside = |c: &Cmd| {
                let (v, n) = cmd_coords(c);
                v[..n].iter().all(|v| in_envelope(*v))
            };
            if !p.cmds.iter().all(inside) {
                return Err(Error::Corrupt("coordinate range"));
            }
        }
        let word_range = |word: u16, what: &'static str| -> Result<std::ops::Range<u64>, Error> {
            let w = page.words.get(word as usize).ok_or(Error::Bounds(what))?;
            Ok(w.first_path as u64..w.first_path as u64 + w.n_paths as u64)
        };
        // Each page path has at most one role. A path may be the source of several pieces.
        let mut role = vec![Role::None; page.paths.len()];
        for p in &self.pieces {
            *role.get_mut(p.source as usize).ok_or(Error::Bounds("piece source"))? = Role::Source;
        }
        let mut piece_used = vec![false; self.pieces.len()];
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
                        match std::mem::replace(&mut role[p as usize], Role::Member) {
                            Role::None => {}
                            Role::Source => return Err(Error::Corrupt("member and source")),
                            _ => return Err(Error::Corrupt("path in two letters")),
                        }
                    }
                    MemberPath::Piece(i) => {
                        let used = piece_used.get_mut(i as usize).ok_or(Error::Bounds("member piece"))?;
                        if std::mem::replace(used, true) {
                            return Err(Error::Corrupt("piece used twice"));
                        }
                        let piece = &self.pieces[i as usize];
                        if !range.contains(&(piece.source as u64)) {
                            return Err(Error::Bounds("piece source"));
                        }
                        let source = &page.paths[piece.source as usize];
                        if source.kind != PathKind::Body {
                            return Err(Error::Corrupt("piece source kind"));
                        }
                        // A glyph instance draws a shared outline. A piece is cut from
                        // the page's own geometry only.
                        if source.flags & PF_GLYPH != 0 {
                            return Err(Error::Corrupt("piece source glyph"));
                        }
                    }
                }
            }
        }
        // With every piece used once, the source checks above cover every piece.
        if piece_used.iter().any(|u| !u) {
            return Err(Error::Corrupt("piece unused"));
        }
        for w in &self.word_marks {
            let range = word_range(w.word, "word mark word")?;
            if !range.contains(&(w.path as u64)) || w.path as usize >= page.paths.len() {
                return Err(Error::Bounds("word mark path"));
            }
            match std::mem::replace(&mut role[w.path as usize], Role::WordMark) {
                Role::None => {}
                Role::Source => return Err(Error::Corrupt("word mark and source")),
                Role::Member => return Err(Error::Corrupt("word mark and member")),
                Role::WordMark => return Err(Error::Corrupt("word mark twice")),
            }
        }
        // Every path of every word has a role, so the layer covers every word of its
        // page. Word runs may overlap, so the paths inside some run come from a running
        // count of the runs that start and end at each path: linear in words and paths.
        let mut starts = vec![0i64; page.paths.len() + 1];
        for w in &page.words {
            let (a, b) = (w.first_path as usize, w.first_path as usize + w.n_paths as usize);
            if w.n_paths == 0 {
                continue;
            }
            if b > page.paths.len() {
                return Err(Error::Bounds("word path"));
            }
            starts[a] += 1;
            starts[b] -= 1;
        }
        let mut open = 0i64;
        for (r, s) in role.iter().zip(&starts) {
            open += s;
            if open > 0 && matches!(r, Role::None) {
                return Err(Error::Corrupt("path without role"));
            }
        }
        Ok(())
    }
}

/// The one role a page path can have in a layer.
#[derive(Clone, Copy)]
enum Role {
    None,
    Source,
    Member,
    WordMark,
}

/// The coordinates of one command, control points included.
fn cmd_coords(c: &Cmd) -> ([i32; 6], usize) {
    match *c {
        Cmd::MoveTo(x, y) | Cmd::LineTo(x, y) => ([x, y, 0, 0, 0, 0], 2),
        Cmd::QuadTo(x1, y1, x, y) => ([x1, y1, x, y, 0, 0], 4),
        Cmd::CubicTo(x1, y1, x2, y2, x, y) => ([x1, y1, x2, y2, x, y], 6),
        Cmd::Close => ([0; 6], 0),
    }
}

#[cfg(test)]
mod tests;
#[cfg(test)]
mod vectors;
