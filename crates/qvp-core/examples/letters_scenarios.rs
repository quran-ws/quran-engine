//! Write the cross-wrapper letter scenarios: small synthetic pages, their letter layers and
//! the engine's answers to every letter call. Every wrapper's test suite loads the pages and
//! the layers from the hex files, replays the calls and compares each answer.
//!
//! The pages and the layers come from `crates/qvp-core/src/fixtures.rs`, the code the core's
//! unit tests use, so the scenario and the tests describe the same pages. They hold squares,
//! no artwork. The generator reads no page data.
//!
//! The generator runs the core's Rust API. Where the C ABI does more than the Rust call it
//! wraps, the `c_*` helpers below do what the C function does, and say so: the selector and
//! query codes, the hit options, and the draw-list calls that apply pending rules on a page
//! with a layer. Every expected value is the engine's answer, never a value worked out here.
//! A few values that the fixture's geometry fixes are asserted too, so a broken engine cannot
//! write a wrong file.
//!
//! Run: cargo run -p qvp-core --release --example letters_scenarios -- conformance
//! It writes conformance/scenarios/letters.json and the hex files it names.
use qvp_core::*;
use std::collections::BTreeSet;
use std::sync::Mutex;

#[allow(dead_code)]
#[path = "../src/fixtures.rs"]
mod fixtures;
use fixtures::*;
use qvp_format::letters::LetterData;
use qvp_format::{Mark, PageData};

const RED: Rgba = 0xd02020ff;
const BLUE: Rgba = 0x2050d0ff;
const QVP_NONE: u32 = u32::MAX;

/// The words sidecar of the letter fixture, as a host passes it to `qvp_attach_words`. Word
/// 1:1:2 gets a shorter `qpc` text, so its ha (offset 2) and its waqf sign (offset 3) are past
/// the text. Word 1:2:1 gets a longer one, so offset 1 is a character that no path draws.
const WORDS: &str = r#"{"1:1:2": {"qpc": "لـ"}, "1:2:1": {"qpc": "وا"}}"#;

/// Bytes that are no letter layer.
const GARBAGE: [u8; 12] = [0x51, 0x56, 0x4c, 0x31, 0xde, 0xad, 0xbe, 0xef, 0, 1, 2, 3];

/// The bytes of a letter layer, as a wrapper attaches them. This is the only call of the
/// encoder in this file. The layer is encoded against `page`, the data its page file is
/// written from.
fn layer_bytes(page: &PageData, layer: &LetterData) -> Vec<u8> {
    qvp_format::letters::encode_letters(layer, page).expect("the scenario's layer encodes")
}

// ── the symbols the scenario exercises ──

/// Every C symbol the letter scenario must exercise. The generator fails when one is missing.
const SYMBOLS: [&str; 19] = [
    "qvp_attach_letters",
    "qvp_has_letters",
    "qvp_letter_count",
    "qvp_letter_info",
    "qvp_letter_members",
    "qvp_path_letter",
    "qvp_letter_at_offset",
    "qvp_find_letters",
    "qvp_hit_test_letter",
    "qvp_hit_test_letter_view",
    "qvp_layout_draw_revision",
    "qvp_style_add_letters",
    "QVP_SELECTOR_LETTER",
    "QVP_SELECTOR_LETTER_MARK",
    "QVP_SELECTOR_LETTER_MARK_NAMED",
    "QVP_SELECTOR_WORD_CHARS",
    "QVP_LETTER_PART_ALL",
    "QVP_LETTER_PART_BODY_AND_DOTS",
    "QVP_LETTER_PART_MARKS",
];

/// (top-level key, symbol) pairs: which key exercises which symbol.
static COVER: Mutex<BTreeSet<(&str, &str)>> = Mutex::new(BTreeSet::new());
static KEY: Mutex<&str> = Mutex::new("");

fn cover(symbol: &'static str) {
    let key = *KEY.lock().unwrap();
    COVER.lock().unwrap().insert((key, symbol));
}

fn in_key(key: &'static str) {
    *KEY.lock().unwrap() = key;
}

// ── JSON, written by hand as the other generators do ──

enum J {
    Raw(String),
    Str(String),
    Arr(Vec<J>),
    /// an array written one item a line
    Lines(Vec<J>),
    Obj(Vec<(&'static str, J)>),
}

impl J {
    fn write(&self, indent: usize, out: &mut String) {
        match self {
            J::Raw(s) => out.push_str(s),
            J::Str(s) => {
                out.push('"');
                for c in s.chars() {
                    match c {
                        '"' => out.push_str("\\\""),
                        '\\' => out.push_str("\\\\"),
                        c => out.push(c),
                    }
                }
                out.push('"');
            }
            J::Arr(v) => {
                out.push('[');
                for (i, x) in v.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    x.write(indent, out);
                }
                out.push(']');
            }
            J::Lines(v) => {
                if v.is_empty() {
                    out.push_str("[]");
                    return;
                }
                out.push_str("[\n");
                for (i, x) in v.iter().enumerate() {
                    out.push_str(&" ".repeat(indent + 2));
                    x.write(indent + 2, out);
                    out.push_str(if i + 1 < v.len() { ",\n" } else { "\n" });
                }
                out.push_str(&" ".repeat(indent));
                out.push(']');
            }
            J::Obj(v) => {
                out.push('{');
                for (i, (k, x)) in v.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    out.push_str(&format!("\"{k}\": "));
                    x.write(indent, out);
                }
                out.push('}');
            }
        }
    }
}

fn num(v: impl std::fmt::Display) -> J {
    J::Raw(v.to_string())
}
/// An index, with the wrappers' -1 for `QVP_NONE`.
fn idx(v: u32) -> J {
    if v == QVP_NONE {
        J::Raw("-1".into())
    } else {
        num(v)
    }
}
fn boolean(v: bool) -> J {
    J::Raw(v.to_string())
}
fn null() -> J {
    J::Raw("null".into())
}
fn s(v: &str) -> J {
    J::Str(v.into())
}
fn nums(v: &[u32]) -> J {
    J::Arr(v.iter().map(|&x| num(x)).collect())
}
fn pairs(v: &[(u32, u32)]) -> J {
    J::Arr(v.iter().map(|&(a, b)| J::Arr(vec![num(a), num(b)])).collect())
}

// ── the C ABI's inputs, as the scenario writes them ──

/// A `QvpSelector`.
#[derive(Clone, Copy)]
struct Sel {
    kind: u8,
    a: u32,
    b: u32,
    c: u32,
}

const SEL_PATH: u8 = 1;
const SEL_WORD: u8 = 7;
const SEL_LETTER: u8 = 16;
const SEL_LETTER_MARK: u8 = 17;
const SEL_LETTER_MARK_NAMED: u8 = 18;
const SEL_WORD_CHARS: u8 = 19;

fn sel(kind: u8, a: u32, b: u32, c: u32) -> Sel {
    Sel { kind, a, b, c }
}

impl Sel {
    fn json(self) -> J {
        J::Obj(vec![("selector", num(self.kind)), ("a", num(self.a)), ("b", num(self.b)), ("c", num(self.c))])
    }

