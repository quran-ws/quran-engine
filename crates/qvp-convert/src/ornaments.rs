//! `quran-assets` catalogue → QVO. Reads a checkout (or a vendored copy) of
//! `quran-ws/quran-assets` — `catalog.json` plus the colour SVG of every asset —
//! and writes one ornament set the engine can load without an XML parser.
//!
//! Names: the catalogue calls the three asset types `ayah-markers`,
//! `surah-headers` and `page-frames`; those are its file names and stay quoted
//! here. The engine's own names are `ayah_mark`, `surah_header` and `page_frame`.
//!
//! Nothing about the artwork is changed. `<use>` instances are expanded (an
//! asset draws one quadrant and mirrors it, and ids are file-local, so two
//! assets in one document would collide), transforms are flattened into asset
//! units, and each outline keeps the part — the printed colour — it belongs to.
use crate::affine::Affine;
use crate::json::Json;
use qvp_format::ornaments::*;
use qvp_format::{Cmd, IBox};
use roxmltree::Node;
use std::collections::HashMap;
use std::path::Path;

#[derive(Default)]
pub struct Report {
    pub warnings: Vec<String>,
    /// (style, kind, outlines) for each asset that made it in.
    pub assets: Vec<(String, &'static str, u32)>,
}

/// Build an ornament set from a catalogue directory.
///
/// `styles` limits the build to the named mushafs; empty takes every style the
/// catalogue lists. An asset the catalogue names but does not deliver is a
/// warning, not an error — a partial style still dresses what it can.
pub fn build(dir: &Path, styles: &[String]) -> Result<(OrnamentSet, Report), String> {
    let catalog_path = dir.join("catalog.json");
    let bytes = std::fs::read(&catalog_path).map_err(|e| format!("{}: {e}", catalog_path.display()))?;
    let cat = Json::parse(&bytes).map_err(|e| format!("{}: {e}", catalog_path.display()))?;
    let assets = cat.get("assets").map(|a| a.arr()).unwrap_or(&[]);
    if assets.is_empty() {
        return Err(format!("{}: no assets", catalog_path.display()));
    }
    let mut names: Vec<String> = Vec::new();
    for a in assets {
        let s = a.s("style");
        if !s.is_empty() && !names.iter().any(|n| n == s) && (styles.is_empty() || styles.iter().any(|w| w == s)) {
            names.push(s.to_owned());
        }
    }
    names.sort();

    let mut set = OrnamentSet::default();
    let mut report = Report::default();
    for name in names {
        let mut style = Style { name: name.clone(), ..Style::default() };
        let mut parts: Vec<Part> = Vec::new();
        let mut drew_any = false;
        for kind in OrnamentKind::ALL {
            let Some(rec) = assets.iter().find(|a| a.s("type") == kind.catalog_type() && a.s("style") == name) else {
                continue;
            };
            let rel = rec.s("variants.color");
            if rel.is_empty() {
                report.warnings.push(format!("{name}/{}: no colour variant in the catalogue", kind.as_str()));
                continue;
            }
            // A tiled frame is assembled from its slices, so the whole drawing
            // is dead weight — two thirds of a style's bytes. Its viewBox and
            // its text-area slot are still the measurement the border is placed
            // from, so the record stays and only the outlines go.
            let slices = if kind == OrnamentKind::PageFrame { read_slices(dir, rec, &name, &mut set, &mut parts, &mut report) } else { None };
            let asset = match read_asset(&dir.join(rel), &mut set, &mut parts, slices.is_none()) {
                Ok(a) => a,
                Err(e) => {
                    report.warnings.push(format!("{name}/{}: {e}", kind.as_str()));
                    continue;
                }
            };
            report.assets.push((name.clone(), kind.as_str(), asset.n_paths));
            drew_any = true;
            if style.riwayah.is_empty() {
                style.riwayah = rec.s("riwaya").to_owned();
            }
            if style.license.id.is_empty() {
                style.license = License {
                    id: rec.s("license.id").to_owned(),
                    status: rec.s("license.status").to_owned(),
                    redistributable: rec.path("license.redistributable").and_then(|v| v.bool()).unwrap_or(false),
                    attribution: rec.s("license.attribution").to_owned(),
                };
            }
            style.slices = slices;
            match kind {
                OrnamentKind::AyahMark => style.ayah_mark = Some(asset),
                OrnamentKind::SurahHeader => style.surah_header = Some(asset),
                OrnamentKind::PageFrame => style.page_frame = Some(asset),
            }
        }
        if !drew_any {
            report.warnings.push(format!("{name}: no asset could be read; the style is left out"));
            continue;
        }
        style.parts = parts;
        set.styles.push(style);
    }
    if set.styles.is_empty() {
        return Err(format!("{}: nothing to build", catalog_path.display()));
    }
    Ok((set, report))
}

/// The corner and the two repeat units of a frame that tiles, when the
/// catalogue publishes them and all three read.
fn read_slices(dir: &Path, rec: &Json, name: &str, set: &mut OrnamentSet, parts: &mut Vec<Part>, report: &mut Report) -> Option<Slices> {
    let files = rec.path("slices.files").and_then(|v| v.obj())?;
    let mut slice = |key: &str| -> Result<Asset, String> {
        let p = files.get(key).and_then(|v| v.str()).ok_or_else(|| format!("slices.files has no {key:?}"))?;
        read_asset(&dir.join(p), set, parts, true)
    };
    let (a, b, c) = (slice("corner"), slice("edge-h"), slice("edge-v"));
    match (a, b, c) {
        (Ok(corner), Ok(edge_h), Ok(edge_v)) => Some(Slices {
            corner_w: rec.f("slices.corner.w").unwrap_or(corner.width() as f64) as f32,
            corner_h: rec.f("slices.corner.h").unwrap_or(corner.height() as f64) as f32,
            repeat_h: rec.f("slices.repeat.h").unwrap_or(edge_h.width() as f64) as f32,
            repeat_v: rec.f("slices.repeat.v").unwrap_or(edge_v.height() as f64) as f32,
            rotate: rec.s("slices.corner_mode") == "rotate",
            corner,
            edge_h,
            edge_v,
        }),
        (a, b, c) => {
            for e in [a, b, c].into_iter().filter_map(|r| r.err()) {
                report.warnings.push(format!("{name}/page_frame slices: {e}"));
            }
            report.warnings.push(format!("{name}/page_frame: no usable slices — the whole frame will be stretched"));
            None
        }
    }
}

/// Parse one asset SVG and append its outlines to `set`. With `geometry` off
/// only the viewBox and the slot are kept — the measurements a placement needs.
fn read_asset(path: &Path, set: &mut OrnamentSet, parts: &mut Vec<Part>, geometry: bool) -> Result<Asset, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let doc = roxmltree::Document::parse(&text).map_err(|e| format!("{}: xml: {e}", path.display()))?;
    let root = doc.root_element();
    let view_box = numbers(root.attribute("viewBox").unwrap_or("")).ok_or_else(|| format!("{}: missing or bad viewBox", path.display()))?;
    let slot = numbers(root.attribute("data-slot").unwrap_or(""));

