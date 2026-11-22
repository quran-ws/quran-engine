//! Style engine: layered rules with handles that undo exactly, sub-word
//! selectors, and per-path colour transitions driven by the engine clock.
use crate::letters::{LetterPart, LetterQuery, LetterSelection, LetterView};
use crate::{Page, Rgba, DEFAULT_INK, NONE};
use qvp_format::*;

/// What a rule applies to. Word/path indices are page-local.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Selector {
    Page,
    /// one engine path
    Path(u32),
    /// nth path of a word (0-based, in file order)
    WordPath(u32, u16),
    /// nth *mark* path of a word (0-based, counting marks only)
    WordMark(u32, u16),
    /// nth mark of that name in the word (0-based)
    WordMarkNamed(u32, Mark, u16),
    /// only the body (letter) paths of a word
    WordBody(u32),
    /// only the mark paths of a word
    WordMarks(u32),
    Word(u32),
    Ayah(u16, u16),
    Line(u8),
    /// mark paths by name, optionally restricted to a word list (empty = page)
    Mark(Mark),
    Category(Category),
    Family(Family),
    Kind(PathKind),
    Deco(DecoKind),
    DecoIdx(u32),
    /// A letter of a word: (word, letter index in the word, part). The parts go by offset
    /// ([`LetterPart`]): `BodyAndDots` takes every member at the letter's own offset, so it
    /// takes the same paths as `WordChars(word, at, at)` for that offset. With no letter
    /// layer it selects nothing.
    Letter(u32, u16, LetterPart),
    /// The nth mark of a letter: (word, letter, n). The marks count in text order: by
    /// offset, then in the layer's member order, and a mark with no offset comes last. Dots
    /// count, and a body never counts. This is the order of the marks in
    /// [`Page::letter_members`]. No part limits it: a hamzah sign at the letter's own offset
    /// is still one of its marks.
    LetterMark(u32, u16, u16),
    /// Every member of a letter whose path has that mark: (word, letter, mark). A body that
    /// the page stores as a mark counts. No part limits it. `Mark::None` selects nothing.
    LetterMarkNamed(u32, u16, Mark),
    /// The ink of a word's `qpc` characters, by code-point offset: (word, from, to), both
    /// ends in. An offset takes every member at that offset, and an extra offset the body
    /// draws takes the body. A word-level mark is taken by its own offset. A character with
    /// no ink, or a mark with no offset, is never taken.
    WordChars(u32, u16, u16),
    /// Letters by what they are: the rule [`Page::style_letters`] adds.
    Letters(LetterSelection),
}

impl Selector {
    fn specificity(&self) -> u8 {
        match self {
            Selector::Page => 0,
            Selector::Kind(_) | Selector::Deco(_) => 1,
            Selector::Family(_) => 2,
            Selector::Category(_) => 3,
            Selector::Mark(_) => 4,
            Selector::Line(_) => 5,
            Selector::Ayah(..) => 6,
            Selector::DecoIdx(_) => 7,
            Selector::Word(_) => 8,
            Selector::WordBody(_) | Selector::WordMarks(_) => 9,
            Selector::Letters(_) => 9,
            Selector::WordMark(..) | Selector::WordMarkNamed(..) | Selector::WordPath(..) => 10,
            Selector::Letter(..)
            | Selector::LetterMark(..)
            | Selector::LetterMarkNamed(..)
            | Selector::WordChars(..) => 10,
            Selector::Path(_) => 11,
        }
    }
}

/// Colour plus how long a change to/from it takes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Paint {
    pub color: Rgba,
    pub transition_ms: u32,
}

impl Paint {
    pub const fn new(color: Rgba) -> Paint {
        Paint { color, transition_ms: 0 }
    }
    pub const fn fade(color: Rgba, ms: u32) -> Paint {
        Paint { color, transition_ms: ms }
    }
    pub const HIDDEN: Paint = Paint { color: 0, transition_ms: 0 };
}

/// Opaque handle returned by every mutating style call; `remove(handle)` undoes exactly that call.
pub type Handle = u32;

