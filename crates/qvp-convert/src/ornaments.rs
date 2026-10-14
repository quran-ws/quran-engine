//! `quran-assets` catalogue to QVO. Reads a checkout of `quran-ws/quran-assets`
//! (`catalog.json`, schema `quran-assets/catalog@1`, and the colour SVG of every asset)
//! and writes one ornament set the engine loads without an XML parser.
//!
//! The catalogue calls the three asset types `ayah-markers`, `surah-headers` and
//! `page-frames`. The engine's own names are `ayah_mark`, `surah_header` and
//! `page_frame`.
//!
//! Nothing about the artwork changes. `<use>` instances are expanded (an asset draws
//! one half and mirrors it, and ids are file-local), transforms are flattened into
//! asset units, and each outline keeps the part its `data-part` names. The palette is
//! the catalogue's, in its order; a part the SVG draws and the palette lacks is added
//! after it with the SVG's own colour.
//! The pure white fill of a scanned design is its paper and is left open, like the slot.
//!
//! An `<ellipse>`, `<circle>`, `<rect>`, `<polygon>` or `<polyline>` becomes path data
//! and goes through the same parser as a `<path>`. An element the engine cannot draw, a
//! `<use>` that points to nothing, and a clip or a mask are warnings in the report.
use crate::affine::Affine;
use crate::json::Json;
use qvp_format::ornaments::*;
use qvp_format::Cmd;
use roxmltree::Node;
use std::collections::HashMap;
use std::path::Path;

#[derive(Clone, Debug, Default)]
pub struct BuildOptions {
    /// Only these styles; empty takes every style the catalogue lists.
    pub styles: Vec<String>,
    /// Only assets whose licence status is `confirmed`.
    pub redistributable_only: bool,
}

#[derive(Default)]
pub struct Report {
    pub warnings: Vec<String>,
    /// (style, kind, outlines) for each asset that made it in.
    pub assets: Vec<(String, &'static str, u32)>,
}

/// Build an ornament set from a catalogue directory. An asset the catalogue names but
/// does not deliver is a warning, not an error.
pub fn build(dir: &Path, opts: &BuildOptions) -> Result<(OrnamentSet, Report), String> {
    let catalog_path = dir.join("catalog.json");
    let bytes = std::fs::read(&catalog_path).map_err(|e| format!("{}: {e}", catalog_path.display()))?;
    let cat = Json::parse(&bytes).map_err(|e| format!("{}: {e}", catalog_path.display()))?;
    let assets = cat.get("assets").map(|a| a.arr()).unwrap_or(&[]);
    if assets.is_empty() {
        return Err(format!("{}: no assets", catalog_path.display()));
    }
    let mut set = OrnamentSet::default();
    let mut report = Report::default();
    for rec in assets {
        let style = rec.s("style");
        let Some(kind) = OrnamentKind::ALL.iter().copied().find(|k| k.catalog_type() == rec.s("type")) else {
            report.warnings.push(format!("{}: unknown type {:?}", rec.s("id"), rec.s("type")));
            continue;
        };
        if !opts.styles.is_empty() && !opts.styles.iter().any(|w| w == style) {
            continue;
        }
        let status = rec.s("license.status");
        let redistributable = status == "confirmed";
        if opts.redistributable_only && !redistributable {
            report.warnings.push(format!("{style}/{}: {status}, left out", kind.as_str()));
            continue;
        }
        let rel = rec.s("variants.color");
        if rel.is_empty() {
            report.warnings.push(format!("{style}/{}: no colour variant in the catalogue", kind.as_str()));
            continue;
        }
        let scan = rec.s("lineage") != "font";
        // The lightest fill of a scanned design is the traced paper, quantised to pure
        // white. It is not ink: on a dark page it would paint white slabs behind the
        // numbers and names. It is left open, like the slot, so the host's page shows
        // through; a host repaints `c1` to put a paper colour back.
        let mut parts: Vec<Part> = rec
            .get("palette")
            .map(|p| p.arr())
            .unwrap_or(&[])
            .iter()
            .map(|p| {
                let color = css_rgba(p.s("hex"));
                Part {
                    name: p.s("name").to_owned(),
                    color: if scan && color == 0xffffffff { 0 } else { color },
                    stroke: p.path("stroke").and_then(|v| v.bool()).unwrap_or(false),
                }
            })
            .collect();
        let slot = rec
            .get("slots")
            .and_then(|s| s.arr().first())
            .and_then(|s| Some([s.f("x")? as f32, s.f("y")? as f32, s.f("w")? as f32, s.f("h")? as f32]));
        // A tiled frame is assembled from its slices, so the whole drawing is dead
        // weight. Its viewBox and slot are still the measurement the border is placed
        // from, so the record stays and only the outlines go.
        // A failed asset drops the whole record, so the outlines its slices added go too.
        let mark = Mark::of(&set, &parts);
        let slices = if kind == OrnamentKind::PageFrame {
            read_slices(dir, rec, style, &mut set, &mut parts, &mut report)
        } else {
            None
        };
        let asset = match read_asset(dir, rel, &mut set, &mut parts, slices.is_none(), slot, &mut report.warnings) {
            Ok(a) => a,
            Err(e) => {
                mark.restore(&mut set, &mut parts);
                report.warnings.push(format!("{style}/{}: {e}", kind.as_str()));
                continue;
            }
        };
        report.assets.push((style.to_owned(), kind.as_str(), asset.n_paths));
        set.ornaments.push(Ornament {
            kind,
            style: style.to_owned(),
            lineage: if scan { Lineage::Scan } else { Lineage::Font },
            riwayah: rec.s("riwayah").to_owned(),
            license: License {
                id: rec.s("license.id").to_owned(),
                status: status.to_owned(),
                redistributable,
                attribution: rec.s("license.attribution").to_owned(),
            },
            parts,
            asset,
            slices,
        });
    }
    if set.ornaments.is_empty() {
        return Err(format!("{}: nothing to build", catalog_path.display()));
    }
    Ok((set, report))
}

/// The lengths of the outline pool and the palette before a read. A read that fails
/// cuts them back, so the set keeps no outline that no asset owns.
struct Mark {
    paths: usize,
    ops: usize,
    parts: usize,
}

impl Mark {
    fn of(set: &OrnamentSet, parts: &[Part]) -> Mark {
        Mark { paths: set.paths.len(), ops: set.ops.len(), parts: parts.len() }
    }
    fn restore(&self, set: &mut OrnamentSet, parts: &mut Vec<Part>) {
        set.paths.truncate(self.paths);
        set.ops.truncate(self.ops);
        parts.truncate(self.parts);
    }
}

/// The corner and the two repeat units of a frame that tiles, when the catalogue
/// publishes them and all three read.
fn read_slices(
    dir: &Path,
    rec: &Json,
    style: &str,
    set: &mut OrnamentSet,
    parts: &mut Vec<Part>,
    report: &mut Report,
) -> Option<Slices> {
    let files = rec.path("slices.files").and_then(|v| v.obj())?;
    let mark = Mark::of(set, parts);
    let mut slice = |key: &str| -> Result<Asset, String> {
        let p = files.get(key).and_then(|v| v.str()).ok_or_else(|| format!("slices.files has no {key:?}"))?;
        read_asset(dir, p, set, parts, true, None, &mut report.warnings)
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
            mark.restore(set, parts);
            for e in [a, b, c].into_iter().filter_map(|r| r.err()) {
                report.warnings.push(format!("{style}/page_frame slices: {e}"));
            }
            report.warnings.push(format!("{style}/page_frame: no usable slices, the whole frame will be stretched"));
            None
        }
    }
}