    /// What `qvp_style_add` makes of the selector (`convert::selector` in the FFI crate), for
    /// the codes this scenario uses. `None` is a bad selector: the handle is 0.
    fn c_selector(self) -> Option<Selector> {
        let clamp16 = |v: u32| v.min(u16::MAX as u32) as u16;
        let part = |c: u32| {
            let p = LetterPart::from_u8(u8::try_from(c).ok()?)?;
            cover(match p {
                LetterPart::All => "QVP_LETTER_PART_ALL",
                LetterPart::BodyAndDots => "QVP_LETTER_PART_BODY_AND_DOTS",
                LetterPart::Marks => "QVP_LETTER_PART_MARKS",
            });
            Some(p)
        };
        Some(match self.kind {
            SEL_PATH => Selector::Path(self.a),
            SEL_WORD => Selector::Word(self.a),
            SEL_LETTER => {
                cover("QVP_SELECTOR_LETTER");
                Selector::Letter(self.a, clamp16(self.b), part(self.c)?)
            }
            SEL_LETTER_MARK => {
                cover("QVP_SELECTOR_LETTER_MARK");
                Selector::LetterMark(self.a, clamp16(self.b), clamp16(self.c))
            }
            SEL_LETTER_MARK_NAMED => {
                cover("QVP_SELECTOR_LETTER_MARK_NAMED");
                Selector::LetterMarkNamed(
                    self.a,
                    clamp16(self.b),
                    u8::try_from(self.c).map_or(Mark::None, Mark::from_u8),
                )
            }
            SEL_WORD_CHARS => {
                cover("QVP_SELECTOR_WORD_CHARS");
                if self.b > self.c {
                    return None;
                }
                Selector::WordChars(self.a, self.b.min(0xFFFE) as u16, self.c.min(0xFFFE) as u16)
            }
            k => panic!("selector {k} is not used by this scenario"),
        })
    }
}

/// A `QvpTarget`: `target` is `QVP_TARGET_*`.
#[derive(Clone)]
struct Scope {
    target: u8,
    a: u32,
    b: u32,
    c: u32,
    words: Vec<u32>,
}

fn page_scope() -> Scope {
    Scope { target: 0, a: 0, b: 0, c: 0, words: vec![] }
}
fn word_scope(w: u32) -> Scope {
    Scope { target: 1, a: w, b: 0, c: 0, words: vec![] }
}
fn words_scope(words: &[u32]) -> Scope {
    Scope { target: 2, a: 0, b: 0, c: 0, words: words.to_vec() }
}
fn ayah_scope(surah: u32, ayah: u32) -> Scope {
    Scope { target: 3, a: surah, b: ayah, c: 0, words: vec![] }
}
fn ayah_range_scope(surah: u32, from: u32, to: u32) -> Scope {
    Scope { target: 4, a: surah, b: from, c: to, words: vec![] }
}
fn line_scope(line_number: u32) -> Scope {
    Scope { target: 5, a: line_number, b: 0, c: 0, words: vec![] }
}

impl Scope {
    fn json(&self) -> J {
        J::Obj(vec![
            ("target", num(self.target)),
            ("a", num(self.a)),
            ("b", num(self.b)),
            ("c", num(self.c)),
            ("words", nums(&self.words)),
        ])
    }
    /// `convert::target` in the FFI crate.
    fn c_target(&self) -> Target {
        match self.target {
            1 => Target::Word(self.a),
            2 => Target::Words(self.words.clone()),
            3 => Target::Ayah(self.a as u16, self.b as u16),
            4 => Target::AyahRange(self.a as u16, self.b as u16, self.c as u16),
            5 => Target::Line(self.a as u8),
            _ => Target::Page,
        }
    }
}

/// A `QvpLetterQuery`.
#[derive(Clone)]
struct Query {
    scope: Scope,
    text: u32,
    mark: u8,
    part: u8,
}

fn query(scope: Scope, text: u32, mark: u8, part: u8) -> Query {
    Query { scope, text, mark, part }
}

impl Query {
    fn json(&self) -> J {
        J::Obj(vec![
            ("scope", self.scope.json()),
            ("text", num(self.text)),
            ("mark", num(self.mark)),
            ("part", num(self.part)),
        ])
    }
    /// `convert::letter_query` in the FFI crate. `None` is a bad query.
    fn c_query(&self) -> Option<LetterQuery> {
        let part = LetterPart::from_u8(self.part)?;
        cover(match part {
            LetterPart::All => "QVP_LETTER_PART_ALL",
            LetterPart::BodyAndDots => "QVP_LETTER_PART_BODY_AND_DOTS",
            LetterPart::Marks => "QVP_LETTER_PART_MARKS",
        });
        Some(LetterQuery {
            scope: self.scope.c_target(),
            text: match self.text {
                0 => None,
                t => Some(char::from_u32(t)?),
            },
            mark: Mark::from_u8(self.mark),
            part,
        })
    }
}

/// A `QvpHitOptions`, as its raw fields. `None` is a NULL pointer: the defaults.
#[derive(Clone, Copy)]
struct Opt {
    max_distance: f32,
    gap_bias: f32,
    prefer_exact: bool,
}

fn opt_json(o: Option<Opt>) -> J {
    match o {
        None => null(),
        Some(o) => J::Obj(vec![
            ("maxDistance", num(o.max_distance)),
            ("gapBias", num(o.gap_bias)),
            ("preferExact", boolean(o.prefer_exact)),
        ]),
    }
}

/// `hopt` in the FFI crate: max_distance <= 0 is unlimited, gap_bias < 0 the default.
fn c_options(o: Option<Opt>) -> HitOptions {
    let Some(o) = o else { return HitOptions::default() };
    HitOptions {
        max_distance: if o.max_distance <= 0.0 { f32::INFINITY } else { o.max_distance },
        gap_bias: if o.gap_bias < 0.0 { HitOptions::default().gap_bias } else { o.gap_bias },
        prefer_exact: o.prefer_exact,
    }
}

// ── the fixtures ──

/// One page and one letter layer, as the hex files hold them.
struct Fixture {
    name: &'static str,
    page_file: &'static str,
    layer_file: &'static str,
    page: Vec<u8>,
    layer: Vec<u8>,
}

fn fixtures() -> Vec<Fixture> {
    let letters = qvp_format::encode(&letters_page_data());
    // the seam and the body-mark pages are the letter page, settled as the core's tests load
    // them: the same bytes, so they share its file
    assert_eq!(qvp_format::encode(&settled(letters_page_data())), letters, "a settled letter page is the same page");
    let make = |name, page_file, layer_file, data: PageData, layer: LetterData| Fixture {
        name,
        page_file,
        layer_file,
        layer: layer_bytes(&data, &layer),
        page: qvp_format::encode(&data),
    };
    let (overlap, overlap_layer) = overlap_fixture();
    let (counter, counter_layer) = counter_under_a_box_fixture();
    let (decoration, decoration_layer) = letters_decoration_fixture();
    vec![
        make("letters", "qvp1-letters.hex", "qvl1-letters.hex", letters_page_data(), letters_layer()),
        make("seam", "qvp1-letters.hex", "qvl1-letters-seam.hex", letters_page_data(), seam_layer()),
        make("bodyMark", "qvp1-letters.hex", "qvl1-letters-body-mark.hex", letters_page_data(), body_mark_layer()),
        make(
            "variant",
            "qvp1-letters-variant.hex",
            "qvl1-letters-variant.hex",
            letters_variant_page_data(),
            letters_variant_layer(),
        ),
        make("overlap", "qvp1-letters-overlap.hex", "qvl1-letters-overlap.hex", overlap, overlap_layer),
        make("counter", "qvp1-letters-counter.hex", "qvl1-letters-counter.hex", settled(counter), counter_layer),
        make("decoration", "qvp1-letters-decoration.hex", "qvl1-letters-decoration.hex", decoration, decoration_layer),
    ]
}

fn fixture<'a>(all: &'a [Fixture], name: &str) -> &'a Fixture {
    all.iter().find(|f| f.name == name).unwrap_or_else(|| panic!("no fixture {name}"))
}

fn hex(bytes: &[u8]) -> String {
    let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    hex.as_bytes().chunks(64).map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join("\n") + "\n"
}

/// The printed layout of the scenarios, and the reflowed one.
fn spec(name: &str) -> LayoutSpec {
    let printed = LayoutSpec { viewport_w: 200.0, viewport_h: 200.0, ..Default::default() };
    match name {
        "printed" => printed,
        "reflow" => LayoutSpec { reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }), ..printed },
        _ => panic!("no layout {name}"),
    }
}

