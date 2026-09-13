//! Style engine: layered rules with handles that undo exactly, sub-word
//! selectors, and per-path colour transitions driven by the engine clock.
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
            Selector::WordMark(..) | Selector::WordMarkNamed(..) | Selector::WordPath(..) => 10,
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
    /// Add one rule. Returns its handle.
    pub fn add(&mut self, layer: i32, sel: Selector, paint: Paint) -> Handle {
        let h = self.alloc();
        self.push(h, layer, sel, paint);
        h
    }
    /// Add several rules under one handle.
    pub fn add_many(&mut self, layer: i32, sels: impl IntoIterator<Item = Selector>, paint: Paint) -> Handle {
        let h = self.alloc();
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
    pub(crate) fn push_under(&mut self, handle: Handle, layer: i32, sel: Selector, paint: Paint) {
        self.push(handle, layer, sel, paint);
    }
    pub(crate) fn new_handle(&mut self) -> Handle {
        self.alloc()
    }
    /// Remove everything added under a handle. Returns the number of rules removed.
    pub fn remove(&mut self, handle: Handle) -> usize {
        let n = self.rules.len();
        self.rules.retain(|r| r.handle != handle);
        let removed = n - self.rules.len();
        if removed > 0 {
            self.dirty = true;
        }
        removed
    }
    /// Change the paint of an existing handle in place (keeps selectors and layer).
    pub fn repaint(&mut self, handle: Handle, paint: Paint) -> usize {
        let mut n = 0;
        for r in self.rules.iter_mut().filter(|r| r.handle == handle) {
            r.paint = paint;
            n += 1;
        }
        if n > 0 {
            self.dirty = true;
        }
        n
    }
    pub fn clear(&mut self) {
        if !self.rules.is_empty() {
            self.rules.clear();
            self.dirty = true;
        }
    }
    pub fn clear_layer(&mut self, layer: i32) {
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
    pub deco: u32,
    pub line_no: u8,
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
}

impl Rule {
    fn matches(&self, c: &PathCtx, pi: u32) -> bool {
        match &self.sel {
            Selector::Page => true,
            Selector::Path(p) => *p == pi,
            Selector::WordPath(w, n) => c.word == *w && c.nth_in_word == *n,
            Selector::WordMark(w, n) => c.word == *w && c.kind == PathKind::Mark && c.nth_mark == *n,
            Selector::WordMarkNamed(w, m, n) => c.word == *w && c.mark == *m && c.nth_mark_named == *n,
            Selector::WordBody(w) => c.word == *w && c.kind == PathKind::Body,
            Selector::WordMarks(w) => c.word == *w && c.kind == PathKind::Mark,
            Selector::Word(w) => c.word == *w,
            Selector::Ayah(s, a) => c.surah == *s && c.ayah == *a && c.ayah != 0,
            Selector::Line(l) => c.line_no == *l,
            Selector::Mark(m) => c.mark == *m,
            Selector::Category(k) => c.category == *k && c.kind == PathKind::Mark,
            Selector::Family(f) => c.family == *f,
            Selector::Kind(k) => c.kind == *k,
            Selector::Deco(k) => c.deco != NONE && c.deco_kind == *k,
            Selector::DecoIdx(d) => c.deco == *d,
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
        }
    }
}

pub(crate) fn ease_out(t: f64) -> f64 {
    let u = 1.0 - t.clamp(0.0, 1.0);
    1.0 - u * u * u
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
        let mut best: Option<(&Rule, u8)> = None;
        for r in &self.styles.rules {
            if !r.matches(c, pi) {
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
            return Paint::HIDDEN;
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
    pub fn tick(&mut self, now_ms: f64) -> bool {
        self.clock_ms = now_ms;
        self.refresh_targets();
        let mut moving = false;
        for a in self.anim.iter_mut() {
            if a.dur > 0.0 {
                let t = (self.clock_ms - a.t0) / a.dur;
                if t >= 1.0 {
                    a.cur = a.to;
                    a.dur = 0.0;
                } else {
                    a.cur = lerp_rgba(a.from, a.to, ease_out(t));
                    moving = true;
                }
            }
        }
        moving | self.highlights_moving()
    }

    /// Recompute targets when rules changed; start transitions where targets moved.
    pub(crate) fn refresh_targets(&mut self) {
        if !self.styles.dirty && !self.state_dirty {
            return;
        }
        self.styles.dirty = false;
        self.state_dirty = false;
        for pi in 0..self.anim.len() {
            let p = self.target_paint(pi as u32);
            let a = &mut self.anim[pi];
            if p.color != a.target {
                let prev_ms = a.target_ms;
                a.target = p.color;
                a.target_ms = p.transition_ms;
                let ms = p.transition_ms.max(prev_ms) as f64;
                if ms > 0.0 {
                    a.from = a.cur;
                    a.to = p.color;
                    a.t0 = self.clock_ms;
                    a.dur = ms;
                } else {
                    a.from = p.color;
                    a.to = p.color;
                    a.cur = p.color;
                    a.dur = 0.0;
                }
            } else if p.transition_ms != a.target_ms {
                a.target_ms = p.transition_ms;
            }
        }
    }

    /// Current (possibly mid-transition) colour of a path.
    pub fn color_of(&mut self, pi: u32) -> Rgba {
        self.refresh_targets();
        self.anim[pi as usize].cur
    }

    /// Full display list: current colour per path.
    pub fn colors(&mut self) -> &[Rgba] {
        self.refresh_targets();
        for (i, a) in self.anim.iter().enumerate() {
            self.colors[i] = a.cur;
        }
        &self.colors
    }

    /// Paths whose current colour differs from the default ink (overlay repaint).
    pub fn styled_paths(&mut self) -> Vec<(u32, Rgba)> {
        self.refresh_targets();
        let d = self.styles.default_ink;
        self.anim.iter().enumerate().filter(|(_, a)| a.cur != d).map(|(i, a)| (i as u32, a.cur)).collect()
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
