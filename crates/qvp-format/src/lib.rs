//! QVP (Quran Vector Page) binary format, version 1.
//!
//! One file = one mushaf page. Coordinates are in *page units* (the SVG
//! viewBox space, origin top-left, y down), quantised by `Header::quant`
//! (default 100 → 0.01 unit). The records in this module are the decoded,
//! in-memory form; the bytes on disk are a 64-byte header and six sections
//! (delta-coded tables, packed opcodes, close indices, x deltas, y deltas,
//! strings). `codec.rs` implements it and `docs/FORMAT.md` specifies it.
#![forbid(unsafe_code)]

pub mod atlas;
pub mod codec;

use std::fmt;

pub const MAGIC: &[u8; 4] = b"QVP1";
pub const VERSION: u16 = 1;
pub const DEFAULT_QUANT: u16 = 100;
pub const NONE_U16: u16 = 0xFFFF;

// ───────────────────────────── enums ─────────────────────────────

/// What a path is (drives default styling and hit-test precedence).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum PathKind {
    Body = 0,
    Mark = 1,
    AyahNumber = 2,
    AyahMarkOrnament = 3,
    HeaderInk = 4,
    Ornament = 5,
    PageNumber = 6,
    RunningHead = 7,
    Other = 255,
}

impl PathKind {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::Body,
            1 => Self::Mark,
            2 => Self::AyahNumber,
            3 => Self::AyahMarkOrnament,
            4 => Self::HeaderInk,
            5 => Self::Ornament,
            6 => Self::PageNumber,
            7 => Self::RunningHead,
            _ => Self::Other,
        }
    }
    pub fn from_svg(v: &str) -> Self {
        match v {
            "body" => Self::Body,
            "mark" => Self::Mark,
            "ayah_number" => Self::AyahNumber,
            "ayah_mark_ornament" => Self::AyahMarkOrnament,
            "header_ink" => Self::HeaderInk,
            "ornament" => Self::Ornament,
            "page_number" => Self::PageNumber,
            "running_head" => Self::RunningHead,
            _ => Self::Other,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Body => "body",
            Self::Mark => "mark",
            Self::AyahNumber => "ayah_number",
            Self::AyahMarkOrnament => "ayah_mark_ornament",
            Self::HeaderInk => "header_ink",
            Self::Ornament => "ornament",
            Self::PageNumber => "page_number",
            Self::RunningHead => "running_head",
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
    Tanwin = 2, // "diacritic tanwin"
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
            2 => Self::Tanwin,
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
            "diacritic tanwin" => Self::Tanwin,
            "dots" => Self::Dots,
            "waqf" => Self::Waqf,
            "sifr" => Self::Sifr,
            "sajdah" => Self::Sajdah,
            "reading_sign" => Self::ReadingSign,
            _ => Self::Other,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Diacritic => "diacritic",
            Self::Tanwin => "diacritic tanwin",
            Self::Dots => "dots",
            Self::Waqf => "waqf",
            Self::Sifr => "sifr",
            Self::Sajdah => "sajdah",
            Self::ReadingSign => "reading_sign",
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
    Fathah = 1 => "fathah",
    Kasrah = 2 => "kasrah",
    Dammah = 3 => "dammah",
    TanwinAlFath = 4 => "tanwin_al_fath",
    TanwinAlKasr = 5 => "tanwin_al_kasr",
    TanwinAlDamm = 6 => "tanwin_al_damm",
    Shaddah = 7 => "shaddah",
    Sukun = 8 => "sukun",
    Maddah = 9 => "maddah",
    Hamzah = 10 => "hamzah",
    HamzatAlWasl = 11 => "hamzat_al_wasl",
    OmittedAlif = 12 => "omitted_alif",
    SmallWaw = 13 => "small_waw",
    SmallYaa = 14 => "small_yaa",
    SmallNoon = 15 => "small_noon",
    Dot = 16 => "dot",
    TwoDots = 17 => "two_dots",
    ThreeDots = 18 => "three_dots",
    RoundedZero = 19 => "rounded_zero",
    RectangularZero = 20 => "rectangular_zero",
    WaqfJaizMustawiAlTarafayn = 21 => "waqf_jaiz_mustawi_al_tarafayn",
    WaqfJaizWaqfAwla = 22 => "waqf_jaiz_waqf_awla",
    WaqfJaizWaslAwla = 23 => "waqf_jaiz_wasl_awla",
    WaqfLazim = 24 => "waqf_lazim",
    WaqfAlMuanaqah = 25 => "waqf_al_muanaqah",
    Saktah = 26 => "saktah",
    SmallMeem = 27 => "small_meem",
    Hizb = 28 => "hizb",
    Sajdah = 29 => "sajdah",
    SajdahMark = 30 => "sajdah_mark",
    SajdahLine = 31 => "sajdah_line",
    SeenAlQiraah = 32 => "seen_al_qiraah",
    Tashil = 33 => "tashil",
    Ishmam = 34 => "ishmam",
    Imalah = 35 => "imalah",
    Unknown = 255 => "unknown",
}

/// Non-word groups on a page.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
#[derive(Default)]
pub enum DecoKind {
    AyahMark = 0,
    SurahName = 1,
    Basmalah = 2,
    DivisionMark = 3,
    SajdahMark = 4,
    PageNumber = 5,
    RunningHead = 6,
    #[default]
    Other = 255,
}

impl DecoKind {
    pub fn from_u8(v: u8) -> Self {
        match v {
            0 => Self::AyahMark,
            1 => Self::SurahName,
            2 => Self::Basmalah,
            3 => Self::DivisionMark,
            4 => Self::SajdahMark,
            5 => Self::PageNumber,
            6 => Self::RunningHead,
            _ => Self::Other,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AyahMark => "ayah-mark",
            Self::SurahName => "surah-name",
            Self::Basmalah => "basmalah",
            Self::DivisionMark => "division-mark",
            Self::SajdahMark => "sajdah-mark",
            Self::PageNumber => "page-number",
            Self::RunningHead => "running-head",
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
/// `data-duplicate`: the artwork draws this ornament twice, in place (pages 1-2).
pub const PF_DUPLICATE: u8 = 32;

/// Ayah flag bits.
pub const AF_JUZ_START: u8 = 1;
pub const AF_HIZB_START: u8 = 2;
pub const AF_RUBU_AL_HIZB_START: u8 = 4;
pub const AF_NISF_START: u8 = 8;

// ───────────────────────────── records ─────────────────────────────

/// Integer bbox in quantised page units, inclusive min / exclusive max not
/// enforced. Treat it as the rectangle `[x0, x1] × [y0, y1]`.
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
    pub surah: u16,
    pub ayah: u16,
    pub fragment: u8,
    pub fragments: u8,
    pub flags: u8,
    pub first_word: u16,
    pub n_words: u16,
    /// Index into decos, or NONE_U16.
    pub ayah_mark_deco: u16,
    /// `rubu_al_hizb` number (1..240) that starts at this ayah when any AF_*_START flag is
    /// set, else 0. juz = (rubu_al_hizb-1)/8+1, hizb = (rubu_al_hizb-1)/4+1, nisf = (rubu_al_hizb-1)/2+1.
    pub rubu_al_hizb: u16,
    pub bbox: IBox,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct WordRec {
    pub surah: u16,
    pub ayah: u16,
    pub word: u16,
    pub line_idx: u16,
    pub ayah_idx: u16,
    /// Index into strings (rasm_uthmani text) or NONE_U16.
    pub text: u16,
    /// Other text forms (rasm_imlai, qpc, rasm, search): string index or NONE_U16.
    pub rasm_imlai: u16,
    pub qpc: u16,
    pub rasm: u16,
    pub search: u16,
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
    pub surah: u16,
    pub ayah: u16,
    /// Index into strings (e.g. surah Arabic name / hizb info), or NONE_U16.
    pub text: u16,
    pub first_path: u32,
    pub n_paths: u16,
    /// Line the decoration sits in (surah header, basmalah, hizb mark), or NONE_U16.
    pub line: u16,
    pub bbox: IBox,
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
    Bounds(&'static str),
    BadVersion(u16),
    Truncated(&'static str),
    Corrupt(&'static str),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BadMagic => write!(f, "not a QVP file"),
            Error::Bounds(w) => write!(f, "index out of range at {w}"),
            Error::BadVersion(v) => write!(f, "unsupported QVP version {v}"),
            Error::Truncated(w) => write!(f, "truncated at {w}"),
            Error::Corrupt(w) => write!(f, "corrupt: {w}"),
        }
    }
}
impl std::error::Error for Error {}

// ───────────────────────────── encode ─────────────────────────────

pub fn encode(p: &PageData) -> Vec<u8> {
    codec::encode(p)
}

// ───────────────────────────── decode ─────────────────────────────

pub fn decode(bytes: &[u8]) -> Result<PageData, Error> {
    if bytes.len() < 6 || &bytes[0..4] != MAGIC {
        return Err(Error::BadMagic);
    }
    match u16::from_le_bytes([bytes[4], bytes[5]]) {
        VERSION => codec::decode(bytes),
        v => Err(Error::BadVersion(v)),
    }
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

impl PageData {
    /// Put the opcode stream in the order the codec writes it: every inline path's run
    /// in path order, then every glyph outline in glyph order. The converter appends a
    /// glyph outline the moment it first sees one, so a freshly built page interleaves
    /// them; the codec reconstructs the canonical order, and `decode(encode(p)) == p`
    /// only holds for a page already in it.
    pub fn canonicalize_ops(&mut self) {
        let mut ops = Vec::with_capacity(self.ops.len());
        for p in self.paths.iter_mut() {
            if p.flags & PF_GLYPH != 0 {
                continue;
            }
            let run = &self.ops[p.op_off as usize..(p.op_off + p.op_len) as usize];
            let off = ops.len() as u32;
            ops.extend_from_slice(run);
            p.op_off = off;
        }
        for g in self.glyphs.iter_mut() {
            let run = &self.ops[g.op_off as usize..(g.op_off + g.op_len) as usize];
            let off = ops.len() as u32;
            ops.extend_from_slice(run);
            g.op_off = off;
        }
        self.ops = ops;
    }
}

/// Format quantised commands as an SVG `d` string (exact decimals, trimmed).
pub fn svg_path_d(cmds: &[Cmd], quant: u16) -> String {
    use std::fmt::Write;
    fn num(v: i32, quant: u16, out: &mut String) {
        let q = quant as i32;
        let a = v.abs();
        if v < 0 {
            out.push('-');
        }
        write!(out, "{}", a / q).unwrap();
        let fp = a % q;
        if fp != 0 {
            let digits = (quant as f64).log10().ceil() as usize;
            let s = format!("{fp:0digits$}");
            out.push('.');
            out.push_str(s.trim_end_matches('0'));
        }
    }
    let mut d = String::new();
    let pt = |d: &mut String, x: i32, y: i32| {
        num(x, quant, d);
        d.push(' ');
        num(y, quant, d);
    };
    for c in cmds {
        match *c {
            Cmd::MoveTo(x, y) => {
                d.push('M');
                pt(&mut d, x, y);
            }
            Cmd::LineTo(x, y) => {
                d.push('L');
                pt(&mut d, x, y);
            }
            Cmd::QuadTo(x1, y1, x, y) => {
                d.push('Q');
                pt(&mut d, x1, y1);
                d.push(' ');
                pt(&mut d, x, y);
            }
            Cmd::CubicTo(x1, y1, x2, y2, x, y) => {
                d.push('C');
                pt(&mut d, x1, y1);
                d.push(' ');
                pt(&mut d, x2, y2);
                d.push(' ');
                pt(&mut d, x, y);
            }
            Cmd::Close => d.push('Z'),
        }
    }
    d
}

/// Mark taxonomy category (`mark-taxonomy` v2 of the quran-svg pipeline).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Category {
    None = 0,
    Harakah = 1,
    Tanwin = 2,
    LetterDot = 3,
    Orthographic = 4,
    Dabt = 5,
    Waqf = 6,
    ReadingSign = 7,
    Standalone = 8,
}

impl Category {
    pub fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Harakah,
            2 => Self::Tanwin,
            3 => Self::LetterDot,
            4 => Self::Orthographic,
            5 => Self::Dabt,
            6 => Self::Waqf,
            7 => Self::ReadingSign,
            8 => Self::Standalone,
            _ => Self::None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "",
            Self::Harakah => "harakah",
            Self::Tanwin => "tanwin",
            Self::LetterDot => "letter_dot",
            Self::Orthographic => "orthographic",
            Self::Dabt => "dabt",
            Self::Waqf => "waqf",
            Self::ReadingSign => "reading_sign",
            Self::Standalone => "standalone",
        }
    }
}

impl Mark {
    pub fn category(self) -> Category {
        use Mark::*;
        match self {
            Fathah | Kasrah | Dammah | Sukun | Shaddah => Category::Harakah,
            TanwinAlFath | TanwinAlKasr | TanwinAlDamm => Category::Tanwin,
            Dot | TwoDots | ThreeDots => Category::LetterDot,
            Hamzah | HamzatAlWasl | OmittedAlif | Maddah | SmallWaw | SmallYaa | SmallNoon => Category::Orthographic,
            RoundedZero | RectangularZero | SmallMeem => Category::Dabt,
            WaqfJaizMustawiAlTarafayn | WaqfJaizWaslAwla | WaqfJaizWaqfAwla | WaqfLazim | WaqfAlMuanaqah => {
                Category::Waqf
            }
            Saktah | SeenAlQiraah | Imalah | Ishmam | Tashil => Category::ReadingSign,
            SajdahMark | SajdahLine | Sajdah | Hizb => Category::Standalone,
            None | Unknown => Category::None,
        }
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
            ayahs: vec![AyahRec {
                surah: 2,
                ayah: 3,
                fragment: 1,
                fragments: 1,
                flags: AF_RUBU_AL_HIZB_START,
                first_word: 0,
                n_words: 1,
                ayah_mark_deco: 0,
                rubu_al_hizb: 5,
                bbox: bb,
            }],
            words: vec![WordRec {
                surah: 2,
                ayah: 3,
                word: 1,
                line_idx: 0,
                ayah_idx: 0,
                text: 0,
                rasm_imlai: NONE_U16,
                qpc: NONE_U16,
                rasm: NONE_U16,
                search: 0,
                first_path: 0,
                n_paths: 1,
                bbox: bb,
            }],
            paths: vec![
                PathRec {
                    kind: PathKind::Body,
                    mark: Mark::None,
                    family: Family::None,
                    flags: PF_EVENODD,
                    ox: 500,
                    oy: 600,
                    op_off: 0,
                    op_len: ops.len() as u32,
                    bbox: bb,
                },
                PathRec {
                    kind: PathKind::AyahMarkOrnament,
                    mark: Mark::None,
                    family: Family::None,
                    flags: 0,
                    ox: 500,
                    oy: 600,
                    op_off: 0,
                    op_len: ops.len() as u32,
                    bbox: bb,
                },
            ],
            decos: vec![DecoRec {
                kind: DecoKind::AyahMark,
                surah: 2,
                ayah: 3,
                text: NONE_U16,
                first_path: 1,
                n_paths: 1,
                line: 0,
                bbox: bb,
            }],
            glyphs: vec![GlyphRec { op_off: 0, op_len: ops.len() as u32, bbox: bb }],
            insts: vec![InstRec { glyph: 0, a: 2.0, b: 0.0, c: 0.0, d: 2.0, e: 10.0, f: 20.0 }],
            ops,
            strings: vec!["ذَٰلِكَ".to_owned()],
        };
        let mut p = p;
        p.paths.push(PathRec {
            kind: PathKind::AyahMarkOrnament,
            mark: Mark::None,
            family: Family::None,
            flags: PF_GLYPH,
            ox: 0,
            oy: 0,
            op_off: 0,
            op_len: 0,
            bbox: bb,
        });
        // `encode` expects converter-shaped input: one contiguous run per path.
        p.canonicalize_ops();
        let bytes = encode(&p);
        let q = decode(&bytes).unwrap();
        assert_eq!(p, q);
        assert_eq!(decode(b"QVP1\x09\x00xx").unwrap_err(), Error::BadVersion(9));
        // instance: shared ops decode at glyph origin (0,0) → page (10,20) → quantised 1000,2000
        assert_eq!(q.path_cmds(2).unwrap()[0], Cmd::MoveTo(1000, 2000));
        assert_eq!(q.path_cmds(0).unwrap(), cmds.to_vec());
        assert!(matches!(decode(&bytes[..bytes.len() - 3]), Err(Error::Truncated(_))));
        assert_eq!(decode(b"nope").unwrap_err(), Error::BadMagic);
    }
}