#[derive(Clone, Debug)]
pub struct Rule {
    pub handle: Handle,
    pub layer: i32,
    pub sel: Selector,
    pub paint: Paint,
    seq: u64,
}

/// Well-known layers (higher wins). Apps may use any i32.
pub const LAYER_BASE: i32 = 0;
pub const LAYER_THEME: i32 = 10;
pub const LAYER_HIGHLIGHT: i32 = 50;
pub const LAYER_SELECTION: i32 = 60;
pub const LAYER_TOP: i32 = 100;
/// The engine's own layer, above every host layer. A dressed page hides its printed
/// rings and bands here. A host cannot add a rule on it: [`StyleEngine::add`] and
/// [`StyleEngine::add_many`] put a rule asked for at this layer one layer below. A
/// higher layer always wins, so no host rule beats an engine rule, whatever its
/// selector. [`StyleEngine::clear`] and [`StyleEngine::clear_layer`] leave it alone.
/// It is not part of the C header or the wrappers.
pub(crate) const LAYER_ENGINE: i32 = i32::MAX;

#[derive(Debug, Default)]
pub struct StyleEngine {
    pub(crate) rules: Vec<Rule>,
    next_handle: Handle,
    seq: u64,
    pub(crate) dirty: bool,
    pub default_ink: Rgba,
}

impl StyleEngine {
    pub fn new() -> Self {
        StyleEngine { rules: vec![], next_handle: 1, seq: 0, dirty: true, default_ink: DEFAULT_INK }
    }
    fn alloc(&mut self) -> Handle {
        let h = self.next_handle;
        self.next_handle += 1;
        h
    }
    /// Add one rule. Returns its handle. The engine's own layer, `i32::MAX`, is reserved:
    /// a rule asked for there goes one layer below.
    pub fn add(&mut self, layer: i32, sel: Selector, paint: Paint) -> Handle {
        let h = self.alloc();
        self.push(h, layer.min(LAYER_ENGINE - 1), sel, paint);
        h
    }
    /// Add one rule on the engine's own layer. Returns its handle.
    pub(crate) fn add_engine(&mut self, sel: Selector, paint: Paint) -> Handle {
        let h = self.alloc();
        self.push(h, LAYER_ENGINE, sel, paint);
        h
    }
    /// Add several rules under one handle. The engine's own layer, `i32::MAX`, is
    /// reserved: rules asked for there go one layer below.
    pub fn add_many(&mut self, layer: i32, sels: impl IntoIterator<Item = Selector>, paint: Paint) -> Handle {
        let h = self.alloc();
        let layer = layer.min(LAYER_ENGINE - 1);
        for s in sels {
            self.push(h, layer, s, paint);
        }
        h
    }
    pub(crate) fn push(&mut self, handle: Handle, layer: i32, sel: Selector, paint: Paint) {
        self.seq += 1;
        self.rules.push(Rule { handle, layer, sel, paint, seq: self.seq });
        self.dirty = true;
    }
    /// Add a rule under an existing handle, on a host layer: `LAYER_ENGINE` becomes
    /// the layer below it.
    pub(crate) fn push_under(&mut self, handle: Handle, layer: i32, sel: Selector, paint: Paint) {
        self.push(handle, layer.min(LAYER_ENGINE - 1), sel, paint);
    }
    pub(crate) fn new_handle(&mut self) -> Handle {
        self.alloc()
    }
    /// Remove everything added under a handle. Returns the number of rules removed.
    /// Rules on the engine's own layer, `i32::MAX`, stay: a handle that names them
    /// removes nothing and returns 0.
    pub fn remove(&mut self, handle: Handle) -> usize {
        self.remove_where(|r| r.handle == handle && r.layer != LAYER_ENGINE)
    }
    /// Remove the engine's own rules under a handle. Returns the number removed.
    pub(crate) fn remove_engine(&mut self, handle: Handle) -> usize {
        self.remove_where(|r| r.handle == handle && r.layer == LAYER_ENGINE)
    }
    fn remove_where(&mut self, f: impl Fn(&Rule) -> bool) -> usize {
        let n = self.rules.len();
        self.rules.retain(|r| !f(r));
        let removed = n - self.rules.len();
        if removed > 0 {
            self.dirty = true;
        }
        removed
    }
    /// Change the paint of an existing handle in place (keeps selectors and layer).
    /// Rules on the engine's own layer, `i32::MAX`, keep their paint: a handle that
    /// names them changes nothing and returns 0.
    pub fn repaint(&mut self, handle: Handle, paint: Paint) -> usize {
        let mut n = 0;
        for r in self.rules.iter_mut().filter(|r| r.handle == handle && r.layer != LAYER_ENGINE) {
            r.paint = paint;
            n += 1;
        }
        if n > 0 {
            self.dirty = true;
        }
        n
    }
    /// Remove every host rule. Rules on the engine's own layer, `i32::MAX`, stay: they
    /// belong to the engine, and the engine removes them by handle.
    pub fn clear(&mut self) {
        let n = self.rules.len();
        self.rules.retain(|r| r.layer == LAYER_ENGINE);
        if n != self.rules.len() {
            self.dirty = true;
        }
    }
    /// Remove every rule on one layer. On the engine's own layer, `i32::MAX`, this does
    /// nothing.
    pub fn clear_layer(&mut self, layer: i32) {
        if layer == LAYER_ENGINE {
            return;
        }
        let n = self.rules.len();
        self.rules.retain(|r| r.layer != layer);
        if n != self.rules.len() {
            self.dirty = true;
        }
    }
    pub fn set_default_ink(&mut self, c: Rgba) {
        if self.default_ink != c {
            self.default_ink = c;
            self.dirty = true;
        }
    }
    pub fn len(&self) -> usize {
        self.rules.len()
    }
    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }
    pub fn handles(&self) -> Vec<Handle> {
        let mut v: Vec<Handle> = self.rules.iter().map(|r| r.handle).collect();
        v.sort_unstable();
        v.dedup();
        v
    }
}

