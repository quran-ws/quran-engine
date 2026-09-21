//! Reusable surah-name QVP, SVG and font assets derived from converted pages.
use kurbo::{BezPath, Rect, Shape};
use qvp_format::{
    encode, encode_cmds, Cmd, DecoKind, DecoRec, Header, IBox, LineRec, PageData, PathKind, PathRec, NONE_U16,
    PF_EVENODD, PF_GLYPH,
};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;

pub const DIR: &str = "surah-names";
const LAST_SURAH: u16 = 114;
const BOUND_SCALE: i64 = 1_000_000;
const FIRST_CODEPOINT: u32 = 0xe001;

#[derive(Clone, Debug)]
pub(crate) struct TitlePath {
    pub commands: Vec<Cmd>,
    pub kind: PathKind,
    pub mark: qvp_format::Mark,
    pub family: qvp_format::Family,
    pub flags: u8,
}

#[derive(Clone, Copy, Debug)]
struct VisualBounds {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
}

impl VisualBounds {
    fn width(self) -> i64 {
        self.x1 - self.x0
    }
    fn height(self) -> i64 {
        self.y1 - self.y0
    }
}

#[derive(Clone, Debug)]
pub(crate) struct TitleAsset {
    pub surah: u16,
    pub source_page: u16,
    pub quant: u16,
    pub version: u16,
    pub header_flags: u16,
    pub metadata: String,
    pub paths: Vec<TitlePath>,
    pub bounds: IBox,
    visual: VisualBounds,
}

#[derive(Default)]
pub struct Builder {
    titles: BTreeMap<u16, TitleAsset>,
}

impl Builder {
    pub fn add_page(&mut self, page: &PageData) -> Result<(), String> {
        for decoration in page.decorations.iter().filter(|decoration| decoration.kind == DecoKind::SurahName) {
            let surah = decoration.surah;
            if !(1..=LAST_SURAH).contains(&surah) {
                return Err(format!("surah-name decoration has invalid surah {surah}"));
            }
            if self.titles.contains_key(&surah) {
                return Err(format!("duplicate surah-name decoration for surah {surah}"));
            }
            self.titles.insert(surah, extract(page, decoration)?);
        }
        Ok(())
    }

    pub fn write(&self, out_dir: &Path) -> Result<usize, String> {
        let missing: Vec<_> = (1..=LAST_SURAH).filter(|number| !self.titles.contains_key(number)).collect();
        if !missing.is_empty() {
            return Err(format!(
                "surah-name asset set is incomplete; missing {}",
                missing.iter().map(u16::to_string).collect::<Vec<_>>().join(", ")
            ));
        }
        let titles: Vec<_> = self.titles.values().collect();
        let sheet = Sheet::new(&titles)?;
        let all_qvp = encode(&sheet_qvp(&titles, &sheet)?);
        let all_svg = sheet_svg(&titles, &sheet)?;
        let font = crate::surah_font::build(&titles)?;
        let map = map_json(&titles);
        let css = font_css();

        fs::create_dir_all(out_dir).map_err(|error| format!("create {}: {error}", out_dir.display()))?;
        let directory = out_dir.join(DIR);
        let temporary = out_dir.join(format!(".{DIR}.tmp"));
        if temporary.exists() {
            fs::remove_dir_all(&temporary).map_err(|error| format!("remove {}: {error}", temporary.display()))?;
        }
        let qvp_dir = temporary.join("qvp");
        let svg_dir = temporary.join("svg");
        fs::create_dir_all(&qvp_dir).map_err(|error| format!("create {}: {error}", qvp_dir.display()))?;
        fs::create_dir_all(&svg_dir).map_err(|error| format!("create {}: {error}", svg_dir.display()))?;
        let write_asset = |path: &Path, bytes: &[u8]| -> Result<(), String> {
            fs::write(path, bytes).map_err(|error| format!("write {}: {error}", path.display()))
        };
        for title in &titles {
            write_asset(&qvp_dir.join(format!("{:03}.qvp", title.surah)), &encode(&title_qvp(title)))?;
            write_asset(&svg_dir.join(format!("{:03}.svg", title.surah)), render_svg(title).as_bytes())?;
        }
        write_asset(&qvp_dir.join("all.qvp"), &all_qvp)?;
        write_asset(&svg_dir.join("all.svg"), all_svg.as_bytes())?;
        write_asset(&temporary.join("surah-names.woff2"), &font)?;
        write_asset(&temporary.join("surah-names.css"), css.as_bytes())?;
        write_asset(&temporary.join("map.json"), map.as_bytes())?;

        if directory.exists() {
            fs::remove_dir_all(&directory).map_err(|error| format!("remove {}: {error}", directory.display()))?;
        }
        if let Err(error) = fs::rename(&temporary, &directory) {
            let _ = fs::remove_dir_all(&temporary);
            return Err(format!("rename {} to {}: {error}", temporary.display(), directory.display()));
        }
        Ok(self.titles.len())
    }
}