    let mut by_id: HashMap<&str, Node> = HashMap::new();
    for n in root.descendants().filter(|n| n.is_element()) {
        if let Some(id) = n.attribute("id") {
            by_id.entry(id).or_insert(n);
        }
    }
    let first_path = set.paths.len() as u32;
    if geometry {
        let mut cx = AssetCtx { quant: set.quant as f64, set, parts, by_id, depth: 0 };
        cx.walk(root, Affine::IDENTITY, &Paint::default()).map_err(|e| format!("{}: {e}", path.display()))?;
    }
    Ok(Asset { view_box, slot, first_path, n_paths: set.paths.len() as u32 - first_path })
}

fn numbers(s: &str) -> Option<[f32; 4]> {
    let v: Vec<f32> = s.split([' ', ',', '\t', '\n']).filter(|t| !t.is_empty()).filter_map(|t| t.parse().ok()).collect();
    (v.len() == 4).then(|| [v[0], v[1], v[2], v[3]])
}

/// Inherited paint state while walking an asset.
#[derive(Clone, Default)]
struct Paint {
    part: Option<u16>,
    stroke: bool,
    stroke_width: f64,
    evenodd: bool,
}

struct AssetCtx<'a, 'd, 'i> {
    set: &'a mut OrnamentSet,
    parts: &'a mut Vec<Part>,
    by_id: HashMap<&'d str, Node<'d, 'i>>,
    quant: f64,
    depth: u32,
}

impl<'a, 'd, 'i> AssetCtx<'a, 'd, 'i> {
    fn part_of(&mut self, name: &str, color: u32, stroke: bool) -> u16 {
        if let Some(i) = self.parts.iter().position(|p| p.name == name) {
            return i as u16;
        }
        self.parts.push(Part { name: name.to_owned(), color, stroke });
        (self.parts.len() - 1) as u16
    }