/// Per-path facts the matcher needs, precomputed at load.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PathCtx {
    pub word: u32,
    pub decoration: u32,
    pub line_number: u8,
    pub surah: u16,
    pub ayah: u16,
    pub kind: PathKind,
    pub mark: Mark,
    pub family: Family,
    pub category: Category,
    /// index of this path within its word (all paths), and among its word's marks
    pub nth_in_word: u16,
    pub nth_mark: u16,
    pub nth_mark_named: u16,
    pub deco_kind: DecoKind,
    /// The path's role in the letter layer. A path with no role when no layer is attached.
    pub letter: crate::letters::PathLetter,
}

impl Rule {
    /// True when the rule takes path `pi`, whose context is `c`. `lx` holds the letter
    /// layer, which the letter selectors need.
    fn matches(&self, c: &PathCtx, pi: u32, lx: LetterView<'_>) -> bool {
        match &self.sel {
            Selector::Page => true,
            // a piece follows the path it is cut from: hiding a body hides its pieces
            Selector::Path(p) => *p == pi || c.letter.source == *p,
            Selector::WordPath(w, n) => c.word == *w && c.nth_in_word == *n,
            Selector::WordMark(w, n) => c.word == *w && c.kind == PathKind::Mark && c.nth_mark == *n,
            Selector::WordMarkNamed(w, m, n) => c.word == *w && c.mark == *m && c.nth_mark_named == *n,
            Selector::WordBody(w) => c.word == *w && c.kind == PathKind::Body,
            Selector::WordMarks(w) => c.word == *w && c.kind == PathKind::Mark,
            Selector::Word(w) => c.word == *w,
            Selector::Ayah(s, a) => c.surah == *s && c.ayah == *a && c.ayah != 0,
            Selector::Line(l) => c.line_number == *l,
            Selector::Mark(m) => c.mark == *m,
            Selector::Category(k) => c.category == *k && c.kind == PathKind::Mark,
            Selector::Family(f) => c.family == *f,
            Selector::Kind(k) => c.kind == *k,
            Selector::Deco(k) => c.decoration != NONE && c.deco_kind == *k,
            Selector::DecoIdx(d) => c.decoration == *d,
            Selector::Letter(..)
            | Selector::LetterMark(..)
            | Selector::LetterMarkNamed(..)
            | Selector::WordChars(..)
            | Selector::Letters(_) => lx.takes(&self.sel, c),
        }
    }
}