/// Parse one asset SVG (`rel` under `dir`) and append its outlines to `set`. With
/// `geometry` off only the viewBox and the slot are kept. `slot` is the catalogue's;
/// the SVG's `data-slot` is the fallback. What the walk cannot draw goes to `warnings`.
fn read_asset(
    dir: &Path,
    rel: &str,
    set: &mut OrnamentSet,
    parts: &mut Vec<Part>,
    geometry: bool,
    slot: Option<[f32; 4]>,
    warnings: &mut Vec<String>,
) -> Result<Asset, String> {
    let path = &dir.join(rel);
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let doc = roxmltree::Document::parse(&text).map_err(|e| format!("{}: xml: {e}", path.display()))?;
    let root = doc.root_element();
    let view_box = numbers(root.attribute("viewBox").unwrap_or(""))
        .ok_or_else(|| format!("{}: missing or bad viewBox", path.display()))?;
    let slot = slot.or_else(|| numbers(root.attribute("data-slot").unwrap_or("")));
    let mut by_id: HashMap<&str, Node> = HashMap::new();
    for n in root.descendants().filter(|n| n.is_element()) {
        if let Some(id) = n.attribute("id") {
            by_id.entry(id).or_insert(n);
        }
    }
    let first_path = set.paths.len() as u32;
    if geometry {
        let mark = Mark::of(set, parts);
        let mut cx =
            AssetCtx { quant: set.quant as f64, set, parts, by_id, depth: 0, file: rel, warnings, warned_clip: false };
        cx.warn_clip(root);
        if let Err(e) = cx.walk(root, Affine::IDENTITY, &Paint::default()) {
            mark.restore(set, parts);
            return Err(format!("{}: {e}", path.display()));
        }
    }
    Ok(Asset { view_box, slot, first_path, n_paths: set.paths.len() as u32 - first_path })
}

fn numbers(s: &str) -> Option<[f32; 4]> {
    let v: Vec<f32> = s
        .split(|c: char| c == ',' || c.is_ascii_whitespace())
        .filter(|t| !t.is_empty())
        .filter_map(|t| t.parse().ok())
        .collect();
    (v.len() == 4).then(|| [v[0], v[1], v[2], v[3]])
}

/// Inherited paint state while walking an asset.
#[derive(Clone)]
struct Paint {
    part: Option<u16>,
    stroke: bool,
    /// In user units. SVG's initial value is 1.
    stroke_width: f64,
    evenodd: bool,
}

impl Default for Paint {
    fn default() -> Self {
        Paint { part: None, stroke: false, stroke_width: 1.0, evenodd: false }
    }
}

struct AssetCtx<'a, 'd, 'i> {
    set: &'a mut OrnamentSet,
    parts: &'a mut Vec<Part>,
    by_id: HashMap<&'d str, Node<'d, 'i>>,
    quant: f64,
    depth: u32,
    /// The asset's path in the catalogue, for warnings.
    file: &'a str,
    warnings: &'a mut Vec<String>,
    /// A clip or a mask is reported once per file.
    warned_clip: bool,
}

impl<'a, 'd, 'i> AssetCtx<'a, 'd, 'i> {
    fn part_of(&mut self, name: &str, color: u32, stroke: bool) -> u16 {
        if let Some(i) = self.parts.iter().position(|p| p.name == name) {
            return i as u16;
        }
        self.parts.push(Part { name: name.to_owned(), color, stroke });
        (self.parts.len() - 1) as u16
    }

    fn warn(&mut self, msg: String) {
        self.warnings.push(format!("{}: {msg}", self.file));
    }

    /// A clip or a mask is not applied: the outlines are drawn whole.
    fn warn_clip(&mut self, n: Node<'d, 'i>) {
        if !self.warned_clip && (n.attribute("clip-path").is_some() || n.attribute("mask").is_some()) {
            self.warned_clip = true;
            self.warn(format!("<{}> has a clip-path or a mask, which is ignored", n.tag_name().name()));
        }
    }

