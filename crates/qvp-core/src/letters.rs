//! The letter layer of a page: which paths draw which letter, the pieces cut from joined
//! outlines, the letters a host reads back, the four letter selectors, the letter query, the
//! cut paths that draw their pieces, and the letter under a point.
//!
//! A layer is optional. [`Page::attach_letters`] appends its pieces to the page's path
//! table, after the page's own paths, so no page path changes its index. No word range
//! holds a piece.
use crate::defaults::MAX_LETTER_LAYER_BYTES;
use crate::style::{PathAnim, PathCtx, Selector};
use crate::{load, reflow, HitOptions, Page, PathGeom, Rgba, NONE};
use qvp_format::letters::{decode_letters, LetterData, MemberPath, NO_AT};
use qvp_format::{cmds_bbox, Error, IBox, Mark, PageData, PathKind, NONE_U16};

/// What part of a letter a selector or a query takes. `QVP_LETTER_PART_*`.
///
/// The parts go by offset, not by the kind of mark. The ink of the letter's own character
/// is every member at the letter's own offset: the body, the dots, and a sign that is part
/// of the code point, such as a hamzah sign, a hamzat al-wasl sign, or a dagger alif that
/// the page stores as a mark. So `Letter(w, l, BodyAndDots)` takes the same paths as
/// `WordChars(w, at, at)` for the letter's offset `at`, when that offset passes the text
/// check. A mark with no offset is in `Marks`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum LetterPart {
    /// Every member.
    All = 0,
    /// Every member at the letter's own offset: the ink of the letter's own character.
    BodyAndDots = 1,
    /// Every other member: the marks at another offset, and the marks with no offset.
    Marks = 2,
}

impl LetterPart {
    /// The part for a code: 0, 1 or 2. Any other code gives `None`.
    pub fn from_u8(v: u8) -> Option<LetterPart> {
        match v {
            0 => Some(LetterPart::All),
            1 => Some(LetterPart::BodyAndDots),
            2 => Some(LetterPart::Marks),
            _ => None,
        }
    }

    /// True when this part takes a member. `own` is true when the member's offset is its
    /// letter's own offset.
    pub(crate) fn takes(self, own: bool) -> bool {
        match self {
            LetterPart::All => true,
            LetterPart::BodyAndDots => own,
            LetterPart::Marks => !own,
        }
    }
}

/// Letters by what they are. `scope` resolves to words when the rule is added.
#[derive(Clone, Debug, PartialEq)]
pub struct LetterQuery {
    /// The words to look in.
    pub scope: crate::Target,
    /// The letter's character. `None` takes any letter.
    pub text: Option<char>,
    /// A mark that one of the letter's members must carry. `Mark::None` takes any letter.
    pub mark: Mark,
    /// The part of each letter found.
    pub part: LetterPart,
}

/// The rule a letter query leaves: [`Selector::Letters`]. The query's scope is resolved to
/// words when the rule is added. The engine resolves the rest per path each time the rules run.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct LetterSelection {
    /// The words to look in, ascending, with no repeat.
    pub words: Vec<u32>,
    /// The letter's character. `None` takes any letter.
    pub text: Option<char>,
    /// A mark that one of the letter's members must carry. `Mark::None` takes any letter.
    pub mark: Mark,
    /// The part of each letter found. The parts go by offset: see [`LetterPart`].
    pub part: LetterPart,
}

/// One letter, as `qvp_letter_info` reports it. An offset is `NONE` when it fails the text
/// check. The box is in page units: the union of the members' own boxes.
#[derive(Clone, Debug, PartialEq)]
pub struct LetterInfo {
    /// The word's index on the page.
    pub word: u32,
    /// The letter's index in its word.
    pub index: u32,
    /// The letter's code-point offset in the word's `qpc` text.
    pub at: u32,
    /// The letter's character.
    pub text: char,
    /// How many paths draw the letter, body and marks.
    pub n_members: u32,
    /// How many of those paths are not body ink. Dots count.
    pub n_marks: u32,
    /// The offsets of the other characters the body draws, checked. Empty for every letter
    /// but one in the Hafs data.
    pub more_at: Vec<u32>,
    /// Left edge of the box.
    pub x0: f32,
    /// Top edge of the box.
    pub y0: f32,
    /// Right edge of the box.
    pub x1: f32,
    /// Bottom edge of the box.
    pub y1: f32,
    /// True when a member has an outline. A path drawn off the page has none.
    pub has_ink: bool,
}

/// One path of a letter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LetterMember {
    /// A page path, or a piece at an index past the page's own paths.
    pub path: u32,
    /// The checked offset, `NONE` when there is none.
    pub at: u32,
    /// The path's mark. A piece has its source's mark.
    pub mark: Mark,
    /// True for the letter's body ink.
    pub is_body: bool,
    /// True for a piece cut from a joined outline.
    pub is_cut: bool,
}

