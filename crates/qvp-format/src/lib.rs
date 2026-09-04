//! QVP (Quran Vector Page) binary format, version 1.
//!
//! One file = one mushaf page. Coordinates are in *page units* (the SVG
//! viewBox space, origin top-left, y down), quantised by `Header::quant`
//! (default 100 → 0.01 unit). Tables are fixed-width little-endian records;
//! path outlines are an opcode stream with zigzag-LEB128 point deltas.
//!
//! Layout:
//! ```text
//! Header (72 B) | lines[] | ayahs[] | words[] | paths[] | decos[] | ops | strings
//! ```
#![forbid(unsafe_code)]

use std::fmt;

pub const MAGIC: &[u8; 4] = b"QVP1";
pub const VERSION: u16 = 1;
pub const DEFAULT_QUANT: u16 = 100;
pub const HEADER_LEN: usize = 80;
pub const LINE_LEN: usize = 24;
pub const AYAH_LEN: usize = 32;
pub const WORD_LEN: usize = 36;
pub const PATH_LEN: usize = 36;
pub const DECO_LEN: usize = 32;
pub const GLYPH_LEN: usize = 28;
pub const INST_LEN: usize = 28;
pub const NONE_U16: u16 = 0xFFFF;

// ───────────────────────────── enums ─────────────────────────────

/// What a path is (drives default styling and hit-test precedence).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PathKind {
    Body = 0,
    Mark = 1,
    AyahNumber = 2,
    AyahOrnament = 3,
    HeaderInk = 4,
    Other = 255,
}

impl PathKind {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Body,
            1 => Self::Mark,
            2 => Self::AyahNumber,
            3 => Self::AyahOrnament,
            4 => Self::HeaderInk,
            _ => Self::Other,
        }
    }
    pub fn from_svg(v: &str) -> Self {
        match v {
            "body" => Self::Body,
            "mark" => Self::Mark,
            "ayah-number" => Self::AyahNumber,
            "ayah-marker-ornament" => Self::AyahOrnament,
            "header-ink" => Self::HeaderInk,
            _ => Self::Other,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Body => "body",
            Self::Mark => "mark",
            Self::AyahNumber => "ayah-number",
            Self::AyahOrnament => "ayah-marker-ornament",
            Self::HeaderInk => "header-ink",
            Self::Other => "other",
        }
    }
}

/// Mark family (coarse grouping, useful for "colour all diacritics").
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Family {
    None = 0,
    Diacritic = 1,
    Tanween = 2, // "diacritic tanween"
    Dots = 3,
    Waqf = 4,
    Sifr = 5,
    Sajdah = 6,
    ReadingSign = 7,
    Other = 255,
}

impl Family {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::None,
            1 => Self::Diacritic,
            2 => Self::Tanween,
            3 => Self::Dots,
            4 => Self::Waqf,
            5 => Self::Sifr,
            6 => Self::Sajdah,
            7 => Self::ReadingSign,
            _ => Self::Other,
        }
    }
    pub fn from_svg(v: &str) -> Self {
        match v {
            "" => Self::None,
            "diacritic" => Self::Diacritic,
            "diacritic tanween" => Self::Tanween,
            "dots" => Self::Dots,
            "waqf" => Self::Waqf,
            "sifr" => Self::Sifr,
            "sajdah" => Self::Sajdah,
            "reading-sign" => Self::ReadingSign,
            _ => Self::Other,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Diacritic => "diacritic",
            Self::Tanween => "diacritic tanween",
            Self::Dots => "dots",
            Self::Waqf => "waqf",
            Self::Sifr => "sifr",
            Self::Sajdah => "sajdah",
            Self::ReadingSign => "reading-sign",
            Self::Other => "other",
        }
    }
}

macro_rules! marks {
    ($( $name:ident = $val:expr => $svg:literal ),* $(,)?) => {
        /// Individual mark type (`data-mark` in the source SVG).
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        #[repr(u8)]
        pub enum Mark { $( $name = $val, )* }
        impl Mark {
            pub fn from_u8(v: u8) -> Self {
                match v { $( $val => Self::$name, )* _ => Self::Unknown }
            }
            pub fn from_svg(v: &str) -> Self {
                match v { $( $svg => Self::$name, )* _ => Self::Unknown }
            }
            pub fn as_str(self) -> &'static str {
                match self { $( Self::$name => $svg, )* }
            }
            pub const ALL: &'static [Mark] = &[ $( Self::$name, )* ];
        }
    };
}

