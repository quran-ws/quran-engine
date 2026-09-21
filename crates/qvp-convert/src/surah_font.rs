//! A compact OpenType/CFF webfont for the reusable surah-name artwork.
//!
//! CFF keeps the source cubic Béziers. A TrueType `glyf` table would require converting
//! them to quadratics, which would make the generated font an approximation.
use crate::surah_names::{TitleAsset, TitlePath};
use kurbo::{flatten, BezPath, PathEl, Point, Shape};
use qvp_format::{Cmd, PF_EVENODD};

const FAMILY: &str = "Quran Surah Names";
const POSTSCRIPT_NAME: &str = "QuranSurahNames-Regular";
const UNITS_PER_EM: u16 = 5000;

pub(crate) fn build(titles: &[&TitleAsset]) -> Result<Vec<u8>, String> {
    if titles.len() != 114 {
        return Err(format!("font needs 114 surah names, got {}", titles.len()));
    }
    let otf = otf(titles)?;
    let woff2 = woofwoof::compress(&otf, "", 11, false).ok_or("compress surah-name font as WOFF2")?;
    let decoded = woofwoof::decompress(&woff2).ok_or("verify surah-name WOFF2")?;
    if decoded.get(..4) != Some(b"OTTO") {
        return Err("surah-name WOFF2 did not decode to an OpenType/CFF font".into());
    }
    validate(&decoded, titles)?;
    Ok(woff2)
}

#[derive(Debug, PartialEq)]
enum OutlineCommand {
    Move(i32, i32),
    Line(i32, i32),
    Quad(i32, i32, i32, i32),
    Cubic(i32, i32, i32, i32, i32, i32),
}

#[derive(Default)]
struct Outline(Vec<OutlineCommand>);

impl ttf_parser::OutlineBuilder for Outline {
    fn move_to(&mut self, x: f32, y: f32) {
        self.0.push(OutlineCommand::Move(x.round() as i32, y.round() as i32));
    }
    fn line_to(&mut self, x: f32, y: f32) {
        self.0.push(OutlineCommand::Line(x.round() as i32, y.round() as i32));
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        self.0.push(OutlineCommand::Quad(x1.round() as i32, y1.round() as i32, x.round() as i32, y.round() as i32));
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        self.0.push(OutlineCommand::Cubic(
            x1.round() as i32,
            y1.round() as i32,
            x2.round() as i32,
            y2.round() as i32,
            x.round() as i32,
            y.round() as i32,
        ));
    }
    fn close(&mut self) {}
}

fn validate(otf: &[u8], titles: &[&TitleAsset]) -> Result<(), String> {
    let face =
        ttf_parser::Face::parse(otf, 0).map_err(|error| format!("parse generated surah-name font: {error:?}"))?;
    if face.units_per_em() != UNITS_PER_EM {
        return Err("generated surah-name font has the wrong units per em".into());
    }
    for title in titles {
        let character = char::from_u32(0xe000 + title.surah as u32).ok_or("invalid surah-name code point")?;
        let glyph =
            face.glyph_index(character).ok_or_else(|| format!("font has no glyph for surah {}", title.surah))?;
        let expected_advance = (title.bounds.x1 - title.bounds.x0) as u16;
        if face.glyph_hor_advance(glyph) != Some(expected_advance) {
            return Err(format!("font has the wrong advance for surah {}", title.surah));
        }
        let mut outline = Outline::default();
        if face.outline_glyph(glyph, &mut outline).is_none() {
            return Err(format!("font has no outline for surah {}", title.surah));
        }
        let expected: Vec<_> = title
            .paths
            .iter()
            .flat_map(|path| font_commands(title, path))
            .filter_map(|command| match command {
                Cmd::MoveTo(x, y) => Some(OutlineCommand::Move(x, y)),
                Cmd::LineTo(x, y) => Some(OutlineCommand::Line(x, y)),
                Cmd::QuadTo(x1, y1, x, y) => Some(OutlineCommand::Quad(x1, y1, x, y)),
                Cmd::CubicTo(x1, y1, x2, y2, x, y) => Some(OutlineCommand::Cubic(x1, y1, x2, y2, x, y)),
                Cmd::Close => None,
            })
            .collect();
        if outline.0 != expected {
            return Err(format!("font changed the outline geometry for surah {}", title.surah));
        }
    }
    Ok(())
}