/// Per-path animation state.
#[derive(Clone, Copy, Debug)]
pub(crate) struct PathAnim {
    pub target: Rgba,
    pub target_ms: u32,
    pub from: Rgba,
    pub to: Rgba,
    pub t0: f64,
    pub dur: f64,
    pub cur: Rgba,
    /// Ease in and out instead of out: a word fading under the mask.
    pub smooth: bool,
}

impl Default for PathAnim {
    fn default() -> Self {
        PathAnim {
            target: DEFAULT_INK,
            target_ms: 0,
            from: DEFAULT_INK,
            to: DEFAULT_INK,
            t0: 0.0,
            dur: 0.0,
            cur: DEFAULT_INK,
            smooth: false,
        }
    }
}

pub(crate) fn ease_out(t: f64) -> f64 {
    let u = 1.0 - t.clamp(0.0, 1.0);
    1.0 - u * u * u
}

/// Smoothstep. A word fading in under an ease-out curve is most of the way to full ink in its
/// first frames and then creeps, which reads as a jump to grey; this one starts and ends gently.
pub(crate) fn ease_in_out(t: f64) -> f64 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The paint of a cut path: its own, or the colour all its pieces resolve to when they agree
/// on one. Then a rule on each letter of a joined outline, all in one colour, draws the outline
/// whole in that colour. The cut path's transition is the longest of its pieces', so it never
/// settles before them. With no pieces, or pieces that do not agree, the path keeps its own.
fn shared_paint(own: Paint, pieces: &[Paint]) -> Paint {
    let Some(first) = pieces.first() else { return own };
    if first.color == own.color || pieces.iter().any(|p| p.color != first.color) {
        return own;
    }
    Paint { color: first.color, transition_ms: pieces.iter().map(|p| p.transition_ms).max().unwrap_or(0) }
}

pub fn lerp_rgba(a: Rgba, b: Rgba, t: f64) -> Rgba {
    if t <= 0.0 {
        return a;
    }
    if t >= 1.0 {
        return b;
    }
    let ch = |sh: u32| -> u32 {
        let x = ((a >> sh) & 255) as f64;
        let y = ((b >> sh) & 255) as f64;
        ((x + (y - x) * t).round() as u32).min(255)
    };
    ch(24) << 24 | ch(16) << 16 | ch(8) << 8 | ch(0)
}

impl Page {
    /// Winning rule paint for a path, or None when no rule applies.
    pub(crate) fn rule_paint(&self, pi: u32) -> Option<Paint> {
        let c = &self.path_ctx[pi as usize];
        let lx = LetterView { letters: self.letters.as_deref(), ctx: &self.path_ctx };
        let mut best: Option<(&Rule, u8)> = None;
        for r in &self.styles.rules {
            if !r.matches(c, pi, lx) {
                continue;
            }
            let sp = r.sel.specificity();
            let better = match best {
                None => true,
                Some((b, bsp)) => (r.layer, sp, r.seq) > (b.layer, bsp, b.seq),
            };
            if better {
                best = Some((r, sp));
            }
        }
        best.map(|(r, _)| r.paint)
    }

    /// Resolve the *target* colour of a path: mask → rules → reveal → default.
    pub(crate) fn target_paint(&self, pi: u32) -> Paint {
        let c = &self.path_ctx[pi as usize];
        if c.word != NONE && self.mask.hidden.contains(&c.word) && self.mask.mode == crate::memorize::MaskMode::Hide {
            // a fade keeps the ink's colour and moves only its alpha; a reveal fades back
            // because a transition runs at the longer of the old and new paint's durations
            return match self.mask.transition_ms {
                0 => Paint::HIDDEN,
                ms => Paint::fade(self.styles.default_ink & 0xffff_ff00, ms),
            };
        }
        if let Some(p) = self.rule_paint(pi) {
            return p;
        }
        if let Some(rv) = &self.reveal {
            if let Some(col) = rv.color_for(self, pi) {
                return Paint::fade(col, rv.transition_ms);
            }
        }
        Paint::new(self.styles.default_ink)
    }

