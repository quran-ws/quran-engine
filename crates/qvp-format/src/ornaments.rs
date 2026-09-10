//! QVO1: an *ornament set* — the ayah medallions, surah bands and page frames
//! of other printed mushafs, converted from the `quran-ws/quran-assets` catalogue
//! so the engine can dress a page in them without an XML parser at runtime.
//!
//! One file holds several styles (one per mushaf). A style carries a palette of
//! named parts, up to three assets, and — when the publisher's frame tiles — the
//! corner and the two repeat units it is assembled from.
//!
//! Assets keep their own coordinate system: outlines are in the asset's own
//! viewBox units, quantised by `quant`, and [`crate::ornaments::Asset::view_box`]
//! is the box they are drawn in. Placement is the engine's job, not the file's.
use crate::{cmds_bbox, decode_cmds, encode_cmds, read_varint, write_varint, zigzag, unzigzag, Cmd, Error, IBox};

pub const ORNAMENTS_MAGIC: &[u8; 4] = b"QVO1";
/// 1/1000 of an asset unit. Assets are normalised to 100 units tall, and a
/// frame slice is only ~6 units across, so the page's 1/100 would be coarse.
pub const ORNAMENT_QUANT: u16 = 1000;

/// Which piece of page furniture an asset replaces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum OrnamentKind {
    /// The ring around an ayah number (`ayah-markers` in the catalogue).
    AyahMark = 0,
    /// The band a surah name is printed in (`surah-headers`).
    SurahHeader = 1,
    /// The border around the text area (`page-frames`).
    PageFrame = 2,
}

impl OrnamentKind {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::AyahMark),
            1 => Some(Self::SurahHeader),
            2 => Some(Self::PageFrame),
            _ => None,
        }
    }
    /// The engine's name for it.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AyahMark => "ayah_mark",
            Self::SurahHeader => "surah_header",
            Self::PageFrame => "page_frame",
        }
    }
    /// The catalogue's name for the same thing.
    pub fn catalog_type(self) -> &'static str {
        match self {
            Self::AyahMark => "ayah-markers",
            Self::SurahHeader => "surah-headers",
            Self::PageFrame => "page-frames",
        }
    }
    pub const ALL: [OrnamentKind; 3] = [Self::AyahMark, Self::SurahHeader, Self::PageFrame];
}

/// One printed colour of a design. Every outline belongs to exactly one.
///
/// `slot` is the transparent window the design leaves for the thing it frames —
/// the number, the surah name, the text area. It has no colour of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub name: String,
    /// The mushaf's own ink as 0xRRGGBBAA; alpha 0 when the design leaves it open.
    pub color: u32,
    /// Painted as a stroke rather than a fill (the `line` part).
    pub stroke: bool,
}

/// Path outline flags.
pub const OPF_EVENODD: u8 = 1;
pub const OPF_STROKE: u8 = 2;

/// One outline of an asset, in the asset's own units, quantised.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrnamentPath {
    /// Index into [`Style::parts`].
    pub part: u16,
    pub flags: u8,
    /// Stroke width in quantised asset units (0 when filled).
    pub stroke_width: i32,
    pub op_off: u32,
    pub op_len: u32,
    pub bbox: IBox,
}

/// A drawing with its own viewBox, and the window it leaves open.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Asset {
    /// `x, y, w, h` of the asset's viewBox, in asset units (not quantised).
    pub view_box: [f32; 4],
    /// `data-slot`: the transparent window, in the same units.
    pub slot: Option<[f32; 4]>,
    pub first_path: u32,
    pub n_paths: u32,
}

impl Asset {
    pub fn width(&self) -> f32 {
        self.view_box[2]
    }
    pub fn height(&self) -> f32 {
        self.view_box[3]
    }
}

/// How a page frame is assembled when it tiles: one corner and two repeat units.
///
/// Stretching a single drawing onto a page of a different aspect makes the side
/// borders fatter than the top and bottom; keeping the corner and repeating only
/// the straight run does not.
#[derive(Clone, Debug, PartialEq)]
pub struct Slices {
    /// Corner size in frame units.
    pub corner_w: f32,
    pub corner_h: f32,
    /// Repeat unit length along the horizontal and the vertical run.
    pub repeat_h: f32,
    pub repeat_v: f32,
    /// The other three corners are 180°-rotated copies rather than mirrored ones.
    pub rotate: bool,
    pub corner: Asset,
    pub edge_h: Asset,
    pub edge_v: Asset,
}