marks! {
    None = 0 => "",
    Fatha = 1 => "fatha",
    Kasra = 2 => "kasra",
    Damma = 3 => "damma",
    Fathatan = 4 => "fathatan",
    Kasratan = 5 => "kasratan",
    Dammatan = 6 => "dammatan",
    Shadda = 7 => "shadda",
    Sukun = 8 => "sukun",
    Maddah = 9 => "maddah",
    Hamza = 10 => "hamza",
    Wasla = 11 => "wasla",
    SmallAlef = 12 => "small-alef",
    SmallWaw = 13 => "small-waw",
    SmallYa = 14 => "small-ya",
    SmallNoon = 15 => "small-noon",
    Dot = 16 => "dot",
    TwoDots = 17 => "two-dots",
    ThreeDots = 18 => "three-dots",
    SifrMustadir = 19 => "sifr-mustadir",
    SifrMustatil = 20 => "sifr-mustatil",
    WaqfJaiz = 21 => "waqf-jaiz",
    WaqfAwla = 22 => "waqf-awla",
    WaslAwla = 23 => "wasl-awla",
    WaqfLazim = 24 => "waqf-lazim",
    Muanaqah = 25 => "muanaqah",
    Saktah = 26 => "saktah",
    MeemIqlab = 27 => "meem-iqlab",
    Hizb = 28 => "hizb",
    Sajdah = 29 => "sajdah",
    SajdahSign = 30 => "sajdah-sign",
    SajdahLine = 31 => "sajdah-line",
    SeenReading = 32 => "seen-reading",
    Tashil = 33 => "tashil",
    Ishmam = 34 => "ishmam",
    Imalah = 35 => "imalah",
    Unknown = 255 => "unknown",
}

/// Non-word groups on a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DecoKind {
    AyahMarker = 0,
    SurahName = 1,
    Basmalah = 2,
    HizbMark = 3,
    SajdahMark = 4,
    Other = 255,
}

impl DecoKind {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::AyahMarker,
            1 => Self::SurahName,
            2 => Self::Basmalah,
            3 => Self::HizbMark,
            4 => Self::SajdahMark,
            _ => Self::Other,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AyahMarker => "ayah-marker",
            Self::SurahName => "surah-name",
            Self::Basmalah => "basmalah",
            Self::HizbMark => "hizb-mark",
            Self::SajdahMark => "sajdah-mark",
            Self::Other => "other",
        }
    }
}

/// Path flag bits.
pub const PF_EVENODD: u8 = 1;
/// Path form (`data-form`): bit1 stacked, bit2 staggered.
pub const PF_STACKED: u8 = 2;
pub const PF_STAGGERED: u8 = 4;
/// Standalone mark (hizb / sajdah sign not attached to a word).
pub const PF_STANDALONE: u8 = 8;
/// Path is a glyph instance: `op_off` is an index into `insts`, `op_len` = 0.
pub const PF_GLYPH: u8 = 16;

/// Ayah flag bits.
pub const AF_JUZ_START: u8 = 1;
pub const AF_HIZB_START: u8 = 2;
pub const AF_RUB_START: u8 = 4;
pub const AF_NISF_START: u8 = 8;

// ───────────────────────────── records ─────────────────────────────