struct Metadata<'a> {
    arabic: &'a str,
    latin: &'a str,
    english: &'a str,
    place: &'a str,
    ayah_count: &'a str,
}

fn metadata(value: &str, surah: u16) -> Result<Metadata<'_>, String> {
    let fields: Vec<_> = value.splitn(5, '|').collect();
    if fields.len() != 5 || fields.iter().any(|field| field.is_empty()) || fields[4].parse::<u16>().is_err() {
        return Err(format!("surah {surah} has invalid name metadata"));
    }
    Ok(Metadata { arabic: fields[0], latin: fields[1], english: fields[2], place: fields[3], ayah_count: fields[4] })
}

fn decoration_metadata<'a>(page: &'a PageData, decoration: &DecoRec) -> Result<&'a str, String> {
    if decoration.text == NONE_U16 {
        return Err(format!("surah {} has no name metadata", decoration.surah));
    }
    let value = page
        .strings
        .get(decoration.text as usize)
        .ok_or_else(|| format!("surah {} name metadata points outside the string table", decoration.surah))?;
    metadata(value, decoration.surah)?;
    Ok(value)
}

fn extract(page: &PageData, decoration: &DecoRec) -> Result<TitleAsset, String> {
    if page.header.quant == 0 {
        return Err("page quantisation is zero".into());
    }
    let end = decoration
        .first_path
        .checked_add(decoration.n_paths as u32)
        .filter(|end| *end as usize <= page.paths.len())
        .ok_or_else(|| format!("surah {} path range is outside the page", decoration.surah))?;
    let mut paths = Vec::new();
    let mut bounds = IBox::EMPTY;
    let mut geometry = BezPath::new();
    for index in decoration.first_path..end {
        let record = &page.paths[index as usize];
        match record.kind {
            PathKind::HeaderInk => {
                let commands = page
                    .path_cmds(index as usize)
                    .map_err(|error| format!("surah {} path {index}: {error}", decoration.surah))?;
                append(&mut geometry, &commands, page.header.quant);
                bounds.union(&record.bbox);
                paths.push(TitlePath {
                    commands,
                    kind: record.kind,
                    mark: record.mark,
                    family: record.family,
                    flags: record.flags,
                });
            }
            PathKind::Ornament => {}
            kind => return Err(format!("surah {} contains unexpected {} ink", decoration.surah, kind.as_str())),
        }
    }
    if paths.is_empty() {
        return Err(format!("surah {} has no calligraphic title ink", decoration.surah));
    }
    let visual = geometry.bounding_box();
    if bounds.is_empty()
        || bounds.x1 <= bounds.x0
        || bounds.y1 <= bounds.y0
        || visual.width() <= 0.0
        || visual.height() <= 0.0
    {
        return Err(format!("surah {} has empty calligraphic title ink", decoration.surah));
    }
    Ok(TitleAsset {
        surah: decoration.surah,
        source_page: page.header.page,
        quant: page.header.quant,
        version: page.header.version,
        header_flags: page.header.flags,
        metadata: decoration_metadata(page, decoration)?.to_owned(),
        paths,
        bounds,
        visual: outward(visual),
    })
}