fn spec_json(name: &str) -> J {
    let s = spec(name);
    let mut v = vec![
        ("viewportW", num(s.viewport_w)),
        ("viewportH", num(s.viewport_h)),
        ("padTop", num(s.pad_top)),
        ("padBottom", num(s.pad_bottom)),
        ("padLeft", num(s.pad_left)),
        ("padRight", num(s.pad_right)),
    ];
    if let Some(r) = s.reflow {
        v.push(("reflow", J::Obj(vec![("zoom", num(r.zoom))])));
    }
    J::Obj(v)
}

// ── a page as a wrapper drives it ──

/// A page and the handles a sequence names.
struct Host<'a> {
    all: &'a [Fixture],
    p: Page,
    handles: Vec<(&'static str, u32)>,
}

impl<'a> Host<'a> {
    fn load(all: &'a [Fixture], name: &str) -> Host<'a> {
        let p = Page::load(&fixture(all, name).page).expect("load the fixture page");
        Host { all, p, handles: vec![] }
    }

    fn attach(&mut self, name: &str) -> i32 {
        cover("qvp_attach_letters");
        match self.p.attach_letters(&fixture(self.all, name).layer) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }

    fn attach_bytes(&mut self, bytes: &[u8]) -> i32 {
        cover("qvp_attach_letters");
        match self.p.attach_letters(bytes) {
            Ok(()) => 0,
            Err(_) => -1,
        }
    }

    fn attach_words(&mut self) -> i64 {
        self.p.attach_words_json(WORDS.as_bytes()).map_or(-1, |n| n as i64)
    }

    /// A fixture page with its layer, the sidecar when `words`, laid out, at time 0.
    fn ready(all: &'a [Fixture], name: &str, words: bool, layout: Option<&str>) -> Host<'a> {
        let mut h = Host::load(all, name);
        assert_eq!(h.attach(name), 0, "the {name} layer attaches to its page");
        if words {
            assert!(h.attach_words() > 0);
        }
        if let Some(l) = layout {
            h.p.layout(&spec(l));
        }
        h.p.tick(0.0);
        h
    }

    /// The path count that `qvp_page_info`, `qvp_geometry` and the length of `qvp_colors` give.
    fn n_paths(&self) -> u32 {
        self.p.geometry().table.len() as u32
    }

    fn rows(&self, first: u32, count: u32) -> J {
        let t = &self.p.geometry().table;
        J::Arr(
            (first..first + count)
                .map(|i| {
                    let g = t[i as usize];
                    nums(&[g.op_start, g.op_count, g.pt_start, g.pt_count, g.flags, g.word, g.line, g.extra])
                })
                .collect(),
        )
    }

    /// `qvp_layout_draw_list` over the whole page. With a layer, the C call applies the pending
    /// rule changes first; with none, it reads the state of the last refresh.
    fn c_draws(&mut self) -> Vec<(u32, u32)> {
        if self.p.has_letters() {
            self.p.refresh_display();
        }
        self.p.layout_draw_list().iter().map(|d| (d.path, d.placement)).collect()
    }

    /// `qvp_layout_draw_revision`, with the same refresh rule as the draw list.
    fn c_revision(&mut self) -> u32 {
        cover("qvp_layout_draw_revision");
        if self.p.has_letters() {
            self.p.refresh_display();
        }
        self.p.draw_revision()
    }

    /// The paths whose colour is `rgba`, from `qvp_colors`.
    fn painted(&mut self, rgba: Rgba) -> Vec<u32> {
        self.p.colors().iter().enumerate().filter(|&(_, &c)| c == rgba).map(|(i, _)| i as u32).collect()
    }

    fn style(&mut self, s: Sel, rgba: Rgba, ms: u32) -> u32 {
        match s.c_selector() {
            Some(sel) => self.p.style_in(0, sel, Paint::fade(rgba, ms)),
            None => 0,
        }
    }

    fn style_letters(&mut self, q: &Query, rgba: Rgba, ms: u32) -> u32 {
        cover("qvp_style_add_letters");
        match q.c_query() {
            Some(lq) => self.p.style_letters(0, &lq, Paint::fade(rgba, ms)),
            None => 0,
        }
    }

    fn find(&mut self, q: &Query) -> Vec<(u32, u32)> {
        cover("qvp_find_letters");
        match q.c_query() {
            Some(lq) => self.p.find_letters(&lq),
            None => vec![],
        }
    }

    fn info(&self, word: u32, letter: u32) -> J {
        cover("qvp_letter_info");
        match self.p.letter_info(word, letter) {
            None => null(),
            Some(i) => J::Obj(vec![
                ("word", num(i.word)),
                ("index", num(i.index)),
                ("at", idx(i.at)),
                ("text", num(i.text as u32)),
                ("char", s(&i.text.to_string())),
                ("nMembers", num(i.n_members)),
                ("nMarks", num(i.n_marks)),
                ("moreAt", idx(i.more_at.first().copied().unwrap_or(QVP_NONE))),
                ("nMoreAt", num(i.more_at.len())),
                ("x0", num(i.x0)),
                ("y0", num(i.y0)),
                ("x1", num(i.x1)),
                ("y1", num(i.y1)),
                ("hasInk", boolean(i.has_ink)),
            ]),
        }
    }

    fn members(&self, word: u32, letter: u32) -> J {
        cover("qvp_letter_members");
        J::Arr(
            self.p
                .letter_members(word, letter)
                .iter()
                .map(|m| {
                    J::Obj(vec![
                        ("path", num(m.path)),
                        ("at", idx(m.at)),
                        ("mark", s(m.mark.as_str())),
                        ("isBody", boolean(m.is_body)),
                        ("isCut", boolean(m.is_cut)),
                    ])
                })
                .collect(),
        )
    }

    fn path_letter(&self, path: u32) -> J {
        cover("qvp_path_letter");
        match self.p.path_letter(path) {
            None => null(),
            Some((l, at)) => J::Obj(vec![("letter", idx(l)), ("at", idx(at))]),
        }
    }

    fn letter_at_offset(&self, word: u32, at: u32) -> J {
        cover("qvp_letter_at_offset");
        self.p.letter_at_offset(word, at).map_or(null(), num)
    }

    fn hit(&self, view: bool, x: f32, y: f32, o: Option<Opt>) -> Option<LetterHit> {
        cover(if view { "qvp_hit_test_letter_view" } else { "qvp_hit_test_letter" });
        let o = c_options(o);
        if view {
            self.p.hit_test_letter_view(x, y, &o)
        } else {
            self.p.hit_test_letter(x, y, &o)
        }
    }

    fn handle(&self, name: &str) -> u32 {
        self.handles.iter().find(|h| h.0 == name).unwrap_or_else(|| panic!("no handle {name}")).1
    }

    /// A point of a path, in page units, as layout px under the current layout.
    fn view_point(&self, path: u32, x: f32, y: f32) -> (f32, f32) {
        placed_point(&self.p, self.p.current_layout().expect("a layout"), path, x, y)
    }
}

fn hit_json(h: Option<LetterHit>) -> J {
    match h {
        None => null(),
        Some(h) => J::Obj(vec![
            ("word", idx(h.word)),
            ("letter", idx(h.letter)),
            ("path", idx(h.path)),
            ("at", idx(h.at)),
            ("line", idx(h.line)),
            ("distance", num(h.distance)),
            ("mark", s(h.mark.as_str())),
            ("isExact", boolean(h.is_exact)),
            ("isBody", boolean(h.is_body)),
        ]),
    }
}

// ── "letters": what a layer reads back ──

fn letters_case(all: &[Fixture], name: &'static str, fx: &'static str, words: bool) -> J {
    let h = Host::ready(all, fx, words, None);
    cover("qvp_has_letters");
    let n_words = h.p.data().words.len() as u32;
    let mut counts = vec![];
    let mut letters = vec![];
    let mut missing = vec![];
    for w in 0..=n_words {
        cover("qvp_letter_count");
        let n = h.p.letter_count(w);
        counts.push(n);
        for l in 0..n {
            let J::Obj(mut info) = h.info(w, l) else { panic!("letter {w}:{l} has no info") };
            info.push(("members", h.members(w, l)));
            letters.push(J::Obj(info));
        }
        missing.push(J::Obj(vec![
            ("word", num(w)),
            ("letter", num(n)),
            ("info", h.info(w, n)),
            ("members", h.members(w, n)),
        ]));
    }
    let n = h.n_paths();
    let path_letters = (0..=n).map(|p| J::Obj(vec![("path", num(p)), ("value", h.path_letter(p))])).collect();
    let mut offsets = vec![];
    for w in 0..n_words {
        for at in [0, 1, 2, 3, 4, QVP_NONE] {
            offsets.push(J::Arr(vec![num(w), idx(at), h.letter_at_offset(w, at)]));
        }
    }
    J::Obj(vec![
        ("name", s(name)),
        ("fixture", s(fx)),
        ("words", boolean(words)),
        ("hasLetters", boolean(h.p.has_letters())),
        ("nPaths", num(n)),
        ("counts", nums(&counts)),
        ("letters", J::Lines(letters)),
        ("missing", J::Arr(missing)),
        ("pathLetters", J::Arr(path_letters)),
        ("offsets", J::Arr(offsets)),
    ])
}

// ── "selections": one selector on a fresh page ──

struct Selection {
    name: &'static str,
    fixture: &'static str,
    words: bool,
    sel: Sel,
}

fn selection_case(all: &[Fixture], c: &Selection) -> (J, Vec<u32>, Vec<(u32, u32)>) {
    let mut h = Host::ready(all, c.fixture, c.words, Some("printed"));
    let handle = h.style(c.sel, RED, 0);
    h.p.tick(0.0);
    let red = h.painted(RED);
    let revision = h.c_revision();
    let draws = h.c_draws();
    let j = J::Obj(vec![
        ("name", s(c.name)),
        ("fixture", s(c.fixture)),
        ("words", boolean(c.words)),
        ("selector", c.sel.json()),
        ("handle", boolean(handle != 0)),
        ("red", nums(&red)),
        ("drawRevision", num(revision)),
        ("drawList", pairs(&draws)),
    ]);
    (j, red, draws)
}

// ── "queries": one letter query on a fresh page ──

struct QueryCase {
    name: &'static str,
    fixture: &'static str,
    q: Query,
}

fn query_case(all: &[Fixture], c: &QueryCase) -> (J, Vec<(u32, u32)>, Vec<u32>) {
    let mut h = Host::ready(all, c.fixture, false, Some("printed"));
    let found = h.find(&c.q);
    let handle = h.style_letters(&c.q, RED, 0);
    h.p.tick(0.0);
    let red = h.painted(RED);
    let draws = h.c_draws();
    let j = J::Obj(vec![
        ("name", s(c.name)),
        ("fixture", s(c.fixture)),
        ("query", c.q.json()),
        ("found", pairs(&found)),
        ("handle", boolean(handle != 0)),
        ("red", nums(&red)),
        ("drawList", pairs(&draws)),
    ]);
    (j, found, red)
}

// ── "hits": the letter under a point ──

struct HitCase {
    name: &'static str,
    fixture: &'static str,
    words: bool,
    layout: Option<&'static str>,
    /// page units, or, for a view hit, a point of `path` in page units to place under the layout
    x: f32,
    y: f32,
    view_of: Option<u32>,
    opt: Option<Opt>,
}

fn hit_case(all: &[Fixture], c: &HitCase) -> (J, Option<LetterHit>) {
    let h = Host::ready(all, c.fixture, c.words, c.layout);
    let (view, (x, y)) = match c.view_of {
        Some(path) => (true, h.view_point(path, c.x, c.y)),
        None => (false, (c.x, c.y)),
    };
    let hit = h.hit(view, x, y, c.opt);
    let layout = c.layout.map_or(null(), s);
    let j = J::Obj(vec![
        ("name", s(c.name)),
        ("fixture", s(c.fixture)),
        ("words", boolean(c.words)),
        ("layout", layout),
        ("units", s(if view { "view" } else { "page" })),
        ("x", num(x)),
        ("y", num(y)),
        ("options", opt_json(c.opt)),
        ("hit", hit_json(hit)),
    ]);
    (j, hit)
}

// ── "sequences": calls in order, each with the engine's answer ──

enum Step {
    Tick(f64),
    Attach(&'static str),
    AttachBytes(&'static [u8]),
    AttachWords,
    Layout(&'static str),
    Style(Sel, Rgba, u32, &'static str),
    StyleLetters(Query, Rgba, u32, &'static str),
    Remove(&'static str),
    Theme(Rgba, Rgba, u32),
    Highlight(u32, Rgba, u32, &'static str),
    RemoveHighlight(&'static str),
    MaskTransition(u32),
    Mask(u32),
    Unmask,
    HasLetters,
    NPaths,
    Rows(u32, u32),
    Colors,
    StyledPaths,
    DrawRevision,
    DrawList,
    LetterCount(u32),
    LetterInfo(u32, u32),
    LetterMembers(u32, u32),
    PathLetter(u32),
    LetterAtOffset(u32, u32),
    FindLetters(Query),
    /// page units
    HitTestLetter(f32, f32),
    /// a point of a path in page units, placed under the current layout
    HitTestLetterView(u32, f32, f32),
}

fn run_step(h: &mut Host<'_>, step: &Step) -> J {
    use Step::*;
    let call = |name: &str, mut rest: Vec<(&'static str, J)>| {
        let mut v = vec![("call", s(name))];
        v.append(&mut rest);
        J::Obj(v)
    };
    match step {
        Tick(ms) => {
            let animating = h.p.tick(*ms);
            call("tick", vec![("ms", num(ms)), ("animating", boolean(animating))])
        }
        Attach(name) => {
            let r = h.attach(name);
            call("attach", vec![("letters", s(name)), ("result", num(r))])
        }
        AttachBytes(bytes) => {
            let r = h.attach_bytes(bytes);
            let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
            call("attachBytes", vec![("hex", s(&hex)), ("result", num(r))])
        }
        AttachWords => {
            let r = h.attach_words();
            call("attachWords", vec![("result", num(r))])
        }
        Layout(name) => {
            h.p.layout(&spec(name));
            call("layout", vec![("spec", s(name))])
        }
        Style(sel, rgba, ms, name) => {
            let handle = h.style(*sel, *rgba, *ms);
            h.handles.push((name, handle));
            call(
                "style",
                vec![
                    ("selector", sel.json()),
                    ("rgba", num(rgba)),
                    ("ms", num(ms)),
                    ("as", s(name)),
                    ("handle", boolean(handle != 0)),
                ],
            )
        }
        StyleLetters(q, rgba, ms, name) => {
            let handle = h.style_letters(q, *rgba, *ms);
            h.handles.push((name, handle));
            call(
                "styleLetters",
                vec![
                    ("query", q.json()),
                    ("rgba", num(rgba)),
                    ("ms", num(ms)),
                    ("as", s(name)),
                    ("handle", boolean(handle != 0)),
                ],
            )
        }
        Remove(name) => {
            let r = h.p.remove_style(h.handle(name));
            call("remove", vec![("handle", s(name)), ("result", num(r))])
        }
        Theme(ink, dots, ms) => {
            h.p.theme(&qvp_core::Theme {
                ink: Some(*ink),
                dots: Some(*dots),
                transition_ms: *ms,
                ..Default::default()
            });
            call("theme", vec![("ink", num(ink)), ("dots", num(dots)), ("ms", num(ms))])
        }
        Highlight(word, ink, ms, name) => {
            let style =
                HighlightStyle { mode: HighlightMode::Ink, ink: *ink, transition_ms: *ms, ..Default::default() };
            let handle = h.p.highlight(&Target::Word(*word), style);
            h.handles.push((name, handle));
            call("highlight", vec![("word", num(word)), ("ink", num(ink)), ("ms", num(ms)), ("as", s(name))])
        }
        RemoveHighlight(name) => {
            let r = h.p.remove_highlight(h.handle(name));
            call("removeHighlight", vec![("handle", s(name)), ("result", boolean(r))])
        }
        MaskTransition(ms) => {
            h.p.set_mask_transition(*ms);
            call("maskTransition", vec![("ms", num(ms))])
        }
        Mask(word) => {
            h.p.mask(&Target::Word(*word), MaskMode::Hide);
            call("mask", vec![("word", num(word))])
        }
        Unmask => {
            h.p.unmask();
            call("unmask", vec![])
        }
        HasLetters => {
            cover("qvp_has_letters");
            call("hasLetters", vec![("value", boolean(h.p.has_letters()))])
        }
        NPaths => call("nPaths", vec![("value", num(h.n_paths()))]),
        Rows(first, count) => {
            call("rows", vec![("first", num(first)), ("count", num(count)), ("value", h.rows(*first, *count))])
        }
        Colors => {
            let c = h.p.colors().to_vec();
            call("colors", vec![("value", nums(&c))])
        }
        StyledPaths => {
            let v = h.p.styled_paths();
            call("styledPaths", vec![("value", pairs(&v))])
        }
        DrawRevision => {
            let r = h.c_revision();
            call("drawRevision", vec![("value", num(r))])
        }
        DrawList => {
            let d = h.c_draws();
            call("drawList", vec![("value", pairs(&d))])
        }
        LetterCount(w) => {
            cover("qvp_letter_count");
            call("letterCount", vec![("word", num(w)), ("value", num(h.p.letter_count(*w)))])
        }
        LetterInfo(w, l) => call("letterInfo", vec![("word", num(w)), ("letter", num(l)), ("value", h.info(*w, *l))]),
        LetterMembers(w, l) => {
            call("letterMembers", vec![("word", num(w)), ("letter", num(l)), ("value", h.members(*w, *l))])
        }
        PathLetter(p) => call("pathLetter", vec![("path", num(p)), ("value", h.path_letter(*p))]),
        LetterAtOffset(w, at) => {
            call("letterAtOffset", vec![("word", num(w)), ("at", idx(*at)), ("value", h.letter_at_offset(*w, *at))])
        }
        FindLetters(q) => {
            let found = h.find(q);
            call("findLetters", vec![("query", q.json()), ("value", pairs(&found))])
        }
        HitTestLetter(x, y) => {
            let hit = h.hit(false, *x, *y, None);
            call("hitTestLetter", vec![("x", num(x)), ("y", num(y)), ("options", null()), ("value", hit_json(hit))])
        }
        HitTestLetterView(path, x, y) => {
            let (vx, vy) = h.view_point(*path, *x, *y);
            let hit = h.hit(true, vx, vy, None);
            call(
                "hitTestLetterView",
                vec![("x", num(vx)), ("y", num(vy)), ("options", null()), ("value", hit_json(hit))],
            )
        }
    }
}

fn sequence(all: &[Fixture], name: &'static str, fx: &'static str, steps: &[Step]) -> (J, Vec<J>) {
    let mut h = Host::load(all, fx);
    let done: Vec<J> = steps.iter().map(|st| run_step(&mut h, st)).collect();
    let copy: Vec<J> = done.iter().map(|j| J::Raw(render(j, 0))).collect();
    (J::Obj(vec![("name", s(name)), ("fixture", s(fx)), ("steps", J::Lines(done))]), copy)
}

fn render(j: &J, indent: usize) -> String {
    let mut out = String::new();
    j.write(indent, &mut out);
    out
}

/// The value a step recorded, as written.
fn value_of(steps: &[J], i: usize) -> String {
    let J::Raw(r) = &steps[i] else { unreachable!() };
    let at = r.find("\"value\": ").or_else(|| r.find("\"result\": ")).expect("a step with a value");
    let rest = &r[at..];
    rest[rest.find(": ").unwrap() + 2..rest.len() - 1].to_string()
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out_dir = std::path::PathBuf::from(args.get(1).map(String::as_str).unwrap_or("conformance"));
    let all = fixtures();
    let lam = 'ل' as u32;
    let (sel_l, sel_m, sel_n, sel_c) = (SEL_LETTER, SEL_LETTER_MARK, SEL_LETTER_MARK_NAMED, SEL_WORD_CHARS);

    // ── letters ──
    in_key("letters");
    let letters = vec![
        letters_case(&all, "the letter fixture", "letters", false),
        letters_case(
            &all,
            "the letter fixture with the words sidecar: offsets past the qpc text are none",
            "letters",
            true,
        ),
        letters_case(
            &all,
            "marks out of offset order, a sign at the letter's own offset, a mark with no offset",
            "variant",
            false,
        ),
        letters_case(&all, "a body the page stores as a mark", "bodyMark", false),
    ];
    {
        let h = Host::ready(&all, "letters", false, None);
        let i = h.p.letter_info(0, 0).unwrap();
        assert_eq!(
            (i.x0, i.y0, i.x1, i.y1),
            (50.0, 6.0, 60.0, 24.0),
            "the ba's piece, its fathah above and its dot below"
        );
        let i = h.p.letter_info(0, 1).unwrap();
        assert_eq!(
            (i.x0, i.y0, i.x1, i.y1),
            (39.0, 10.0, 50.0, 20.0),
            "the lam's piece reaches one unit past its word"
        );
        assert_eq!(h.n_paths(), 10, "8 page paths and 2 pieces");
        let h = Host::ready(&all, "variant", false, None);
        let order: Vec<u32> = h.p.letter_members(2, 0).iter().map(|m| m.path).collect();
        assert_eq!(order, [6, 9, 8, 7], "the body, then the marks in text order");
    }

    // ── selections ──
    in_key("selections");
    let none = QVP_NONE;
    let cases = [
        ("a letter, all of it: the ba's piece and its marks", "letters", false, sel(sel_l, 0, 0, 0)),
        ("a letter's body and dots: the members at its own offset", "letters", false, sel(sel_l, 0, 0, 1)),
        ("a letter's marks: the members at another offset", "letters", false, sel(sel_l, 0, 0, 2)),
        (
            "one letter of a cut path: its piece only, and the path draws as its pieces",
            "letters",
            false,
            sel(sel_l, 0, 1, 0),
        ),
        ("a mark with no offset is in the marks part", "letters", false, sel(sel_l, 2, 0, 2)),
        ("a part past 2 is a bad selector", "letters", false, sel(sel_l, 0, 0, 3)),
        ("a letter past its word selects nothing", "letters", false, sel(sel_l, 0, 5, 0)),
        ("a letter of a word past the page selects nothing", "letters", false, sel(sel_l, 9, 0, 0)),
        ("a sign at the letter's own offset is body and dots", "variant", false, sel(sel_l, 2, 0, 1)),
        ("the marks part: a mark at another offset and a mark with no offset", "variant", false, sel(sel_l, 2, 0, 2)),
        ("the first mark in text order: the hamzah sign at offset 0", "variant", false, sel(sel_m, 2, 0, 0)),
        ("the second mark in text order: the kasrah at offset 1", "variant", false, sel(sel_m, 2, 0, 1)),
        ("the third mark in text order: the mark with no offset, last", "variant", false, sel(sel_m, 2, 0, 2)),
        ("a mark past the last selects nothing", "variant", false, sel(sel_m, 2, 0, 3)),
        ("the ba's first mark: its dot", "letters", false, sel(sel_m, 0, 0, 0)),
        ("a named mark: dot (16)", "letters", false, sel(sel_n, 0, 0, 16)),
        ("a named mark: fathah (1)", "letters", false, sel(sel_n, 0, 0, 1)),
        ("a named mark the letter does not have: kasrah (2)", "letters", false, sel(sel_n, 0, 0, 2)),
        ("mark id 0 selects nothing", "letters", false, sel(sel_n, 0, 0, 0)),
        ("an unnamed mark id (36) acts as unknown (255)", "letters", false, sel(sel_n, 0, 0, 36)),
        ("an unnamed mark id (254) acts as unknown (255)", "letters", false, sel(sel_n, 0, 0, 254)),
        ("unknown (255): the fixture has no such mark", "letters", false, sel(sel_n, 0, 0, 255)),
        ("a named mark that is a mark member: omitted_alif (12)", "letters", false, sel(sel_n, 2, 0, 12)),
        ("a named mark that is the letter's body, stored as a mark", "bodyMark", false, sel(sel_n, 2, 0, 12)),
        ("a body stored as a mark is body and dots", "bodyMark", false, sel(sel_l, 2, 0, 1)),
        ("a letter's offset: the same paths as its body and dots", "letters", false, sel(sel_c, 0, 0, 0)),
        ("a mark's own offset: the fathah at 1", "letters", false, sel(sel_c, 0, 1, 1)),
        ("a more_at offset: the kashida at 1 is the lam's body", "letters", false, sel(sel_c, 1, 1, 1)),
        ("a word-level mark by its offset: the waqf sign at 3", "letters", false, sel(sel_c, 1, 3, 3)),
        ("a range of a word: the lam, the ha and the waqf sign", "letters", false, sel(sel_c, 1, 0, 3)),
        ("a mark with no offset is never in a range", "letters", false, sel(sel_c, 2, 0, 9)),
        ("every character of a cut path: the path is taken whole", "letters", false, sel(sel_c, 0, 0, 2)),
        ("from > to is a bad selector", "letters", false, sel(sel_c, 0, 2, 1)),
        ("offsets past the text select nothing", "letters", false, sel(sel_c, 1, 4, 9)),
        ("an offset past 0xfffe counts as 0xfffe", "letters", false, sel(sel_c, 0, 0, none)),
        ("the sidecar's text: offsets past it select nothing", "letters", true, sel(sel_c, 1, 2, 3)),
        ("the sidecar's text: a character no path draws selects nothing", "letters", true, sel(sel_c, 2, 1, 1)),
        ("the sidecar's text: an offset inside it still selects", "letters", true, sel(sel_c, 1, 1, 1)),
        ("the sidecar's text: a letter keeps its index", "letters", true, sel(sel_l, 1, 1, 0)),
        ("a letter's offset in the variant: the body and the hamzah sign", "variant", false, sel(sel_c, 2, 0, 0)),
        ("PATH on a cut path reaches its pieces", "letters", false, sel(SEL_PATH, 0, 0, 0)),
        ("WORD reaches the pieces, and the path draws whole", "letters", false, sel(SEL_WORD, 0, 0, 0)),
    ];
    let mut selections = vec![];
    for (name, fixture, words, sel) in cases {
        let (j, red, draws) = selection_case(&all, &Selection { name, fixture, words, sel });
        match (fixture, sel.kind, sel.a, sel.b, sel.c) {
            ("letters", SEL_LETTER, 0, 1, 0) => {
                assert_eq!(red, [9], "the lam is piece 9");
                assert!(draws.contains(&(8, 0)) && draws.contains(&(9, 0)) && !draws.iter().any(|d| d.0 == 0));
            }
            ("letters", SEL_WORD, 0, _, _) => assert_eq!(red, [0, 1, 2, 8, 9]),
            ("letters", SEL_PATH, 0, _, _) => assert_eq!(red, [0, 8, 9]),
            _ => {}
        }
        selections.push(j);
    }

    // ── queries ──
    in_key("queries");
    let alif_hamzah = 'ؤ' as u32;
    let cases = [
        ("every lam of the page", "letters", query(page_scope(), lam, 0, 0)),
        ("letters with a dot", "letters", query(page_scope(), 0, 16, 0)),
        ("text 0 and mark 0: every letter of a word", "letters", query(word_scope(1), 0, 0, 0)),
        ("a words scope with a repeat and a word past the page", "letters", query(words_scope(&[2, 0, 2, 9]), 0, 0, 0)),
        ("an ayah scope", "letters", query(ayah_scope(1, 2), 0, 0, 0)),
        ("an ayah range scope", "letters", query(ayah_range_scope(1, 1, 2), 0, 0, 0)),
        ("a line scope (line number 2)", "letters", query(line_scope(2), 0, 0, 0)),
        ("the body-and-dots part of every letter", "letters", query(page_scope(), 0, 0, 1)),
        ("the marks part of every letter", "letters", query(page_scope(), 0, 0, 2)),
        ("a text and a part: the marks of the waw with hamzah", "variant", query(page_scope(), alif_hamzah, 0, 2)),
        ("a mark no letter has", "letters", query(page_scope(), 0, 2, 0)),
        ("an unnamed mark id (36) acts as unknown (255)", "letters", query(page_scope(), 0, 36, 0)),
        ("a part past 2 is a bad query", "letters", query(page_scope(), 0, 0, 3)),
        ("a text that is no code point is a bad query", "letters", query(page_scope(), 0x110000, 0, 0)),
        ("a surrogate is a bad query", "letters", query(page_scope(), 0xD800, 0, 0)),
    ];
    let mut queries = vec![];
    for (name, fixture, q) in cases {
        let (j, found, red) = query_case(&all, &QueryCase { name, fixture, q: q.clone() });
        if fixture == "letters" && q.text == lam && q.part == 0 && q.scope.target == 0 {
            assert_eq!(found, [(0, 1), (1, 0)]);
            assert_eq!(red, [3, 9]);
        }
        queries.push(j);
    }

    // ── hits ──
    in_key("hits");
    let far = Some(Opt { max_distance: 1.0, gap_bias: -1.0, prefer_exact: true });
    let gap = Some(Opt { max_distance: 0.0, gap_bias: -1.0, prefer_exact: false });
    let page_hit =
        |name, fixture, words, x, y, opt| HitCase { name, fixture, words, layout: None, x, y, view_of: None, opt };
    let view_hit = |name, fixture, layout, path, x, y| HitCase {
        name,
        fixture,
        words: false,
        layout: Some(layout),
        x,
        y,
        view_of: Some(path),
        opt: None,
    };
    let cases = [
        page_hit("on a piece: the ba", "letters", false, 55.0, 15.0, None),
        page_hit("on a piece: the lam", "letters", false, 45.0, 15.0, None),
        page_hit("on the lam's piece past its word's box", "letters", false, 39.5, 15.0, None),
        page_hit("on an uncut body", "letters", false, 25.0, 15.0, None),
        page_hit("on a dot: a mark member at the letter's own offset", "letters", false, 55.0, 23.0, None),
        page_hit("on a mark at its own offset", "letters", false, 55.0, 7.0, None),
        page_hit("on a mark with no offset", "letters", false, 15.0, 47.0, None),
        page_hit("on a word-level mark: no letter", "letters", false, 15.0, 5.0, None),
        page_hit("between words: the nearest letter of the word hit test's word", "letters", false, 35.0, 15.0, None),
        page_hit("in a letter's box, off its ink: a near hit at distance 0", "letters", false, 55.0, 21.0, None),
        page_hit("max_distance limits the distance to the word", "letters", false, 35.0, 15.0, far),
        page_hit("a miss: no word within max_distance", "letters", false, 50.0, 90.0, far),
        page_hit("the sidecar: on a body past the text, at is none", "letters", true, 15.0, 15.0, None),
        page_hit("the sidecar: on a word-level mark past the text, at is none", "letters", true, 15.0, 5.0, None),
        page_hit("the sidecar: a near hit reports its letter's at, none", "letters", true, 15.0, 25.0, None),
        page_hit("a seam: the nearest piece, the ba's", "seam", false, 49.5, 15.0, None),
        page_hit("a seam: the nearest piece, the lam's", "seam", false, 48.5, 15.0, None),
        page_hit("overlapping marks over a body: the one drawn last", "overlap", false, 55.0, 15.0, None),
        page_hit("a mark of one letter over another letter's body: the mark", "overlap", false, 53.5, 13.5, None),
        page_hit("a body under no mark", "overlap", false, 51.0, 11.0, None),
        page_hit("a counter under another word's box names its own letter", "counter", false, 44.5, 15.0, None),
        page_hit("ink wins over the gap split", "counter", false, 45.0, 11.0, gap),
        page_hit("off ink and out of a counter, the gap split decides", "counter", false, 35.0, 15.0, gap),
        page_hit("a body stored as a mark: is_body, and its mark", "bodyMark", false, 15.0, 47.0, None),
        page_hit("a decoration is no letter", "decoration", false, 75.0, 55.0, None),
        view_hit("printed view: on the ba's piece", "letters", "printed", 8, 55.0, 15.0),
        view_hit("printed view: on a word-level mark", "letters", "printed", 5, 15.0, 5.0),
        view_hit("printed view: on the waw of line 2", "letters", "printed", 6, 15.0, 55.0),
        view_hit("reflowed view: on the ba's piece", "letters", "reflow", 8, 55.0, 15.0),
        view_hit("reflowed view: on the lam's piece", "letters", "reflow", 9, 45.0, 15.0),
        view_hit("reflowed view: on a word-level mark", "letters", "reflow", 5, 15.0, 5.0),
        view_hit("reflowed view: on the waw of line 2", "letters", "reflow", 6, 15.0, 55.0),
        view_hit("reflowed view: a seam", "seam", "reflow", 0, 49.5, 15.0),
        view_hit("printed view: a decoration is no letter", "decoration", "printed", 8, 75.0, 55.0),
    ];
    let mut hits = vec![];
    for c in &cases {
        let (j, hit) = hit_case(&all, c);
        let got = hit.map(|h| (h.word, h.letter, h.path, h.is_exact));
        let at = |fixture: &str, x: f32, y: f32| c.fixture == fixture && (c.x, c.y) == (x, y);
        if at("letters", 55.0, 15.0) {
            assert_eq!(got, Some((0, 0, 8, true)), "the ba's piece");
        }
        if at("seam", 49.5, 15.0) {
            assert_eq!(got, Some((0, 0, 8, true)), "the seam goes to the ba");
        }
        if at("seam", 48.5, 15.0) {
            assert_eq!(got, Some((0, 1, 9, true)), "the seam goes to the lam");
        }
        if at("counter", 44.5, 15.0) {
            let h = Host::ready(&all, "counter", false, None);
            assert_eq!(h.p.hit_test(44.5, 15.0, &HitOptions::default()).map(|h| h.word), Some(1), "word 1's box");
            assert_eq!(got, Some((0, 1, 9, false)), "the counter names the lam");
        }
        if at("counter", 45.0, 11.0) {
            let h = Host::ready(&all, "counter", false, None);
            assert_eq!(
                h.p.hit_test(45.0, 11.0, &c_options(gap)).map(|h| h.word),
                Some(1),
                "the gap split picks word 1"
            );
            assert_eq!(got.map(|g| (g.0, g.3)), Some((0, true)), "the ink is word 0's");
        }
        if c.fixture == "decoration" {
            assert_eq!(got, None, "a decoration is no letter");
        }
        hits.push(j);
    }

    // ── sequences ──
    in_key("sequences");
    use Step::*;
    let lams = query(page_scope(), lam, 0, 0);
    let mut sequences = vec![];
    let (j, st) = sequence(
        &all,
        "every letter call answers nothing before an attach, and a rule added then waits for the layer",
        "letters",
        &[
            Layout("printed"),
            Tick(0.0),
            HasLetters,
            NPaths,
            LetterCount(0),
            LetterInfo(0, 0),
            LetterMembers(0, 0),
            PathLetter(0),
            LetterAtOffset(0, 0),
            FindLetters(query(page_scope(), 0, 0, 0)),
            HitTestLetter(55.0, 15.0),
            HitTestLetterView(0, 55.0, 15.0),
            DrawRevision,
            Style(sel(sel_l, 0, 1, 0), RED, 0, "lam"),
            StyleLetters(lams.clone(), BLUE, 0, "lams"),
            Tick(0.0),
            Colors,
            DrawRevision,
            DrawList,
            Attach("letters"),
            Tick(0.0),
            Colors,
            DrawRevision,
            DrawList,
            FindLetters(lams.clone()),
        ],
    );
    assert_eq!(value_of(&st, 3), "8");
    assert_eq!(value_of(&st, 12), "0", "no layer, revision 0");
    sequences.push(j);
    let (j, st) = sequence(
        &all,
        "a refused attach changes nothing, an attach appends the pieces, a second attach replaces the first",
        "letters",
        &[
            NPaths,
            Rows(0, 8),
            AttachBytes(&GARBAGE),
            NPaths,
            HasLetters,
            Attach("variant"),
            NPaths,
            HasLetters,
            Attach("letters"),
            HasLetters,
            NPaths,
            Rows(0, 8),
            Rows(8, 2),
            DrawRevision,
            Attach("letters"),
            NPaths,
            DrawRevision,
            AttachBytes(&GARBAGE),
            Attach("variant"),
            HasLetters,
            NPaths,
            DrawRevision,
            LetterInfo(0, 1),
            Attach("seam"),
            NPaths,
            Rows(8, 2),
            LetterInfo(0, 1),
            DrawRevision,
        ],
    );
    assert_eq!(value_of(&st, 2), "-1");
    assert_eq!(value_of(&st, 10), "10");
    assert_eq!(value_of(&st, 1), value_of(&st, 11), "the page's own rows do not change");
    sequences.push(j);
    let (j, _) = sequence(
        &all,
        "a words sidecar attached before the layer checks its offsets",
        "letters",
        &[
            AttachWords,
            Attach("letters"),
            LetterInfo(1, 0),
            LetterInfo(1, 1),
            LetterMembers(1, 1),
            PathLetter(5),
            LetterAtOffset(1, 1),
            LetterAtOffset(1, 2),
            LetterAtOffset(2, 1),
        ],
    );
    sequences.push(j);
    let (j, _) = sequence(
        &all,
        "a letter colour draws the pieces, a word colour draws the path whole",
        "letters",
        &[
            Attach("letters"),
            Layout("printed"),
            Tick(0.0),
            DrawList,
            DrawRevision,
            Style(sel(sel_l, 0, 1, 0), RED, 0, "lam"),
            Tick(0.0),
            Colors,
            StyledPaths,
            DrawRevision,
            DrawList,
            Remove("lam"),
            Tick(0.0),
            DrawRevision,
            DrawList,
            Style(sel(SEL_WORD, 0, 0, 0), RED, 0, "word"),
            Tick(0.0),
            Colors,
            StyledPaths,
            DrawRevision,
            DrawList,
        ],
    );
    sequences.push(j);
    let (j, _) = sequence(
        &all,
        "the pieces stay until the fade ends",
        "letters",
        &[
            Attach("letters"),
            Layout("printed"),
            Tick(0.0),
            Style(sel(sel_l, 0, 1, 0), RED, 200, "lam"),
            Tick(0.0),
            DrawRevision,
            DrawList,
            Tick(100.0),
            Colors,
            Remove("lam"),
            Tick(100.0),
            Tick(150.0),
            Colors,
            DrawList,
            Tick(299.0),
            DrawRevision,
            DrawList,
            Tick(300.0),
            DrawRevision,
            DrawList,
            Colors,
        ],
    );
    sequences.push(j);
    let (j, _) = sequence(
        &all,
        "every letter of a cut path in one colour draws it whole, in that colour",
        "letters",
        &[
            Attach("letters"),
            Layout("printed"),
            Tick(0.0),
            Style(sel(sel_l, 0, 0, 0), RED, 200, "ba"),
            Style(sel(sel_l, 0, 1, 0), RED, 200, "lam"),
            Tick(0.0),
            DrawList,
            Tick(100.0),
            DrawList,
            Tick(200.0),
            Colors,
            StyledPaths,
            DrawRevision,
            Style(sel(sel_l, 0, 1, 0), BLUE, 200, "lam blue"),
            Tick(400.0),
            DrawList,
            Style(sel(sel_l, 0, 0, 0), BLUE, 200, "ba blue"),
            Tick(500.0),
            DrawList,
            Tick(600.0),
            DrawList,
            Tick(700.0),
            DrawList,
            Colors,
            DrawRevision,
        ],
    );
    sequences.push(j);
    let (j, _) = sequence(
        &all,
        "a theme, a word highlight and a mask never swap by themselves",
        "letters",
        &[
            Attach("letters"),
            Layout("printed"),
            Tick(0.0),
            DrawRevision,
            Theme(0x202020ff, BLUE, 300),
            Highlight(0, RED, 200, "highlight"),
            MaskTransition(150),
            Mask(0),
            Tick(0.0),
            DrawList,
            Tick(100.0),
            DrawList,
            Tick(400.0),
            DrawList,
            Unmask,
            RemoveHighlight("highlight"),
            Tick(450.0),
            DrawList,
            Tick(900.0),
            DrawList,
            Style(sel(SEL_PATH, 0, 0, 0), BLUE, 100, "path"),
            Tick(1000.0),
            Tick(1200.0),
            DrawList,
            DrawRevision,
        ],
    );
    sequences.push(j);
    let (j, st) = sequence(
        &all,
        "with a layer, the draw list and the draw revision apply a pending rule",
        "letters",
        &[
            Attach("letters"),
            Layout("printed"),
            Tick(0.0),
            DrawList,
            DrawRevision,
            Style(sel(sel_l, 0, 1, 0), RED, 0, "lam"),
            DrawList,
            DrawRevision,
            DrawRevision,
            Remove("lam"),
            DrawRevision,
            DrawList,
        ],
    );
    assert_ne!(value_of(&st, 3), value_of(&st, 6), "the draw list applies the rule");
    sequences.push(j);
    let (j, _) = sequence(
        &all,
        "with no layer, a draw-list read leaves a pending fade alone",
        "letters",
        &[
            Layout("printed"),
            Tick(1000.0),
            Style(sel(SEL_WORD, 0, 0, 0), RED, 200, "word"),
            DrawList,
            DrawRevision,
            Tick(60000.0),
            Colors,
            Tick(60100.0),
            Colors,
            Tick(60200.0),
            Colors,
        ],
    );
    sequences.push(j);
    let (j, _) = sequence(
        &all,
        "a swap under a reflow keeps the word's placement",
        "letters",
        &[
            Attach("letters"),
            Layout("reflow"),
            Tick(0.0),
            DrawList,
            Style(sel(sel_l, 0, 0, 1), RED, 0, "ba"),
            Tick(0.0),
            DrawList,
            DrawRevision,
        ],
    );
    sequences.push(j);
    let (j, _) = sequence(
        &all,
        "a second attach keeps the rules, and a query takes the new layer",
        "letters",
        &[
            Attach("letters"),
            Layout("printed"),
            Tick(0.0),
            Style(sel(sel_l, 0, 1, 0), RED, 0, "lam"),
            StyleLetters(query(word_scope(2), 0, 0, 0), BLUE, 0, "waw"),
            Tick(0.0),
            Colors,
            DrawList,
            Attach("letters"),
            Tick(0.0),
            DrawRevision,
            DrawList,
            Attach("bodyMark"),
            Tick(0.0),
            Colors,
            DrawList,
        ],
    );
    sequences.push(j);

    // ── every symbol is exercised ──
    let cover = COVER.lock().unwrap().clone();
    for sym in SYMBOLS {
        assert!(cover.iter().any(|(_, s)| *s == sym), "no case exercises {sym}");
    }
    let table: Vec<J> = SYMBOLS
        .iter()
        .map(|sym| {
            let keys: Vec<J> = cover.iter().filter(|(_, s)| s == sym).map(|(k, _)| s(k)).collect();
            J::Obj(vec![("symbol", s(sym)), ("keys", J::Arr(keys))])
        })
        .collect();

    let files = J::Obj(
        all.iter().map(|f| (f.name, J::Obj(vec![("page", s(f.page_file)), ("letters", s(f.layer_file))]))).collect(),
    );
    let doc = vec![
        (
            "note",
            s(concat!(
                "Replayed by every wrapper (docs/standards/TESTING.md). Indices are -1 for QVP_NONE, in inputs and in answers. ",
                "A null answer is a call that returns 0. Marks are names from qvp_mark_name. Colours are RGBA numbers. ",
                "Boxes, distances and points are floats within tolerance; every other number is exact. ",
                "Each case of letters, selections, queries and hits starts from a fresh page: load the fixture's page, attach its layer, ",
                "attach the words sidecar when words is true, lay out (printed, for selections and queries), then qvp_tick(0). ",
                "A selection adds its selector with qvp_style_add (layer 0, red, 0 ms), then qvp_tick(0), then reads qvp_colors, ",
                "qvp_layout_draw_revision and qvp_layout_draw_list over the whole page. ",
                "A sequence starts from the fixture's page with no layer and makes its calls in order."
            )),
        ),
        ("tolerance", num(0.0001)),
        ("red", num(RED)),
        ("blue", num(BLUE)),
        ("defaultInk", num(DEFAULT_INK)),
        ("layouts", J::Obj(vec![("printed", spec_json("printed")), ("reflow", spec_json("reflow"))])),
        ("fixtures", files),
        ("words", s(WORDS)),
        ("letters", J::Lines(letters)),
        ("selections", J::Lines(selections)),
        ("queries", J::Lines(queries)),
        ("hits", J::Lines(hits)),
        ("sequences", J::Lines(sequences)),
        ("coverage", J::Lines(table)),
    ];
    let mut out = String::from("{\n");
    for (i, (k, v)) in doc.iter().enumerate() {
        out.push_str(&format!("  \"{k}\": "));
        v.write(2, &mut out);
        out.push_str(if i + 1 < doc.len() { ",\n" } else { "\n" });
    }
    out.push_str("}\n");

    let scenarios = out_dir.join("scenarios");
    std::fs::create_dir_all(&scenarios).expect("mkdir");
    let json_path = scenarios.join("letters.json");
    std::fs::write(&json_path, out).expect("write the scenarios");
    let mut written = BTreeSet::new();
    for f in &all {
        if written.insert(f.page_file) {
            std::fs::write(out_dir.join(f.page_file), hex(&f.page)).expect("write a page");
        }
        if written.insert(f.layer_file) {
            std::fs::write(out_dir.join(f.layer_file), hex(&f.layer)).expect("write a layer");
        }
    }
    println!("{}: {} fixtures, {} hex files", json_path.display(), all.len(), written.len());
}