fn otf(titles: &[&TitleAsset]) -> Result<Vec<u8>, String> {
    let max_width = titles.iter().map(|title| title.bounds.x1 - title.bounds.x0).max().unwrap_or(0);
    let max_height = titles.iter().map(|title| title.bounds.y1 - title.bounds.y0).max().unwrap_or(0);
    if max_width <= 0 || max_height <= 0 || max_width > i16::MAX as i32 || max_height > i16::MAX as i32 {
        return Err(format!("surah-name font bounds are outside OpenType limits: {max_width}x{max_height}"));
    }

    let advances: Vec<u16> = std::iter::once(UNITS_PER_EM / 2)
        .chain(titles.iter().map(|title| (title.bounds.x1 - title.bounds.x0) as u16))
        .collect();
    let mut tables = vec![
        (*b"CFF ", cff(titles, max_width, max_height)?),
        (*b"OS/2", os2(&advances, max_height as u16)),
        (*b"cmap", cmap()),
        (*b"head", head(max_width as i16, max_height as i16)),
        (*b"hhea", hhea(&advances, max_width as i16, max_height as i16)),
        (*b"hmtx", hmtx(&advances)),
        (*b"maxp", maxp(advances.len() as u16)),
        (*b"name", name()),
        (*b"post", post()),
    ];
    sfnt(&mut tables)
}

fn cff(titles: &[&TitleAsset], max_width: i32, max_height: i32) -> Result<Vec<u8>, String> {
    let mut strings: Vec<Vec<u8>> =
        titles.iter().map(|title| format!("surah{:03}", title.surah).into_bytes()).collect();
    let full_sid = 391 + strings.len() as i32;
    strings.push(FAMILY.as_bytes().to_vec());
    let family_sid = 391 + strings.len() as i32;
    strings.push(FAMILY.as_bytes().to_vec());
    let weight_sid = 391 + strings.len() as i32;
    strings.push(b"Regular".to_vec());

    let mut charstrings = Vec::with_capacity(titles.len() + 1);
    charstrings.push(notdef());
    for title in titles {
        charstrings.push(charstring(title)?);
    }
    let charstrings = index(&charstrings)?;

    let mut charset = vec![0]; // format 0: one SID for every glyph after .notdef
    for i in 0..titles.len() {
        charset.extend_from_slice(&((391 + i) as u16).to_be_bytes());
    }

    let header = [1, 0, 4, 4];
    let names = index(&[POSTSCRIPT_NAME.as_bytes().to_vec()])?;
    let strings = index(&strings)?;
    let global_subrs = index(&[])?;
    let mut top = Vec::new();
    for _ in 0..8 {
        let top_index = index(&[top.clone()])?;
        let charset_offset = header.len() + names.len() + top_index.len() + strings.len() + global_subrs.len();
        let charstrings_offset = charset_offset + charset.len();
        let mut next = Vec::new();
        dict_int(full_sid, &mut next);
        next.push(2); // FullName
        dict_int(family_sid, &mut next);
        next.push(3); // FamilyName
        dict_int(weight_sid, &mut next);
        next.push(4); // Weight
        for value in [0, 0, max_width, max_height] {
            dict_int(value, &mut next);
        }
        next.push(5); // FontBBox
        dict_real("0.0002", &mut next);
        dict_int(0, &mut next);
        dict_int(0, &mut next);
        dict_real("0.0002", &mut next);
        dict_int(0, &mut next);
        dict_int(0, &mut next);
        next.extend_from_slice(&[12, 7]); // FontMatrix: 1 / 5000 units per em
        dict_int(charset_offset as i32, &mut next);
        next.push(15); // charset
        dict_int(charstrings_offset as i32, &mut next);
        next.push(17); // CharStrings
        if next == top {
            break;
        }
        top = next;
    }
    let top = index(&[top])?;
    let mut out = Vec::new();
    out.extend_from_slice(&header);
    out.extend_from_slice(&names);
    out.extend_from_slice(&top);
    out.extend_from_slice(&strings);
    out.extend_from_slice(&global_subrs);
    out.extend_from_slice(&charset);
    out.extend_from_slice(&charstrings);
    Ok(out)
}

fn notdef() -> Vec<u8> {
    let mut out = Vec::new();
    type2_int((UNITS_PER_EM / 2) as i32, &mut out);
    type2_int(0, &mut out);
    type2_int(0, &mut out);
    out.push(21); // rmoveto, with width
    out.push(14); // endchar
    out
}