    /// Fold the paint attributes of `n` into `p`: its part, stroke or fill colour,
    /// stroke width and fill rule. A child and a `<use>` target read them the same way.
    fn apply_attrs(&mut self, n: Node<'d, 'i>, p: &mut Paint) {
        self.warn_clip(n);
        if let Some(name) = n.attribute("data-part") {
            let stroke_attr = n.attribute("stroke").filter(|v| *v != "none");
            let (color, is_stroke) = match stroke_attr {
                Some(s) => (css_rgba(s), true),
                None => (css_rgba(n.attribute("fill").unwrap_or("#000000")), false),
            };
            p.stroke = is_stroke;
            p.part = Some(self.part_of(name, color, is_stroke));
        }
        if let Some(v) = n.attribute("stroke-width") {
            let t = v.trim();
            match t.strip_suffix("px").unwrap_or(t).trim().parse::<f64>() {
                Ok(w) => p.stroke_width = w,
                Err(_) => self.warn(format!("stroke-width {v:?} is not in user units, the inherited width is kept")),
            }
        }
        match n.attribute("fill-rule") {
            Some("evenodd") => p.evenodd = true,
            Some(_) => p.evenodd = false,
            None => {}
        }
    }

    fn walk(&mut self, n: Node<'d, 'i>, tf: Affine, paint: &Paint) -> Result<(), String> {
        if self.depth > 32 {
            return Err("nested <use> too deep".into());
        }
        for child in n.children().filter(|c| c.is_element()) {
            let tag = child.tag_name().name();
            // `<defs>`, `<symbol>`, `<clipPath>`, `<mask>`, `<pattern>` and `<marker>` draw
            // nothing where they stand. A `<use>` reaches their children through `by_id`.
            if matches!(
                tag,
                "metadata" | "title" | "desc" | "defs" | "symbol" | "clipPath" | "mask" | "pattern" | "marker"
            ) {
                continue;
            }
            let local = match child.attribute("transform") {
                Some(t) => tf.then(&Affine::parse(t)?),
                None => tf,
            };
            let mut p = paint.clone();
            self.apply_attrs(child, &mut p);
            match tag {
                "use" => {
                    let href = child
                        .attribute("href")
                        .or_else(|| child.attribute(("http://www.w3.org/1999/xlink", "href")))
                        .unwrap_or("");
                    let Some(target) = href.strip_prefix('#').and_then(|id| self.by_id.get(id).copied()) else {
                        self.warn(format!("<use href={href:?}> points to nothing, it is not drawn"));
                        continue;
                    };
                    // `x` and `y` move the instance: SVG appends `translate(x, y)` to the
                    // `<use>`'s own transform. `width` and `height` only size a `<symbol>`
                    // or an `<svg>`, which an asset does not reference.
                    let mut at = [0.0f64; 2];
                    for (k, name) in ["x", "y"].iter().enumerate() {
                        let Some(v) = child.attribute(*name) else { continue };
                        let t = v.trim();
                        match t.strip_suffix("px").unwrap_or(t).trim().parse::<f64>() {
                            Ok(n) => at[k] = n,
                            Err(_) => self.warn(format!("<use {name}={v:?}> is not in user units, it is taken as 0")),
                        }
                    }
                    if child.attribute("width").is_some() || child.attribute("height").is_some() {
                        self.warn(format!("<use href={href:?}> has a width or a height, which is ignored"));
                    }
                    let local = local.then(&Affine::translate(at[0], at[1]));
                    // An instance is the referenced element drawn again, so the element
                    // itself is walked, not only its children.
                    let inner = match target.attribute("transform") {
                        Some(t) => local.then(&Affine::parse(t)?),
                        None => local,
                    };
                    self.depth += 1;
                    self.walk_element(target, inner, &p)?;
                    self.depth -= 1;
                }
                _ => {
                    if !self.leaf(child, &local, &p)? {
                        self.walk(child, local, &p)?;
                    }
                }
            }
        }
        Ok(())
    }

    /// Walk `n` itself as if it were a child: its own `data-part`, paint and shape count.
    fn walk_element(&mut self, n: Node<'d, 'i>, tf: Affine, paint: &Paint) -> Result<(), String> {
        let mut p = paint.clone();
        self.apply_attrs(n, &mut p);
        if self.leaf(n, &tf, &p)? {
            return Ok(());
        }
        self.walk(n, tf, &p)
    }

    /// Draw `n` when it is an element that draws. A basic shape becomes path data and
    /// goes through the same parser as a `<path>`. An element the engine cannot draw is
    /// a warning. Returns false for a container, which the caller walks.
    fn leaf(&mut self, n: Node<'d, 'i>, tf: &Affine, p: &Paint) -> Result<bool, String> {
        let tag = n.tag_name().name();
        match tag {
            "path" => {
                if let Some(d) = n.attribute("d") {
                    self.emit_d(n, d, tf, p)?;
                }
            }
            "ellipse" | "circle" | "rect" | "polygon" | "polyline" => {
                if let Some(d) = shape_d(n) {
                    self.emit_d(n, &d, tf, p)?;
                }
            }
            "text" | "image" | "line" | "foreignObject" => self.warn(format!("<{tag}> is not drawn")),
            _ => return Ok(false),
        }
        Ok(true)
    }

    /// Push the outline of `n`, drawn from the path data `d`. An outline with no
    /// `data-part` on itself or an ancestor has no colour to take, so it is left out with a
    /// warning.
    fn emit_d(&mut self, n: Node<'d, 'i>, d: &str, tf: &Affine, p: &Paint) -> Result<(), String> {
        let Some(part) = p.part else {
            let id = n.attribute("id").map(|id| format!(" id={id:?}")).unwrap_or_default();
            let line = n.document().text_pos_at(n.range().start).row;
            self.warn(format!("<{}{id}> on line {line} has no data-part, it is not drawn", n.tag_name().name()));
            return Ok(());
        };
        let cmds = parse_d(d, tf, self.quant)?;
        if cmds.is_empty() {
            return Ok(());
        }
        let mut flags = 0u8;
        if p.evenodd {
            flags |= OPF_EVENODD;
        }
        let width = if p.stroke {
            flags |= OPF_STROKE;
            // a stroke is drawn in the space the path is in, so the width travels
            // through the same transform
            (p.stroke_width * stroke_scale(tf) * self.quant).round() as i32
        } else {
            0
        };
        self.set.push_path(part, flags, width, &cmds);
        Ok(())
    }
}