/// The letter under a point: what [`Page::hit_test_letter`] and
/// [`Page::hit_test_letter_view`] return.
///
/// An exact hit (`is_exact` true) names the path whose outline holds the point: a page path,
/// or a piece. A near hit (`is_exact` false) names the letter whose box is nearest the point,
/// and `path` is then that letter's first body member.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LetterHit {
    /// The word's index on the page. Always the word that [`Page::hit_test`] reports.
    pub word: u32,
    /// The letter's index in its word. `NONE` for a word-level mark.
    pub letter: u32,
    /// An exact hit: the path that holds the point. Never a cut path: its pieces are hit in
    /// its place. A near hit: the letter's first body member.
    pub path: u32,
    /// The checked offset of the path that was hit, or of the letter on a near hit. `NONE`
    /// when there is none, or when it fails the text check.
    pub at: u32,
    /// The word's printed line index, as `Hit::line`.
    pub line: u32,
    /// How far the point is from the letter's box, in page units. 0 on an exact hit, and 0
    /// on a near hit inside the box but off the ink.
    pub distance: f32,
    /// The mark of the path that was hit, as [`LetterMember::mark`] gives it. Body ink has
    /// `Mark::None`, unless the page stores the body as a mark, such as a dagger alif. A near
    /// hit has `Mark::None`.
    pub mark: Mark,
    /// True when the point is inside the path's outline. False when the hit is the nearest
    /// letter's box.
    pub is_exact: bool,
}

/// The letter facts of one path, inside its `PathCtx`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct PathLetter {
    /// The word of the letter or the word mark that holds this path, as the layer says it.
    /// For a cut path, the word of its pieces' letters. `NONE` when the path has no role.
    /// Every letter call takes a member's word from here, never from the path's own context.
    pub word: u32,
    /// Index into `Letters::data.letters`. `NONE` when the path is no letter's member.
    pub letter: u32,
    /// The letter's index inside its word. `u16::MAX` when there is none.
    pub letter_in_word: u16,
    /// The member's offset, or the word mark's, after the text check. `NO_AT` when none.
    pub at: u16,
    /// The offset as the layer stores it, kept for a later check.
    pub raw_at: u16,
    /// True for a body member.
    pub is_body: bool,
    /// True for a member whose offset, as the layer stores it, is its letter's own offset:
    /// the ink of the letter's own character. False for every other path.
    pub own: bool,
    /// Index among the letter's members that are not body, in text order: by offset, then
    /// in the layer's member order, a mark with no offset last. `u16::MAX` for a body member.
    pub nth_letter_mark: u16,
    /// The page path a piece replaces. The path itself for every other path.
    pub source: u32,
    /// For a cut path: the path index of its first piece and the piece count. (0, 0) for
    /// every other path.
    pub pieces: (u32, u32),
}

impl PathLetter {
    /// A path with no letter role.
    pub(crate) fn page_path(i: u32) -> PathLetter {
        PathLetter {
            word: NONE,
            letter: NONE,
            letter_in_word: u16::MAX,
            at: NO_AT,
            raw_at: NO_AT,
            is_body: false,
            own: false,
            nth_letter_mark: u16::MAX,
            source: i,
            pieces: (0, 0),
        }
    }
}

/// A loaded layer and what the engine derives from it when it is attached.
pub(crate) struct Letters {
    pub data: LetterData,
    /// The path index of the first piece: the page's own path count.
    pub first_piece: u32,
    /// Per page word: (first letter, letter count) into `data.letters`.
    pub word_letters: Vec<(u32, u32)>,
    /// Per letter: the union of its members' boxes, quantised.
    pub letter_box: Vec<IBox>,
    /// Per letter: the mark ids its members' paths carry, as a 256-bit set.
    pub letter_marks: Vec<[u64; 4]>,
    /// The members of each letter in the order the engine reports them, as indices into
    /// `data.members`, laid out as `data.members` is (a letter's slots start at its
    /// `first_member`). The body members come first, in the layer's order, then the marks in
    /// text order: by offset, then in the layer's order, a mark with no offset last. The
    /// layer's own data keeps its order.
    pub member_order: Vec<u32>,
    /// Per piece: its box, quantised.
    pub piece_box: Vec<IBox>,
    /// Per page word: the word's box with its pieces' boxes.
    pub word_box: Vec<IBox>,
    /// The cut paths, ascending: (source path, first piece path, piece count).
    pub sources: Vec<(u32, u32, u32)>,
    /// Per entry of `sources`: true while it draws its pieces.
    pub swapped: Vec<bool>,
    /// Per page path: its index in `sources`, `NONE` when it is not cut.
    pub slot_of: Vec<u32>,
    /// Per page word: the `qpc` length in code points. `None` when the page has no `qpc`
    /// text for it.
    pub qpc_len: Vec<Option<u16>>,
    /// The lengths of the geometry buffers before the pieces, to drop them on a second
    /// attach.
    pub page_ops: usize,
    pub page_pts: usize,
}

impl Letters {
    /// `at` if it passes the text check for `word`, else `NO_AT`.
    pub(crate) fn checked(&self, word: u32, at: u16) -> u16 {
        if at == NO_AT {
            return NO_AT;
        }
        match self.qpc_len.get(word as usize) {
            Some(Some(n)) if at >= *n => NO_AT,
            _ => at,
        }
    }

    /// True when a member of the letter has a path with the mark `m`.
    pub(crate) fn letter_has_mark(&self, letter: u32, m: Mark) -> bool {
        let id = m as u8 as usize;
        self.letter_marks[letter as usize][id / 64] >> (id % 64) & 1 == 1
    }