    fn walk(&mut self, n: Node<'d, 'i>, tf: Affine, paint: &Paint) -> Result<(), String> {
        if self.depth > 32 {
            return Err("nested <use> too deep".into());
        }
        for child in n.children().filter(|c| c.is_element()) {
            let tag = child.tag_name().name();
            if tag == "metadata" || tag == "title" || tag == "desc" {
                continue;
            }
            let local = match child.attribute("transform") {
                Some(t) => tf.then(&Affine::parse(t)?),
                None => tf,
            };
            let mut p = paint.clone();
            if let Some(name) = child.attribute("data-part") {
                // A part group carries its own paint: a stroke for `line`, a fill
                // for every other printed colour, and `none` for the window the
                // design leaves open.
                let stroke_attr = child.attribute("stroke").filter(|v| *v != "none");
                let fill_attr = child.attribute("fill");
                let (color, is_stroke) = match stroke_attr {
                    Some(s) => (css_rgba(s), true),
                    None => (css_rgba(fill_attr.unwrap_or("#000000")), false),
                };
                p.stroke = is_stroke;
                p.part = Some(self.part_of(name, color, is_stroke));
            }
            if let Some(w) = child.attribute("stroke-width").and_then(|v| v.parse::<f64>().ok()) {
                p.stroke_width = w;
            }
            match child.attribute("fill-rule") {
                Some("evenodd") => p.evenodd = true,
                Some(_) => p.evenodd = false,
                None => {}
            }
            match tag {
                "use" => {
                    let href = child.attribute("href").or_else(|| child.attribute(("http://www.w3.org/1999/xlink", "href"))).unwrap_or("");
                    let Some(target) = href.strip_prefix('#').and_then(|id| self.by_id.get(id).copied()) else {
                        continue;
                    };
                    // An instance is the referenced element drawn again, so the
                    // element itself is walked, not only its children.
                    let inner = match target.attribute("transform") {
                        Some(t) => local.then(&Affine::parse(t)?),
                        None => local,
                    };
                    self.depth += 1;
                    self.walk(target, inner, &p)?;
                    self.depth -= 1;
                }
                "path" => {
                    let Some(d) = child.attribute("d") else { continue };
                    let Some(part) = p.part else { continue };
                    let cmds = parse_d(d, &local, self.quant)?;
                    if cmds.is_empty() {
                        continue;
                    }
                    let mut flags = 0u8;
                    if p.evenodd {
                        flags |= OPF_EVENODD;
                    }
                    let width = if p.stroke {
                        flags |= OPF_STROKE;
                        // a stroke is drawn in the space the path is in, so the
                        // width travels through the same transform
                        (p.stroke_width * stroke_scale(&local) * self.quant).round() as i32
                    } else {
                        0
                    };
                    self.set.push_path(part, flags, width, &cmds);
                }
                _ => self.walk(child, local, &p)?,
            }
        }
        Ok(())
    }
}

fn stroke_scale(m: &Affine) -> f64 {
    (m.a * m.d - m.b * m.c).abs().sqrt()
}

fn parse_d(d: &str, tf: &Affine, quant: f64) -> Result<Vec<Cmd>, String> {
    let mut cmds = Vec::new();
    let q = |x: f64, y: f64| {
        let (px, py) = tf.apply(x, y);
        ((px * quant).round() as i32, (py * quant).round() as i32)
    };
    for seg in svgtypes::SimplifyingPathParser::from(d) {
        let seg = seg.map_err(|e| format!("path d: {e}"))?;
        use svgtypes::SimplePathSegment as S;
        cmds.push(match seg {
            S::MoveTo { x, y } => {
                let (x, y) = q(x, y);
                Cmd::MoveTo(x, y)
            }
            S::LineTo { x, y } => {
                let (x, y) = q(x, y);
                Cmd::LineTo(x, y)
            }
            S::Quadratic { x1, y1, x, y } => {
                let (x1, y1) = q(x1, y1);
                let (x, y) = q(x, y);
                Cmd::QuadTo(x1, y1, x, y)
            }
            S::CurveTo { x1, y1, x2, y2, x, y } => {
                let (x1, y1) = q(x1, y1);
                let (x2, y2) = q(x2, y2);
                let (x, y) = q(x, y);
                Cmd::CubicTo(x1, y1, x2, y2, x, y)
            }
            S::ClosePath => Cmd::Close,
        });
    }
    Ok(cmds)
}

/// `#rgb`, `#rrggbb`, `none` → 0xRRGGBBAA. Anything else is treated as opaque black.
fn css_rgba(s: &str) -> u32 {
    let t = s.trim();
    if t.eq_ignore_ascii_case("none") || t.is_empty() {
        return 0;
    }
    let h = t.strip_prefix('#').unwrap_or(t);
    let full = match h.len() {
        3 | 4 => h.chars().flat_map(|c| [c, c]).collect::<String>(),
        _ => h.to_owned(),
    };
    match full.len() {
        6 => u32::from_str_radix(&full, 16).map(|v| v << 8 | 0xff).unwrap_or(0x000000ff),
        8 => u32::from_str_radix(&full, 16).unwrap_or(0x000000ff),
        _ => 0x000000ff,
    }
}