/// The path data of a basic shape, or `None` when it draws nothing (a zero or missing
/// size, no points). Arcs stay arcs here; `parse_d` turns them into cubics, as it does
/// for a `<path>`.
fn shape_d(n: Node) -> Option<String> {
    let num =
        |a: &str| n.attribute(a).and_then(|v| v.trim().strip_suffix("px").unwrap_or(v.trim()).parse::<f64>().ok());
    match n.tag_name().name() {
        "ellipse" | "circle" => {
            let (cx, cy) = (num("cx").unwrap_or(0.0), num("cy").unwrap_or(0.0));
            let (rx, ry) = match n.tag_name().name() {
                "circle" => (num("r")?, num("r")?),
                _ => (num("rx")?, num("ry")?),
            };
            if rx <= 0.0 || ry <= 0.0 {
                return None;
            }
            Some(format!("M{},{} a{rx},{ry} 0 1 0 {},0 a{rx},{ry} 0 1 0 {},0 Z", cx - rx, cy, 2.0 * rx, -2.0 * rx))
        }
        "rect" => {
            let (x, y) = (num("x").unwrap_or(0.0), num("y").unwrap_or(0.0));
            let (w, h) = (num("width")?, num("height")?);
            if w <= 0.0 || h <= 0.0 {
                return None;
            }
            // one corner radius given stands for both; each is at most half the side
            let (rx, ry) = match (num("rx"), num("ry")) {
                (None, None) => (0.0, 0.0),
                (Some(r), None) | (None, Some(r)) => (r, r),
                (Some(rx), Some(ry)) => (rx, ry),
            };
            let (rx, ry) = (rx.clamp(0.0, w / 2.0), ry.clamp(0.0, h / 2.0));
            if rx == 0.0 || ry == 0.0 {
                return Some(format!("M{x},{y} H{} V{} H{x} Z", x + w, y + h));
            }
            let arc = |ex: f64, ey: f64| format!("A{rx},{ry} 0 0 1 {ex},{ey}");
            Some(format!(
                "M{},{y} H{} {} V{} {} H{} {} V{} {} Z",
                x + rx,
                x + w - rx,
                arc(x + w, y + ry),
                y + h - ry,
                arc(x + w - rx, y + h),
                x + rx,
                arc(x, y + h - ry),
                y + ry,
                arc(x + rx, y),
            ))
        }
        // After a moveto, more coordinate pairs are linetos: the points are path data.
        tag @ ("polygon" | "polyline") => {
            let pts = n.attribute("points")?.trim();
            if pts.is_empty() {
                return None;
            }
            Some(format!("M{pts}{}", if tag == "polygon" { " Z" } else { "" }))
        }
        _ => None,
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

/// `#rgb`, `#rrggbb`, `none` to 0xRRGGBBAA. Anything else is opaque black.
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
    for o in &set.ornaments {
        s.push_str(&format!(
            "{} {}: {} parts, {} outlines{}, {} {}, redistributable: {}\n",
            o.style,
            o.kind.as_str(),
            o.parts.len(),
            o.asset.n_paths,
            if o.slices.is_some() { " (tiled)" } else { "" },
            if o.license.id.is_empty() { "unstated licence" } else { &o.license.id },
            o.license.status,
            o.license.redistributable
        ));
    }
    s
}

/// A dressed page as one standalone SVG, the way `qvp2svg` is for the page itself. The
/// ornament layer comes first so it paints behind the print, as a host must draw it.
/// Draws are written in page units with no placement, so this shows the printed layout.
pub fn dress_svg(page: &mut qvp_core::Page, dress: &qvp_core::Dress) -> String {
    use std::fmt::Write;
    let vb = dress.view_box;
    let mut s = String::with_capacity(256 * 1024);
    write!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{:.3} {:.3} {:.3} {:.3}\">",
        vb[0], vb[1], vb[2], vb[3]
    )
    .unwrap();
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
        if d.flags & qvp_core::ODF_STROKE != 0 {
            write!(s, " fill=\"none\" stroke=\"{hex}\" stroke-width=\"{:.4}\" stroke-linecap=\"round\" stroke-linejoin=\"round\"", d.stroke_width).unwrap();
            put_opacity(&mut s, "stroke-opacity", d.color);
        } else {
            write!(s, " fill=\"{hex}\"").unwrap();
            put_opacity(&mut s, "fill-opacity", d.color);
            if d.flags & qvp_core::ODF_EVENODD != 0 {
                s.push_str(" fill-rule=\"evenodd\"");
            }
        }
        s.push_str("/>");
    }
    s.push_str("</g>");
    // The print on top, in each path's current colour. The rings the dress hid are
    // transparent and are left out.
    let colors = page.colors().to_vec();
    let p = page.data();
    for (i, r) in p.paths.iter().enumerate() {
        let color = colors[i];
        if color & 0xff == 0 {
            continue;
        }
        let Ok(cmds) = p.path_cmds(i) else { continue };
        write!(s, "<path d=\"{}\" fill=\"#{:06x}\"", qvp_format::svg_path_d(&cmds, p.header.quant), color >> 8)
            .unwrap();
        put_opacity(&mut s, "fill-opacity", color);
        if r.flags & qvp_format::PF_EVENODD != 0 {
            s.push_str(" fill-rule=\"evenodd\"");
        }
        s.push_str("/>");
    }
    s.push_str("</svg>");
    s
}