fn charstring(title: &TitleAsset) -> Result<Vec<u8>, String> {
    let width = title.bounds.x1 - title.bounds.x0;
    let mut out = Vec::new();
    let (mut current_x, mut current_y) = (0, 0);
    let mut first_move = true;
    for path in &title.paths {
        for command in font_commands(title, path) {
            match command {
                Cmd::MoveTo(x, y) => {
                    if first_move {
                        type2_int(width, &mut out);
                        first_move = false;
                    }
                    type2_int(x - current_x, &mut out);
                    type2_int(y - current_y, &mut out);
                    out.push(21); // rmoveto
                    current_x = x;
                    current_y = y;
                }
                Cmd::LineTo(x, y) => {
                    type2_int(x - current_x, &mut out);
                    type2_int(y - current_y, &mut out);
                    out.push(5); // rlineto
                    current_x = x;
                    current_y = y;
                }
                Cmd::CubicTo(x1, y1, x2, y2, x, y) => {
                    type2_int(x1 - current_x, &mut out);
                    type2_int(y1 - current_y, &mut out);
                    type2_int(x2 - x1, &mut out);
                    type2_int(y2 - y1, &mut out);
                    type2_int(x - x2, &mut out);
                    type2_int(y - y2, &mut out);
                    out.push(8); // rrcurveto
                    current_x = x;
                    current_y = y;
                }
                Cmd::QuadTo(..) => {
                    return Err(format!(
                        "surah {} uses a quadratic curve; CFF generation would not be lossless",
                        title.surah
                    ));
                }
                // Type 2 closes every contour implicitly at the next move or at endchar.
                Cmd::Close => {}
            }
        }
    }
    if first_move {
        return Err(format!("surah {} has no font outline", title.surah));
    }
    out.push(14); // endchar
    Ok(out)
}

fn font_commands(title: &TitleAsset, path: &TitlePath) -> Vec<Cmd> {
    let transformed: Vec<_> = path
        .commands
        .iter()
        .map(|command| match *command {
            Cmd::MoveTo(x, y) => Cmd::MoveTo(x - title.bounds.x0, title.bounds.y1 - y),
            Cmd::LineTo(x, y) => Cmd::LineTo(x - title.bounds.x0, title.bounds.y1 - y),
            Cmd::QuadTo(x1, y1, x, y) => {
                Cmd::QuadTo(x1 - title.bounds.x0, title.bounds.y1 - y1, x - title.bounds.x0, title.bounds.y1 - y)
            }
            Cmd::CubicTo(x1, y1, x2, y2, x, y) => Cmd::CubicTo(
                x1 - title.bounds.x0,
                title.bounds.y1 - y1,
                x2 - title.bounds.x0,
                title.bounds.y1 - y2,
                x - title.bounds.x0,
                title.bounds.y1 - y,
            ),
            Cmd::Close => Cmd::Close,
        })
        .collect();
    if path.flags & PF_EVENODD == 0 {
        return transformed;
    }
    normalize_evenodd(&transformed)
}

/// Fonts use non-zero winding, while the source title paths use even-odd fill. Reverse every
/// other nested contour so both rules paint the same pixels without changing any curve.
fn normalize_evenodd(commands: &[Cmd]) -> Vec<Cmd> {
    let mut contours = Vec::<BezPath>::new();
    let mut current = BezPath::new();
    for command in commands {
        if matches!(command, Cmd::MoveTo(..)) && !current.is_empty() {
            contours.push(current);
            current = BezPath::new();
        }
        match *command {
            Cmd::MoveTo(x, y) => current.move_to((x as f64, y as f64)),
            Cmd::LineTo(x, y) => current.line_to((x as f64, y as f64)),
            Cmd::QuadTo(x1, y1, x, y) => current.quad_to((x1 as f64, y1 as f64), (x as f64, y as f64)),
            Cmd::CubicTo(x1, y1, x2, y2, x, y) => {
                current.curve_to((x1 as f64, y1 as f64), (x2 as f64, y2 as f64), (x as f64, y as f64));
            }
            Cmd::Close => current.close_path(),
        }
    }
    if !current.is_empty() {
        contours.push(current);
    }
    let samples: Vec<_> = contours.iter().map(interior_point).collect();
    let mut out = Vec::new();
    for (index, contour) in contours.iter().enumerate() {
        let depth = samples[index]
            .map(|point| {
                contours.iter().enumerate().filter(|(other, path)| *other != index && path.winding(point) != 0).count()
            })
            .unwrap_or(0);
        let wants_positive = depth % 2 == 0;
        let positive = contour.area() >= 0.0;
        let contour = if positive == wants_positive { contour.clone() } else { contour.reverse_subpaths() };
        for element in contour.elements() {
            out.push(match *element {
                PathEl::MoveTo(point) => Cmd::MoveTo(point.x.round() as i32, point.y.round() as i32),
                PathEl::LineTo(point) => Cmd::LineTo(point.x.round() as i32, point.y.round() as i32),
                PathEl::QuadTo(control, point) => Cmd::QuadTo(
                    control.x.round() as i32,
                    control.y.round() as i32,
                    point.x.round() as i32,
                    point.y.round() as i32,
                ),
                PathEl::CurveTo(a, b, point) => Cmd::CubicTo(
                    a.x.round() as i32,
                    a.y.round() as i32,
                    b.x.round() as i32,
                    b.y.round() as i32,
                    point.x.round() as i32,
                    point.y.round() as i32,
                ),
                PathEl::ClosePath => Cmd::Close,
            });
        }
    }
    out
}