    /// Advance the engine clock. Returns true while any colour or band is still moving.
    ///
    /// Only the paths with a transition running are visited, so a settled page pays nothing
    /// per frame however many paths it has.
    pub fn tick(&mut self, now_ms: f64) -> bool {
        self.clock_ms = now_ms;
        self.refresh_targets();
        // every listed path has a transition running: `refresh_targets` compacts the list
        // when it settles one before its time
        let (anim, colors, now) = (&mut self.anim, &mut self.colors, self.clock_ms);
        // the cut paths of the paths that settle in this tick: only they can go back to whole
        let (letters, ctx) = (self.letters.as_deref(), &self.path_ctx);
        let mut settled: Vec<u32> = Vec::new();
        self.animating.retain(|&pi| {
            let a = &mut anim[pi as usize];
            let t = (now - a.t0) / a.dur;
            let moving = t < 1.0;
            if moving {
                a.cur = lerp_rgba(a.from, a.to, if a.smooth { ease_in_out(t) } else { ease_out(t) });
            } else {
                a.cur = a.to;
                a.dur = 0.0;
                if let Some(slot) = letters.and_then(|lt| lt.slot_of_path(ctx[pi as usize].letter.source)) {
                    settled.push(slot);
                }
            }
            colors[pi as usize] = a.cur;
            moving
        });
        if !settled.is_empty() {
            self.update_swaps(Some(&settled));
        }
        !self.animating.is_empty() | self.highlights_moving()
    }

    /// Recompute targets when rules changed; start transitions where targets moved.
    pub(crate) fn refresh_targets(&mut self) {
        if !self.styles.dirty && !self.state_dirty {
            return;
        }
        self.styles.dirty = false;
        self.state_dirty = false;
        // words under a fading mask (revealed ones included) ease in and out
        let fading: std::collections::HashSet<u32> =
            if self.mask.transition_ms > 0 { self.mask.words.iter().copied().collect() } else { Default::default() };
        let mut cut_short = false;
        // a letter layer's pieces resolve first: a cut path takes the colour they share
        let first_piece = self.letters.as_deref().map_or(self.anim.len(), |lt| lt.first_piece as usize);
        let piece_paints: Vec<Paint> = (first_piece..self.anim.len()).map(|pi| self.target_paint(pi as u32)).collect();
        for pi in 0..self.anim.len() {
            let p = match self.path_ctx[pi].letter.pieces {
                _ if pi >= first_piece => piece_paints[pi - first_piece],
                (first, n) if n > 0 => {
                    let from = first as usize - first_piece;
                    shared_paint(self.target_paint(pi as u32), &piece_paints[from..from + n as usize])
                }
                _ => self.target_paint(pi as u32),
            };
            let word = self.path_ctx[pi].word;
            let a = &mut self.anim[pi];
            if p.color != a.target {
                let prev_ms = a.target_ms;
                a.target = p.color;
                a.target_ms = p.transition_ms;
                let ms = p.transition_ms.max(prev_ms) as f64;
                if ms > 0.0 {
                    self.start_transition(pi, p.color, ms, word != NONE && fading.contains(&word));
                } else {
                    cut_short |= self.settle(pi, p.color);
                }
            } else if p.transition_ms != a.target_ms {
                a.target_ms = p.transition_ms;
            }
        }
        // a path settled mid-transition leaves the list here, once for the whole pass, so
        // cutting a page-wide fade short costs the list one walk and not one per path
        if cut_short {
            let anim = &self.anim;
            self.animating.retain(|&pi| anim[pi as usize].dur > 0.0);
        }
        // a change of rules or state is the one time a cut path can start to differ from its
        // pieces; the clock only brings them back together, which `tick` checks
        self.update_swaps(None);
    }

    /// Start a path's colour moving to `to` over `ms` from where it is now, and list it as
    /// animating. A path whose transition is already running is listed once.
    fn start_transition(&mut self, pi: usize, to: Rgba, ms: f64, smooth: bool) {
        let a = &mut self.anim[pi];
        let running = a.dur > 0.0;
        a.from = a.cur;
        a.to = to;
        a.t0 = self.clock_ms;
        a.dur = ms;
        a.smooth = smooth;
        if !running {
            self.animating.push(pi as u32);
        }
    }

