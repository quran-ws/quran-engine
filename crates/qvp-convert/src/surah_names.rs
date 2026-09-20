//! Reusable, title-only surah-name SVGs derived from converted page decorations.
use kurbo::{BezPath, Rect, Shape};
use qvp_format::{svg_path_d, Cmd, DecoKind, DecoRec, PageData, PathKind, NONE_U16, PF_EVENODD};
use std::collections::BTreeMap;
use std::fmt::Write;
use std::fs;
use std::path::Path;

pub const DIR: &str = "surah-names";
const LAST_SURAH: u16 = 114;

#[derive(Default)]
pub struct Builder {
    files: BTreeMap<u16, String>,
}

impl Builder {
    pub fn add_page(&mut self, page: &PageData) -> Result<(), String> {
        for decoration in page.decorations.iter().filter(|decoration| decoration.kind == DecoKind::SurahName) {
            let surah = decoration.surah;
            if !(1..=LAST_SURAH).contains(&surah) {
                return Err(format!("surah-name decoration has invalid surah {surah}"));
            }
            if self.files.contains_key(&surah) {
                return Err(format!("duplicate surah-name decoration for surah {surah}"));
            }
            self.files.insert(surah, render(page, decoration)?);
        }
        Ok(())
    }

    pub fn write(&self, out_dir: &Path) -> Result<usize, String> {
        let missing: Vec<_> = (1..=LAST_SURAH).filter(|number| !self.files.contains_key(number)).collect();
        if !missing.is_empty() {
            return Err(format!(
                "surah-name asset set is incomplete; missing {}",
                missing.iter().map(u16::to_string).collect::<Vec<_>>().join(", ")
            ));
        }

        fs::create_dir_all(out_dir).map_err(|error| format!("create {}: {error}", out_dir.display()))?;
        let directory = out_dir.join(DIR);
        let temporary = out_dir.join(format!(".{DIR}.tmp"));
        if temporary.exists() {
            fs::remove_dir_all(&temporary).map_err(|error| format!("remove {}: {error}", temporary.display()))?;
        }
        fs::create_dir(&temporary).map_err(|error| format!("create {}: {error}", temporary.display()))?;
        for (surah, svg) in &self.files {
            let path = temporary.join(format!("{surah:03}.svg"));
            if let Err(error) = fs::write(&path, svg) {
                let _ = fs::remove_dir_all(&temporary);
                return Err(format!("write {}: {error}", path.display()));
            }
        }
        if directory.exists() {
            fs::remove_dir_all(&directory).map_err(|error| format!("remove {}: {error}", directory.display()))?;
        }
        if let Err(error) = fs::rename(&temporary, &directory) {
            let _ = fs::remove_dir_all(&temporary);
            return Err(format!("rename {} to {}: {error}", temporary.display(), directory.display()));
        }
        Ok(self.files.len())
    }
}

struct Metadata<'a> {
    arabic: &'a str,
    latin: &'a str,
    english: &'a str,
    place: &'a str,
    ayah_count: &'a str,
}

fn metadata<'a>(page: &'a PageData, decoration: &DecoRec) -> Result<Metadata<'a>, String> {
    if decoration.text == NONE_U16 {
        return Err(format!("surah {} has no name metadata", decoration.surah));
    }
    let value = page
        .strings
        .get(decoration.text as usize)
        .ok_or_else(|| format!("surah {} name metadata points outside the string table", decoration.surah))?;
    let fields: Vec<_> = value.splitn(5, '|').collect();
    if fields.len() != 5 || fields.iter().any(|field| field.is_empty()) || fields[4].parse::<u16>().is_err() {
        return Err(format!("surah {} has invalid name metadata", decoration.surah));
    }
    Ok(Metadata { arabic: fields[0], latin: fields[1], english: fields[2], place: fields[3], ayah_count: fields[4] })
}

struct TitlePath {
    commands: Vec<Cmd>,
    evenodd: bool,
}

struct Bounds {
    x0: i64,
    y0: i64,
    x1: i64,
    y1: i64,
}

impl Bounds {
    fn width(&self) -> i64 {
        self.x1 - self.x0
    }