/// What the assets may be redistributed under. Traced ornaments belong to the
/// mushaf's publisher; this travels with the file so a host can say so on screen.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct License {
    pub id: String,
    /// "provisional" while written permission is being sought.
    pub status: String,
    pub redistributable: bool,
    pub attribution: String,
}

/// One mushaf's ornaments.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Style {
    /// The catalogue's style key, e.g. "qalon".
    pub name: String,
    /// The riwayah the mushaf prints, when the catalogue states one.
    pub riwayah: String,
    pub license: License,
    /// The union of the parts its assets draw, in printing order.
    pub parts: Vec<Part>,
    pub ayah_mark: Option<Asset>,
    pub surah_header: Option<Asset>,
    pub page_frame: Option<Asset>,
    pub slices: Option<Slices>,
}

impl Style {
    pub fn asset(&self, kind: OrnamentKind) -> Option<&Asset> {
        match kind {
            OrnamentKind::AyahMark => self.ayah_mark.as_ref(),
            OrnamentKind::SurahHeader => self.surah_header.as_ref(),
            OrnamentKind::PageFrame => self.page_frame.as_ref(),
        }
    }
    pub fn part_index(&self, name: &str) -> Option<u16> {
        self.parts.iter().position(|p| p.name == name).map(|i| i as u16)
    }
}

/// A whole ornament set: several styles sharing one outline pool.
#[derive(Clone, Debug, PartialEq)]
pub struct OrnamentSet {
    pub quant: u16,
    pub styles: Vec<Style>,
    pub paths: Vec<OrnamentPath>,
    pub ops: Vec<u8>,
}

impl Default for OrnamentSet {
    fn default() -> Self {
        OrnamentSet { quant: ORNAMENT_QUANT, styles: vec![], paths: vec![], ops: vec![] }
    }
}

impl OrnamentSet {
    pub fn style(&self, name: &str) -> Option<&Style> {
        self.styles.iter().find(|s| s.name == name)
    }
    pub fn path_cmds(&self, i: usize) -> Result<Vec<Cmd>, Error> {
        let p = self.paths.get(i).ok_or(Error::Corrupt("ornament path index"))?;
        let end = (p.op_off + p.op_len) as usize;
        let ops = self.ops.get(p.op_off as usize..end).ok_or(Error::Truncated("ornament ops"))?;
        decode_cmds(ops, 0, 0)
    }
    /// Append one outline; returns its index.
    pub fn push_path(&mut self, part: u16, flags: u8, stroke_width: i32, cmds: &[Cmd]) -> u32 {
        let op_off = self.ops.len() as u32;
        let bbox = encode_cmds(cmds, 0, 0, &mut self.ops);
        let op_len = self.ops.len() as u32 - op_off;
        self.paths.push(OrnamentPath { part, flags, stroke_width, op_off, op_len, bbox });
        (self.paths.len() - 1) as u32
    }

