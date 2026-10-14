//! QVO1: an ornament set. The ayah medallions, surah bands and page frames of other
//! printed mushafs and fonts, converted from the `quran-ws/quran-assets` catalogue so
//! the engine can dress a page in them without an XML parser at runtime.
//!
//! One file holds many ornaments, one per catalogue asset. An ornament carries its
//! kind, style, lineage, riwayah, licence, palette and one asset. A page frame that
//! tiles also carries the corner and the two repeat units it is assembled from.
//!
//! Outlines are in the asset's own viewBox units, quantised by `quant`. Placement is
//! the engine's job, not the file's.
use crate::{cmds_bbox, decode_cmds, encode_cmds, read_varint, unzigzag, write_varint, zigzag, Cmd, Error, IBox};

pub const ORNAMENTS_MAGIC: &[u8; 4] = b"QVO1";
/// 1/1000 of an asset unit. Assets are 100 units tall and a frame slice is about 6
/// units across, so the page's 1/100 would be coarse.
pub const ORNAMENT_QUANT: u16 = 1000;

/// Which piece of page furniture an ornament replaces.
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

/// Where the artwork came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Lineage {
    /// Traced from a scan of a printed mushaf.
    Scan = 0,
    /// Taken from a glyph of a font.
    Font = 1,
}

impl Lineage {
    pub fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Scan),
            1 => Some(Self::Font),
            _ => None,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Scan => "scan",
            Self::Font => "font",
        }
    }
}

/// One printed colour of a design. Every outline belongs to exactly one.
///
/// `slot` is the transparent window the design leaves for the thing it frames. It has
/// no colour of its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Part {
    pub name: String,
    /// 0xRRGGBBAA; alpha 0 when the design leaves it open.
    pub color: u32,
    /// Painted as a stroke rather than a fill (the `line` part).
    pub stroke: bool,
}

pub const OPF_EVENODD: u8 = 1;
pub const OPF_STROKE: u8 = 2;

/// One outline of an asset, in the asset's own units, quantised.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrnamentPath {
    /// Index into [`Ornament::parts`].
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
    /// `x, y, w, h` of the viewBox, in asset units (not quantised).
    pub view_box: [f32; 4],
    /// The catalogue's slot: the number box, the name box or the text area, in the
    /// same units.
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
    /// The centre of the slot, or of the viewBox when the asset has no slot.
    pub fn slot_centre(&self) -> (f32, f32) {
        let b = self.slot.unwrap_or(self.view_box);
        (b[0] + b[2] / 2.0, b[1] + b[3] / 2.0)
    }
}

/// How a page frame is assembled when it tiles: one corner and two repeat units.
///
/// Stretching one drawing onto a page of a different aspect makes the side borders
/// fatter than the top and bottom. Keeping the corner and repeating only the straight
/// run does not.
#[derive(Clone, Debug, PartialEq)]
pub struct Slices {
    pub corner_w: f32,
    pub corner_h: f32,
    pub repeat_h: f32,
    pub repeat_v: f32,
    /// When true, the top-right and bottom-left corners are 180° turns of the corner and the
    /// bottom-right is a plain copy. When false, the other three corners are mirror images.
    /// Rotate turns the corners only. The straight runs are always mirrored, as the
    /// catalogue assembles them.
    pub rotate: bool,
    pub corner: Asset,
    pub edge_h: Asset,
    pub edge_v: Asset,
}

/// What the artwork may be redistributed under. It travels with the file so a host can
/// say so on screen.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct License {
    pub id: String,
    /// "confirmed", "provisional" or "pending".
    pub status: String,
    /// True when the status is "confirmed".
    pub redistributable: bool,
    pub attribution: String,
}

/// One catalogue asset: a kind, a style and one drawing.
#[derive(Clone, Debug, PartialEq)]
pub struct Ornament {
    pub kind: OrnamentKind,
    /// The catalogue's style key, e.g. "mushaf-qalon" or "font-003-regular".
    pub style: String,
    pub lineage: Lineage,
    /// The riwayah the mushaf prints; empty for a font.
    pub riwayah: String,
    pub license: License,
    /// The palette, in printing order.
    pub parts: Vec<Part>,
    pub asset: Asset,
    /// Only a page frame that tiles has these.
    pub slices: Option<Slices>,
}

impl Ornament {
    pub fn part_index(&self, name: &str) -> Option<u16> {
        self.parts.iter().position(|p| p.name == name).and_then(|i| u16::try_from(i).ok())
    }
}