fn shifted(command: Cmd, dx: i32, dy: i32) -> Cmd {
    match command {
        Cmd::MoveTo(x, y) => Cmd::MoveTo(x + dx, y + dy),
        Cmd::LineTo(x, y) => Cmd::LineTo(x + dx, y + dy),
        Cmd::QuadTo(x1, y1, x2, y2) => Cmd::QuadTo(x1 + dx, y1 + dy, x2 + dx, y2 + dy),
        Cmd::CubicTo(x1, y1, x2, y2, x3, y3) => Cmd::CubicTo(x1 + dx, y1 + dy, x2 + dx, y2 + dy, x3 + dx, y3 + dy),
        Cmd::Close => Cmd::Close,
    }
}

fn push_paths(title: &TitleAsset, dx: i32, dy: i32, paths: &mut Vec<PathRec>, ops: &mut Vec<u8>) -> (u32, u16, IBox) {
    let first = paths.len() as u32;
    let mut group = IBox::EMPTY;
    for source in &title.paths {
        let commands: Vec<_> = source.commands.iter().copied().map(|command| shifted(command, dx, dy)).collect();
        let op_off = ops.len() as u32;
        let bbox = encode_cmds(&commands, 0, 0, ops);
        group.union(&bbox);
        paths.push(PathRec {
            kind: source.kind,
            mark: source.mark,
            family: source.family,
            flags: source.flags & !PF_GLYPH,
            ox: 0,
            oy: 0,
            op_off,
            op_len: ops.len() as u32 - op_off,
            bbox,
        });
    }
    (first, paths.len() as u16 - first as u16, group)
}

fn title_qvp(title: &TitleAsset) -> PageData {
    let mut paths = Vec::new();
    let mut ops = Vec::new();
    let (first_path, n_paths, bbox) = push_paths(title, -title.bounds.x0, -title.bounds.y0, &mut paths, &mut ops);
    let quant = title.quant as f32;
    PageData {
        header: Header {
            version: title.version,
            quant: title.quant,
            page: title.source_page,
            flags: title.header_flags,
            width: (title.bounds.x1 - title.bounds.x0) as f32 / quant,
            height: (title.bounds.y1 - title.bounds.y0) as f32 / quant,
        },
        lines: vec![LineRec { line_number: 1, first_word: 0, n_words: 0, bbox: IBox::EMPTY }],
        ayahs: vec![],
        words: vec![],
        paths,
        decorations: vec![DecoRec {
            kind: DecoKind::SurahName,
            surah: title.surah,
            ayah: 0,
            text: 0,
            first_path,
            n_paths,
            line: 0,
            bbox,
        }],
        glyphs: vec![],
        insts: vec![],
        ops,
        strings: vec![title.metadata.clone()],
    }
}

struct Sheet {
    quant: u16,
    width: i32,
    height: i32,
    row_height: i32,
    max_width: i32,
    max_height: i32,
    padding: i32,
}

impl Sheet {
    fn new(titles: &[&TitleAsset]) -> Result<Self, String> {
        let quant = titles.first().ok_or("no surah names")?.quant;
        if titles.iter().any(|title| title.quant != quant) {
            return Err("surah-name pages use different quantisation".into());
        }
        let max_width = titles.iter().map(|title| title.bounds.x1 - title.bounds.x0).max().unwrap_or(0);
        let max_height = titles.iter().map(|title| title.bounds.y1 - title.bounds.y0).max().unwrap_or(0);
        let padding = quant as i32 * 2;
        let row_height = max_height + padding * 2;
        Ok(Self {
            quant,
            width: max_width + padding * 2,
            height: row_height * titles.len() as i32,
            row_height,
            max_width,
            max_height,
            padding,
        })
    }

    fn placement(&self, title: &TitleAsset, row: usize) -> (i32, i32) {
        let width = title.bounds.x1 - title.bounds.x0;
        let height = title.bounds.y1 - title.bounds.y0;
        (
            self.padding + (self.max_width - width) / 2 - title.bounds.x0,
            row as i32 * self.row_height + self.padding + (self.max_height - height) / 2 - title.bounds.y0,
        )
    }
}