    /// Put a path at `color` now, with no transition. True when this cut a running
    /// transition short, in which case the path is still listed and the caller compacts.
    fn settle(&mut self, pi: usize, color: Rgba) -> bool {
        let a = &mut self.anim[pi];
        let running = a.dur > 0.0;
        a.from = color;
        a.to = color;
        a.cur = color;
        a.dur = 0.0;
        self.colors[pi] = color;
        running
    }

    /// Current (possibly mid-transition) colour of a path.
    pub fn color_of(&mut self, pi: u32) -> Rgba {
        self.refresh_targets();
        self.colors[pi as usize]
    }

    /// Full display list: current colour per path. Kept up to date by the clock and read
    /// here, never rebuilt.
    pub fn colors(&mut self) -> &[Rgba] {
        self.refresh_targets();
        &self.colors
    }

    /// Paths whose current colour differs from the default ink (overlay repaint).
    ///
    /// With a letter layer, a path the draw list cannot hold now is left out: a piece whose
    /// cut path is drawn whole, or a cut path drawn as its pieces. So a host that paints this
    /// list over its cached ink never paints a cut path and its pieces both. [`Page::colors`]
    /// still gives every path a colour, the pieces that are not drawn included.
    pub fn styled_paths(&mut self) -> Vec<(u32, Rgba)> {
        self.refresh_targets();
        let d = self.styles.default_ink;
        let drawn = |i: usize| self.letters.as_deref().is_none_or(|lt| lt.is_drawn(i as u32));
        self.colors.iter().enumerate().filter(|&(i, &c)| c != d && drawn(i)).map(|(i, &c)| (i as u32, c)).collect()
    }

    // ── convenience API (what apps call) ──

    pub fn style(&mut self, sel: Selector, paint: Paint) -> Handle {
        self.styles.add(LAYER_BASE, sel, paint)
    }
    pub fn style_in(&mut self, layer: i32, sel: Selector, paint: Paint) -> Handle {
        self.styles.add(layer, sel, paint)
    }
    /// Recolour the ink of everything a target resolves to.
    pub fn style_target(&mut self, layer: i32, target: &crate::Target, paint: Paint) -> Handle {
        let words = self.target_words(target);
        self.styles.add_many(layer, words.into_iter().map(Selector::Word), paint)
    }
    /// Recolour the letters a query finds, under one handle and one rule.
    ///
    /// The scope resolves to words now. The rest of the query is kept and resolved per path
    /// each time the rules are evaluated, so the rule takes the letters of a layer attached
    /// later, and of a second attach too. A handle is returned with no layer: the rule then
    /// selects nothing until a layer is attached. `part` says which members of each letter
    /// found take the paint.
    pub fn style_letters(&mut self, layer: i32, q: &LetterQuery, paint: Paint) -> Handle {
        // sorted and with no repeat: the rule looks a word up by binary search
        let words = self.target_words(&q.scope);
        let sel = LetterSelection { words, text: q.text, mark: q.mark, part: q.part };
        self.styles.add(layer, Selector::Letters(sel), paint)
    }
    pub fn remove_style(&mut self, handle: Handle) -> usize {
        self.styles.remove(handle)
    }
    pub fn recolor_style(&mut self, handle: Handle, paint: Paint) -> usize {
        self.styles.repaint(handle, paint)
    }
    pub fn hide(&mut self, sel: Selector) -> Handle {
        self.styles.add(LAYER_TOP, sel, Paint::HIDDEN)
    }
    pub fn set_default_ink(&mut self, c: Rgba) {
        self.styles.set_default_ink(c);
    }
    pub fn default_ink(&self) -> Rgba {
        self.styles.default_ink
    }