/// Write the opacity attribute `name` when `color` is not fully opaque.
fn put_opacity(s: &mut String, name: &str, color: u32) {
    use std::fmt::Write;
    let a = color & 0xff;
    if a < 0xff {
        write!(s, " {name}=\"{:.4}\"", a as f32 / 255.0).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qvp_format::ornaments::{Lineage, OrnamentKind, OPF_EVENODD, OPF_STROKE};

    /// A fixture directory that is unique to one test and removed when it drops.
    struct Fixture(std::path::PathBuf);

    impl std::ops::Deref for Fixture {
        type Target = std::path::Path;
        fn deref(&self) -> &std::path::Path {
            &self.0
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn fresh_dir() -> Fixture {
        static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("qvp-ornaments-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Fixture(dir)
    }

    fn fixture() -> Fixture {
        let dir = fresh_dir();
        std::fs::create_dir_all(dir.join("assets/surah-headers/mushaf-test")).unwrap();
        std::fs::create_dir_all(dir.join("assets/ayah-markers/font-001-regular")).unwrap();
        std::fs::write(
            dir.join("assets/surah-headers/mushaf-test/color.svg"),
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100" data-slot="50 10 100 80">
<g class="slot" data-part="slot" fill="none"><path d="M50 10h100v80h-100z"/></g>
<g id="half"><g class="c1" data-part="c1" fill="#ffffff"><path d="M0 0h100v100H0z"/></g>
<g class="line" data-part="line" stroke="#343f49" fill="none" stroke-width="1.5"><path d="M0 50h100"/></g></g>
<use href="#half" transform="matrix(-1 0 0 1 200 0)"/>
</svg>"##,
        )
        .unwrap();
        std::fs::write(
            dir.join("assets/ayah-markers/font-001-regular/color.svg"),
            r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 99.456 100">
<g class="c1" data-part="c1" fill="#fff8e7"><path d="M0 0h99.456v100H0z" fill-rule="evenodd"/></g>
</svg>"##,
        )
        .unwrap();
        std::fs::write(
            dir.join("catalog.json"),
            r##"{"schema":"quran-assets/catalog@1","assets":[
{"id":"surah-headers/mushaf-test","type":"surah-headers","style":"mushaf-test","lineage":"scan","riwayah":"Hafs 'an 'Asim","viewBox":"0 0 200 100",
 "variants":{"color":"assets/surah-headers/mushaf-test/color.svg"},
 "palette":[{"name":"slot","hex":"none"},{"name":"c1","hex":"#ffffff"},{"name":"line","hex":"#343f49","stroke":true}],
 "slots":[{"role":"surah-name","x":50,"y":10,"w":100,"h":80,"cx":100,"cy":50}],
 "license":{"id":"CC-BY-NC-SA-4.0","status":"provisional","attribution":"Traced from a scan"}},
{"id":"ayah-markers/font-001-regular","type":"ayah-markers","style":"font-001-regular","lineage":"font","riwayah":null,"viewBox":"0 0 99.456 100",
 "variants":{"color":"assets/ayah-markers/font-001-regular/color.svg"},
 "palette":[{"name":"c1","hex":"#fff8e7"}],
 "slots":[{"role":"ayah-number","x":32.7,"y":40.5,"w":34.1,"h":19.1,"cx":49.8,"cy":50.1,"r":24.0}],
 "license":{"id":"OFL-1.1","status":"confirmed","attribution":"From an OFL font"}}
]}"##,
        )
        .unwrap();
        dir
    }

    #[test]
    fn builds_every_asset_of_the_catalogue() {
        let dir = fixture();
        let (set, report) = build(&dir, &BuildOptions::default()).unwrap();
        assert_eq!(report.warnings, Vec::<String>::new());
        assert_eq!(set.ornaments.len(), 2);
        let header = &set.ornaments[set.find(OrnamentKind::SurahHeader, "mushaf-test").unwrap()];
        assert_eq!(header.lineage, Lineage::Scan);
        assert_eq!(header.riwayah, "Hafs 'an 'Asim");
        assert!(!header.license.redistributable);
        assert_eq!(header.asset.view_box, [0.0, 0.0, 200.0, 100.0]);
        assert_eq!(header.asset.slot, Some([50.0, 10.0, 100.0, 80.0]));
        // the palette comes from the catalogue, in its order
        assert_eq!(header.parts.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["slot", "c1", "line"]);
        assert_eq!(header.parts[1].color, 0, "the paper of a scanned design is left open");
        assert_eq!(header.parts[2].color, 0x343f49ff);
        assert!(header.parts[2].stroke);
        // slot + (c1 + line) drawn twice: the `<use>` mirror is expanded
        assert_eq!(header.asset.n_paths, 5);
        let line = &set.paths[(header.asset.first_path + 2) as usize];
        assert_eq!(line.part, 2);
        assert_ne!(line.flags & OPF_STROKE, 0);
        assert_eq!(line.stroke_width, 1500);
        // the mirrored copy lands on the right half, in asset units
        let mirrored = &set.paths[(header.asset.first_path + 3) as usize];
        assert_eq!((mirrored.bbox.x0, mirrored.bbox.x1), (100_000, 200_000));

        let marker = &set.ornaments[set.find(OrnamentKind::AyahMark, "font-001-regular").unwrap()];
        assert_eq!(marker.lineage, Lineage::Font);
        assert!(marker.license.redistributable);
        assert_eq!(marker.riwayah, "");
        assert_eq!(marker.asset.slot.map(|s| s[0]), Some(32.7));
        assert_eq!(marker.asset.n_paths, 1);
        assert_ne!(set.paths[marker.asset.first_path as usize].flags & qvp_format::ornaments::OPF_EVENODD, 0);
    }

    #[test]
    fn redistributable_only_keeps_confirmed_licences() {
        let dir = fixture();
        let (set, report) = build(&dir, &BuildOptions { styles: vec![], redistributable_only: true }).unwrap();
        assert_eq!(set.ornaments.len(), 1);
        assert_eq!(set.ornaments[0].style, "font-001-regular");
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
    }

    #[test]
    fn style_filter() {
        let dir = fixture();
        let (set, _) =
            build(&dir, &BuildOptions { styles: vec!["mushaf-test".into()], redistributable_only: false }).unwrap();
        assert_eq!(set.ornaments.len(), 1);
        assert!(build(&dir, &BuildOptions { styles: vec!["nope".into()], redistributable_only: false }).is_err());
    }

    /// One catalogue with a single asset of `kind`, plus the files it names.
    fn single(kind: &str, extra: &str, files: &[(&str, &str)]) -> Fixture {
        let dir = fresh_dir();
        for (name, svg) in files {
            let path = dir.join(name);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, svg).unwrap();
        }
        std::fs::write(
            dir.join("catalog.json"),
            format!(
                r##"{{"schema":"quran-assets/catalog@1","assets":[
{{"id":"{kind}/t","type":"{kind}","style":"t","lineage":"scan","riwayah":null,"viewBox":"0 0 200 100",
 "variants":{{"color":"assets/whole.svg"}},
 "palette":[{{"name":"c1","hex":"#ffffff"}}],
 "slots":[{{"role":"text-area","x":20,"y":10,"w":160,"h":80}}],
 "license":{{"id":"CC0-1.0","status":"confirmed","attribution":"x"}}{extra}}}
]}}"##
            ),
        )
        .unwrap();
        dir
    }

    const WHOLE: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