    /// The pieces to draw in place of `path`, when it is a cut path that draws them now.
    pub(crate) fn drawn_pieces(&self, path: u32) -> Option<std::ops::Range<u32>> {
        let slot = *self.slot_of.get(path as usize)?;
        if slot == NONE || !self.swapped[slot as usize] {
            return None;
        }
        let (_, first, n) = self.sources[slot as usize];
        Some(first..first + n)
    }

    /// Whether the draw list can hold `path` now: false for a cut path that draws its
    /// pieces, and for a piece whose cut path is drawn whole.
    pub(crate) fn is_drawn(&self, path: u32) -> bool {
        if path < self.first_piece {
            return match self.slot_of.get(path as usize) {
                Some(&slot) if slot != NONE => !self.swapped[slot as usize],
                _ => true,
            };
        }
        let Some(piece) = self.data.pieces.get((path - self.first_piece) as usize) else { return false };
        let slot = self.slot_of[piece.source as usize];
        self.swapped[slot as usize]
    }

    /// The box of any path index: a page path's record box, or a piece's own box.
    pub(crate) fn path_box(&self, page: &PageData, path: u32) -> IBox {
        if path < self.first_piece {
            return page.paths.get(path as usize).map_or(IBox::EMPTY, |p| p.bbox);
        }
        self.piece_box.get((path - self.first_piece) as usize).copied().unwrap_or(IBox::EMPTY)
    }

    /// The slot in `sources` of a page path, when it is a cut path.
    pub(crate) fn slot_of_path(&self, path: u32) -> Option<u32> {
        self.slot_of.get(path as usize).copied().filter(|&s| s != NONE)
    }

    /// True when letter `letter` (an index into `data.letters`) is the character `text`
    /// and carries the mark `mark`. `None` and `Mark::None` take any letter.
    pub(crate) fn letter_is(&self, letter: u32, text: Option<char>, mark: Mark) -> bool {
        let Some(l) = self.data.letters.get(letter as usize) else { return false };
        text.is_none_or(|t| l.text == t) && (mark == Mark::None || self.letter_has_mark(letter, mark))
    }

    /// The path index of a member.
    fn member_path(&self, path: MemberPath) -> u32 {
        match path {
            MemberPath::Page(p) => p,
            MemberPath::Piece(i) => self.first_piece + i,
        }
    }

    /// The global letter index of a word's letter, when there is one.
    fn letter_of(&self, word: u32, letter: u32) -> Option<u32> {
        let &(first, n) = self.word_letters.get(word as usize)?;
        (letter < n).then_some(first + letter)
    }
}

/// What the letter selectors read: the layer, when one is attached, and every path's
/// context. The style engine builds one per path it resolves.
#[derive(Clone, Copy)]
pub(crate) struct LetterView<'a> {
    pub letters: Option<&'a Letters>,
    pub ctx: &'a [PathCtx],
}

impl LetterView<'_> {
    /// True when a letter selector takes the path whose context is `c`. With no layer,
    /// nothing is taken.
    pub(crate) fn takes(&self, sel: &Selector, c: &PathCtx) -> bool {
        let Some(lt) = self.letters else { return false };
        let (first, n) = c.letter.pieces;
        if n > 0 {
            // a cut path is taken when every piece is: a rule over the whole outline draws
            // it whole
            return (first..first + n).all(|p| self.ctx.get(p as usize).is_some_and(|pc| member_matches(lt, sel, pc)));
        }
        member_matches(lt, sel, c)
    }
}

/// True when a letter selector takes one path that is not cut: a letter member, a word
/// mark, or a path with no role. Every test is constant time: the facts come from the
/// path's own context and from what the attach indexed per letter.
pub(crate) fn member_matches(lt: &Letters, sel: &Selector, c: &PathCtx) -> bool {
    let m = &c.letter;
    // the word comes from the layer's letter or word mark, never from the path's context
    let of_letter = |w: u32, l: u16| m.letter != NONE && m.word == w && m.letter_in_word == l;
    match sel {
        Selector::Letter(w, l, part) => of_letter(*w, *l) && part.takes(m.own),
        Selector::LetterMark(w, l, n) => of_letter(*w, *l) && !m.is_body && m.nth_letter_mark == *n,
        Selector::LetterMarkNamed(w, l, mark) => of_letter(*w, *l) && *mark != Mark::None && c.mark == *mark,
        Selector::WordChars(w, from, to) => {
            if m.word == NONE || m.word != *w {
                return false;
            }
            let inside = |at: u16| at != NO_AT && *from <= at && at <= *to;
            // a member's own offset: a body and its dots carry the letter's, a mark its own
            if inside(m.at) {
                return true;
            }
            // the other characters a body draws, such as the kashida of a lam
            m.is_body
                && m.letter != NONE
                && lt
                    .data
                    .letters
                    .get(m.letter as usize)
                    .is_some_and(|l| l.more_at.iter().any(|&a| inside(lt.checked(*w, a))))
        }
        Selector::Letters(q) => {
            m.letter != NONE
                && q.words.binary_search(&m.word).is_ok()
                && lt.letter_is(m.letter, q.text, q.mark)
                && q.part.takes(m.own)
        }
        _ => false,
    }
}

