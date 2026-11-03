//! The cross-reference checks a page must pass before the engine indexes it by its own
//! values. Every record that names another table names a record that exists, and every
//! run lies inside the table it points into. The decoder runs these once, in one pass,
//! before the geometry is decoded; a page built in memory can run them with
//! [`PageData::validate`].
//!
//! A failed check is `Error::Bounds(what)`, where `what` names the table and the field.
use crate::*;

/// The tables of a page, borrowed, so the decoder can check them before it owns a
/// `PageData`.
pub(crate) struct Tables<'a> {
    pub lines: &'a [LineRec],
    pub ayahs: &'a [AyahRec],
    pub words: &'a [WordRec],
    pub paths: &'a [PathRec],
    pub decorations: &'a [DecoRec],
    pub insts: &'a [InstRec],
    pub n_glyphs: usize,
    pub n_strings: usize,
}

/// True when `first..first+n` lies inside a table of `len` records.
fn run_fits(first: u32, n: u32, len: usize) -> bool {
    (first as u64 + n as u64) <= len as u64
}

/// True when `i` names a record of a table of `len` records, or is absent.
fn index_or_none(i: u16, len: usize) -> bool {
    i == NONE_U16 || (i as usize) < len
}

pub(crate) fn check(t: &Tables) -> Result<(), Error> {
    let n_paths = t.paths.len();
    for l in t.lines {
        if !run_fits(l.first_word as u32, l.n_words as u32, t.words.len()) {
            return Err(Error::Bounds("line words"));
        }
    }
    for a in t.ayahs {
        if !run_fits(a.first_word as u32, a.n_words as u32, t.words.len()) {
            return Err(Error::Bounds("ayah words"));
        }
        if !index_or_none(a.ayah_mark_decoration, t.decorations.len()) {
            return Err(Error::Bounds("ayah decoration"));
        }
    }
    for w in t.words {
        if !run_fits(w.first_path, w.n_paths as u32, n_paths) {
            return Err(Error::Bounds("word paths"));
        }
        if w.line_index as usize >= t.lines.len() {
            return Err(Error::Bounds("word line"));
        }
        if w.ayah_index as usize >= t.ayahs.len() {
            return Err(Error::Bounds("word ayah"));
        }
        if [w.text, w.rasm_imlai, w.qpc, w.rasm, w.search].iter().any(|&s| !index_or_none(s, t.n_strings)) {
            return Err(Error::Bounds("word text"));
        }
    }
    for p in t.paths {
        if p.flags & PF_GLYPH != 0 && p.op_off as usize >= t.insts.len() {
            return Err(Error::Bounds("path instance"));
        }
    }
    // The engine gives every path a line, so a page that draws anything has one.
    if n_paths > 0 && t.lines.is_empty() {
        return Err(Error::Bounds("path line"));
    }
    for d in t.decorations {
        // A decoration is a drawn thing: its first path is the one the engine reads
        // the decoration's line from.
        if d.n_paths == 0 || !run_fits(d.first_path, d.n_paths as u32, n_paths) {
            return Err(Error::Bounds("decoration paths"));
        }
        if !index_or_none(d.line, t.lines.len()) {
            return Err(Error::Bounds("decoration line"));
        }
        if !index_or_none(d.text, t.n_strings) {
            return Err(Error::Bounds("decoration text"));
        }
    }
    for i in t.insts {
        if i.glyph as usize >= t.n_glyphs {
            return Err(Error::Bounds("instance glyph"));
        }
    }
    Ok(())
}

impl PageData {
    /// Check every cross reference and every opcode run of a page built in memory. The
    /// decoder makes the same checks, so a page it returns needs none.
    pub fn validate(&self) -> Result<(), Error> {
        check(&Tables {
            lines: &self.lines,
            ayahs: &self.ayahs,
            words: &self.words,
            paths: &self.paths,
            decorations: &self.decorations,
            insts: &self.insts,
            n_glyphs: self.glyphs.len(),
            n_strings: self.strings.len(),
        })?;
        let in_ops = |off: u32, len: u32| run_fits(off, len, self.ops.len());
        if self.paths.iter().any(|p| p.flags & PF_GLYPH == 0 && !in_ops(p.op_off, p.op_len)) {
            return Err(Error::Bounds("path ops"));
        }
        if self.glyphs.iter().any(|g| !in_ops(g.op_off, g.op_len)) {
            return Err(Error::Bounds("glyph ops"));
        }
        Ok(())
    }
}