    pub fn encode(&self) -> Vec<u8> {
        let mut o = Vec::with_capacity(64 * 1024);
        o.extend_from_slice(ORNAMENTS_MAGIC);
        o.extend_from_slice(&self.quant.to_le_bytes());
        write_varint(&mut o, self.styles.len() as u32);
        for s in &self.styles {
            put_str(&mut o, &s.name);
            put_str(&mut o, &s.riwayah);
            put_str(&mut o, &s.license.id);
            put_str(&mut o, &s.license.status);
            write_varint(&mut o, s.license.redistributable as u32);
            put_str(&mut o, &s.license.attribution);
            write_varint(&mut o, s.parts.len() as u32);
            for p in &s.parts {
                put_str(&mut o, &p.name);
                o.extend_from_slice(&p.color.to_le_bytes());
                write_varint(&mut o, p.stroke as u32);
            }
            let mask = (s.ayah_mark.is_some() as u32) | (s.surah_header.is_some() as u32) << 1 | (s.page_frame.is_some() as u32) << 2 | (s.slices.is_some() as u32) << 3;
            write_varint(&mut o, mask);
            for a in [&s.ayah_mark, &s.surah_header, &s.page_frame].into_iter().flatten() {
                put_asset(&mut o, a);
            }
            if let Some(sl) = &s.slices {
                for v in [sl.corner_w, sl.corner_h, sl.repeat_h, sl.repeat_v] {
                    o.extend_from_slice(&v.to_le_bytes());
                }
                write_varint(&mut o, sl.rotate as u32);
                put_asset(&mut o, &sl.corner);
                put_asset(&mut o, &sl.edge_h);
                put_asset(&mut o, &sl.edge_v);
            }
        }
        write_varint(&mut o, self.paths.len() as u32);
        for p in &self.paths {
            write_varint(&mut o, p.part as u32);
            o.push(p.flags);
            write_varint(&mut o, zigzag(p.stroke_width));
            write_varint(&mut o, p.op_off);
            write_varint(&mut o, p.op_len);
        }
        write_varint(&mut o, self.ops.len() as u32);
        o.extend_from_slice(&self.ops);
        o
    }

    pub fn decode(b: &[u8]) -> Result<OrnamentSet, Error> {
        if b.len() < 6 || &b[0..4] != ORNAMENTS_MAGIC {
            return Err(Error::BadMagic);
        }
        let quant = u16::from_le_bytes([b[4], b[5]]);
        if quant == 0 {
            return Err(Error::Corrupt("ornament quant"));
        }
        let mut pos = 6;
        let n_styles = read_varint(b, &mut pos)? as usize;
        let mut styles = Vec::with_capacity(n_styles);
        for _ in 0..n_styles {
            let name = get_str(b, &mut pos)?;
            let riwayah = get_str(b, &mut pos)?;
            let license = License {
                id: get_str(b, &mut pos)?,
                status: get_str(b, &mut pos)?,
                redistributable: read_varint(b, &mut pos)? != 0,
                attribution: get_str(b, &mut pos)?,
            };
            let n_parts = read_varint(b, &mut pos)? as usize;
            let mut parts = Vec::with_capacity(n_parts);
            for _ in 0..n_parts {
                let name = get_str(b, &mut pos)?;
                let color = get_u32(b, &mut pos)?;
                parts.push(Part { name, color, stroke: read_varint(b, &mut pos)? != 0 });
            }
            let mask = read_varint(b, &mut pos)?;
            let opt = |bit: u32, pos: &mut usize| -> Result<Option<Asset>, Error> { if mask & bit != 0 { Ok(Some(get_asset(b, pos)?)) } else { Ok(None) } };
            let ayah_mark = opt(1, &mut pos)?;
            let surah_header = opt(2, &mut pos)?;
            let page_frame = opt(4, &mut pos)?;
            let slices = if mask & 8 != 0 {
                let mut f = [0f32; 4];
                for v in f.iter_mut() {
                    *v = get_f32(b, &mut pos)?;
                }
                let rotate = read_varint(b, &mut pos)? != 0;
                Some(Slices { corner_w: f[0], corner_h: f[1], repeat_h: f[2], repeat_v: f[3], rotate, corner: get_asset(b, &mut pos)?, edge_h: get_asset(b, &mut pos)?, edge_v: get_asset(b, &mut pos)? })
            } else {
                None
            };
            styles.push(Style { name, riwayah, license, parts, ayah_mark, surah_header, page_frame, slices });
        }
        let n_paths = read_varint(b, &mut pos)? as usize;
        let mut paths = Vec::with_capacity(n_paths);
        for _ in 0..n_paths {
            let part = read_varint(b, &mut pos)? as u16;
            let flags = *b.get(pos).ok_or(Error::Truncated("ornament path flags"))?;
            pos += 1;
            let stroke_width = unzigzag(read_varint(b, &mut pos)?);
            let op_off = read_varint(b, &mut pos)?;
            let op_len = read_varint(b, &mut pos)?;
            paths.push(OrnamentPath { part, flags, stroke_width, op_off, op_len, bbox: IBox::EMPTY });
        }
        let n_ops = read_varint(b, &mut pos)? as usize;
        let ops = b.get(pos..pos + n_ops).ok_or(Error::Truncated("ornament ops"))?.to_vec();
        let mut set = OrnamentSet { quant, styles, paths, ops };
        for i in 0..set.paths.len() {
            let bb = cmds_bbox(&set.path_cmds(i)?);
            set.paths[i].bbox = bb;
        }
        Ok(set)
    }
}