/// True when paths `a` and `b` are in one colour state: the same colour now, the same
/// target, and, while either moves, the same transition. Two paths in one state move
/// alike on every tick, so they stay in it until a rule changes.
fn same_colour_state(colors: &[Rgba], anim: &[PathAnim], a: usize, b: usize) -> bool {
    let (Some(x), Some(y)) = (anim.get(a), anim.get(b)) else { return true };
    if colors.get(a) != colors.get(b) || x.target != y.target {
        return false;
    }
    match (x.dur > 0.0, y.dur > 0.0) {
        (false, false) => true,
        (true, true) => {
            (x.from, x.to, x.smooth) == (y.from, y.to, y.smooth)
                && x.t0.to_bits() == y.t0.to_bits()
                && x.dur.to_bits() == y.dur.to_bits()
        }
        _ => false,
    }
}

/// An offset as the C ABI reports it: `NONE` for `NO_AT`.
fn offset(at: u16) -> u32 {
    if at == NO_AT {
        NONE
    } else {
        at as u32
    }
}

/// True when a page path lies in the runs of two words. The page format allows it, and a
/// letter layer could then give a path to a letter of one word while its context names the
/// other.
pub(crate) fn word_runs_overlap(page: &PageData) -> bool {
    let mut owned = vec![false; page.paths.len()];
    for w in &page.words {
        let first = w.first_path as usize;
        for p in first..first.saturating_add(w.n_paths as usize) {
            match owned.get_mut(p) {
                Some(o) if *o => return true,
                Some(o) => *o = true,
                // `PageData::validate` keeps every run inside the table
                None => return true,
            }
        }
    }
    false
}

/// Refuse a layer whose counts do not fit the indices the engine keeps them in. `n_paths`,
/// `ops` and `pts` are the page's own path count and geometry lengths. After this passes,
/// every narrowing cast in [`Page::attach_letters`] is in range.
pub(crate) fn check_fits(data: &LetterData, n_paths: usize, ops: usize, pts: usize) -> Result<(), Error> {
    // a path index is a u32 and NONE is reserved
    if n_paths as u64 + data.pieces.len() as u64 >= NONE as u64 {
        return Err(Error::Bounds("letter layer pieces"));
    }
    // a letter's index in its word is a u16 and u16::MAX means none
    // (the letters of one word are next to each other: `check_against` keeps them in word
    // order), and a letter index is a u32 and NONE is reserved
    if data.letters.len() as u64 >= NONE as u64 {
        return Err(Error::Bounds("letter count"));
    }
    let mut run: Option<(u16, u64)> = None;
    for l in &data.letters {
        let n = match run {
            Some((w, n)) if w == l.word => n + 1,
            _ => 1,
        };
        if n > u16::MAX as u64 {
            return Err(Error::Bounds("letters in a word"));
        }
        run = Some((l.word, n));
    }
    // the geometry table holds u32 offsets: a command adds at most one op and six floats
    let cmds: u64 = data.pieces.iter().map(|p| p.cmds.len() as u64).sum();
    if ops as u64 + cmds > u32::MAX as u64 || pts as u64 + cmds * 6 > u32::MAX as u64 {
        return Err(Error::Bounds("letter layer geometry"));
    }
    Ok(())
}

impl Page {
    /// Attach a letter layer (`NNN.qvl`). The layer's pieces are appended to the path
    /// table after the page's own paths, and every rule is evaluated again.
    ///
    /// A file larger than [`MAX_LETTER_LAYER_BYTES`] is refused before it is decoded. A
    /// file that does not decode, or that belongs to another page, is refused too. On an
    /// error the page stays as it was, with the layer it had. A second attach replaces the
    /// first: its pieces are dropped and the new ones start at the same index.
    ///
    /// A page whose word path runs overlap takes no layer: the attach returns
    /// `Corrupt("overlapping word runs")`, so every path of a word belongs to one word only.
    /// A layer whose counts do not fit the engine's indices is refused too.
    ///
    /// An attach makes stale what was read before it: the geometry, the colours, and a
    /// [`crate::Layout`] cloned before it, which keeps the old path groups.
    pub fn attach_letters(&mut self, bytes: &[u8]) -> Result<(), Error> {
        if bytes.len() > MAX_LETTER_LAYER_BYTES {
            return Err(Error::Bounds("letter layer size"));
        }
        let data = decode_letters(bytes)?;
        data.check_against(&self.data)?;
        if word_runs_overlap(&self.data) {
            return Err(Error::Corrupt("overlapping word runs"));
        }
        let (n, ops, pts) = match self.letters.as_deref() {
            Some(lt) => (lt.first_piece as usize, lt.page_ops, lt.page_pts),
            None => (self.data.paths.len(), self.geom.ops.len(), self.geom.pts.len()),
        };
        check_fits(&data, n, ops, pts)?;
        // nothing of the page has changed up to here, and nothing below can fail
        self.drop_letters();
        self.install_letters(data);
        // the rules take the new pieces now, so the next draw list already holds the swaps
        self.refresh_display();
        Ok(())
    }

    /// True when a letter layer is attached.
    pub fn has_letters(&self) -> bool {
        self.letters.is_some()
    }

    /// How many letters a word has. 0 with no layer, or for a word past the page.
    pub fn letter_count(&self, word: u32) -> u32 {
        self.letters.as_deref().and_then(|lt| lt.word_letters.get(word as usize)).map_or(0, |w| w.1)
    }