fn sheet_qvp(titles: &[&TitleAsset], sheet: &Sheet) -> Result<PageData, String> {
    let first = titles.first().ok_or("no surah names")?;
    let mut paths = Vec::new();
    let mut ops = Vec::new();
    let mut decorations = Vec::with_capacity(titles.len());
    let mut lines = Vec::with_capacity(titles.len());
    let mut strings = Vec::with_capacity(titles.len());
    for (row, title) in titles.iter().enumerate() {
        let (dx, dy) = sheet.placement(title, row);
        let (first_path, n_paths, bbox) = push_paths(title, dx, dy, &mut paths, &mut ops);
        lines.push(LineRec { line_number: title.surah as u8, first_word: 0, n_words: 0, bbox: IBox::EMPTY });
        decorations.push(DecoRec {
            kind: DecoKind::SurahName,
            surah: title.surah,
            ayah: 0,
            text: strings.len() as u16,
            first_path,
            n_paths,
            line: row as u16,
            bbox,
        });
        strings.push(title.metadata.clone());
    }
    Ok(PageData {
        header: Header {
            version: first.version,
            quant: sheet.quant,
            page: 0,
            flags: first.header_flags,
            width: sheet.width as f32 / sheet.quant as f32,
            height: sheet.height as f32 / sheet.quant as f32,
        },
        lines,
        ayahs: vec![],
        words: vec![],
        paths,
        decorations,
        glyphs: vec![],
        insts: vec![],
        ops,
        strings,
    })
}

fn render_svg(title: &TitleAsset) -> String {
    let meta = metadata(&title.metadata, title.surah).expect("validated metadata");
    let width = number(title.visual.width());
    let height = number(title.visual.height());
    let mut svg = String::new();
    write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {width} {height}\" width=\"{width}\" height=\"{height}\" fill=\"currentColor\" color=\"#231f20\" data-surah=\"{}\" data-page=\"{}\"",
        number(title.visual.x0),
        number(title.visual.y0),
        title.surah,
        title.source_page,
    )
    .unwrap();
    metadata_attributes(&mut svg, &meta);
    svg.push_str(">\n<title>");
    escaped(&mut svg, meta.arabic, false);
    svg.push_str("</title>\n");
    svg_paths(&mut svg, title);
    svg.push_str("</svg>\n");
    svg
}

fn sheet_svg(titles: &[&TitleAsset], sheet: &Sheet) -> Result<String, String> {
    let mut svg = String::new();
    let width = quant_number(sheet.width, sheet.quant);
    let height = quant_number(sheet.height, sheet.quant);
    writeln!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {width} {height}\" width=\"{width}\" height=\"{height}\" fill=\"currentColor\" color=\"#231f20\" data-surah-count=\"114\">"
    )
    .unwrap();
    for (row, title) in titles.iter().enumerate() {
        let (dx, dy) = sheet.placement(title, row);
        let sx = dx as i64 * BOUND_SCALE / sheet.quant as i64;
        let sy = dy as i64 * BOUND_SCALE / sheet.quant as i64;
        writeln!(
            svg,
            "<view id=\"surah-{:03}\" viewBox=\"{} {} {} {}\"/>",
            title.surah,
            number(title.visual.x0 + sx),
            number(title.visual.y0 + sy),
            number(title.visual.width()),
            number(title.visual.height())
        )
        .unwrap();
    }
    for (row, title) in titles.iter().enumerate() {
        let meta = metadata(&title.metadata, title.surah)?;
        let (dx, dy) = sheet.placement(title, row);
        write!(
            svg,
            "<g id=\"title-{:03}\" data-surah=\"{}\" data-page=\"{}\" transform=\"translate({} {})\"",
            title.surah,
            title.surah,
            title.source_page,
            quant_number(dx, sheet.quant),
            quant_number(dy, sheet.quant)
        )
        .unwrap();
        metadata_attributes(&mut svg, &meta);
        svg.push_str(">\n<title>");
        escaped(&mut svg, meta.arabic, false);
        svg.push_str("</title>\n");
        svg_paths(&mut svg, title);
        svg.push_str("</g>\n");
    }
    svg.push_str("</svg>\n");
    Ok(svg)
}