<g data-part="c1" fill="#ffffff"><path d="M0 0h200v100H0z"/><path d="M5 5h10v10H5z"/></g></svg>"##;
    const TINY: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
<g data-part="c1" fill="#ffffff"><path d="M0 0h10v10H0z"/></g></svg>"##;
    const SLICES: &str = r#","slices":{"files":{"corner":"assets/corner.svg","edge-h":"assets/edge-h.svg","edge-v":"assets/edge-v.svg"},
 "corner":{"w":12,"h":14},"repeat":{"h":30,"v":40},"corner_mode":"rotate"}"#;

    #[test]
    fn use_x_and_y_move_the_instance() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
<g id="half" data-part="c1" fill="#ffffff"><path d="M0 0h100v50H0z"/></g>
<use href="#half" x="100"/>
<use href="#half" transform="translate(0 10)" x="100" y="40" width="5"/></svg>"##;
        let dir = single("surah-headers", "", &[("assets/whole.svg", svg)]);
        let (set, report) = build(&dir, &BuildOptions::default()).unwrap();
        let a = &set.ornaments[0].asset;
        assert_eq!(a.n_paths, 3);
        let b = |i: u32| set.paths[(a.first_path + i) as usize].bbox;
        assert_eq!((b(0).x0, b(0).x1), (0, 100_000), "the original stays where it is");
        assert_eq!((b(1).x0, b(1).y0, b(1).x1, b(1).y1), (100_000, 0, 200_000, 50_000), "x moves the copy");
        // translate(x, y) goes after the `<use>`'s own transform
        assert_eq!((b(2).x0, b(2).y0, b(2).x1, b(2).y1), (100_000, 50_000, 200_000, 100_000));
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        assert!(report.warnings[0].contains("width or a height"), "{:?}", report.warnings);
    }

    #[test]
    fn an_outline_with_no_part_is_a_warning() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
<g data-part="c1" fill="#ffffff"><path d="M0 0h200v100H0z"/></g>
<path id="stray" d="M0 0h10v10H0z"/></svg>"##;
        let dir = single("surah-headers", "", &[("assets/whole.svg", svg)]);
        let (set, report) = build(&dir, &BuildOptions::default()).unwrap();
        assert_eq!(set.ornaments[0].asset.n_paths, 1, "the stray outline is not drawn");
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        let w = &report.warnings[0];
        assert!(w.contains("assets/whole.svg") && w.contains("<path id=\"stray\">") && w.contains("line 3"), "{w}");
    }

    #[test]
    fn use_target_keeps_its_fill_rule() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
<g id="half" data-part="c1" fill="#ffffff" fill-rule="evenodd"><path d="M0 0h100v100H0z"/></g>
<use href="#half" transform="matrix(-1 0 0 1 200 0)"/></svg>"##;
        let dir = single("surah-headers", "", &[("assets/whole.svg", svg)]);
        let (set, _) = build(&dir, &BuildOptions::default()).unwrap();
        let a = &set.ornaments[0].asset;
        assert_eq!(a.n_paths, 2);
        for i in 0..2 {
            let p = &set.paths[(a.first_path + i) as usize];
            assert_ne!(p.flags & OPF_EVENODD, 0, "path {i}");
        }
        let mirrored = &set.paths[(a.first_path + 1) as usize];
        assert_eq!((mirrored.bbox.x0, mirrored.bbox.x1), (100_000, 200_000));
    }

    #[test]
    fn page_frame_slices_replace_the_whole_drawing() {
        let dir = single(
            "page-frames",
            SLICES,
            &[
                ("assets/whole.svg", WHOLE),
                ("assets/corner.svg", TINY),
                ("assets/edge-h.svg", TINY),
                ("assets/edge-v.svg", TINY),
            ],
        );
        let (set, report) = build(&dir, &BuildOptions::default()).unwrap();
        assert_eq!(report.warnings, Vec::<String>::new());
        let o = &set.ornaments[0];
        assert_eq!(o.asset.n_paths, 0);
        assert_eq!(o.asset.view_box, [0.0, 0.0, 200.0, 100.0]);
        assert_eq!(o.asset.slot, Some([20.0, 10.0, 160.0, 80.0]));
        let s = o.slices.as_ref().expect("slices");
        assert_eq!((s.corner_w, s.corner_h, s.repeat_h, s.repeat_v), (12.0, 14.0, 30.0, 40.0));
        assert!(s.rotate);
        for a in [&s.corner, &s.edge_h, &s.edge_v] {
            assert_eq!(a.n_paths, 1);
        }
        assert_eq!(set.paths.len(), 3);
    }

    #[test]
    fn page_frame_with_a_missing_slice_falls_back_to_the_whole_drawing() {
        let dir = single(
            "page-frames",
            SLICES,
            &[("assets/whole.svg", WHOLE), ("assets/corner.svg", TINY), ("assets/edge-h.svg", TINY)],
        );
        let (set, report) = build(&dir, &BuildOptions::default()).unwrap();
        let o = &set.ornaments[0];
        assert!(o.slices.is_none());
        assert_eq!(o.asset.n_paths, 2);
        assert!(report.warnings.iter().any(|w| w.contains("edge-v")), "{:?}", report.warnings);
        assert!(report.warnings.iter().any(|w| w.contains("no usable slices")), "{:?}", report.warnings);
        assert_eq!(set.paths.len(), 2, "the two slices that read leave no outlines behind");
    }

    /// One good outline, then one whose path data does not parse, in a part the
    /// palette lacks.
    const HALF_BAD: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 10 10">