    /// One letter of a word. `None` with no layer, or for a letter that does not exist.
    pub fn letter_info(&self, word: u32, letter: u32) -> Option<LetterInfo> {
        let lt = self.letters.as_deref()?;
        let g = lt.letter_of(word, letter)?;
        let l = &lt.data.letters[g as usize];
        let members = &lt.data.members[l.first_member as usize..l.first_member as usize + l.n_members as usize];
        let q = self.quant();
        let bb = lt.letter_box[g as usize];
        Some(LetterInfo {
            word,
            index: letter,
            at: offset(lt.checked(word, l.at)),
            text: l.text,
            n_members: l.n_members as u32,
            n_marks: members.iter().filter(|m| !m.is_body).count() as u32,
            more_at: l.more_at.iter().map(|&a| offset(lt.checked(word, a))).collect(),
            x0: bb.x0 as f32 / q,
            y0: bb.y0 as f32 / q,
            x1: bb.x1 as f32 / q,
            y1: bb.y1 as f32 / q,
            has_ink: members.iter().any(|m| self.geom.table[lt.member_path(m.path) as usize].op_count > 0),
        })
    }

    /// The paths of one letter. The body members come first, in the layer's order. The
    /// marks follow in text order: by offset, then in the layer's order, and a mark with no
    /// offset comes last. So the nth mark in this list is the one that
    /// `Selector::LetterMark(word, letter, n)` takes. Empty with no layer, or for a letter
    /// that does not exist.
    pub fn letter_members(&self, word: u32, letter: u32) -> Vec<LetterMember> {
        let Some(lt) = self.letters.as_deref() else { return vec![] };
        let Some(g) = lt.letter_of(word, letter) else { return vec![] };
        let l = &lt.data.letters[g as usize];
        lt.member_order[l.first_member as usize..l.first_member as usize + l.n_members as usize]
            .iter()
            .map(|&i| {
                let m = &lt.data.members[i as usize];
                let path = lt.member_path(m.path);
                LetterMember {
                    path,
                    at: offset(lt.checked(word, m.at)),
                    mark: self.path_ctx[path as usize].mark,
                    is_body: m.is_body,
                    is_cut: matches!(m.path, MemberPath::Piece(_)),
                }
            })
            .collect()
    }

    /// The letter a path draws: (letter index in the word, or `NONE` for a word-level mark;
    /// the checked offset, or `NONE`). `None` for a cut path, a decoration path, a path out
    /// of range, or with no layer.
    pub fn path_letter(&self, path: u32) -> Option<(u32, u32)> {
        self.letters.as_deref()?;
        let c = self.path_ctx.get(path as usize)?;
        // every path of a word has a role once the layer is checked: a member, a word
        // mark, or a cut path. A decoration path has none.
        if c.letter.word == NONE || c.letter.pieces.1 > 0 {
            return None;
        }
        let letter = if c.letter.letter == NONE { NONE } else { c.letter.letter_in_word as u32 };
        Some((letter, offset(c.letter.at)))
    }

    /// The letter of a word that draws the character at offset `at`, or that holds the mark
    /// at that offset. `None` for a word-level mark, an offset no letter has, or with no
    /// layer.
    pub fn letter_at_offset(&self, word: u32, at: u32) -> Option<u32> {
        let lt = self.letters.as_deref()?;
        let at = u16::try_from(at).ok().filter(|&a| a != NO_AT)?;
        let &(first, n) = lt.word_letters.get(word as usize)?;
        (first..first + n).find_map(|g| {
            let l = &lt.data.letters[g as usize];
            let members = &lt.data.members[l.first_member as usize..l.first_member as usize + l.n_members as usize];
            let hit = lt.checked(word, l.at) == at
                || l.more_at.iter().any(|&a| lt.checked(word, a) == at)
                || members.iter().any(|m| !m.is_body && lt.checked(word, m.at) == at);
            hit.then_some(g - first)
        })
    }

    /// The letters a query finds: (word, letter index in the word), in word order, then in
    /// letter order. The query's `text` and `mark` filter the letters, and its `part` does
    /// not. Empty with no layer.
    pub fn find_letters(&self, q: &LetterQuery) -> Vec<(u32, u32)> {
        let Some(lt) = self.letters.as_deref() else { return vec![] };
        let mut found = vec![];
        for w in self.target_words(&q.scope) {
            let Some(&(first, n)) = lt.word_letters.get(w as usize) else { continue };
            for g in first..first + n {
                if lt.letter_is(g, q.text, q.mark) {
                    found.push((w, g - first));
                }
            }
        }
        found
    }