fn svg_paths(out: &mut String, title: &TitleAsset) {
    for path in &title.paths {
        write!(out, "<path data-kind=\"header_ink\" d=\"{}\"", qvp_format::svg_path_d(&path.commands, title.quant))
            .unwrap();
        if path.flags & PF_EVENODD != 0 {
            out.push_str(" fill-rule=\"evenodd\"");
        }
        out.push_str("/>\n");
    }
}

fn metadata_attributes(out: &mut String, meta: &Metadata<'_>) {
    attribute(out, "data-surah-name-ar", meta.arabic);
    attribute(out, "data-surah-name-latin", meta.latin);
    attribute(out, "data-surah-name-en", meta.english);
    attribute(out, "data-revelation-place", meta.place);
    attribute(out, "data-ayah-count", meta.ayah_count);
}

fn map_json(titles: &[&TitleAsset]) -> String {
    let mut out = String::from("{\"font_family\":\"Quran Surah Names\",\"first_codepoint\":\"E001\",\"surahs\":[");
    for (index, title) in titles.iter().enumerate() {
        if index != 0 {
            out.push(',');
        }
        let meta = metadata(&title.metadata, title.surah).expect("validated metadata");
        write!(
            out,
            "{{\"surah\":{},\"source_page\":{},\"codepoint\":\"{:04X}\",\"arabic\":{},\"latin\":{},\"english\":{},\"revelation_place\":{},\"ayah_count\":{}}}",
            title.surah,
            title.source_page,
            FIRST_CODEPOINT + title.surah as u32 - 1,
            crate::json_str(meta.arabic),
            crate::json_str(meta.latin),
            crate::json_str(meta.english),
            crate::json_str(meta.place),
            meta.ayah_count,
        )
        .unwrap();
    }
    out.push_str("]}\n");
    out
}

fn font_css() -> String {
    let mut out = String::from(
        "@font-face{font-family:'Quran Surah Names';src:url('./surah-names.woff2') format('woff2');font-display:swap}\n.qvp-surah-name{font-family:'Quran Surah Names';font-style:normal;font-weight:400;line-height:1}\n",
    );
    for surah in 1..=LAST_SURAH {
        writeln!(
            out,
            ".qvp-surah-name[data-surah=\"{surah}\"]::before{{content:\"\\{:04x}\"}}",
            FIRST_CODEPOINT + surah as u32 - 1
        )
        .unwrap();
    }
    out
}

fn append(path: &mut BezPath, commands: &[Cmd], quant: u16) {
    let point = |x: i32, y: i32| (x as f64 / quant as f64, y as f64 / quant as f64);
    for command in commands {
        match *command {
            Cmd::MoveTo(x, y) => path.move_to(point(x, y)),
            Cmd::LineTo(x, y) => path.line_to(point(x, y)),
            Cmd::QuadTo(x1, y1, x2, y2) => path.quad_to(point(x1, y1), point(x2, y2)),
            Cmd::CubicTo(x1, y1, x2, y2, x3, y3) => path.curve_to(point(x1, y1), point(x2, y2), point(x3, y3)),
            Cmd::Close => path.close_path(),
        }
    }
}

fn outward(bounds: Rect) -> VisualBounds {
    let scale = BOUND_SCALE as f64;
    VisualBounds {
        x0: (bounds.x0 * scale).floor() as i64,
        y0: (bounds.y0 * scale).floor() as i64,
        x1: (bounds.x1 * scale).ceil() as i64,
        y1: (bounds.y1 * scale).ceil() as i64,
    }
}

fn attribute(out: &mut String, name: &str, value: &str) {
    write!(out, " {name}=\"").unwrap();
    escaped(out, value, true);
    out.push('"');
}

fn escaped(out: &mut String, value: &str, attribute: bool) {
    for character in value.chars() {
        match character {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' if attribute => out.push_str("&quot;"),
            '\'' if attribute => out.push_str("&apos;"),
            _ => out.push(character),
        }
    }
}

fn number(value: i64) -> String {
    decimal(value, BOUND_SCALE as u64)
}

fn quant_number(value: i32, quant: u16) -> String {
    decimal(value as i64, quant as u64)
}