fn interior_point(path: &BezPath) -> Option<Point> {
    let area = path.area();
    if area.abs() < f64::EPSILON {
        return None;
    }
    let mut points = Vec::new();
    flatten(path, 0.25, |element| match element {
        PathEl::MoveTo(point) | PathEl::LineTo(point) => points.push(point),
        _ => {}
    });
    for pair in points.windows(2) {
        let (a, b) = (pair[0], pair[1]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let length = dx.hypot(dy);
        if length <= f64::EPSILON {
            continue;
        }
        let direction = area.signum();
        for epsilon in [0.01, 0.1, 1.0] {
            let point = Point::new(
                (a.x + b.x) / 2.0 - dy / length * epsilon * direction,
                (a.y + b.y) / 2.0 + dx / length * epsilon * direction,
            );
            if path.winding(point) != 0 {
                return Some(point);
            }
        }
    }
    let centre = path.bounding_box().center();
    (path.winding(centre) != 0).then_some(centre)
}

fn type2_int(value: i32, out: &mut Vec<u8>) {
    if (-107..=107).contains(&value) {
        out.push((value + 139) as u8);
    } else if (108..=1131).contains(&value) {
        let value = value - 108;
        out.push((value / 256 + 247) as u8);
        out.push((value % 256) as u8);
    } else if (-1131..=-108).contains(&value) {
        let value = -value - 108;
        out.push((value / 256 + 251) as u8);
        out.push((value % 256) as u8);
    } else if i16::try_from(value).is_ok() {
        out.push(28);
        out.extend_from_slice(&(value as i16).to_be_bytes());
    } else {
        out.push(255);
        out.extend_from_slice(&(value << 16).to_be_bytes());
    }
}

fn dict_real(value: &str, out: &mut Vec<u8>) {
    let mut nibbles = Vec::new();
    for byte in value.bytes() {
        nibbles.push(match byte {
            b'0'..=b'9' => byte - b'0',
            b'.' => 0x0a,
            b'-' => 0x0e,
            _ => unreachable!("font DICT real is a fixed decimal"),
        });
    }
    nibbles.push(0x0f);
    if nibbles.len() % 2 != 0 {
        nibbles.push(0x0f);
    }
    out.push(30);
    for pair in nibbles.as_chunks::<2>().0 {
        out.push(pair[0] << 4 | pair[1]);
    }
}

fn dict_int(value: i32, out: &mut Vec<u8>) {
    if (-107..=107).contains(&value) {
        out.push((value + 139) as u8);
    } else if (108..=1131).contains(&value) {
        let value = value - 108;
        out.push((value / 256 + 247) as u8);
        out.push((value % 256) as u8);
    } else if (-1131..=-108).contains(&value) {
        let value = -value - 108;
        out.push((value / 256 + 251) as u8);
        out.push((value % 256) as u8);
    } else if i16::try_from(value).is_ok() {
        out.push(28);
        out.extend_from_slice(&(value as i16).to_be_bytes());
    } else {
        out.push(29);
        out.extend_from_slice(&value.to_be_bytes());
    }
}

fn index(items: &[Vec<u8>]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    out.extend_from_slice(&(items.len() as u16).to_be_bytes());
    if items.is_empty() {
        return Ok(out);
    }
    let total = 1usize + items.iter().map(Vec::len).sum::<usize>();
    let off_size = if total <= 0xff {
        1
    } else if total <= 0xffff {
        2
    } else if total <= 0xff_ffff {
        3
    } else {
        4
    };
    out.push(off_size);
    let mut offset = 1usize;
    for item in items.iter().chain(std::iter::once(&Vec::new())) {
        for shift in (0..off_size).rev() {
            out.push((offset >> (shift * 8)) as u8);
        }
        offset += item.len();
    }
    for item in items {
        out.extend_from_slice(item);
    }
    Ok(out)
}

fn cmap() -> Vec<u8> {
    let mut sub = Vec::new();
    be16(&mut sub, 4); // format
    be16(&mut sub, 32); // length
    be16(&mut sub, 0); // language
    be16(&mut sub, 4); // segCountX2: PUA range + sentinel
    be16(&mut sub, 4); // searchRange
    be16(&mut sub, 1); // entrySelector
    be16(&mut sub, 0); // rangeShift
    be16(&mut sub, 0xe072); // end PUA
    be16(&mut sub, 0xffff);
    be16(&mut sub, 0); // reservedPad
    be16(&mut sub, 0xe001); // start PUA
    be16(&mut sub, 0xffff);
    be16(&mut sub, 1u16.wrapping_sub(0xe001)); // glyph 1 at E001
    be16(&mut sub, 1); // sentinel maps to glyph 0
    be16(&mut sub, 0); // idRangeOffset
    be16(&mut sub, 0);

    let mut out = Vec::new();
    be16(&mut out, 0);
    be16(&mut out, 1);
    be16(&mut out, 3); // Windows
    be16(&mut out, 1); // Unicode BMP
    be32(&mut out, 12);
    out.extend_from_slice(&sub);
    out
}

fn head(x_max: i16, y_max: i16) -> Vec<u8> {
    let mut out = Vec::new();
    be32(&mut out, 0x0001_0000);
    be32(&mut out, 0x0001_0000);
    be32(&mut out, 0); // checksumAdjustment, patched in sfnt
    be32(&mut out, 0x5f0f_3cf5);
    be16(&mut out, 3);
    be16(&mut out, UNITS_PER_EM);
    out.extend_from_slice(&0u64.to_be_bytes());
    out.extend_from_slice(&0u64.to_be_bytes());
    bei16(&mut out, 0);
    bei16(&mut out, 0);
    bei16(&mut out, x_max);
    bei16(&mut out, y_max);
    be16(&mut out, 0);
    be16(&mut out, 8);
    bei16(&mut out, 2);
    bei16(&mut out, 0);
    bei16(&mut out, 0);
    out
}

fn hhea(advances: &[u16], x_max: i16, y_max: i16) -> Vec<u8> {
    let mut out = Vec::new();
    be32(&mut out, 0x0001_0000);
    bei16(&mut out, y_max);
    bei16(&mut out, 0);
    bei16(&mut out, 0);
    be16(&mut out, *advances.iter().max().unwrap_or(&0));
    bei16(&mut out, 0);
    bei16(&mut out, 0);
    bei16(&mut out, x_max);
    bei16(&mut out, 1);
    bei16(&mut out, 0);
    bei16(&mut out, 0);
    for _ in 0..4 {
        bei16(&mut out, 0);
    }
    bei16(&mut out, 0);
    be16(&mut out, advances.len() as u16);
    out
}

fn hmtx(advances: &[u16]) -> Vec<u8> {
    let mut out = Vec::with_capacity(advances.len() * 4);
    for advance in advances {
        be16(&mut out, *advance);
        bei16(&mut out, 0);
    }
    out
}

fn maxp(glyphs: u16) -> Vec<u8> {
    let mut out = Vec::new();
    be32(&mut out, 0x0000_5000);
    be16(&mut out, glyphs);
    out
}

fn name() -> Vec<u8> {
    let records = [(1, FAMILY), (2, "Regular"), (4, FAMILY), (5, "Version 1.000"), (6, POSTSCRIPT_NAME)];
    let count = records.len() as u16;
    let mut strings = Vec::new();
    let mut encoded = Vec::new();
    for (_, value) in records {
        let offset = strings.len() as u16;
        let bytes: Vec<u8> = value.encode_utf16().flat_map(u16::to_be_bytes).collect();
        let len = bytes.len() as u16;
        strings.extend_from_slice(&bytes);
        encoded.push((value, offset, len));
    }
    let mut out = Vec::new();
    be16(&mut out, 0);
    be16(&mut out, count);
    be16(&mut out, 6 + count * 12);
    for ((name_id, _), (_, offset, len)) in records.into_iter().zip(encoded) {
        be16(&mut out, 3);
        be16(&mut out, 1);
        be16(&mut out, 0x0409);
        be16(&mut out, name_id);
        be16(&mut out, len);
        be16(&mut out, offset);
    }
    out.extend_from_slice(&strings);
    out
}

fn os2(advances: &[u16], y_max: u16) -> Vec<u8> {
    let average = advances.iter().map(|value| *value as u32).sum::<u32>() / advances.len() as u32;
    let mut out = Vec::new();
    be16(&mut out, 4);
    bei16(&mut out, average.min(i16::MAX as u32) as i16);
    be16(&mut out, 400);
    be16(&mut out, 5);
    be16(&mut out, 0);
    for _ in 0..10 {
        bei16(&mut out, 0);
    }
    bei16(&mut out, 0);
    out.extend_from_slice(&[0; 10]);
    be32(&mut out, 0);
    be32(&mut out, 1 << 28); // Unicode range bit 60: BMP Private Use Area
    be32(&mut out, 0);
    be32(&mut out, 0);
    out.extend_from_slice(b"QWS ");
    be16(&mut out, 0x0040); // regular
    be16(&mut out, 0xe001);
    be16(&mut out, 0xe072);
    bei16(&mut out, y_max.min(i16::MAX as u16) as i16);
    bei16(&mut out, 0);
    bei16(&mut out, 0);
    be16(&mut out, y_max);
    be16(&mut out, 0);
    be32(&mut out, 0);
    be32(&mut out, 0);
    bei16(&mut out, 0);
    bei16(&mut out, y_max.min(i16::MAX as u16) as i16);
    be16(&mut out, 0);
    be16(&mut out, 0x0020);
    be16(&mut out, 1);
    out
}

fn post() -> Vec<u8> {
    let mut out = Vec::new();
    be32(&mut out, 0x0003_0000);
    be32(&mut out, 0);
    bei16(&mut out, 0);
    bei16(&mut out, 0);
    for _ in 0..5 {
        be32(&mut out, 0);
    }
    out
}

fn sfnt(tables: &mut [([u8; 4], Vec<u8>)]) -> Result<Vec<u8>, String> {
    tables.sort_by_key(|(tag, _)| *tag);
    let count = tables.len() as u16;
    let power = 1u16 << (15 - count.leading_zeros() as u16);
    let search_range = power * 16;
    let entry_selector = power.trailing_zeros() as u16;
    let range_shift = count * 16 - search_range;
    let mut offset = 12 + tables.len() * 16;
    let mut records = Vec::new();
    for (tag, bytes) in tables.iter() {
        let aligned = (bytes.len() + 3) & !3;
        records.push((*tag, checksum(bytes), offset as u32, bytes.len() as u32));
        offset += aligned;
    }
    let mut out = Vec::with_capacity(offset);
    out.extend_from_slice(b"OTTO");
    be16(&mut out, count);
    be16(&mut out, search_range);
    be16(&mut out, entry_selector);
    be16(&mut out, range_shift);
    for (tag, sum, offset, len) in &records {
        out.extend_from_slice(tag);
        be32(&mut out, *sum);
        be32(&mut out, *offset);
        be32(&mut out, *len);
    }
    for (_, bytes) in tables.iter() {
        out.extend_from_slice(bytes);
        while out.len() % 4 != 0 {
            out.push(0);
        }
    }
    let head = records.iter().find(|(tag, _, _, _)| tag == b"head").ok_or("font has no head table")?.2 as usize;
    let adjustment = 0xb1b0_afbau32.wrapping_sub(checksum(&out));
    out[head + 8..head + 12].copy_from_slice(&adjustment.to_be_bytes());
    Ok(out)
}

fn checksum(bytes: &[u8]) -> u32 {
    bytes
        .chunks(4)
        .map(|chunk| {
            let mut word = [0u8; 4];
            word[..chunk.len()].copy_from_slice(chunk);
            u32::from_be_bytes(word)
        })
        .fold(0u32, u32::wrapping_add)
}

fn be16(out: &mut Vec<u8>, value: u16) {
    out.extend_from_slice(&value.to_be_bytes());
}
fn bei16(out: &mut Vec<u8>, value: i16) {
    out.extend_from_slice(&value.to_be_bytes());
}
fn be32(out: &mut Vec<u8>, value: u32) {
    out.extend_from_slice(&value.to_be_bytes());
}