    fn height(&self) -> i64 {
        self.y1 - self.y0
    }
}

fn title(page: &PageData, decoration: &DecoRec) -> Result<(Vec<TitlePath>, Bounds), String> {
    let end = decoration
        .first_path
        .checked_add(decoration.n_paths as u32)
        .filter(|end| *end as usize <= page.paths.len())
        .ok_or_else(|| format!("surah {} path range is outside the page", decoration.surah))?;
    let mut paths = Vec::new();
    let mut geometry = BezPath::new();
    for index in decoration.first_path..end {
        let record = &page.paths[index as usize];
        match record.kind {
            PathKind::HeaderInk => {
                let commands = page
                    .path_cmds(index as usize)
                    .map_err(|error| format!("surah {} path {index}: {error}", decoration.surah))?;
                append(&mut geometry, &commands, page.header.quant);
                paths.push(TitlePath { commands, evenodd: record.flags & PF_EVENODD != 0 });
            }
            PathKind::Ornament => {}
            kind => return Err(format!("surah {} contains unexpected {} ink", decoration.surah, kind.as_str())),
        }
    }
    if paths.is_empty() {
        return Err(format!("surah {} has no calligraphic title ink", decoration.surah));
    }
    let bounds = geometry.bounding_box();
    if bounds.width() <= 0.0 || bounds.height() <= 0.0 {
        return Err(format!("surah {} has empty calligraphic title ink", decoration.surah));
    }
    Ok((paths, outward(bounds)))
}

fn append(path: &mut BezPath, commands: &[Cmd], quant: u16) {
    let point = |x: i32, y: i32| (x as f64 / quant as f64, y as f64 / quant as f64);
    for command in commands {
        match *command {
            Cmd::MoveTo(x, y) => path.move_to(point(x, y)),
            Cmd::LineTo(x, y) => path.line_to(point(x, y)),
            Cmd::QuadTo(x1, y1, x2, y2) => path.quad_to(point(x1, y1), point(x2, y2)),
            Cmd::CubicTo(x1, y1, x2, y2, x3, y3) => {
                path.curve_to(point(x1, y1), point(x2, y2), point(x3, y3));
            }
            Cmd::Close => path.close_path(),
        }
    }
}

const BOUND_SCALE: i64 = 1_000_000;

fn outward(bounds: Rect) -> Bounds {
    let scale = BOUND_SCALE as f64;
    Bounds {
        x0: (bounds.x0 * scale).floor() as i64,
        y0: (bounds.y0 * scale).floor() as i64,
        x1: (bounds.x1 * scale).ceil() as i64,
        y1: (bounds.y1 * scale).ceil() as i64,
    }
}