    /// The letter under a point in page units, and the mark when the point is on a mark.
    ///
    /// The word is the word [`Page::hit_test`] finds, with the same options: `max_distance`,
    /// `gap_bias` and `prefer_exact` choose the word exactly as they do there. Inside that
    /// word the engine always tests the outlines, whatever `prefer_exact` is:
    ///
    /// - A point inside the outline of a member, or of a word-level mark, is an exact hit.
    ///   Ink the page stores as a mark wins over body ink, and of two paths of the same ink,
    ///   the one drawn last wins. A mark is small and sits on or over a body, so a tap on it
    ///   means the mark. A word-level mark gives `letter` `NONE`.
    /// - A cut path is never hit: its pieces are. The pieces are tested whether the path
    ///   draws whole or as its pieces now, so a hit does not change while colours move. Each
    ///   path and piece is culled by its own box, and a piece can reach past its word's box.
    /// - A point off every outline is a near hit: the letter whose box is nearest, with that
    ///   distance, its first body member as `path`, its offset, and `Mark::None`.
    ///
    /// `None` with no layer, when the word hit test finds no word or a decoration, or when
    /// the word has no letter to be near. The cost is that of [`Page::hit_test`] and a test of
    /// the paths and pieces of one word, then a look at the boxes of its letters.
    pub fn hit_test_letter(&self, x: f32, y: f32, opt: &HitOptions) -> Option<LetterHit> {
        self.letters.as_deref()?;
        let h = self.hit_test(x, y, opt)?;
        self.letter_in_word(h.word, h.line, x, y)
    }

    /// The letter under a point in layout px, as the other `_view` hit tests take it: the
    /// word is the one [`Page::hit_test_view`] finds, and the point goes back to page units
    /// through the current layout's placement of that word, printed or reflowed. Then it is
    /// [`Page::hit_test_letter`]. With no layout, the point is in page units, as
    /// [`Page::hit_test_view`] takes it then.
    pub fn hit_test_letter_view(&self, view_x: f32, view_y: f32, opt: &HitOptions) -> Option<LetterHit> {
        self.letters.as_deref()?;
        let h = self.hit_test_view(view_x, view_y, opt)?;
        let (x, y) = match self.current_layout() {
            None => (view_x, view_y),
            Some(l) => {
                let line = self.data.words.get(h.word as usize)?.line_index as usize;
                let (x, y) = ((view_x - l.offset_x) / l.scale, (view_y - l.offset_y) / l.scale);
                l.word_placement(h.word, line).invert(x, y)
            }
        };
        self.letter_in_word(h.word, h.line, x, y)
    }