/// Integer bbox in quantised page units, inclusive min / exclusive max not
/// enforced — treat as [x0,x1]×[y0,y1].
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct IBox {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl IBox {
    pub const EMPTY: IBox = IBox { x0: i32::MAX, y0: i32::MAX, x1: i32::MIN, y1: i32::MIN };
    pub fn is_empty(&self) -> bool {
        self.x0 > self.x1 || self.y0 > self.y1
    }
    pub fn add_point(&mut self, x: i32, y: i32) {
        self.x0 = self.x0.min(x);
        self.y0 = self.y0.min(y);
        self.x1 = self.x1.max(x);
        self.y1 = self.y1.max(y);
    }
    pub fn union(&mut self, o: &IBox) {
        if o.is_empty() {
            return;
        }
        self.add_point(o.x0, o.y0);
        self.add_point(o.x1, o.y1);
    }
    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x0 && x <= self.x1 && y >= self.y0 && y <= self.y1
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Header {
    pub version: u16,
    pub quant: u16,
    pub page: u16,
    pub flags: u16,
    pub width: f32,
    pub height: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct LineRec {
    pub line_no: u8,
    pub first_word: u16,
    pub n_words: u16,
    pub bbox: IBox,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct AyahRec {
    pub sura: u16,
    pub ayah: u16,
    pub part: u8,
    pub parts: u8,
    pub flags: u8,
    pub first_word: u16,
    pub n_words: u16,
    /// Index into decos, or NONE_U16.
    pub marker_deco: u16,
    pub bbox: IBox,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct WordRec {
    pub sura: u16,
    pub ayah: u16,
    pub word: u16,
    pub line_idx: u16,
    pub ayah_idx: u16,
    /// Index into strings (uthmani text) or NONE_U16.
    pub text: u16,
    pub first_path: u32,
    pub n_paths: u16,
    pub bbox: IBox,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathRec {
    pub kind: PathKind,
    pub mark: Mark,
    pub family: Family,
    pub flags: u8,
    /// Origin the opcode stream's first MoveTo is relative to.
    pub ox: i32,
    pub oy: i32,
    pub op_off: u32,
    pub op_len: u32,
    pub bbox: IBox,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct DecoRec {
    pub kind: DecoKind,
    pub sura: u16,
    pub ayah: u16,
    /// Index into strings (e.g. surah Arabic name / hizb info), or NONE_U16.
    pub text: u16,
    pub first_path: u32,
    pub n_paths: u16,
    /// Line the decoration sits in (surah header, basmalah, hizb mark), or NONE_U16.
    pub line: u16,
    pub bbox: IBox,
}

impl Default for DecoKind {
    fn default() -> Self {
        DecoKind::Other
    }
}

/// Shared outline, stored once per page in glyph space (origin 0,0, same
/// quantisation as the page). Placed by `InstRec`s.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct GlyphRec {
    pub op_off: u32,
    pub op_len: u32,
    pub bbox: IBox,
}

/// Affine placement of a glyph: page = M · glyph (glyph coords in *units*,
/// i.e. already divided by quant).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InstRec {
    pub glyph: u16,
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

/// Fully decoded page (owned). `ops` is the raw opcode stream.
#[derive(Clone, Debug, PartialEq)]
pub struct PageData {
    pub header: Header,
    pub lines: Vec<LineRec>,
    pub ayahs: Vec<AyahRec>,
    pub words: Vec<WordRec>,
    pub paths: Vec<PathRec>,
    pub decos: Vec<DecoRec>,
    pub glyphs: Vec<GlyphRec>,
    pub insts: Vec<InstRec>,
    pub ops: Vec<u8>,
    pub strings: Vec<String>,
}

// ───────────────────────────── path opcodes ─────────────────────────────

pub const OP_MOVE: u8 = 0;
pub const OP_LINE: u8 = 1;
pub const OP_QUAD: u8 = 2;
pub const OP_CUBIC: u8 = 3;
pub const OP_CLOSE: u8 = 4;

/// Absolute, quantised path command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    MoveTo(i32, i32),
    LineTo(i32, i32),
    QuadTo(i32, i32, i32, i32),
    CubicTo(i32, i32, i32, i32, i32, i32),
    Close,
}

pub fn zigzag(v: i32) -> u32 {
    ((v << 1) ^ (v >> 31)) as u32
}
pub fn unzigzag(v: u32) -> i32 {
    ((v >> 1) as i32) ^ -((v & 1) as i32)
}
pub fn write_varint(out: &mut Vec<u8>, mut v: u32) {
    while v >= 0x80 {
        out.push((v as u8) | 0x80);
        v >>= 7;
    }
    out.push(v as u8);
}
pub fn read_varint(buf: &[u8], pos: &mut usize) -> Result<u32, Error> {
    let mut shift = 0;
    let mut v: u32 = 0;
    loop {
        let b = *buf.get(*pos).ok_or(Error::Truncated("varint"))?;
        *pos += 1;
        v |= ((b & 0x7f) as u32) << shift;
        if b & 0x80 == 0 {
            return Ok(v);
        }
        shift += 7;
        if shift > 28 {
            return Err(Error::Corrupt("varint too long"));
        }
    }
}

/// Encode absolute commands into a delta opcode stream relative to `(ox, oy)`.
/// Returns the bbox of all points (control points included).
pub fn encode_cmds(cmds: &[Cmd], ox: i32, oy: i32, out: &mut Vec<u8>) -> IBox {
    let (mut cx, mut cy) = (ox, oy);
    let mut bb = IBox::EMPTY;
    let mut put = |out: &mut Vec<u8>, x: i32, y: i32, cx: &mut i32, cy: &mut i32| {
        write_varint(out, zigzag(x - *cx));
        write_varint(out, zigzag(y - *cy));
        *cx = x;
        *cy = y;
        bb.add_point(x, y);
    };
    for c in cmds {
        match *c {
            Cmd::MoveTo(x, y) => {
                out.push(OP_MOVE);
                put(out, x, y, &mut cx, &mut cy);
            }
            Cmd::LineTo(x, y) => {
                out.push(OP_LINE);
                put(out, x, y, &mut cx, &mut cy);
            }
            Cmd::QuadTo(x1, y1, x, y) => {
                out.push(OP_QUAD);
                put(out, x1, y1, &mut cx, &mut cy);
                put(out, x, y, &mut cx, &mut cy);
            }
            Cmd::CubicTo(x1, y1, x2, y2, x, y) => {
                out.push(OP_CUBIC);
                put(out, x1, y1, &mut cx, &mut cy);
                put(out, x2, y2, &mut cx, &mut cy);
                put(out, x, y, &mut cx, &mut cy);
            }
            Cmd::Close => out.push(OP_CLOSE),
        }
    }
    bb
}

/// Decode an opcode stream back to absolute commands.
pub fn decode_cmds(ops: &[u8], ox: i32, oy: i32) -> Result<Vec<Cmd>, Error> {
    let mut out = Vec::with_capacity(ops.len() / 3);
    let (mut cx, mut cy) = (ox, oy);
    let mut pos = 0;
    let pt = |pos: &mut usize, cx: &mut i32, cy: &mut i32| -> Result<(i32, i32), Error> {
        let dx = unzigzag(read_varint(ops, pos)?);
        let dy = unzigzag(read_varint(ops, pos)?);
        *cx += dx;
        *cy += dy;
        Ok((*cx, *cy))
    };
    while pos < ops.len() {
        let op = ops[pos];
        pos += 1;
        match op {
            OP_MOVE => {
                let (x, y) = pt(&mut pos, &mut cx, &mut cy)?;
                out.push(Cmd::MoveTo(x, y));
            }
            OP_LINE => {
                let (x, y) = pt(&mut pos, &mut cx, &mut cy)?;
                out.push(Cmd::LineTo(x, y));
            }
            OP_QUAD => {
                let (x1, y1) = pt(&mut pos, &mut cx, &mut cy)?;
                let (x, y) = pt(&mut pos, &mut cx, &mut cy)?;
                out.push(Cmd::QuadTo(x1, y1, x, y));
            }
            OP_CUBIC => {
                let (x1, y1) = pt(&mut pos, &mut cx, &mut cy)?;
                let (x2, y2) = pt(&mut pos, &mut cx, &mut cy)?;
                let (x, y) = pt(&mut pos, &mut cx, &mut cy)?;
                out.push(Cmd::CubicTo(x1, y1, x2, y2, x, y));
            }
            OP_CLOSE => out.push(Cmd::Close),
            _ => return Err(Error::Corrupt("bad opcode")),
        }
    }
    Ok(out)
}

// ───────────────────────────── errors ─────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    BadMagic,
    BadVersion(u16),
    Truncated(&'static str),
    Corrupt(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadMagic => write!(f, "not a QVP file"),
            Error::BadVersion(v) => write!(f, "unsupported QVP version {v}"),
            Error::Truncated(w) => write!(f, "truncated at {w}"),
            Error::Corrupt(w) => write!(f, "corrupt: {w}"),
        }
    }
}
impl std::error::Error for Error {}

// ───────────────────────────── encode ─────────────────────────────

struct W(Vec<u8>);
impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn u32(&mut self, v: u32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn f32(&mut self, v: f32) {
        self.0.extend_from_slice(&v.to_le_bytes());
    }
    fn bbox(&mut self, b: &IBox) {
        self.i32(b.x0);
        self.i32(b.y0);
        self.i32(b.x1);
        self.i32(b.y1);
    }
    fn patch_u32(&mut self, at: usize, v: u32) {
        self.0[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
}

pub fn encode(p: &PageData) -> Vec<u8> {
    let mut w = W(Vec::with_capacity(
        HEADER_LEN
            + p.lines.len() * LINE_LEN
            + p.ayahs.len() * AYAH_LEN
            + p.words.len() * WORD_LEN
            + p.paths.len() * PATH_LEN
            + p.decos.len() * DECO_LEN
            + p.ops.len()
            + 1024,
    ));
    // header
    w.0.extend_from_slice(MAGIC);
    w.u16(p.header.version);
    w.u16(p.header.quant);
    w.u16(p.header.page);
    w.u16(p.header.flags);
    w.f32(p.header.width);
    w.f32(p.header.height);
    w.u16(p.lines.len() as u16);
    w.u16(p.ayahs.len() as u16);
    w.u16(p.words.len() as u16);
    w.u16(p.decos.len() as u16);
    w.u32(p.paths.len() as u32);
    let off_pos = w.0.len(); // 32: seven u32 offsets + len_ops
    for _ in 0..8 {
        w.u32(0);
    }
    w.u16(p.strings.len() as u16);
    w.u16(p.glyphs.len() as u16);
    w.u16(p.insts.len() as u16);
    w.u16(0);
    let off_pos2 = w.0.len();
    w.u32(0);
    w.u32(0);
    debug_assert_eq!(w.0.len(), HEADER_LEN);

    let off_lines = w.0.len();
    for l in &p.lines {
        w.u8(l.line_no);
        w.u8(0);
        w.u16(l.first_word);
        w.u16(l.n_words);
        w.u16(0);
        w.bbox(&l.bbox);
    }
    let off_ayahs = w.0.len();
    for a in &p.ayahs {
        w.u16(a.sura);
        w.u16(a.ayah);
        w.u8(a.part);
        w.u8(a.parts);
        w.u16(a.first_word);
        w.u16(a.n_words);
        w.u16(a.marker_deco);
        w.u8(a.flags);
        w.u8(0);
        w.u16(0);
        w.bbox(&a.bbox);
    }
    let off_words = w.0.len();
    for x in &p.words {
        w.u16(x.sura);
        w.u16(x.ayah);
        w.u16(x.word);
        w.u16(x.line_idx);
        w.u16(x.ayah_idx);
        w.u16(x.text);
        w.u32(x.first_path);
        w.u16(x.n_paths);
        w.u16(0);
        w.bbox(&x.bbox);
    }
    let off_paths = w.0.len();
    for x in &p.paths {
        w.u8(x.kind as u8);
        w.u8(x.mark as u8);
        w.u8(x.family as u8);
        w.u8(x.flags);
        w.i32(x.ox);
        w.i32(x.oy);
        w.u32(x.op_off);
        w.u32(x.op_len);
        w.bbox(&x.bbox);
    }
    let off_decos = w.0.len();
    for d in &p.decos {
        w.u8(d.kind as u8);
        w.u8(0);
        w.u16(d.sura);
        w.u16(d.ayah);
        w.u16(d.text);
        w.u32(d.first_path);
        w.u16(d.n_paths);
        w.u16(d.line);
        w.bbox(&d.bbox);
    }
    let off_glyphs = w.0.len();
    for g in &p.glyphs {
        w.u32(g.op_off);
        w.u32(g.op_len);
        w.bbox(&g.bbox);
        w.u32(0);
    }
    let off_insts = w.0.len();
    for i in &p.insts {
        w.u16(i.glyph);
        w.u16(0);
        w.f32(i.a);
        w.f32(i.b);
        w.f32(i.c);
        w.f32(i.d);
        w.f32(i.e);
        w.f32(i.f);
    }
    w.patch_u32(off_pos2, off_glyphs as u32);
    w.patch_u32(off_pos2 + 4, off_insts as u32);
    let off_ops = w.0.len();
    w.0.extend_from_slice(&p.ops);
    let off_strings = w.0.len();
    for s in &p.strings {
        let b = s.as_bytes();
        w.u16(b.len() as u16);
        w.0.extend_from_slice(b);
    }
    for (i, v) in [
        off_lines,
        off_ayahs,
        off_words,
        off_paths,
        off_decos,
        off_ops,
        p.ops.len(),
        off_strings,
    ]
    .iter()
    .enumerate()
    {
        w.patch_u32(off_pos + i * 4, *v as u32);
    }
    w.0
}

// ───────────────────────────── decode ─────────────────────────────

struct R<'a> {
    b: &'a [u8],
    pos: usize,
}
impl<'a> R<'a> {
    fn need(&self, n: usize, what: &'static str) -> Result<(), Error> {
        if self.pos + n > self.b.len() {
            Err(Error::Truncated(what))
        } else {
            Ok(())
        }
    }
    fn u8(&mut self) -> u8 {
        let v = self.b[self.pos];
        self.pos += 1;
        v
    }
    fn u16(&mut self) -> u16 {
        let v = u16::from_le_bytes([self.b[self.pos], self.b[self.pos + 1]]);
        self.pos += 2;
        v
    }
    fn u32(&mut self) -> u32 {
        let v = u32::from_le_bytes(self.b[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        v
    }
    fn i32(&mut self) -> i32 {
        self.u32() as i32
    }
    fn f32(&mut self) -> f32 {
        f32::from_bits(self.u32())
    }
    fn bbox(&mut self) -> IBox {
        IBox { x0: self.i32(), y0: self.i32(), x1: self.i32(), y1: self.i32() }
    }
}

pub fn decode(bytes: &[u8]) -> Result<PageData, Error> {
    if bytes.len() < 4 || &bytes[0..4] != MAGIC {
        return Err(Error::BadMagic);
    }
    let mut r = R { b: bytes, pos: 0 };
    r.need(HEADER_LEN, "header")?;
    r.pos = 4;
    let version = r.u16();
    if version != VERSION {
        return Err(Error::BadVersion(version));
    }
    let quant = r.u16();
    let page = r.u16();
    let flags = r.u16();
    let width = r.f32();
    let height = r.f32();
    let n_lines = r.u16() as usize;
    let n_ayahs = r.u16() as usize;
    let n_words = r.u16() as usize;
    let n_decos = r.u16() as usize;
    let n_paths = r.u32() as usize;
    let off_lines = r.u32() as usize;
    let off_ayahs = r.u32() as usize;
    let off_words = r.u32() as usize;
    let off_paths = r.u32() as usize;
    let off_decos = r.u32() as usize;
    let off_ops = r.u32() as usize;
    let len_ops = r.u32() as usize;
    let off_strings = r.u32() as usize;
    let n_strings = r.u16() as usize;
    let n_glyphs = r.u16() as usize;
    let n_insts = r.u16() as usize;
    r.u16();
    let off_glyphs = r.u32() as usize;
    let off_insts = r.u32() as usize;

    r.pos = off_lines;
    r.need(n_lines * LINE_LEN, "lines")?;
    let mut lines = Vec::with_capacity(n_lines);
    for _ in 0..n_lines {
        let line_no = r.u8();
        r.u8();
        let first_word = r.u16();
        let n_words = r.u16();
        r.u16();
        let bbox = r.bbox();
        lines.push(LineRec { line_no, first_word, n_words, bbox });
    }
    r.pos = off_ayahs;
    r.need(n_ayahs * AYAH_LEN, "ayahs")?;
    let mut ayahs = Vec::with_capacity(n_ayahs);
    for _ in 0..n_ayahs {
        let sura = r.u16();
        let ayah = r.u16();
        let part = r.u8();
        let parts = r.u8();
        let first_word = r.u16();
        let n_words = r.u16();
        let marker_deco = r.u16();
        let flags = r.u8();
        r.u8();
        r.u16();
        let bbox = r.bbox();
        ayahs.push(AyahRec { sura, ayah, part, parts, flags, first_word, n_words, marker_deco, bbox });
    }
    r.pos = off_words;
    r.need(n_words * WORD_LEN, "words")?;
    let mut words = Vec::with_capacity(n_words);
    for _ in 0..n_words {
        let sura = r.u16();
        let ayah = r.u16();
        let word = r.u16();
        let line_idx = r.u16();
        let ayah_idx = r.u16();
        let text = r.u16();
        let first_path = r.u32();
        let n_paths = r.u16();
        r.u16();
        let bbox = r.bbox();
        words.push(WordRec { sura, ayah, word, line_idx, ayah_idx, text, first_path, n_paths, bbox });
    }
    r.pos = off_paths;
    r.need(n_paths * PATH_LEN, "paths")?;
    let mut paths = Vec::with_capacity(n_paths);
    for _ in 0..n_paths {
        let kind = PathKind::from_u8(r.u8());
        let mark = Mark::from_u8(r.u8());
        let family = Family::from_u8(r.u8());
        let flags = r.u8();
        let ox = r.i32();
        let oy = r.i32();
        let op_off = r.u32();
        let op_len = r.u32();
        let bbox = r.bbox();
        if flags & PF_GLYPH == 0 && (op_off as usize) + (op_len as usize) > len_ops {
            return Err(Error::Corrupt("path op range"));
        }
        paths.push(PathRec { kind, mark, family, flags, ox, oy, op_off, op_len, bbox });
    }
    r.pos = off_decos;
    r.need(n_decos * DECO_LEN, "decos")?;
    let mut decos = Vec::with_capacity(n_decos);
    for _ in 0..n_decos {
        let kind = DecoKind::from_u8(r.u8());
        r.u8();
        let sura = r.u16();
        let ayah = r.u16();
        let text = r.u16();
        let first_path = r.u32();
        let n_paths = r.u16();
        let line = r.u16();
        let bbox = r.bbox();
        decos.push(DecoRec { kind, sura, ayah, text, first_path, n_paths, line, bbox });
    }
    r.pos = off_glyphs;
    r.need(n_glyphs * GLYPH_LEN, "glyphs")?;
    let mut glyphs = Vec::with_capacity(n_glyphs);
    for _ in 0..n_glyphs {
        let op_off = r.u32();
        let op_len = r.u32();
        let bbox = r.bbox();
        r.u32();
        if (op_off as usize) + (op_len as usize) > len_ops {
            return Err(Error::Corrupt("glyph op range"));
        }
        glyphs.push(GlyphRec { op_off, op_len, bbox });
    }
    r.pos = off_insts;
    r.need(n_insts * INST_LEN, "insts")?;
    let mut insts = Vec::with_capacity(n_insts);
    for _ in 0..n_insts {
        let glyph = r.u16();
        r.u16();
        let (a, b, c, d, e, f) = (r.f32(), r.f32(), r.f32(), r.f32(), r.f32(), r.f32());
        if glyph as usize >= glyphs.len() {
            return Err(Error::Corrupt("inst glyph ref"));
        }
        insts.push(InstRec { glyph, a, b, c, d, e, f });
    }
    r.pos = off_ops;
    r.need(len_ops, "ops")?;
    let ops = bytes[off_ops..off_ops + len_ops].to_vec();
    r.pos = off_strings;
    let mut strings = Vec::with_capacity(n_strings);
    for _ in 0..n_strings {
        r.need(2, "string len")?;
        let n = r.u16() as usize;
        r.need(n, "string")?;
        let s = std::str::from_utf8(&bytes[r.pos..r.pos + n]).map_err(|_| Error::Corrupt("utf8"))?;
        strings.push(s.to_owned());
        r.pos += n;
    }
    for w in &words {
        if (w.first_path as usize) + (w.n_paths as usize) > paths.len() {
            return Err(Error::Corrupt("word path range"));
        }
        if w.text != NONE_U16 && w.text as usize >= strings.len() {
            return Err(Error::Corrupt("word text ref"));
        }
        if w.line_idx as usize >= lines.len() || w.ayah_idx as usize >= ayahs.len() {
            return Err(Error::Corrupt("word line/ayah ref"));
        }
    }
    for d in &decos {
        if (d.first_path as usize) + (d.n_paths as usize) > paths.len() {
            return Err(Error::Corrupt("deco path range"));
        }
    }
    for p in &paths {
        if p.flags & PF_GLYPH != 0 && p.op_off as usize >= insts.len() {
            return Err(Error::Corrupt("path inst ref"));
        }
    }
    Ok(PageData {
        header: Header { version, quant, page, flags, width, height },
        lines,
        ayahs,
        words,
        paths,
        decos,
        glyphs,
        insts,
        ops,
        strings,
    })
}

impl PageData {
    pub fn is_instance(&self, i: usize) -> bool {
        self.paths[i].flags & PF_GLYPH != 0
    }
    /// Instance record of a glyph-instance path.
    pub fn path_inst(&self, i: usize) -> Option<&InstRec> {
        let p = &self.paths[i];
        if p.flags & PF_GLYPH != 0 {
            self.insts.get(p.op_off as usize)
        } else {
            None
        }
    }
    /// Glyph outline commands in glyph space (quantised units).
    pub fn glyph_cmds(&self, g: usize) -> Result<Vec<Cmd>, Error> {
        let g = &self.glyphs[g];
        decode_cmds(&self.ops[g.op_off as usize..(g.op_off + g.op_len) as usize], 0, 0)
    }
    /// Absolute page-space commands for one path. Glyph instances are
    /// transformed and re-quantised; for exact GPU rendering prefer
    /// `path_inst` + `glyph_cmds` with a canvas transform.
    pub fn path_cmds(&self, i: usize) -> Result<Vec<Cmd>, Error> {
        let p = &self.paths[i];
        if let Some(inst) = self.path_inst(i) {
            let q = self.header.quant as f64;
            let m = |x: i32, y: i32| -> (i32, i32) {
                let (gx, gy) = (x as f64 / q, y as f64 / q);
                let px = inst.a as f64 * gx + inst.c as f64 * gy + inst.e as f64;
                let py = inst.b as f64 * gx + inst.d as f64 * gy + inst.f as f64;
                ((px * q).round() as i32, (py * q).round() as i32)
            };
            let cmds = self.glyph_cmds(inst.glyph as usize)?;
            return Ok(cmds
                .into_iter()
                .map(|c| match c {
                    Cmd::MoveTo(x, y) => {
                        let (x, y) = m(x, y);
                        Cmd::MoveTo(x, y)
                    }
                    Cmd::LineTo(x, y) => {
                        let (x, y) = m(x, y);
                        Cmd::LineTo(x, y)
                    }
                    Cmd::QuadTo(x1, y1, x, y) => {
                        let (x1, y1) = m(x1, y1);
                        let (x, y) = m(x, y);
                        Cmd::QuadTo(x1, y1, x, y)
                    }
                    Cmd::CubicTo(x1, y1, x2, y2, x, y) => {
                        let (x1, y1) = m(x1, y1);
                        let (x2, y2) = m(x2, y2);
                        let (x, y) = m(x, y);
                        Cmd::CubicTo(x1, y1, x2, y2, x, y)
                    }
                    Cmd::Close => Cmd::Close,
                })
                .collect());
        }
        decode_cmds(&self.ops[p.op_off as usize..(p.op_off + p.op_len) as usize], p.ox, p.oy)
    }
}

/// Bbox of a command list (control points included).
pub fn cmds_bbox(cmds: &[Cmd]) -> IBox {
    let mut bb = IBox::EMPTY;
    for c in cmds {
        match *c {
            Cmd::MoveTo(x, y) | Cmd::LineTo(x, y) => bb.add_point(x, y),
            Cmd::QuadTo(a, b, x, y) => {
                bb.add_point(a, b);
                bb.add_point(x, y)
            }
            Cmd::CubicTo(a, b, c2, d, x, y) => {
                bb.add_point(a, b);
                bb.add_point(c2, d);
                bb.add_point(x, y)
            }
            Cmd::Close => {}
        }
    }
    bb
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn varint_roundtrip() {
        for v in [0i32, 1, -1, 63, -64, 64, 8191, -8192, 1 << 20, -(1 << 20), i32::MAX / 2, i32::MIN / 2] {
            let mut b = Vec::new();
            write_varint(&mut b, zigzag(v));
            let mut pos = 0;
            assert_eq!(unzigzag(read_varint(&b, &mut pos).unwrap()), v);
            assert_eq!(pos, b.len());
        }
    }

    #[test]
    fn cmds_roundtrip() {
        let cmds = vec![
            Cmd::MoveTo(1000, -2000),
            Cmd::LineTo(1010, -2005),
            Cmd::QuadTo(1020, -2010, 1030, -2000),
            Cmd::CubicTo(1, 2, 3, 4, 5, 6),
            Cmd::Close,
            Cmd::MoveTo(0, 0),
            Cmd::Close,
        ];
        let mut ops = Vec::new();
        let bb = encode_cmds(&cmds, 1000, -2000, &mut ops);
        assert_eq!(decode_cmds(&ops, 1000, -2000).unwrap(), cmds);
        assert_eq!(bb, IBox { x0: 0, y0: -2010, x1: 1030, y1: 6 });
    }

    #[test]
    fn page_roundtrip() {
        let mut ops = Vec::new();
        let cmds = [Cmd::MoveTo(500, 600), Cmd::LineTo(700, 800), Cmd::Close];
        let bb = encode_cmds(&cmds, 500, 600, &mut ops);
        let p = PageData {
            header: Header { version: VERSION, quant: 100, page: 7, flags: 0, width: 345.0, height: 550.0 },
            lines: vec![LineRec { line_no: 1, first_word: 0, n_words: 1, bbox: bb }],
            ayahs: vec![AyahRec { sura: 2, ayah: 3, part: 1, parts: 1, flags: AF_RUB_START, first_word: 0, n_words: 1, marker_deco: 0, bbox: bb }],
            words: vec![WordRec { sura: 2, ayah: 3, word: 1, line_idx: 0, ayah_idx: 0, text: 0, first_path: 0, n_paths: 1, bbox: bb }],
            paths: vec![
                PathRec { kind: PathKind::Body, mark: Mark::None, family: Family::None, flags: PF_EVENODD, ox: 500, oy: 600, op_off: 0, op_len: ops.len() as u32, bbox: bb },
                PathRec { kind: PathKind::AyahOrnament, mark: Mark::None, family: Family::None, flags: 0, ox: 500, oy: 600, op_off: 0, op_len: ops.len() as u32, bbox: bb },
            ],
            decos: vec![DecoRec { kind: DecoKind::AyahMarker, sura: 2, ayah: 3, text: NONE_U16, first_path: 1, n_paths: 1, line: 0, bbox: bb }],
            glyphs: vec![GlyphRec { op_off: 0, op_len: ops.len() as u32, bbox: bb }],
            insts: vec![InstRec { glyph: 0, a: 2.0, b: 0.0, c: 0.0, d: 2.0, e: 10.0, f: 20.0 }],
            ops,
            strings: vec!["ذَٰلِكَ".to_owned()],
        };
        let mut p = p;
        p.paths.push(PathRec { kind: PathKind::AyahOrnament, mark: Mark::None, family: Family::None, flags: PF_GLYPH, ox: 0, oy: 0, op_off: 0, op_len: 0, bbox: bb });
        let bytes = encode(&p);
        let q = decode(&bytes).unwrap();
        assert_eq!(p, q);
        // instance: shared ops decode at glyph origin (0,0) → page (10,20) → quantised 1000,2000
        assert_eq!(q.path_cmds(2).unwrap()[0], Cmd::MoveTo(1000, 2000));
        assert_eq!(q.path_cmds(0).unwrap(), cmds.to_vec());
        assert_eq!(decode(&bytes[..bytes.len() - 3]).unwrap_err(), Error::Truncated("string"));
        assert_eq!(decode(b"nope").unwrap_err(), Error::BadMagic);
    }
}