fn render(page: &PageData, decoration: &DecoRec) -> Result<String, String> {
    if page.header.quant == 0 {
        return Err("page quantisation is zero".into());
    }
    let (paths, bounds) = title(page, decoration)?;
    let meta = metadata(page, decoration)?;
    let width = number(bounds.width());
    let height = number(bounds.height());
    let mut svg = String::new();
    write!(
        svg,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{} {} {width} {height}\" width=\"{width}\" height=\"{height}\" fill=\"currentColor\" color=\"#231f20\" data-surah=\"{}\" data-page=\"{}\"",
        number(bounds.x0),
        number(bounds.y0),
        decoration.surah,
        page.header.page,
    )
    .unwrap();
    attribute(&mut svg, "data-surah-name-ar", meta.arabic);
    attribute(&mut svg, "data-surah-name-latin", meta.latin);
    attribute(&mut svg, "data-surah-name-en", meta.english);
    attribute(&mut svg, "data-revelation-place", meta.place);
    attribute(&mut svg, "data-ayah-count", meta.ayah_count);
    svg.push_str(">\n<title>");
    escaped(&mut svg, meta.arabic, false);
    svg.push_str("</title>\n");
    for path in paths {
        write!(svg, "<path data-kind=\"header_ink\" d=\"{}\"", svg_path_d(&path.commands, page.header.quant)).unwrap();
        if path.evenodd {
            svg.push_str(" fill-rule=\"evenodd\"");
        }
        svg.push_str("/>\n");
    }
    svg.push_str("</svg>\n");
    Ok(svg)
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
    let absolute = value.unsigned_abs();
    let scale = BOUND_SCALE as u64;
    let mut out = String::new();
    if value < 0 {
        out.push('-');
    }
    write!(out, "{}", absolute / scale).unwrap();
    let fraction = absolute % scale;
    if fraction != 0 {
        let fraction = format!("{fraction:06}");
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
    fn asset_is_title_only_tight_and_self_describing() {
        let page = page(3);
        let svg = render(&page, &page.decorations[0]).unwrap();
        assert!(svg.starts_with("<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"45 15 39 10\""));
        assert!(svg.contains("fill=\"currentColor\" color=\"#231f20\""));
        assert!(svg.contains("data-surah=\"3\" data-page=\"17\""));
        assert!(svg.contains("data-surah-name-ar=\"اسم\""));
        assert!(svg.contains("data-surah-name-en=\"Light &amp; Opening\""));
        assert!(svg.contains("<title>اسم</title>"));
        assert_eq!(svg.matches("data-kind=\"header_ink\"").count(), 2);
        assert_eq!(svg.matches("fill-rule=\"evenodd\"").count(), 1);
        assert!(!svg.contains("data-kind=\"ornament\""));
    }

    #[test]
    fn bezier_control_points_do_not_pad_the_view_box() {
        let page = convert(
            r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 200" data-page="1">
              <g class="line" data-line="1"><g class="surah-name" data-sid="1" data-surah-name-ar="اسم" data-surah-name-latin="Name" data-surah-name-en="Name" data-revelation-place="makkah" data-ayah-count="7">
                <path data-kind="header_ink" d="M0 100C50 200 150 0 200 100"/>
              </g></g>
            </svg>"#,
        )
        .unwrap()
        .page;
        let svg = render(&page, &page.decorations[0]).unwrap();
        let view_box = svg.split_once("viewBox=\"").unwrap().1.split_once('"').unwrap().0;
        let values: Vec<f64> = view_box.split_whitespace().map(|value| value.parse().unwrap()).collect();
        assert_eq!(values[0], 0.0);
        assert_eq!(values[2], 200.0);
        assert!(values[1] > 70.0 && values[1] < 72.0, "{view_box}");
        assert!(values[3] > 57.0 && values[3] < 59.0, "{view_box}");
    }

    #[test]
    fn title_may_have_no_frame_but_no_other_path_kind() {
        let mut page = page(1);
        let decoration = page.decorations[0];
        page.decorations[0].first_path += 1;
        page.decorations[0].n_paths -= 1;
        render(&page, &page.decorations[0]).unwrap();

        page.paths[decoration.first_path as usize + 1].kind = PathKind::PageNumber;
        assert!(render(&page, &decoration).unwrap_err().contains("unexpected page_number ink"));
    }

    #[test]
    fn rejects_incomplete_metadata() {
        let mut page = page(1);
        let string = page.decorations[0].text as usize;
        page.strings[string] = "اسم||||".into();
        assert!(render(&page, &page.decorations[0]).unwrap_err().contains("invalid name metadata"));
    }

    #[test]
    fn requires_each_surah_exactly_once_without_replacing_the_first() {
        let mut builder = Builder::default();
        let page = page(1);
        builder.add_page(&page).unwrap();
        let first = builder.files[&1].clone();
        assert!(builder.add_page(&page).unwrap_err().contains("duplicate"));
        assert_eq!(builder.files[&1], first);
        assert!(builder.write(Path::new("unused")).unwrap_err().contains("missing 2"));
    }

    #[test]
    fn writes_under_the_output_without_replacing_siblings() {
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
        assert_eq!(fs::read_dir(root.join(DIR)).unwrap().count(), LAST_SURAH as usize);
        assert!(root.join(DIR).join("001.svg").is_file());
        assert!(root.join(DIR).join("114.svg").is_file());
        assert!(!root.join(format!(".{DIR}.tmp")).exists());
        fs::remove_dir_all(root).unwrap();
    }
}