    /// The letter of word `w` under a point in page units: the exact pass over the word's
    /// paths and pieces, then the nearest letter's box. `line` is the word hit's line.
    fn letter_in_word(&self, w: u32, line: u32, x: f32, y: f32) -> Option<LetterHit> {
        let lt = self.letters.as_deref()?;
        let word = self.data.words.get(w as usize)?;
        let q = self.quant();
        let (qx, qy) = ((x * q).round() as i32, (y * q).round() as i32);
        // the layer's word box holds every member's box, pieces included
        if lt.word_box.get(w as usize).is_some_and(|b| b.contains(qx, qy)) {
            // in reverse draw order, so the first mark found is the one drawn last. A body
            // counts only when no mark holds the point
            let mut body = None;
            let first = word.first_path;
            for p in (first..first + word.n_paths as u32).rev() {
                let Some(c) = self.path_ctx.get(p as usize) else { continue };
                // a cut path is hit as its pieces, which are drawn in its place
                let (a, n) = c.letter.pieces;
                let paths = if n > 0 { a..a + n } else { p..p + 1 };
                for path in paths.rev() {
                    let Some(pc) = self.path_ctx.get(path as usize) else { continue };
                    if pc.letter.word != w {
                        continue;
                    }
                    let is_mark = pc.kind != PathKind::Body;
                    if !is_mark && body.is_some() {
                        continue;
                    }
                    if !lt.path_box(&self.data, path).contains(qx, qy) || !self.point_in_path(path, x, y) {
                        continue;
                    }
                    if is_mark {
                        return Some(self.exact_letter_hit(path, line));
                    }
                    body = Some(path);
                }
            }
            if let Some(path) = body {
                return Some(self.exact_letter_hit(path, line));
            }
        }
        // the nearest letter's box. The first letter wins a tie
        let &(first, n) = lt.word_letters.get(w as usize)?;
        let mut best: Option<(u32, f32)> = None;
        for g in first..first + n {
            let b = lt.letter_box[g as usize];
            if b.is_empty() {
                continue;
            }
            let dx = (b.x0 as f32 / q - x).max(x - b.x1 as f32 / q).max(0.0);
            let dy = (b.y0 as f32 / q - y).max(y - b.y1 as f32 / q).max(0.0);
            let d = (dx * dx + dy * dy).sqrt();
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((g, d));
            }
        }
        let (g, distance) = best?;
        let l = &lt.data.letters[g as usize];
        let slots = &lt.member_order[l.first_member as usize..l.first_member as usize + l.n_members as usize];
        // the body members come first in the engine's order, and every letter has one
        let path = slots
            .iter()
            .map(|&i| &lt.data.members[i as usize])
            .find(|m| m.is_body)
            .map_or(NONE, |m| lt.member_path(m.path));
        Some(LetterHit {
            word: w,
            letter: g - first,
            path,
            at: offset(lt.checked(w, l.at)),
            line,
            distance,
            mark: Mark::None,
            is_exact: false,
        })
    }

    /// An exact hit on `path`, a letter member or a word-level mark.
    fn exact_letter_hit(&self, path: u32, line: u32) -> LetterHit {
        let c = &self.path_ctx[path as usize];
        let m = &c.letter;
        LetterHit {
            word: m.word,
            letter: if m.letter == NONE { NONE } else { m.letter_in_word as u32 },
            path,
            at: offset(m.at),
            line,
            distance: 0.0,
            mark: c.mark,
            is_exact: true,
        }
    }

    /// What is drawn for page path `path` now: its pieces, when it is a cut path that draws
    /// them, else the path itself.
    pub(crate) fn drawn_in_place_of(&self, path: u32) -> std::ops::Range<u32> {
        self.letters.as_deref().and_then(|lt| lt.drawn_pieces(path)).unwrap_or(path..path + 1)
    }

    /// Decide again which cut paths draw their pieces: the paths of `slots` (indices into
    /// `Letters::sources`), or every cut path with `None`.
    ///
    /// A cut path draws its pieces while one of them is in another colour state than the
    /// path: another colour now, another target, or another transition running. A rule that
    /// is gone keeps the pieces until its transition has ended. Two paths in one state stay
    /// in it as the clock moves, so this needs to run only after a rule change and when a
    /// transition ends. When the drawn set changes, the draw revision counts up and the
    /// current layout's cached draw list is dropped.
    pub(crate) fn update_swaps(&mut self, slots: Option<&[u32]>) {
        let (colors, anim) = (&self.colors, &self.anim);
        let Some(lt) = self.letters.as_deref_mut() else { return };
        let mut changed = false;
        let mut update = |slot: usize| {
            let Some(&(src, first, n)) = lt.sources.get(slot) else { return };
            let now = (first..first + n).any(|p| !same_colour_state(colors, anim, src as usize, p as usize));
            if lt.swapped[slot] != now {
                lt.swapped[slot] = now;
                changed = true;
            }
        };
        match slots {
            Some(slots) => slots.iter().for_each(|&s| update(s as usize)),
            None => (0..lt.sources.len()).for_each(update),
        }
        if changed {
            self.draw_revision = self.draw_revision.wrapping_add(1);
            if let Some(l) = &self.layout {
                l.draw_list.clear();
            }
        }
    }

    /// Put each member's checked offset into its context. Runs after an attach and after a
    /// words sidecar.
    pub(crate) fn check_letter_offsets(&mut self) {
        let Some(lt) = self.letters.as_deref_mut() else { return };
        // only an inline or attached `qpc` counts: the other forms are not the text the
        // offsets count in
        let (words, strings) = (&self.data.words, &self.data.strings);
        lt.qpc_len = words
            .iter()
            .map(|w| {
                if w.qpc == NONE_U16 {
                    return None;
                }
                let n = strings.get(w.qpc as usize)?.chars().count();
                Some(u16::try_from(n).unwrap_or(u16::MAX))
            })
            .collect();
        for c in &mut self.path_ctx {
            c.letter.at = if c.letter.word == NONE { NO_AT } else { lt.checked(c.letter.word, c.letter.raw_at) };
        }
    }

    /// Drop the layer, its pieces and every state sized by them. The page's own paths keep
    /// their geometry, colours and transitions.
    fn drop_letters(&mut self) {
        let Some(old) = self.letters.take() else { return };
        let n = old.first_piece as usize;
        self.geom.ops.truncate(old.page_ops);
        self.geom.pts.truncate(old.page_pts);
        self.geom.table.truncate(n);
        self.colors.truncate(n);
        self.anim.truncate(n);
        self.path_ctx.truncate(n);
        self.path_deco.truncate(n);
        self.animating.retain(|&p| (p as usize) < n);
        for (i, c) in self.path_ctx.iter_mut().enumerate() {
            c.letter = PathLetter::page_path(i as u32);
        }
    }

    /// Append the pieces of a checked layer and index its letters.
    ///
    /// The casts to u32 and u16 below are in range: `check_against` and `check_fits` have
    /// passed. The page's path count is the layer's `n_page_paths`, a u32. Paths, pieces and
    /// letters stay below `NONE`, the geometry buffers below `u32::MAX`, and a word holds at
    /// most `u16::MAX` letters, so a letter's index in its word stays below `u16::MAX`.
    fn install_letters(&mut self, data: LetterData) {
        let n = self.data.paths.len() as u32;
        let (page_ops, page_pts) = (self.geom.ops.len(), self.geom.pts.len());
        let q = self.data.header.quant as f32;
        let moving: std::collections::HashSet<u32> = self.animating.iter().copied().collect();
        let mut piece_box = Vec::with_capacity(data.pieces.len());

        // the pieces, as paths n.. in the layer's order
        for (i, piece) in data.pieces.iter().enumerate() {
            let src = piece.source as usize;
            let crate::Geometry { ops, pts, table } = &mut self.geom;
            let (op_start, pt_start) = (ops.len() as u32, pts.len() as u32);
            // a piece of ink off the page has no outline, as its source has none
            if load::on_page(&self.data, &self.data.paths[src].bbox) {
                let mut push = |x: f32, y: f32| {
                    pts.push(x);
                    pts.push(y);
                };
                for c in &piece.cmds {
                    load::emit(ops, &mut push, *c, |x, y| (x as f32 / q, y as f32 / q));
                }
            }
            let s = table[src];
            table.push(PathGeom {
                op_start,
                op_count: ops.len() as u32 - op_start,
                pt_start,
                pt_count: pts.len() as u32 - pt_start,
                // a piece is the page's own geometry, never a glyph instance: the glyph bit
                // of `flatten` is cleared
                flags: s.flags & !(2 << 24),
                ..s
            });
            let pi = n + i as u32;
            let mut ctx = self.path_ctx[src];
            ctx.letter = PathLetter { source: piece.source, ..PathLetter::page_path(pi) };
            self.path_ctx.push(ctx);
            self.path_deco.push(NONE);
            // a piece starts where its source is: same colour, same target, same transition
            self.anim.push(self.anim[src]);
            self.colors.push(self.colors[src]);
            if moving.contains(&piece.source) {
                self.animating.push(pi);
            }
            piece_box.push(cmds_bbox(&piece.cmds));
        }

        // the cut paths: the pieces of one source are next to each other
        let mut sources: Vec<(u32, u32, u32)> = Vec::new();
        for (i, piece) in data.pieces.iter().enumerate() {
            match sources.last_mut() {
                Some(s) if s.0 == piece.source => s.2 += 1,
                _ => sources.push((piece.source, n + i as u32, 1)),
            }
        }
        let mut slot_of = vec![NONE; n as usize];
        for (slot, &(src, first, count)) in sources.iter().enumerate() {
            slot_of[src as usize] = slot as u32;
            self.path_ctx[src as usize].letter.pieces = (first, count);
        }

        // the letters of each word, and the letter facts of each member
        let mut word_letters = vec![(0u32, 0u32); self.data.words.len()];
        let mut letter_box = Vec::with_capacity(data.letters.len());
        let mut letter_marks = Vec::with_capacity(data.letters.len());
        let mut member_order: Vec<u32> = (0..data.members.len() as u32).collect();
        let path_box = |p: u32| -> IBox {
            if p < n {
                self.data.paths[p as usize].bbox
            } else {
                piece_box[(p - n) as usize]
            }
        };
        for (g, l) in data.letters.iter().enumerate() {
            let w = &mut word_letters[l.word as usize];
            if w.1 == 0 {
                w.0 = g as u32;
            }
            w.1 += 1;
            // below u16::MAX: `check_fits` caps the letters of one word
            let in_word = (g as u32 - w.0) as u16;
            let (first, count) = (l.first_member as usize, l.n_members as usize);
            // the bodies first in the layer's order, then the marks in text order. NO_AT is
            // above every offset, so a mark with no offset sorts last, and the sort is stable
            let order = &mut member_order[first..first + count];
            order.sort_by_key(|&i| {
                let m = &data.members[i as usize];
                if m.is_body {
                    (0u8, 0u16)
                } else {
                    (1, m.at)
                }
            });
            let (mut bb, mut marks, mut nth) = (IBox::EMPTY, [0u64; 4], 0u16);
            for &i in order.iter() {
                let m = &data.members[i as usize];
                let p = match m.path {
                    MemberPath::Page(p) => p,
                    MemberPath::Piece(i) => n + i,
                };
                bb.union(&path_box(p));
                let c = &mut self.path_ctx[p as usize];
                if c.mark != Mark::None {
                    let id = c.mark as u8 as usize;
                    marks[id / 64] |= 1 << (id % 64);
                }
                // the word of the letter, which a page whose runs overlap could make
                // differ from `c.word`; `attach_letters` refuses such a page
                c.letter.word = l.word as u32;
                c.letter.letter = g as u32;
                c.letter.letter_in_word = in_word;
                c.letter.raw_at = m.at;
                c.letter.is_body = m.is_body;
                c.letter.own = m.at == l.at;
                // a letter has at most u16::MAX members and one of them is its body, so
                // this count stays below u16::MAX
                c.letter.nth_letter_mark = if m.is_body {
                    u16::MAX
                } else {
                    nth += 1;
                    nth - 1
                };
            }
            letter_box.push(bb);
            letter_marks.push(marks);
        }
        for wm in &data.word_marks {
            let c = &mut self.path_ctx[wm.path as usize].letter;
            c.word = wm.word as u32;
            c.raw_at = wm.at;
        }
        // a piece's word is the word of the letter that holds it: every piece is a member
        let mut word_box: Vec<IBox> = self.data.words.iter().map(|w| w.bbox).collect();
        for (i, (piece, bb)) in data.pieces.iter().zip(&piece_box).enumerate() {
            let w = self.path_ctx[n as usize + i].letter.word;
            self.path_ctx[piece.source as usize].letter.word = w;
            word_box[w as usize].union(bb);
        }

        let swapped = vec![false; sources.len()];
        self.letters = Some(Box::new(Letters {
            data,
            first_piece: n,
            word_letters,
            letter_box,
            letter_marks,
            member_order,
            piece_box,
            word_box,
            sources,
            swapped,
            slot_of,
            qpc_len: Vec::new(),
            page_ops,
            page_pts,
        }));
        self.check_letter_offsets();

        // every rule is evaluated again, and every cache of the draw list goes
        self.styles.dirty = true;
        self.state_dirty = true;
        self.draw_revision = self.draw_revision.wrapping_add(1);
        if let Some(l) = &self.layout {
            l.draw_list.clear();
        }
        if self.reflow_prepared.get().is_some() {
            let groups = reflow::path_groups(self);
            if let Some(prep) = self.reflow_prepared.get_mut() {
                prep.path_group = std::sync::Arc::clone(&groups);
            }
            if let Some(l) = self.layout.as_mut().filter(|l| l.reflow.is_some()) {
                l.path_group = groups;
            }
        }
    }
}