fn put_str(o: &mut Vec<u8>, s: &str) {
    write_varint(o, s.len() as u32);
    o.extend_from_slice(s.as_bytes());
}

fn put_asset(o: &mut Vec<u8>, a: &Asset) {
    for v in a.view_box {
        o.extend_from_slice(&v.to_le_bytes());
    }
    match a.slot {
        Some(s) => {
            write_varint(o, 1);
            for v in s {
                o.extend_from_slice(&v.to_le_bytes());
            }
        }
        None => write_varint(o, 0),
    }
    write_varint(o, a.first_path);
    write_varint(o, a.n_paths);
}

fn get_str(b: &[u8], pos: &mut usize) -> Result<String, Error> {
    let len = read_varint(b, pos)? as usize;
    let s = b.get(*pos..*pos + len).ok_or(Error::Truncated("ornament string"))?;
    *pos += len;
    Ok(std::str::from_utf8(s).map_err(|_| Error::Corrupt("utf8"))?.to_owned())
}

fn get_u32(b: &[u8], pos: &mut usize) -> Result<u32, Error> {
    let s = b.get(*pos..*pos + 4).ok_or(Error::Truncated("ornament u32"))?;
    *pos += 4;
    Ok(u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

fn get_f32(b: &[u8], pos: &mut usize) -> Result<f32, Error> {
    Ok(f32::from_bits(get_u32(b, pos)?))
}

fn get_asset(b: &[u8], pos: &mut usize) -> Result<Asset, Error> {
    let mut view_box = [0f32; 4];
    for v in view_box.iter_mut() {
        *v = get_f32(b, pos)?;
    }
    let slot = if read_varint(b, pos)? != 0 {
        let mut s = [0f32; 4];
        for v in s.iter_mut() {
            *v = get_f32(b, pos)?;
        }
        Some(s)
    } else {
        None
    };
    Ok(Asset { view_box, slot, first_path: read_varint(b, pos)?, n_paths: read_varint(b, pos)? })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> OrnamentSet {
        let mut set = OrnamentSet::default();
        let first = set.paths.len() as u32;
        set.push_path(1, 0, 0, &[Cmd::MoveTo(0, 0), Cmd::LineTo(1000, 0), Cmd::LineTo(1000, 1000), Cmd::Close]);
        set.push_path(2, OPF_EVENODD | OPF_STROKE, 42, &[Cmd::MoveTo(10, 10), Cmd::CubicTo(20, 20, 30, 30, 40, 40)]);
        set.styles.push(Style {
            name: "qalon".into(),
            riwayah: "Qalun 'an Nafi'".into(),
            license: License { id: "CC-BY-NC-SA-4.0".into(), status: "provisional".into(), redistributable: false, attribution: "Traced from a printed mushaf".into() },
            parts: vec![
                Part { name: "slot".into(), color: 0, stroke: false },
                Part { name: "c1".into(), color: 0xffffffff, stroke: false },
                Part { name: "line".into(), color: 0x303641ff, stroke: true },
            ],
            ayah_mark: Some(Asset { view_box: [0.0, 0.0, 73.162, 100.0], slot: Some([13.2353, 23.1618, 46.3235, 54.0441]), first_path: first, n_paths: 2 }),
            surah_header: None,
            page_frame: None,
            slices: None,
        });
        set
    }

    #[test]
    fn round_trips() {
        let a = sample();
        let b = OrnamentSet::decode(&a.encode()).unwrap();
        assert_eq!(a, b);
        assert_eq!(b.style("qalon").unwrap().part_index("line"), Some(2));
        assert_eq!(b.path_cmds(1).unwrap().len(), 2);
    }

    #[test]
    fn rejects_foreign_bytes() {
        assert!(matches!(OrnamentSet::decode(b"QVP1\x64\x00").unwrap_err(), Error::BadMagic));
    }
}