fn decimal(value: i64, scale: u64) -> String {
    let absolute = value.unsigned_abs();
    let mut out = String::new();
    if value < 0 {
        out.push('-');
    }
    write!(out, "{}", absolute / scale).unwrap();
    let fraction = absolute % scale;
    if fraction != 0 {
        let digits = scale.ilog10() as usize;
        let fraction = format!("{fraction:0digits$}");
        out.push('.');
        out.push_str(fraction.trim_end_matches('0'));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::convert;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_DIR: AtomicU64 = AtomicU64::new(0);

    #[derive(Default)]
    struct OutlineCount(usize);
    impl ttf_parser::OutlineBuilder for OutlineCount {
        fn move_to(&mut self, _x: f32, _y: f32) {
            self.0 += 1;
        }
        fn line_to(&mut self, _x: f32, _y: f32) {}
        fn quad_to(&mut self, _x1: f32, _y1: f32, _x: f32, _y: f32) {}
        fn curve_to(&mut self, _x1: f32, _y1: f32, _x2: f32, _y2: f32, _x: f32, _y: f32) {}
        fn close(&mut self) {}
    }

    fn page(surah: u16) -> PageData {
        convert(&format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 120 40" data-page="17">
              <g class="line" data-line="1">
                <g class="surah-name" data-sid="{surah}" data-surah-name-ar="اسم" data-surah-name-latin="Name" data-surah-name-en="Light &amp; Opening" data-revelation-place="makkah" data-ayah-count="7">
                  <path data-kind="ornament" fill-rule="evenodd" d="M10 5H110V35H10ZM20 10V30H100V10Z"/>
                  <path data-kind="header_ink" d="M45 15H75V25H45Z"/>
                  <path data-kind="header_ink" fill-rule="evenodd" d="M80 16H84V24H80Z"/>
                </g>
              </g>
            </svg>"#
        ))
        .unwrap()
        .page
    }

    fn complete_builder() -> Builder {
        let mut builder = Builder::default();
        let mut page = page(1);
        for surah in 1..=LAST_SURAH {
            page.decorations[0].surah = surah;
            builder.add_page(&page).unwrap();
        }
        builder
    }

    #[test]
    fn individual_assets_are_title_only_compact_and_self_describing() {
        let source = page(3);
        let title = extract(&source, &source.decorations[0]).unwrap();
        let bytes = encode(&title_qvp(&title));
        let asset = qvp_format::decode(&bytes).unwrap();
        assert_eq!((asset.header.width, asset.header.height), (39.0, 10.0));
        assert_eq!(asset.header.page, 17);
        assert_eq!(asset.paths.len(), 2);
        assert_eq!(asset.decorations[0].surah, 3);
        assert_eq!(asset.strings, ["اسم|Name|Light & Opening|makkah|7"]);
        assert!(asset.paths.iter().all(|path| path.kind == PathKind::HeaderInk));
        assert!(bytes.len() < 512, "sample title asset is {} bytes", bytes.len());

        let svg = render_svg(&title);
        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"45 15 39 10\""));
        assert!(svg.contains("data-surah-name-en=\"Light &amp; Opening\""));
        assert!(!svg.contains("data-kind=\"ornament\""));

        let mut rendered = qvp_core::Page::from_data(asset);
        rendered.layout(&qvp_core::LayoutSpec { viewport_w: 39.0, viewport_h: 10.0, ..Default::default() });
        assert_eq!(rendered.layout_draw_list().len(), 2);
    }

    #[test]
    fn bezier_control_points_stay_inside_the_qvp_asset_box() {
        let source = convert(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" data-page="1">
              <g class="line" data-line="1"><g class="surah-name" data-sid="1" data-surah-name-ar="اسم" data-surah-name-latin="Name" data-surah-name-en="Name" data-revelation-place="makkah" data-ayah-count="7">
                <path data-kind="header_ink" d="M0 100C50 200 150 0 200 100"/>
              </g></g>
            </svg>"#,
        )
        .unwrap()
        .page;
        let title = extract(&source, &source.decorations[0]).unwrap();
        let asset = qvp_format::decode(&encode(&title_qvp(&title))).unwrap();
        let bbox = asset.paths[0].bbox;
        assert_eq!(bbox.x0, 0);
        assert_eq!(bbox.y0, 0);
        assert_eq!(bbox.x1 as f32 / asset.header.quant as f32, asset.header.width);
        assert_eq!(bbox.y1 as f32 / asset.header.quant as f32, asset.header.height);
    }

    #[test]
    fn title_may_have_no_frame_but_no_other_path_kind() {
        let mut source = page(1);
        let decoration = source.decorations[0];
        source.decorations[0].first_path += 1;
        source.decorations[0].n_paths -= 1;
        extract(&source, &source.decorations[0]).unwrap();

        source.paths[decoration.first_path as usize + 1].kind = PathKind::PageNumber;
        assert!(extract(&source, &decoration).unwrap_err().contains("unexpected page_number ink"));
    }

    #[test]
    fn rejects_incomplete_metadata() {
        let mut source = page(1);
        let string = source.decorations[0].text as usize;
        source.strings[string] = "اسم||||".into();
        assert!(extract(&source, &source.decorations[0]).unwrap_err().contains("invalid name metadata"));
    }

    #[test]
    fn requires_each_surah_exactly_once_without_replacing_the_first() {
        let mut builder = Builder::default();
        let source = page(1);
        builder.add_page(&source).unwrap();
        let first = builder.titles[&1].metadata.clone();
        assert!(builder.add_page(&source).unwrap_err().contains("duplicate"));
        assert_eq!(builder.titles[&1].metadata, first);
        assert!(builder.write(Path::new("unused")).unwrap_err().contains("missing 2"));
    }

    #[test]
    fn writes_every_delivery_format_and_keeps_siblings() {
        let root = std::env::temp_dir().join(format!(
            "qvp-surah-names-{}-{}",
            std::process::id(),
            NEXT_DIR.fetch_add(1, Ordering::Relaxed)
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        fs::write(root.join("atlas.qva"), b"keep").unwrap();

        assert_eq!(complete_builder().write(&root).unwrap(), LAST_SURAH as usize);
        assert_eq!(fs::read(root.join("atlas.qva")).unwrap(), b"keep");
        let directory = root.join(DIR);
        assert_eq!(fs::read_dir(directory.join("qvp")).unwrap().count(), 115);
        assert_eq!(fs::read_dir(directory.join("svg")).unwrap().count(), 115);
        assert!(directory.join("qvp/001.qvp").is_file());
        assert!(directory.join("qvp/114.qvp").is_file());
        assert!(directory.join("qvp/all.qvp").is_file());
        assert!(directory.join("svg/001.svg").is_file());
        assert!(directory.join("svg/all.svg").is_file());
        let woff2 = fs::read(directory.join("surah-names.woff2")).unwrap();
        assert_eq!(woff2.get(..4), Some(b"wOF2".as_slice()));
        let otf = woofwoof::decompress(&woff2).unwrap();
        let face = ttf_parser::Face::parse(&otf, 0).unwrap();
        let glyph = face.glyph_index('\u{e001}').unwrap();
        assert_eq!(glyph.0, 1);
        let mut outline = OutlineCount::default();
        assert!(face.outline_glyph(glyph, &mut outline).is_some());
        assert_eq!(outline.0, 2);
        assert!(fs::read_to_string(directory.join("surah-names.css")).unwrap().contains("data-surah=\"114\""));
        assert!(fs::read_to_string(directory.join("map.json")).unwrap().contains("\"codepoint\":\"E072\""));

        let all = qvp_format::decode(&fs::read(directory.join("qvp/all.qvp")).unwrap()).unwrap();
        assert_eq!(all.decorations.len(), 114);
        assert_eq!(all.decorations[113].surah, 114);
        let all_svg = fs::read_to_string(directory.join("svg/all.svg")).unwrap();
        assert_eq!(all_svg.matches("<view id=\"surah-").count(), 114);
        assert_eq!(all_svg.matches("<g id=\"title-").count(), 114);

        assert!(!root.join(format!(".{DIR}.tmp")).exists());
        fs::remove_dir_all(root).unwrap();
    }
}