/// A whole ornament set: many ornaments sharing one outline pool.
#[derive(Clone, Debug, PartialEq)]
pub struct OrnamentSet {
    pub quant: u16,
    pub ornaments: Vec<Ornament>,
    pub paths: Vec<OrnamentPath>,
    pub ops: Vec<u8>,
}

impl Default for OrnamentSet {
    fn default() -> Self {
        OrnamentSet { quant: ORNAMENT_QUANT, ornaments: vec![], paths: vec![], ops: vec![] }
    }
}

impl OrnamentSet {
    /// The index of the ornament of `kind` in `style`.
    pub fn find(&self, kind: OrnamentKind, style: &str) -> Option<usize> {
        self.ornaments.iter().position(|o| o.kind == kind && o.style == style)
    }
    pub fn path_cmds(&self, i: usize) -> Result<Vec<Cmd>, Error> {
        let p = self.paths.get(i).ok_or(Error::Corrupt("ornament path index"))?;
        let end = p.op_off.checked_add(p.op_len).ok_or(Error::Corrupt("ornament op range"))? as usize;
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
        write_varint(&mut o, self.ornaments.len() as u32);
        for orn in &self.ornaments {
            o.push(orn.kind as u8);
            o.push(orn.lineage as u8);
            put_str(&mut o, &orn.style);
            put_str(&mut o, &orn.riwayah);
            put_str(&mut o, &orn.license.id);
            put_str(&mut o, &orn.license.status);
            write_varint(&mut o, orn.license.redistributable as u32);
            put_str(&mut o, &orn.license.attribution);
            write_varint(&mut o, orn.parts.len() as u32);
            for p in &orn.parts {
                put_str(&mut o, &p.name);
                o.extend_from_slice(&p.color.to_le_bytes());
                write_varint(&mut o, p.stroke as u32);
            }
            put_asset(&mut o, &orn.asset);
            match &orn.slices {
                Some(sl) => {
                    write_varint(&mut o, 1);
                    for v in [sl.corner_w, sl.corner_h, sl.repeat_h, sl.repeat_v] {
                        o.extend_from_slice(&v.to_le_bytes());
                    }
                    write_varint(&mut o, sl.rotate as u32);
                    put_asset(&mut o, &sl.corner);
                    put_asset(&mut o, &sl.edge_h);
                    put_asset(&mut o, &sl.edge_v);
                }
                None => write_varint(&mut o, 0),
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
        let n = read_varint(b, &mut pos)? as usize;
        let mut ornaments = Vec::with_capacity(n.min(4096));
        for _ in 0..n {
            let kind = OrnamentKind::from_u8(get_u8(b, &mut pos)?).ok_or(Error::Corrupt("ornament kind"))?;
            let lineage = Lineage::from_u8(get_u8(b, &mut pos)?).ok_or(Error::Corrupt("ornament lineage"))?;
            let style = get_str(b, &mut pos)?;
            let riwayah = get_str(b, &mut pos)?;
            let license = License {
                id: get_str(b, &mut pos)?,
                status: get_str(b, &mut pos)?,
                redistributable: read_varint(b, &mut pos)? != 0,
                attribution: get_str(b, &mut pos)?,
            };
            let n_parts = read_varint(b, &mut pos)? as usize;
            let mut parts = Vec::with_capacity(n_parts.min(256));
            for _ in 0..n_parts {
                let name = get_str(b, &mut pos)?;
                let color = get_u32(b, &mut pos)?;
                parts.push(Part { name, color, stroke: read_varint(b, &mut pos)? != 0 });
            }
            let asset = get_asset(b, &mut pos)?;
            let slices = if read_varint(b, &mut pos)? != 0 {
                let mut f = [0f32; 4];
                for v in f.iter_mut() {
                    *v = get_f32(b, &mut pos)?;
                }
                let rotate = read_varint(b, &mut pos)? != 0;
                Some(Slices {
                    corner_w: f[0],
                    corner_h: f[1],
                    repeat_h: f[2],
                    repeat_v: f[3],
                    rotate,
                    corner: get_asset(b, &mut pos)?,
                    edge_h: get_asset(b, &mut pos)?,
                    edge_v: get_asset(b, &mut pos)?,
                })
            } else {
                None
            };
            ornaments.push(Ornament { kind, style, lineage, riwayah, license, parts, asset, slices });
        }
        let n_paths = read_varint(b, &mut pos)? as usize;
        let mut paths = Vec::with_capacity(n_paths.min(1 << 20));
        for _ in 0..n_paths {
            let part = u16::try_from(read_varint(b, &mut pos)?).map_err(|_| Error::Corrupt("ornament path part"))?;
            let flags = get_u8(b, &mut pos)?;
            let stroke_width = unzigzag(read_varint(b, &mut pos)?);
            let op_off = read_varint(b, &mut pos)?;
            let op_len = read_varint(b, &mut pos)?;
            paths.push(OrnamentPath { part, flags, stroke_width, op_off, op_len, bbox: IBox::EMPTY });
        }
        let n_ops = read_varint(b, &mut pos)? as usize;
        let end = pos.checked_add(n_ops).ok_or(Error::Corrupt("ornament ops length"))?;
        let ops = b.get(pos..end).ok_or(Error::Truncated("ornament ops"))?.to_vec();
        for p in &paths {
            match p.op_off.checked_add(p.op_len) {
                Some(end) if end as usize <= ops.len() => {}
                _ => return Err(Error::Corrupt("ornament path ops out of range")),
            }
        }
        // Every asset's outlines are in the pool, and each names a part of the ornament
        // that owns it. A slice draws with its frame's parts.
        for o in &ornaments {
            let slices = o.slices.iter().flat_map(|s| [&s.corner, &s.edge_h, &s.edge_v]);
            for a in std::iter::once(&o.asset).chain(slices) {
                let end = a.first_path.checked_add(a.n_paths).ok_or(Error::Corrupt("ornament asset range"))?;
                let owned =
                    paths.get(a.first_path as usize..end as usize).ok_or(Error::Corrupt("ornament asset range"))?;
                if owned.iter().any(|p| p.part as usize >= o.parts.len()) {
                    return Err(Error::Corrupt("ornament path part"));
                }
            }
        }
        let mut set = OrnamentSet { quant, ornaments, paths, ops };
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

fn get_u8(b: &[u8], pos: &mut usize) -> Result<u8, Error> {
    let v = *b.get(*pos).ok_or(Error::Truncated("ornament byte"))?;
    *pos += 1;
    Ok(v)
}

fn get_str(b: &[u8], pos: &mut usize) -> Result<String, Error> {
    let len = read_varint(b, pos)? as usize;
    let end = pos.checked_add(len).ok_or(Error::Corrupt("ornament string length"))?;
    let s = b.get(*pos..end).ok_or(Error::Truncated("ornament string"))?;
    *pos += len;
    Ok(std::str::from_utf8(s).map_err(|_| Error::Corrupt("utf8"))?.to_owned())
}

fn get_u32(b: &[u8], pos: &mut usize) -> Result<u32, Error> {
    let end = pos.checked_add(4).ok_or(Error::Truncated("ornament u32"))?;
    let s = b.get(*pos..end).ok_or(Error::Truncated("ornament u32"))?;
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
    use crate::Cmd;

    pub(crate) fn sample() -> OrnamentSet {
        let mut set = OrnamentSet::default();
        let first = set.paths.len() as u32;
        set.push_path(1, 0, 0, &[Cmd::MoveTo(0, 0), Cmd::LineTo(1000, 0), Cmd::LineTo(1000, 1000), Cmd::Close]);
        set.push_path(2, OPF_EVENODD | OPF_STROKE, 42, &[Cmd::MoveTo(10, 10), Cmd::CubicTo(20, 20, 30, 30, 40, 40)]);
        set.ornaments.push(Ornament {
            kind: OrnamentKind::AyahMark,
            style: "mushaf-qalon".into(),
            lineage: Lineage::Scan,
            riwayah: "Qalun 'an Nafi'".into(),
            license: License {
                id: "CC-BY-NC-SA-4.0".into(),
                status: "provisional".into(),
                redistributable: false,
                attribution: "Traced from a printed mushaf".into(),
            },
            parts: vec![
                Part { name: "slot".into(), color: 0, stroke: false },
                Part { name: "c1".into(), color: 0xffffffff, stroke: false },
                Part { name: "line".into(), color: 0x303641ff, stroke: true },
            ],
            asset: Asset {
                view_box: [0.0, 0.0, 73.162, 100.0],
                slot: Some([13.2353, 23.1618, 46.3235, 54.0441]),
                first_path: first,
                n_paths: 2,
            },
            slices: None,
        });
        set
    }

    #[test]
    fn round_trips() {
        let a = sample();
        let b = OrnamentSet::decode(&a.encode()).unwrap();
        assert_eq!(a, b);
        assert_eq!(b.ornaments[0].part_index("line"), Some(2));
        assert_eq!(b.path_cmds(1).unwrap().len(), 2);
        assert_eq!(b.find(OrnamentKind::AyahMark, "mushaf-qalon"), Some(0));
        assert_eq!(b.find(OrnamentKind::PageFrame, "mushaf-qalon"), None);
    }

    #[test]
    fn round_trips_slices() {
        let mut a = sample();
        let corner = Asset { view_box: [0.0, 0.0, 9.4557, 6.4871], slot: None, first_path: 0, n_paths: 1 };
        a.ornaments.push(Ornament {
            kind: OrnamentKind::PageFrame,
            style: "mushaf-qalon".into(),
            lineage: Lineage::Scan,
            riwayah: String::new(),
            license: License::default(),
            parts: a.ornaments[0].parts.clone(),
            asset: Asset {
                view_box: [0.0, 0.0, 65.59, 100.0],
                slot: Some([4.92, 5.14, 55.74, 89.77]),
                first_path: 0,
                n_paths: 0,
            },
            slices: Some(Slices {
                corner_w: 9.4557,
                corner_h: 6.4871,
                repeat_h: 6.1572,
                repeat_v: 6.2122,
                rotate: false,
                corner: corner.clone(),
                edge_h: corner.clone(),
                edge_v: corner,
            }),
        });
        assert_eq!(OrnamentSet::decode(&a.encode()).unwrap(), a);
    }

    #[test]
    fn slot_centre_falls_back_to_the_view_box() {
        let a = Asset { view_box: [2.0, 4.0, 10.0, 20.0], slot: None, first_path: 0, n_paths: 0 };
        assert_eq!(a.slot_centre(), (7.0, 14.0));
        let b = Asset { slot: Some([4.0, 6.0, 2.0, 2.0]), ..a };
        assert_eq!(b.slot_centre(), (5.0, 7.0));
    }

    #[test]
    fn rejects_foreign_bytes() {
        assert!(matches!(OrnamentSet::decode(b"QVP1\x64\x00").unwrap_err(), Error::BadMagic));
    }

    #[test]
    fn a_truncated_set_is_an_error_not_a_panic() {
        let bytes = sample().encode();
        for n in 0..bytes.len() {
            match OrnamentSet::decode(&bytes[..n]) {
                Err(Error::Truncated(_) | Error::Corrupt(_) | Error::BadMagic) => {}
                other => panic!("{n} of {} bytes: {other:?}", bytes.len()),
            }
        }
    }

    #[test]
    fn an_asset_past_the_pool_is_corrupt() {
        let mut a = sample();
        a.ornaments[0].asset.n_paths = 3;
        assert!(matches!(OrnamentSet::decode(&a.encode()), Err(Error::Corrupt(_))));
        let mut a = sample();
        a.ornaments[0].asset.first_path = u32::MAX;
        assert!(matches!(OrnamentSet::decode(&a.encode()), Err(Error::Corrupt(_))));
    }

    #[test]
    fn a_path_naming_a_missing_part_is_corrupt() {
        let mut a = sample();
        a.ornaments[0].parts.truncate(2);
        assert!(matches!(OrnamentSet::decode(&a.encode()), Err(Error::Corrupt(_))));
    }

    #[test]
    fn ops_past_the_pool_are_corrupt() {
        let mut a = sample();
        a.paths[1].op_len += 1;
        assert!(matches!(OrnamentSet::decode(&a.encode()), Err(Error::Corrupt(_))));
    }

    #[test]
    fn a_zero_quant_and_a_bad_kind_are_corrupt() {
        let mut bytes = sample().encode();
        bytes[4] = 0;
        bytes[5] = 0;
        assert!(matches!(OrnamentSet::decode(&bytes), Err(Error::Corrupt(_))));
        let mut bytes = sample().encode();
        // after the magic, the quant and the one-byte ornament count
        bytes[7] = 200;
        assert!(matches!(OrnamentSet::decode(&bytes), Err(Error::Corrupt("ornament kind"))));
    }

    #[test]
    fn a_bad_lineage_is_corrupt() {
        let mut bytes = sample().encode();
        // after the magic, the quant, the one-byte ornament count and the kind
        bytes[8] = 200;
        assert!(matches!(OrnamentSet::decode(&bytes), Err(Error::Corrupt("ornament lineage"))));
    }

    #[test]
    fn part_index_finds_a_part_by_name() {
        let o = &sample().ornaments[0];
        assert_eq!(o.part_index(&o.parts[0].name), Some(0));
        assert_eq!(o.part_index("no such part"), None);
    }
}