    /// Theme in one call: paper is returned for the host; the rest become rules
    /// under one handle. `marks` are (mark, colour) pairs.
    pub fn theme(&mut self, t: &Theme) -> Handle {
        let h = self.styles.new_handle();
        let ms = t.transition_ms;
        if let Some(c) = t.ink {
            self.styles.push_under(h, LAYER_THEME, Selector::Page, Paint::fade(c, ms));
        }
        if let Some(c) = t.diacritics {
            self.styles.push_under(h, LAYER_THEME, Selector::Category(Category::Harakah), Paint::fade(c, ms));
            self.styles.push_under(h, LAYER_THEME, Selector::Category(Category::Tanwin), Paint::fade(c, ms));
            self.styles.push_under(h, LAYER_THEME, Selector::Mark(Mark::Maddah), Paint::fade(c, ms));
        }
        if let Some(c) = t.dots {
            self.styles.push_under(h, LAYER_THEME, Selector::Category(Category::LetterDot), Paint::fade(c, ms));
        }
        if let Some(c) = t.waqf {
            self.styles.push_under(h, LAYER_THEME, Selector::Category(Category::Waqf), Paint::fade(c, ms));
        }
        if let Some(c) = t.sifr {
            self.styles.push_under(h, LAYER_THEME, Selector::Family(Family::Sifr), Paint::fade(c, ms));
        }
        if let Some(c) = t.ayah_mark {
            self.styles.push_under(h, LAYER_THEME, Selector::Kind(PathKind::AyahMarkOrnament), Paint::fade(c, ms));
        }
        if let Some(c) = t.numeral {
            self.styles.push_under(h, LAYER_THEME, Selector::Kind(PathKind::AyahNumber), Paint::fade(c, ms));
        }
        if let Some(c) = t.headers {
            self.styles.push_under(h, LAYER_THEME, Selector::Kind(PathKind::HeaderInk), Paint::fade(c, ms));
        }
        for (m, c) in &t.marks {
            self.styles.push_under(h, LAYER_THEME, Selector::Mark(*m), Paint::fade(*c, ms));
        }
        h
    }
}

/// Theme colours; None leaves that colour alone.
#[derive(Clone, Debug, Default)]
pub struct Theme {
    pub ink: Option<Rgba>,
    pub diacritics: Option<Rgba>,
    pub dots: Option<Rgba>,
    pub waqf: Option<Rgba>,
    pub sifr: Option<Rgba>,
    pub ayah_mark: Option<Rgba>,
    pub numeral: Option<Rgba>,
    pub headers: Option<Rgba>,
    pub marks: Vec<(Mark, Rgba)>,
    pub transition_ms: u32,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_keeps_the_engine_layer_and_drops_the_host_ones() {
        let mut s = StyleEngine::new();
        let engine = s.add_engine(Selector::Kind(PathKind::AyahMarkOrnament), Paint::HIDDEN);
        let host = s.add(LAYER_TOP, Selector::Page, Paint::new(0xff0000ff));
        s.clear();
        assert_eq!(s.handles(), vec![engine]);
        s.add(LAYER_TOP, Selector::Page, Paint::new(0xff0000ff));
        s.clear_layer(LAYER_ENGINE);
        assert_eq!(s.len(), 2, "clearing the engine layer does nothing");
        assert_ne!(host, engine);
    }

    #[test]
    fn a_host_cannot_remove_or_repaint_an_engine_rule() {
        let mut s = StyleEngine::new();
        let engine = s.add_engine(Selector::Kind(PathKind::AyahMarkOrnament), Paint::HIDDEN);
        assert_eq!(s.remove(engine), 0);
        assert_eq!(s.repaint(engine, Paint::new(0xff0000ff)), 0);
        assert_eq!(s.rules.len(), 1);
        assert_eq!(s.rules[0].paint, Paint::HIDDEN);
        assert_eq!(s.remove_engine(engine), 1);
        assert!(s.is_empty());
    }

    #[test]
    fn a_host_cannot_add_on_the_engine_layer() {
        let mut s = StyleEngine::new();
        s.add(LAYER_ENGINE, Selector::Page, Paint::HIDDEN);
        s.add_many(i32::MAX, [Selector::Page], Paint::HIDDEN);
        assert!(s.rules.iter().all(|r| r.layer == LAYER_ENGINE - 1));
        s.clear();
        assert!(s.is_empty());
    }
}