/// A human summary of what a set holds, for the converter's output.
pub fn describe(set: &OrnamentSet) -> String {
    let mut s = String::new();
    for st in &set.styles {
        let n: u32 = OrnamentKind::ALL.iter().filter_map(|k| st.asset(*k)).map(|a| a.n_paths).sum();
        s.push_str(&format!(
            "{}: {} parts, {} outlines{}{}{}{} — {} {}, redistributable: {}\n",
            st.name,
            st.parts.len(),
            n,
            if st.ayah_mark.is_some() { " ayah_mark" } else { "" },
            if st.surah_header.is_some() { " surah_header" } else { "" },
            if st.page_frame.is_some() { " page_frame" } else { "" },
            if st.slices.is_some() { " (tiled)" } else { "" },
            if st.license.id.is_empty() { "unstated licence" } else { &st.license.id },
            st.license.status,
            st.license.redistributable
        ));
    }
    s
}

/// The bbox of every outline of an asset, in quantised asset units — a cheap
/// sanity check that a converted asset really fills its own viewBox.
pub fn asset_bbox(set: &OrnamentSet, a: &Asset) -> IBox {
    let mut bb = IBox::EMPTY;
    for i in a.first_path..a.first_path + a.n_paths {
        if let Some(p) = set.paths.get(i as usize) {
            bb.union(&p.bbox);
        }
    }
    bb
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn colours() {
        assert_eq!(css_rgba("#fff"), 0xffffffff);
        assert_eq!(css_rgba("#00649b"), 0x00649bff);
        assert_eq!(css_rgba("none"), 0);
    }
    #[test]
    fn view_box_numbers() {
        assert_eq!(numbers("0 0 73.162 100"), Some([0.0, 0.0, 73.162, 100.0]));
        assert_eq!(numbers("0 0 100"), None);
    }
}

/// A dressed page as one standalone SVG — the debugging aid for placement, the
/// way `qvp2svg` is for the page itself. The ornament layer comes first so it
/// paints behind the print, exactly as a host must draw it.
pub fn dress_svg(page: &mut qvp_core::Page, dress: &qvp_core::Dress) -> String {
    use std::fmt::Write;
    let vb = dress.view_box;
    let mut s = String::with_capacity(256 * 1024);
    write!(s, "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{:.3} {:.3} {:.3} {:.3}\">", vb[0], vb[1], vb[2], vb[3]).unwrap();
    s.push_str("<g class=\"ornaments\">");
    for d in &dress.table {
        let mut path = String::new();
        let pts = &dress.pts[d.pt_start as usize..(d.pt_start + d.pt_count) as usize];
        let mut i = 0usize;
        let mut take = |n: usize, out: &mut String| {
            for k in 0..n {
                write!(out, "{}{:.3} {:.3}", if k > 0 { " " } else { "" }, pts[i + k * 2], pts[i + k * 2 + 1]).unwrap();
            }
            i += n * 2;
        };
        for op in &dress.ops[d.op_start as usize..(d.op_start + d.op_count) as usize] {
            match *op {
                qvp_format::OP_MOVE => {
                    path.push('M');
                    take(1, &mut path);
                }
                qvp_format::OP_LINE => {
                    path.push('L');
                    take(1, &mut path);
                }
                qvp_format::OP_QUAD => {
                    path.push('Q');
                    take(2, &mut path);
                }
                qvp_format::OP_CUBIC => {
                    path.push('C');
                    take(3, &mut path);
                }
                _ => path.push('Z'),
            }
        }
        let hex = format!("#{:06x}", d.color >> 8);
        write!(s, "<path d=\"{path}\"").unwrap();
        if d.flags & qvp_core::dress::ODF_STROKE != 0 {
            write!(s, " fill=\"none\" stroke=\"{hex}\" stroke-width=\"{:.4}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"", d.stroke_width).unwrap();
        } else {
            write!(s, " fill=\"{hex}\"").unwrap();
            if d.flags & qvp_core::dress::ODF_EVENODD != 0 {
                s.push_str(" fill-rule=\"evenodd\"");
            }
        }
        s.push_str("/>");
    }
    s.push_str("</g>");
    // the print on top, with the rings the dress hid left out
    let colors = page.paint().to_vec();
    let p = page.data();
    for (i, r) in p.paths.iter().enumerate() {
        if colors[i] & 0xff == 0 {
            continue;
        }
        let Ok(cmds) = p.path_cmds(i) else { continue };
        write!(s, "<path d=\"{}\" fill=\"#231f20\"", qvp_format::svg_path_d(&cmds, p.header.quant)).unwrap();
        if r.flags & qvp_format::PF_EVENODD != 0 {
            s.push_str(" fill-rule=\"evenodd\"");
        }
        s.push_str("/>");
    }
    s.push_str("</svg>");
    s
}