<g data-part="extra" fill="#123456"><path d="M0 0h10v10H0z"/><path d="M0 0 X"/></g></svg>"##;

    #[test]
    fn a_failed_read_leaves_no_outlines_ops_or_parts_behind() {
        let dir = single("surah-headers", "", &[("assets/whole.svg", TINY), ("assets/bad.svg", HALF_BAD)]);
        let (mut set, _) = build(&dir, &BuildOptions::default()).unwrap();
        let mut parts = set.ornaments[0].parts.clone();
        let (paths, ops, n_parts) = (set.paths.len(), set.ops.len(), parts.len());
        let mut warnings = vec![];
        let r = read_asset(&dir, "assets/bad.svg", &mut set, &mut parts, true, None, &mut warnings);
        assert!(r.unwrap_err().contains("path d"));
        assert_eq!((set.paths.len(), set.ops.len(), parts.len()), (paths, ops, n_parts));
    }

    #[test]
    fn a_slice_that_fails_halfway_leaves_no_outlines_behind() {
        let dir = single(
            "page-frames",
            SLICES,
            &[
                ("assets/whole.svg", WHOLE),
                ("assets/corner.svg", TINY),
                ("assets/edge-h.svg", TINY),
                ("assets/edge-v.svg", HALF_BAD),
            ],
        );
        let (set, report) = build(&dir, &BuildOptions::default()).unwrap();
        let o = &set.ornaments[0];
        assert!(o.slices.is_none());
        assert_eq!(o.asset.first_path, 0);
        assert_eq!(set.paths.len(), 2, "only the whole drawing");
        assert_eq!(o.parts.iter().map(|p| p.name.as_str()).collect::<Vec<_>>(), ["c1"]);
        assert!(report.warnings.iter().any(|w| w.contains("edge-v")), "{:?}", report.warnings);
    }

    #[test]
    fn a_dropped_record_takes_its_slices_with_it() {
        // The slices read, but the record's own drawing is missing, so the record goes.
        let dir = single(
            "page-frames",
            SLICES,
            &[("assets/corner.svg", TINY), ("assets/edge-h.svg", TINY), ("assets/edge-v.svg", TINY)],
        );
        let catalog = std::fs::read_to_string(dir.join("catalog.json")).unwrap();
        let header = r##"{"id":"surah-headers/h","type":"surah-headers","style":"h","lineage":"scan","riwayah":null,
 "variants":{"color":"assets/corner.svg"},"palette":[{"name":"c1","hex":"#ffffff"}],
 "license":{"id":"CC0-1.0","status":"confirmed","attribution":"x"}},"##;
        std::fs::write(dir.join("catalog.json"), catalog.replacen("\"assets\":[", &format!("\"assets\":[{header}"), 1))
            .unwrap();
        let (set, report) = build(&dir, &BuildOptions::default()).unwrap();
        assert_eq!(set.ornaments.len(), 1, "{:?}", report.warnings);
        assert_eq!(set.ornaments[0].kind, OrnamentKind::SurahHeader);
        assert_eq!(set.paths.len(), 1, "the frame's slices are gone");
        assert_eq!(set.ops.len(), set.paths.iter().map(|p| p.op_len as usize).sum::<usize>());
        assert!(report.warnings.iter().any(|w| w.contains("t/page_frame")), "{:?}", report.warnings);
    }

    #[test]
    fn content_in_defs_is_drawn_only_through_use() {
        let svg = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">
<defs><g id="half" data-part="c1" fill="#ffffff"><path d="M0 0h100v100H0z"/></g></defs>
<use href="#half"/><use href="#half" transform="matrix(-1 0 0 1 200 0)"/></svg>"##;
        let dir = single("surah-headers", "", &[("assets/whole.svg", svg)]);
        let (set, _) = build(&dir, &BuildOptions::default()).unwrap();
        let a = &set.ornaments[0].asset;
        assert_eq!(a.n_paths, 2);
        let (l, r) = (&set.paths[a.first_path as usize], &set.paths[(a.first_path + 1) as usize]);
        assert_eq!((l.bbox.x0, l.bbox.x1), (0, 100_000));
        assert_eq!((r.bbox.x0, r.bbox.x1), (100_000, 200_000));
    }

    #[test]
    fn content_inside_a_mask_is_not_drawn() {
        let (set, _) = shapes(
            r##"<mask id="m"><rect data-part="c1" x="0" y="0" width="10" height="10" fill="#ffffff"/></mask>
<g data-part="c1" fill="#ffffff"><path d="M0 0h10v10H0z"/></g>"##,
        );
        assert_eq!(set.ornaments[0].asset.n_paths, 1, "only the path outside the mask");
    }

    /// A one-asset catalogue whose asset is `body` inside a 200 × 100 SVG.
    fn shapes(body: &str) -> (OrnamentSet, Report) {
        let svg = format!(r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 200 100">{body}</svg>"##);
        let dir = single("surah-headers", "", &[("assets/whole.svg", &svg)]);
        build(&dir, &BuildOptions::default()).unwrap()
    }

    #[test]
    fn an_ellipse_becomes_one_outline_with_its_box() {
        let (set, report) =
            shapes(r##"<g data-part="c1" fill="#ffffff"><ellipse cx="100" cy="50" rx="40" ry="20"/></g>"##);
        assert_eq!(report.warnings, Vec::<String>::new());
        let a = &set.ornaments[0].asset;
        assert_eq!(a.n_paths, 1);
        let b = set.paths[a.first_path as usize].bbox;
        for (got, want) in [(b.x0, 60_000), (b.y0, 30_000), (b.x1, 140_000), (b.y1, 70_000)] {
            assert!((got - want).abs() <= 1, "{b:?}");
        }
        let (set, _) = shapes(r##"<g data-part="c1" fill="#ffffff"><circle cx="50" cy="50" r="10"/></g>"##);
        let b = set.paths[0].bbox;
        assert!((b.x0 - 40_000).abs() <= 1 && (b.x1 - 60_000).abs() <= 1, "{b:?}");
    }

    #[test]
    fn a_rect_and_a_polygon_become_outlines() {
        let (set, report) = shapes(
            r##"<g data-part="c1" fill="#ffffff"><rect x="10" y="20" width="30" height="40"/>
<rect x="0" y="0" width="100" height="50" rx="5"/><polygon points="0,0 10,0 10,10"/><polyline points="0 0 5 5"/></g>"##,
        );
        assert_eq!(report.warnings, Vec::<String>::new());
        assert_eq!(set.ornaments[0].asset.n_paths, 4);
        let cmds = set.path_cmds(0).unwrap();
        let corners: Vec<_> = cmds
            .iter()
            .filter_map(|c| match *c {
                Cmd::MoveTo(x, y) | Cmd::LineTo(x, y) => Some((x, y)),
                _ => None,
            })
            .collect();
        assert_eq!(corners, [(10_000, 20_000), (40_000, 20_000), (40_000, 60_000), (10_000, 60_000)]);
        let b = set.paths[0].bbox;
        assert_eq!((b.x0, b.y0, b.x1, b.y1), (10_000, 20_000, 40_000, 60_000));
        let rounded = set.paths[1].bbox;
        assert_eq!((rounded.x0, rounded.y0, rounded.x1, rounded.y1), (0, 0, 100_000, 50_000));
        assert_eq!(set.paths[2].bbox.x1, 10_000);
    }

    #[test]
    fn what_cannot_be_drawn_is_a_warning() {
        let (set, report) = shapes(
            r##"<g data-part="c1" fill="#ffffff" clip-path="url(#c)"><path d="M0 0h10v10H0z"/>
<text>x</text><use href="#gone"/><path d="M0 0h5v5H0z" mask="url(#m)"/></g>"##,
        );
        assert_eq!(set.ornaments[0].asset.n_paths, 2);
        let w = &report.warnings;
        assert!(w.iter().all(|w| w.starts_with("assets/whole.svg: ")), "{w:?}");
        assert!(w.iter().any(|w| w.contains("<text>")), "{w:?}");
        assert!(w.iter().any(|w| w.contains("\"#gone\"")), "{w:?}");
        assert_eq!(w.iter().filter(|w| w.contains("clip-path")).count(), 1, "once per file: {w:?}");
        assert_eq!(w.len(), 3, "{w:?}");
    }

    #[test]
    fn a_stroke_without_a_width_is_one_unit_wide() {
        let (set, report) = shapes(
            r##"<g data-part="c1" stroke="#000000" fill="none"><path d="M0 0h10"/></g>
<g data-part="c1" stroke="#000000" stroke-width="2px"><path d="M0 0h10"/></g>
<g data-part="c1" stroke="#000000" stroke-width="2em"><path d="M0 0h10"/></g>"##,
        );
        assert_eq!(set.paths[0].stroke_width, 1000);
        assert_eq!(set.paths[1].stroke_width, 2000);
        assert_eq!(set.paths[2].stroke_width, 1000, "a width in other units keeps the inherited one");
        assert_eq!(report.warnings.len(), 1, "{:?}", report.warnings);
        assert!(report.warnings[0].contains("2em"));
    }

    #[test]
    fn colours() {
        assert_eq!(css_rgba("#fff"), 0xffffffff);
        assert_eq!(css_rgba("#00649b"), 0x00649bff);
        assert_eq!(css_rgba("none"), 0);
    }

    #[test]
    fn opacity_is_written_only_below_full_alpha() {
        let mut s = String::new();
        put_opacity(&mut s, "fill-opacity", 0x231f20ff);
        assert_eq!(s, "");
        put_opacity(&mut s, "fill-opacity", 0x231f2080);
        assert_eq!(s, " fill-opacity=\"0.5020\"");
    }

    #[test]
    fn view_box_numbers() {
        assert_eq!(numbers("0 0 73.162 100"), Some([0.0, 0.0, 73.162, 100.0]));
        assert_eq!(numbers("0 0 100"), None);
        assert_eq!(numbers("0\r\n0 73.162,\r100"), Some([0.0, 0.0, 73.162, 100.0]));
        assert_eq!(numbers("0\x0c0 1 2"), Some([0.0, 0.0, 1.0, 2.0]));
    }
}
