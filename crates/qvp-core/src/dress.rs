//! Dress a page in another mushaf's ornaments.
//!
//! The medallions, surah bands and page borders of other mushafs are not part of a QVP
//! page. They are an [`OrnamentSet`], and this module puts them on a page they were
//! never drawn for.
//!
//! Every ornament is placed from a measurement, never a fixed offset: the medallion
//! from the printed ring's centre and radius, the band from the surah title's box and
//! the text column, the border from the laid-out page box. The same rules apply on
//! every page.
//!
//! The result is a display list in page units. Each draw carries the placement index
//! the page's own draws carry, so a host applies the transform it already applies to
//! the ink. A host paints the list behind the page ink. Only the printed ring is
//! hidden; the numeral inside it is the print's own ink and stays on top.
use crate::style::{Paint, Selector};
use crate::{Handle, Page, Placement, Rgba, View, NONE};
use qvp_format::ornaments::*;
use qvp_format::{Cmd, PathKind, OP_CLOSE, OP_CUBIC, OP_LINE, OP_MOVE, OP_QUAD};
use std::sync::Arc;

/// What to dress a page in: up to one ornament per kind.
#[derive(Clone, Debug, PartialEq)]
pub struct DressSpec {
    /// Index into [`OrnamentSet::ornaments`], or [`NONE`] for none.
    pub ayah_mark: u32,
    pub surah_header: u32,
    pub page_frame: u32,
    /// Air between the text and the border, in page units.
    pub gap: f32,
    /// Air between a band and the border, in page units. A band is narrower than the
    /// column by twice this.
    pub band_gap: f32,
    /// The medallion's size against the printed ring: 1 is as tall as the ring. 0 takes the
    /// design's own: [`crate::defaults::AYAH_MARK_SCALE_SCAN`] for a scanned mushaf's
    /// medallion, 1 for a font marker.
    pub ayah_mark_scale: f32,
    /// The border's size against the design's own: 1 is the mushaf's proportion. The
    /// pattern scales as a whole, so a bigger border fits fewer motifs along each edge and
    /// a smaller one more; nothing is stretched. 0 is 1. A frame without slices keeps its
    /// printed proportions.
    pub page_frame_scale: f32,
    /// Which kinds are drawn as line art: only their constant-width strokes, the same
    /// drawing with the fills taken away, so one ink is left. A bitmask of
    /// [`LINE_ART_AYAH_MARK`], [`LINE_ART_SURAH_HEADER`] and [`LINE_ART_PAGE_FRAME`];
    /// 0 draws every kind in colour.
    pub line_art: u32,
    /// Colour overrides as (ornament, part, colour). Every part left out keeps the
    /// design's own printed colour.
    pub colors: Vec<(u32, u16, Rgba)>,
}

impl Default for DressSpec {
    fn default() -> Self {
        DressSpec {
            ayah_mark: NONE,
            surah_header: NONE,
            page_frame: NONE,
            gap: 5.0,
            band_gap: 5.0,
            ayah_mark_scale: 0.0,
            page_frame_scale: 1.0,
            line_art: 0,
            colors: vec![],
        }
    }
}

/// Line art for the medallions ([`DressSpec::line_art`]).
pub const LINE_ART_AYAH_MARK: u32 = 1;
/// Line art for the surah bands.
pub const LINE_ART_SURAH_HEADER: u32 = 2;
/// Line art for the border.
pub const LINE_ART_PAGE_FRAME: u32 = 4;
/// Line art for every kind.
pub const LINE_ART_ALL: u32 = 7;

/// The [`DressSpec::line_art`] bit of one kind.
pub fn line_art_bit(kind: OrnamentKind) -> u32 {
    1 << kind as u32
}

/// One placed outline, in page units.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[repr(C)]
pub struct OrnamentDraw {
    pub op_start: u32,
    pub op_count: u32,
    pub pt_start: u32,
    pub pt_count: u32,
    pub color: Rgba,
    /// `kind | ODF_EVENODD | ODF_STROKE`
    pub flags: u32,
    /// Stroke width in page units (0 when filled).
    pub stroke_width: f32,
    /// Index into [`Ornament::parts`].
    pub part: u32,
    /// The placement this draw is drawn under: the same index a page draw carries, into
    /// the layout's placements. Without a layout it is the printed line. [`NONE`] for
    /// the border, which is drawn in layout space with no group transform.
    pub placement: u32,
}

pub const ODF_EVENODD: u32 = 1 << 8;
pub const ODF_STROKE: u32 = 2 << 8;

/// The ornaments placed on a page, and what placing them did to the page.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dress {
    pub ops: Vec<u8>,
    pub pts: Vec<f32>,
    pub table: Vec<OrnamentDraw>,
    /// `x, y, w, h` in page units: the page's own box, or the box the border grew it to.
    pub view_box: [f32; 4],
    pub ayah_mark: u32,
    pub surah_header: u32,
    pub page_frame: u32,
    pub n_ayah_marks: u32,
    pub n_surah_headers: u32,
    /// Repeat units tiled along the four edges (0 when the frame was stretched whole).
    pub n_frame_repeats: u32,
    /// True when the frame had no slices and was stretched whole.
    pub frame_stretched: bool,
    /// Bumped on every rebuild: a new dress, and every layout. A host caches the raster of this layer and this
    /// is what tells it the raster is stale.
    pub revision: u32,
    /// The style rules hiding the printed rings and bands, so undressing puts them back.
    pub(crate) hidden: Vec<Handle>,
    pub(crate) set: Option<Arc<OrnamentSet>>,
    pub(crate) spec: DressSpec,
}

/// `[a c e; b d f]`, enough for everything placement needs.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Tf {
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub d: f32,
    pub e: f32,
    pub f: f32,
}

impl Tf {
    pub const IDENTITY: Tf = Tf { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };
    /// `self` applied after `o` (SVG nesting order).
    pub fn then(&self, o: &Tf) -> Tf {
        Tf {
            a: self.a * o.a + self.c * o.b,
            b: self.b * o.a + self.d * o.b,
            c: self.a * o.c + self.c * o.d,
            d: self.b * o.c + self.d * o.d,
            e: self.a * o.e + self.c * o.f + self.e,
            f: self.b * o.e + self.d * o.f + self.f,
        }
    }
    pub fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }
    /// The factor a stroke width is multiplied by (SVG's rule for a non-uniform matrix).
    pub fn stroke_scale(&self) -> f32 {
        (self.a * self.d - self.b * self.c).abs().sqrt()
    }
    /// Map a viewBox onto a box, as a nested `<svg>` does. `meet` keeps the aspect and
    /// centres; otherwise the drawing is stretched.
    pub fn fit(vb: [f32; 4], x: f32, y: f32, w: f32, h: f32, meet: bool) -> Tf {
        let (vw, vh) = (if vb[2] != 0.0 { vb[2] } else { 1.0 }, if vb[3] != 0.0 { vb[3] } else { 1.0 });
        let (mut sx, mut sy) = (w / vw, h / vh);
        let (mut dx, mut dy) = (0.0, 0.0);
        if meet {
            let s = sx.min(sy);
            dx = (w - vw * s) / 2.0;
            dy = (h - vh * s) / 2.0;
            sx = s;
            sy = s;
        }
        Tf { a: sx, b: 0.0, c: 0.0, d: sy, e: x + dx - vb[0] * sx, f: y + dy - vb[1] * sy }
    }
    pub fn translate(tx: f32, ty: f32) -> Tf {
        Tf { e: tx, f: ty, ..Tf::IDENTITY }
    }
    pub fn scale(sx: f32, sy: f32) -> Tf {
        Tf { a: sx, d: sy, ..Tf::IDENTITY }
    }
}

/// One ornament's colours after the spec's overrides, and its two special parts.
struct Palette {
    colors: Vec<Rgba>,
    line: Option<u16>,
    slot: Option<u16>,
    /// The darkest ink of the design: the last part that is painted. A design with no
    /// linework is drawn as line art in this colour.
    ink: Rgba,
}

/// How much of a band's slot height a surah title may fill under reflow; the rest is air.
const BAND_FILL: f32 = 0.9;

/// The stroke width of line art drawn from a design's fills, in asset units: the width
/// the catalogue's own linework has on a design 100 units tall.
const LINE_ART_WIDTH: f32 = 1.25;

fn palette(o: &Ornament, index: u32, spec: &DressSpec) -> Palette {
    let mut colors: Vec<Rgba> = o.parts.iter().map(|p| p.color).collect();
    for &(oi, part, rgba) in &spec.colors {
        if oi == index {
            if let Some(slot) = colors.get_mut(part as usize) {
                *slot = rgba;
            }
        }
    }
    let ink = colors.iter().rev().copied().find(|c| c & 0xff != 0).unwrap_or(0x000000ff);
    Palette { colors, line: o.part_index("line"), slot: o.part_index("slot"), ink }
}

struct Builder<'a> {
    set: &'a OrnamentSet,
    line_art: u32,
    out: Dress,
}

impl<'a> Builder<'a> {
    /// Emit one asset's outlines through `tf`. `drop_slot` leaves out the transparent
    /// window, which a tiled frame must not keep.
    fn place(&mut self, asset: &Asset, pal: &Palette, kind: OrnamentKind, tf: &Tf, drop_slot: bool, placement: u32) {
        let q = self.set.quant as f32;
        let ss = tf.stroke_scale();
        for i in asset.first_path..asset.first_path + asset.n_paths {
            let Some(p) = self.set.paths.get(i as usize).copied() else { continue };
            // Line art keeps the design's own linework. A design with none (a font marker)
            // is drawn as the outlines of its fills instead, in one ink.
            let line_art = self.line_art & line_art_bit(kind) != 0;
            let outlined = line_art && pal.line.is_none();
            if line_art && !outlined && Some(p.part) != pal.line {
                continue;
            }
            if drop_slot && Some(p.part) == pal.slot {
                continue;
            }
            let own = pal.colors.get(p.part as usize).copied().unwrap_or(0);
            let color = if outlined && own & 0xff != 0 { pal.ink } else { own };
            if color & 0xff == 0 {
                continue;
            }
            let Ok(cmds) = self.set.path_cmds(i as usize) else { continue };
            let op_start = self.out.ops.len() as u32;
            let pt_start = self.out.pts.len() as u32;
            for c in &cmds {
                let mut push = |x: i32, y: i32| {
                    let (px, py) = tf.apply(x as f32 / q, y as f32 / q);
                    self.out.pts.push(px);
                    self.out.pts.push(py);
                };
                match *c {
                    Cmd::MoveTo(x, y) => {
                        self.out.ops.push(OP_MOVE);
                        push(x, y);
                    }
                    Cmd::LineTo(x, y) => {
                        self.out.ops.push(OP_LINE);
                        push(x, y);
                    }
                    Cmd::QuadTo(x1, y1, x, y) => {
                        self.out.ops.push(OP_QUAD);
                        push(x1, y1);
                        push(x, y);
                    }
                    Cmd::CubicTo(x1, y1, x2, y2, x, y) => {
                        self.out.ops.push(OP_CUBIC);
                        push(x1, y1);
                        push(x2, y2);
                        push(x, y);
                    }
                    Cmd::Close => self.out.ops.push(OP_CLOSE),
                }
            }
            let stroke = p.flags & OPF_STROKE != 0 || outlined;
            let stroke_width = if p.flags & OPF_STROKE != 0 {
                p.stroke_width as f32 / q * ss
            } else if outlined {
                LINE_ART_WIDTH * ss
            } else {
                0.0
            };
            self.out.table.push(OrnamentDraw {
                op_start,
                op_count: self.out.ops.len() as u32 - op_start,
                pt_start,
                pt_count: self.out.pts.len() as u32 - pt_start,
                color,
                flags: kind as u32
                    | if p.flags & OPF_EVENODD != 0 { ODF_EVENODD } else { 0 }
                    | if stroke { ODF_STROKE } else { 0 },
                stroke_width,
                part: p.part as u32,
                placement,
            });
        }
    }

    /// The corner keeps its shape; only the straight run repeats. Stretching one drawing
    /// to a different page is what makes the side borders fatter than the top and bottom.
    fn tile_frame(&mut self, sl: &Slices, pal: &Palette, frame_tf: &Tf, unit_w: f32, unit_h: f32) {
        let (cw, ch) = (sl.corner_w, sl.corner_h);
        let turn = sl.rotate;
        let put = |b: &mut Builder<'a>, asset: &Asset, local: Tf| {
            // the slice's own viewBox already sits at the origin at its own size
            let fit = Tf::fit(asset.view_box, 0.0, 0.0, asset.width(), asset.height(), false);
            b.place(asset, pal, OrnamentKind::PageFrame, &frame_tf.then(&local.then(&fit)), true, NONE);
        };
        put(self, &sl.corner, Tf::IDENTITY);
        put(
            self,
            &sl.corner,
            if turn {
                Tf { a: -1.0, b: 0.0, c: 0.0, d: -1.0, e: unit_w, f: ch }
            } else {
                Tf::translate(unit_w, 0.0).then(&Tf::scale(-1.0, 1.0))
            },
        );
        put(
            self,
            &sl.corner,
            if turn {
                Tf { a: -1.0, b: 0.0, c: 0.0, d: -1.0, e: cw, f: unit_h }
            } else {
                Tf::translate(0.0, unit_h).then(&Tf::scale(1.0, -1.0))
            },
        );
        put(
            self,
            &sl.corner,
            if turn {
                Tf::translate(unit_w - cw, unit_h - ch)
            } else {
                Tf { a: -1.0, b: 0.0, c: 0.0, d: -1.0, e: unit_w, f: unit_h }
            },
        );
        // a whole number of repeats, each nudged to fit exactly: a part-drawn motif at the
        // end of a run is the thing the eye catches. When the two corners fill the edge or
        // overlap it (a very big border), the edge has no run.
        let run = |total: f32, u: f32| -> (u32, f32) {
            if total.is_nan() || total <= 0.0 {
                return (0, 0.0);
            }
            let n = ((total / u.max(f32::EPSILON)).round() as i32).max(1) as u32;
            (n, total / n as f32)
        };
        let (nh, step_h) = run(unit_w - 2.0 * cw, sl.repeat_h);
        for i in 0..nh {
            let x = cw + i as f32 * step_h;
            let k = step_h / sl.repeat_h.max(f32::EPSILON);
            put(self, &sl.edge_h, Tf::translate(x, 0.0).then(&Tf::scale(k, 1.0)));
            put(self, &sl.edge_h, Tf::translate(x, unit_h).then(&Tf::scale(k, -1.0)));
        }
        let (nv, step_v) = run(unit_h - 2.0 * ch, sl.repeat_v);
        for i in 0..nv {
            let y = ch + i as f32 * step_v;
            let k = step_v / sl.repeat_v.max(f32::EPSILON);
            put(self, &sl.edge_v, Tf::translate(0.0, y).then(&Tf::scale(1.0, k)));
            put(self, &sl.edge_v, Tf::translate(unit_w, y).then(&Tf::scale(-1.0, k)));
        }
        self.out.n_frame_repeats = (nh + nv) * 2;
    }
}

impl Page {
    /// The placement a path is drawn under: its reflow group, or its printed line.
    fn placement_of_path(&self, path: u32) -> u32 {
        match &self.layout {
            Some(l) if !l.path_group.is_empty() => l.path_group[path as usize],
            _ => self.geom.table[path as usize].line,
        }
    }

    /// The transform a placement index stands for: the layout's group, or identity
    /// without a layout.
    fn placement_at(&self, index: u32) -> Placement {
        let p = self.layout.as_ref().and_then(|l| l.groups.get(index as usize)).copied().unwrap_or(Placement::IDENTITY);
        Placement { kx: if p.kx != 0.0 { p.kx } else { 1.0 }, ky: if p.ky != 0.0 { p.ky } else { 1.0 }, ..p }
    }

    /// How far a surah title may grow under reflow before it outgrows the band it is
    /// dressed in: the most a title's `header_scale` may be, in page units, so that the
    /// tallest title still sits inside the band's slot with some air. Infinite when the
    /// page is not dressed in a band. `laid_w` is the laid-out page width the band spans.
    pub(crate) fn band_banner_cap(&self, laid_w: f32) -> f32 {
        let Some(d) = &self.dress else { return f32::INFINITY };
        let Some(set) = &d.set else { return f32::INFINITY };
        let spec = &d.spec;
        let Some(o) = set.ornaments.get(spec.surah_header as usize).filter(|_| spec.surah_header != NONE) else {
            return f32::INFINITY;
        };
        let Some(slot) = o.asset.slot else { return f32::INFINITY };
        let framed =
            spec.page_frame != NONE && set.ornaments.get(spec.page_frame as usize).and_then(|f| f.asset.slot).is_some();
        let column = if framed { laid_w + 2.0 * spec.gap } else { laid_w };
        let bw = (column - 2.0 * spec.band_gap).max(f32::EPSILON);
        let slot_h = slot[3] * bw / o.asset.width().max(f32::EPSILON);
        let title_h = self.surah_headers().iter().map(|h| h.title_y1 - h.title_y0).fold(0.0, f32::max);
        if title_h <= 0.0 {
            return f32::INFINITY;
        }
        BAND_FILL * slot_h / title_h
    }

    /// How far the border reaches past a laid-out page `laid_w` page units wide, on the
    /// left and on the right, in page units. These are the margins `build_dress` puts the
    /// frame at: the gap, then the frame's own margin at the frame's unit. `None` when the
    /// page is not dressed in a border.
    pub(crate) fn frame_side_margins(&self, laid_w: f32) -> Option<(f32, f32)> {
        let d = self.dress.as_ref()?;
        let set = d.set.as_ref()?;
        let spec = &d.spec;
        if spec.page_frame == NONE {
            return None;
        }
        let o = set.ornaments.get(spec.page_frame as usize).filter(|o| o.kind == OrnamentKind::PageFrame)?;
        let slot = o.asset.slot?;
        let gap = spec.gap;
        let span = laid_w + 2.0 * gap;
        let asset_w = o.asset.width();
        let unit = span / slot[2].max(f32::EPSILON);
        Some(match &o.slices {
            Some(_) => {
                let unit = unit * if spec.page_frame_scale > 0.0 { spec.page_frame_scale } else { 1.0 };
                (gap + slot[0] * unit, gap + (asset_w - slot[0] - slot[2]) * unit)
            }
            None => {
                let vb = o.asset.view_box;
                (gap + (slot[0] - vb[0]) * unit, gap + (vb[0] + asset_w - slot[0] - slot[2]) * unit)
            }
        })
    }

    /// The laid-out page box in page units: `(left, top, width, height)`. Without a
    /// layout that is the printed page itself.
    fn laid_page_box(&self) -> (f32, f32, f32, f32) {
        match &self.layout {
            Some(l) => (l.laid_left, l.laid_top, l.laid_width, l.laid_height),
            None => (0.0, 0.0, self.width(), self.height()),
        }
    }

    /// The page's viewBox: `x, y, w, h`. A dressed page's border grows it.
    pub fn view_box(&self) -> [f32; 4] {
        match &self.dress {
            Some(d) => d.view_box,
            None => [0.0, 0.0, self.width(), self.height()],
        }
    }

    pub fn dress_of(&self) -> Option<&Dress> {
        self.dress.as_ref()
    }

    /// Take the ornaments off: the printed rings and bands come back, the viewBox
    /// returns to the page's own and the current layout's fit is the undressed one again.
    pub fn undress(&mut self) {
        if self.take_dress() {
            self.relayout_dressed();
        }
    }

    /// Remove the dress and its hiding rules, and leave the layout alone. True when the page
    /// was dressed.
    fn take_dress(&mut self) -> bool {
        let Some(d) = self.dress.take() else { return false };
        for h in d.hidden {
            self.styles.remove_engine(h);
        }
        self.state_dirty = true;
        true
    }

    /// Bring the current layout up to date with a new dress, or with none. A reflowed layout
    /// is made again from its own spec, because a band caps how far a title may grow and the
    /// rows were broken before the band was there. A printed one only needs a new fit.
    fn relayout_dressed(&mut self) {
        match self.layout.as_ref().filter(|l| l.reflow.is_some()).map(|l| l.spec) {
            Some(spec) => {
                self.layout(&spec);
            }
            None => self.refit(),
        }
    }

    /// How much bigger the dressed page is than the laid-out content, on each side, in
    /// viewport px through the current layout: `left, top, right, bottom`. An undressed
    /// page answers zeroes.
    pub fn dress_overflow(&self) -> (f32, f32, f32, f32) {
        let Some(d) = &self.dress else { return (0.0, 0.0, 0.0, 0.0) };
        let vb = d.view_box;
        let (scale, ox, oy, cw, ch) = match &self.layout {
            Some(l) => (l.scale, l.offset_x, l.offset_y, l.content_w, l.content_h),
            None => (1.0, 0.0, 0.0, self.width(), self.height()),
        };
        (
            (-(ox + vb[0] * scale)).max(0.0),
            (-(oy + vb[1] * scale)).max(0.0),
            (ox + (vb[0] + vb[2]) * scale - cw).max(0.0),
            (oy + (vb[1] + vb[3]) * scale - ch).max(0.0),
        )
    }

    /// Hold a view against the viewport, as [`View::clamp`] does, but for the dressed box:
    /// the content plus the dress overflow on each side. A drag then reaches the border's
    /// edges and never goes past them. An undressed page gets the plain clamp of its
    /// content.
    pub fn dress_clamp(&self, view: View, viewport_w: f32, viewport_h: f32) -> View {
        let (cw, ch) = match &self.layout {
            Some(l) => (l.content_w, l.content_h),
            None => (self.width(), self.height()),
        };
        let (l, t, r, b) = self.dress_overflow();
        // the dressed box starts `(l, t)` before the content, which is `scale` times that on screen
        let (sx, sy) = (l * view.scale, t * view.scale);
        let shifted = View { offset_x: view.offset_x - sx, offset_y: view.offset_y - sy, ..view };
        let held = shifted.clamp(cw + l + r, ch + t + b, viewport_w, viewport_h);
        View { offset_x: held.offset_x + sx, offset_y: held.offset_y + sy, ..held }
    }

    /// The view that shows the page at `scale` to a host that scrolls the page itself: the
    /// dressed box (the content plus the dress overflow, which grows with `scale` as the
    /// content does) is centred across `viewport_w`, or starts at x = 0 when it is wider, and
    /// its top is at y = 0. The second and third values are the dressed box's width and
    /// height at `scale`: the size such a host gives the canvas, so the whole border is on it.
    /// An undressed page gets its content centred at the top and the content's size.
    ///
    /// A `scale` of 0 or less fits the dressed box to `viewport_w`: the scale is
    /// `min(1, viewport_w / dressed width)`, so the whole border is on the canvas and the
    /// page is never enlarged.
    pub fn dress_view(&self, scale: f32, viewport_w: f32) -> (View, f32, f32) {
        let (cw, ch) = match &self.layout {
            Some(l) => (l.content_w, l.content_h),
            None => (self.width(), self.height()),
        };
        let (l, t, r, b) = self.dress_overflow();
        let scale = if scale > 0.0 {
            scale
        } else {
            let full = cw + l + r;
            if full > 0.0 && viewport_w > 0.0 {
                (viewport_w / full).min(1.0)
            } else {
                1.0
            }
        };
        let (w, h) = ((cw + l + r) * scale, (ch + t + b) * scale);
        // the content's origin stands `(l, t)` inside the dressed box, `scale` times that on screen
        let left = ((viewport_w - w) / 2.0).max(0.0);
        (View { scale, offset_x: left + l * scale, offset_y: t * scale }, w, h)
    }

    /// Re-place the ornaments against the current layout. A layout that respaces the
    /// lines makes the page taller than it is printed, and the border goes around what
    /// is on the page, so it has to follow. Every rebuild bumps the revision, because a
    /// layout can move placements without changing the geometry. The layout's fit is set
    /// last, dressed or not, as it fits the border too.
    pub(crate) fn redress(&mut self) {
        self.rebuild_dress();
        self.refit();
    }

    fn rebuild_dress(&mut self) {
        let Some(d) = self.dress.as_ref() else { return };
        let (Some(set), spec) = (d.set.clone(), d.spec.clone()) else { return };
        if let Some(mut next) = self.build_dress(&set, &spec) {
            let old = self.dress.take().unwrap();
            self.dress_revision = self.dress_revision.wrapping_add(1);
            next.hidden = old.hidden;
            next.set = Some(set);
            next.spec = spec;
            next.revision = self.dress_revision;
            self.dress = Some(next);
        }
    }

    /// Put other ornaments on this page. Returns `None`, and leaves the page undressed,
    /// when an index is out of range, names an ornament of another kind, names a band or a
    /// border with no slot, or every index is [`NONE`]. Replaces any previous dress. A reflowed page is laid out again from its
    /// own spec, so a band holds its title at once; a printed page is refitted.
    pub fn dress(&mut self, set: &Arc<OrnamentSet>, spec: &DressSpec) -> Option<&Dress> {
        let was_dressed = self.take_dress();
        let Some(mut dress) = self.build_dress(set, spec) else {
            if was_dressed {
                self.relayout_dressed();
            }
            return None;
        };
        self.dress_revision = self.dress_revision.wrapping_add(1);
        dress.revision = self.dress_revision;
        if spec.ayah_mark != NONE {
            dress.hidden.push(self.styles.add_engine(Selector::Kind(PathKind::AyahMarkOrnament), Paint::HIDDEN));
        }
        if spec.surah_header != NONE {
            for p in self.surah_name_ornament_paths() {
                dress.hidden.push(self.styles.add_engine(Selector::Path(p), Paint::HIDDEN));
            }
        }
        self.state_dirty = true;
        dress.set = Some(set.clone());
        dress.spec = spec.clone();
        self.dress = Some(dress);
        self.relayout_dressed();
        self.dress.as_ref()
    }

    /// Place the chosen ornaments and return the display list. Pure: it reads the page
    /// and the current layout and changes neither.
    fn build_dress(&self, set: &OrnamentSet, spec: &DressSpec) -> Option<Dress> {
        let pick = |i: u32, kind: OrnamentKind| -> Option<Option<(u32, &Ornament)>> {
            if i == NONE {
                return Some(None);
            }
            let o = set.ornaments.get(i as usize)?;
            (o.kind == kind).then_some(Some((i, o)))
        };
        let mark = pick(spec.ayah_mark, OrnamentKind::AyahMark)?;
        let header = pick(spec.surah_header, OrnamentKind::SurahHeader)?;
        let frame = pick(spec.page_frame, OrnamentKind::PageFrame)?;
        if mark.is_none() && header.is_none() && frame.is_none() {
            return None;
        }
        // A border and a band are placed by their slot: the text area and the title's window.
        // One without a slot cannot be placed, so it is refused, not drawn as nothing.
        if [header, frame].iter().flatten().any(|(_, o)| o.asset.slot.is_none()) {
            return None;
        }
        let pw = self.width();
        let ph = self.height();
        let (laid_left, laid_top, laid_w, laid_h) = self.laid_page_box();
        let mut b = Builder {
            set,
            line_art: spec.line_art,
            out: Dress {
                view_box: [0.0, 0.0, pw, ph],
                ayah_mark: spec.ayah_mark,
                surah_header: spec.surah_header,
                page_frame: spec.page_frame,
                ..Dress::default()
            },
        };
        // The text column a surah band spans: the area the border encloses once one is
        // drawn, and the page's own width otherwise. Never this page's ink: pages 1 and 2
        // hold one short surah centred in the column, and a band measured from their
        // lines would come out a third of the width it has on the other 602 pages.
        let mut column = laid_w;
        let mut column_left = laid_left;

        // The border. A frame publishes its text area as its slot. Mapping the page box
        // onto that slot makes the band thickness follow at the mushaf's own proportions:
        // one uniform scale, never a stretch to the page box. The page the border goes
        // around is the laid-out one, in both dimensions: under reflow a row is narrower
        // than the printed page. A frame without slices is stretched instead, so that its
        // slot lands on the grown box exactly.
        if let Some((oi, o)) = frame {
            if let Some(slot) = o.asset.slot {
                let pal = palette(o, oi, spec);
                let gap = spec.gap;
                let unit = (laid_w + 2.0 * gap) / slot[2].max(f32::EPSILON); // page units per frame unit
                let size = if spec.page_frame_scale > 0.0 { spec.page_frame_scale } else { 1.0 };
                let (asset_w, asset_h) = (o.asset.width(), o.asset.height());
                match &o.slices {
                    Some(sl) => {
                        // A bigger border draws the frame's units bigger, so fewer motifs fit
                        // along each edge, and a smaller one more: the pattern scales as a
                        // whole and nothing is stretched. The margins grow with it; the slot
                        // stays on the grown page box. The bottom margin is the frame's own,
                        // not a copy of its top.
                        let unit = unit * size;
                        let box_x = laid_left - gap - slot[0] * unit;
                        let box_y = laid_top - gap - slot[1] * unit;
                        let unit_w = (laid_w + 2.0 * gap) / unit + slot[0] + (asset_w - slot[0] - slot[2]);
                        let unit_h = (laid_h + 2.0 * gap) / unit + slot[1] + (asset_h - slot[1] - slot[3]);
                        let (box_w, box_h) = (unit_w * unit, unit_h * unit);
                        let frame_tf = Tf::fit([0.0, 0.0, unit_w, unit_h], box_x, box_y, box_w, box_h, false);
                        b.tile_frame(sl, &pal, &frame_tf, unit_w, unit_h);
                        b.out.view_box = [box_x, box_y, box_w, box_h];
                    }
                    None => {
                        // two scales: the slot maps onto the grown box in both dimensions
                        let sx = (laid_w + 2.0 * gap) / slot[2].max(f32::EPSILON);
                        let sy = (laid_h + 2.0 * gap) / slot[3].max(f32::EPSILON);
                        let e = laid_left - gap - slot[0] * sx;
                        let f = laid_top - gap - slot[1] * sy;
                        let vb = o.asset.view_box;
                        b.place(
                            &o.asset,
                            &pal,
                            OrnamentKind::PageFrame,
                            &Tf { a: sx, b: 0.0, c: 0.0, d: sy, e, f },
                            false,
                            NONE,
                        );
                        b.out.frame_stretched = true;
                        b.out.view_box = [e + vb[0] * sx, f + vb[1] * sy, asset_w * sx, asset_h * sy];
                    }
                }
                column = laid_w + 2.0 * gap;
                column_left = laid_left - gap;
            }
        }

        // The band is centred in the column, `band_gap` short of the border on each side,
        // and its slot's vertical centre sits on the printed title's. The column is a
        // measurement, never this page's ink: a title sits where the print put it, which
        // is not always the middle of the column.
        if let Some((oi, o)) = header {
            let pal = palette(o, oi, spec);
            let vb = o.asset.view_box;
            let (_, scy) = o.asset.slot_centre();
            let bw = (column - 2.0 * spec.band_gap).max(f32::EPSILON);
            let k = bw / o.asset.width().max(f32::EPSILON);
            let bh = o.asset.height() * k;
            let x = column_left + (column - bw) / 2.0;
            for h in self.surah_headers() {
                if h.title_x1 <= h.title_x0 || h.title_y1 <= h.title_y0 {
                    continue;
                }
                // The column is a laid-out measurement and the title's placement maps its
                // printed ink into the laid-out page, moving it and, under reflow, scaling
                // it. The band is worked out where it must land, then taken back through the
                // inverse of that placement, so the host's transform puts it there.
                let placement = self.placement_of_path(self.data.decorations[h.decoration as usize].first_path);
                let pl = self.placement_at(placement);
                let tcy = pl.ky * (h.title_y0 + h.title_y1) / 2.0 + pl.dy;
                let y = tcy - (scy - vb[1]) * k;
                let laid = Tf::fit(vb, x, y, bw, bh, false);
                let tf = Tf {
                    a: laid.a / pl.kx,
                    b: 0.0,
                    c: 0.0,
                    d: laid.d / pl.ky,
                    e: (laid.e - pl.dx) / pl.kx,
                    f: (laid.f - pl.dy) / pl.ky,
                };
                b.place(&o.asset, &pal, OrnamentKind::SurahHeader, &tf, false, placement);
                b.out.n_surah_headers += 1;
            }
        }

        // The medallion is as tall as the printed ring, keeps its aspect, and puts its
        // slot centre on the ring's centre, so the numeral sits where the design leaves
        // room for it. Only the ring is replaced; the numeral is the print's own ink.
        if let Some((oi, o)) = mark {
            let pal = palette(o, oi, spec);
            let (scx, scy) = o.asset.slot_centre();
            for m in self.ayah_marks() {
                if m.r <= 0.0 {
                    continue;
                }
                let size = if spec.ayah_mark_scale > 0.0 {
                    spec.ayah_mark_scale
                } else if o.lineage == Lineage::Scan {
                    crate::defaults::AYAH_MARK_SCALE_SCAN
                } else {
                    1.0
                };
                let s = 2.0 * m.r * size / o.asset.height().max(f32::EPSILON);
                let tf = Tf { a: s, b: 0.0, c: 0.0, d: s, e: m.cx - s * scx, f: m.cy - s * scy };
                let path = if m.ornament_path != NONE {
                    m.ornament_path
                } else {
                    self.data.decorations[m.decoration as usize].first_path
                };
                b.place(&o.asset, &pal, OrnamentKind::AyahMark, &tf, false, self.placement_of_path(path));
                b.out.n_ayah_marks += 1;
            }
        }
        Some(b.out)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::LAYER_TOP;
    use crate::tests::{page_with_surah_frame, square};
    use crate::{LayoutSpec, ReflowSpec};
    use qvp_format::{
        cmds_bbox, encode_cmds, DecoKind, DecoRec, Family, Mark, PathKind, PathRec, NONE_U16, PF_EVENODD,
    };

    /// The synthetic page with a surah frame on line 0, plus one real ayah medallion on
    /// line 1: a 6 unit ring at (45, 13) with a numeral inside it.
    fn page() -> (Page, u32, u32) {
        let (page, _, _, _) = page_with_surah_frame(true);
        let mut data = page.data().clone();
        let add = |data: &mut qvp_format::PageData, cmds: &[Cmd], kind: PathKind| {
            let bbox = cmds_bbox(cmds);
            let op_off = data.ops.len() as u32;
            encode_cmds(cmds, bbox.x0, bbox.y0, &mut data.ops);
            let pi = data.paths.len() as u32;
            data.paths.push(PathRec {
                kind,
                mark: Mark::None,
                family: Family::None,
                flags: PF_EVENODD,
                ox: bbox.x0,
                oy: bbox.y0,
                op_off,
                op_len: data.ops.len() as u32 - op_off,
                bbox,
            });
            pi
        };
        let ring = add(&mut data, &square(4200, 1000, 4800, 1600), PathKind::AyahMarkOrnament);
        let numeral = add(&mut data, &square(4400, 1200, 4600, 1400), PathKind::AyahNumber);
        let mut bbox = data.paths[ring as usize].bbox;
        bbox.union(&data.paths[numeral as usize].bbox);
        let decoration = data.decorations.len() as u32;
        data.decorations.push(DecoRec {
            kind: DecoKind::AyahMark,
            surah: 1,
            ayah: 1,
            text: NONE_U16,
            first_path: ring,
            n_paths: 2,
            line: 1,
            bbox,
        });
        data.ayahs[0].ayah_mark_decoration = decoration as u16;
        data.canonicalize_ops();
        (Page::from_data(data), ring, numeral)
    }

    /// One ornament per kind, each a filled unit square and a stroked one, with slots.
    fn set() -> Arc<OrnamentSet> {
        let mut set = OrnamentSet::default();
        let q = set.quant as i32;
        let unit = |w: i32, h: i32| {
            vec![Cmd::MoveTo(0, 0), Cmd::LineTo(w * q, 0), Cmd::LineTo(w * q, h * q), Cmd::LineTo(0, h * q), Cmd::Close]
        };
        let parts = vec![
            Part { name: "c1".into(), color: 0x00649bff, stroke: false },
            Part { name: "line".into(), color: 0x303641ff, stroke: true },
        ];
        let asset = |w: i32, h: i32, slot: Option<[f32; 4]>, set: &mut OrnamentSet| {
            let first = set.paths.len() as u32;
            set.push_path(0, 0, 0, &unit(w, h));
            set.push_path(1, OPF_STROKE, q / 10, &unit(w, h));
            Asset { view_box: [0.0, 0.0, w as f32, h as f32], slot, first_path: first, n_paths: 2 }
        };
        let mark = asset(80, 100, Some([20.0, 20.0, 40.0, 60.0]), &mut set);
        let header = asset(800, 100, Some([100.0, 10.0, 600.0, 80.0]), &mut set);
        let frame = asset(70, 100, Some([5.0, 5.0, 60.0, 90.0]), &mut set);
        let orn = |kind, asset| Ornament {
            kind,
            style: "test".into(),
            lineage: Lineage::Scan,
            riwayah: String::new(),
            license: License::default(),
            parts: parts.clone(),
            asset,
            slices: None,
        };
        set.ornaments.push(orn(OrnamentKind::AyahMark, mark));
        set.ornaments.push(orn(OrnamentKind::SurahHeader, header));
        set.ornaments.push(orn(OrnamentKind::PageFrame, frame));
        Arc::new(set)
    }

    fn all() -> DressSpec {
        DressSpec {
            ayah_mark: 0,
            surah_header: 1,
            page_frame: 2,
            gap: 5.0,
            band_gap: 5.0,
            ayah_mark_scale: 1.0,
            page_frame_scale: 1.0,
            line_art: 0,
            colors: vec![],
        }
    }

    /// The box of one draw's points, page units.
    fn draw_box(d: &Dress, i: usize) -> (f32, f32, f32, f32) {
        let t = &d.table[i];
        let pts = &d.pts[t.pt_start as usize..(t.pt_start + t.pt_count) as usize];
        let xs = pts.iter().step_by(2);
        let ys = pts.iter().skip(1).step_by(2);
        (
            xs.clone().cloned().fold(f32::INFINITY, f32::min),
            ys.clone().cloned().fold(f32::INFINITY, f32::min),
            xs.cloned().fold(f32::NEG_INFINITY, f32::max),
            ys.cloned().fold(f32::NEG_INFINITY, f32::max),
        )
    }

    fn near(a: f32, b: f32) -> bool {
        (a - b).abs() < 1e-3
    }

    #[test]
    fn places_from_the_page_s_own_measurements() {
        let (mut p, ring, _) = page();
        let d = p.dress(&set(), &all()).unwrap().clone();
        assert_eq!((d.n_ayah_marks, d.n_surah_headers, d.frame_stretched), (1, 1, true));
        // two outlines per placed asset: the border, the band, the medallion, in that order
        assert_eq!(d.table.len(), 6);
        assert_eq!(d.table[0].flags & 0xff, OrnamentKind::PageFrame as u32);
        assert_eq!(d.table[0].placement, NONE);
        assert_eq!(d.table[2].flags & 0xff, OrnamentKind::SurahHeader as u32);
        assert_eq!(d.table[2].placement, 0, "the band follows the title's line");
        assert_eq!(d.table[4].flags & 0xff, OrnamentKind::AyahMark as u32);
        assert_eq!(
            d.table[4].placement,
            p.geometry().table[ring as usize].line,
            "the medallion follows its ring's line"
        );
        // the medallion is as tall as the ring, with the slot centre on the ring centre:
        // the slot centre is at (40, 50) of an 80 × 100 asset scaled to 6 units tall
        let (x0, y0, x1, y1) = draw_box(&d, 4);
        assert!(near(y1 - y0, 6.0) && near(x1 - x0, 4.8), "{:?}", (x0, y0, x1, y1));
        assert!(near((x0 + x1) / 2.0, 45.0) && near((y0 + y1) / 2.0, 13.0), "{:?}", (x0, y0, x1, y1));
        // the band is centred in the column the border encloses (x -5..105), a band gap
        // of 5 short of the border on each side
        let (bx0, _, bx1, _) = draw_box(&d, 2);
        assert!(near(bx1 - bx0, p.width()) && near((bx0 + bx1) / 2.0, 50.0), "{:?}", (bx0, bx1));
        // the border grew the page and no word moved
        let vb = d.view_box;
        assert!(vb[0] < 0.0 && vb[1] < 0.0 && vb[2] > p.width() && vb[3] > p.height(), "{vb:?}");
        assert_eq!(p.view_box(), vb);
        let stroked = &d.table[1];
        assert_ne!(stroked.flags & ODF_STROKE, 0);
        assert!(stroked.stroke_width > 0.0);
        assert_eq!(stroked.color, 0x303641ff);
    }

    #[test]
    fn a_band_without_a_border_spans_the_page() {
        let (mut p, _, _) = page();
        let d = p.dress(&set(), &DressSpec { page_frame: NONE, ayah_mark: NONE, ..all() }).unwrap().clone();
        assert_eq!(d.table.len(), 2);
        let (bx0, _, bx1, _) = draw_box(&d, 0);
        assert!(near(bx1 - bx0, p.width() - 10.0) && near((bx0 + bx1) / 2.0, 50.0), "{:?}", (bx0, bx1));
        assert_eq!(p.view_box(), [0.0, 0.0, p.width(), p.height()]);
    }

    #[test]
    fn hides_the_printed_ring_and_band_and_undress_puts_them_back() {
        let (mut p, ring, numeral) = page();
        let band = p.surah_name_ornament_paths()[0];
        assert_ne!(p.color_of(ring) & 0xff, 0);
        p.dress(&set(), &all()).unwrap();
        assert_eq!(p.color_of(ring) & 0xff, 0, "the ring is hidden");
        assert_eq!(p.color_of(band) & 0xff, 0, "the native band is hidden");
        assert_ne!(p.color_of(numeral) & 0xff, 0, "the numeral is the print's own ink");
        p.undress();
        assert!(p.dress_of().is_none());
        assert_ne!(p.color_of(ring) & 0xff, 0);
        assert_ne!(p.color_of(band) & 0xff, 0);
        assert_eq!(p.view_box(), [0.0, 0.0, p.width(), p.height()]);
        assert_eq!(p.dress_overflow(), (0.0, 0.0, 0.0, 0.0));
    }

    #[test]
    fn the_hiding_survives_a_style_clear_and_a_host_rule() {
        let (mut p, ring, numeral) = page();
        let band = p.surah_name_ornament_paths()[0];
        p.dress(&set(), &all()).unwrap();
        p.styles.clear();
        p.styles.clear_layer(LAYER_TOP);
        assert_eq!(p.color_of(ring) & 0xff, 0, "the ring stays hidden");
        assert_eq!(p.color_of(band) & 0xff, 0, "the native band stays hidden");
        p.style_in(LAYER_TOP, Selector::Ayah(1, 1), Paint::new(0xff0000ff));
        assert_eq!(p.color_of(numeral), 0xff0000ff, "the host rule covers the ayah");
        assert_eq!(p.color_of(ring) & 0xff, 0, "an ayah rule does not bring the ring back");
        p.undress();
        assert_eq!(p.color_of(ring), 0xff0000ff, "undressed, the host rule shows on the ring");
    }

    #[test]
    fn a_host_cannot_remove_or_recolour_the_hiding_by_handle() {
        let (mut p, ring, _) = page();
        p.dress(&set(), &all()).unwrap();
        let hidden = p.dress.as_ref().unwrap().hidden.clone();
        assert!(!hidden.is_empty());
        for &h in &hidden {
            assert_eq!(p.remove_style(h), 0);
            assert_eq!(p.recolor_style(h, Paint::new(0xff0000ff)), 0);
        }
        assert_eq!(p.color_of(ring) & 0xff, 0, "the ring stays hidden");
        p.undress();
        assert_ne!(p.color_of(ring) & 0xff, 0, "undress still removes the hiding");
        assert!(p.styles.is_empty());
    }

    #[test]
    fn the_border_follows_the_laid_out_page() {
        let (mut p, _, _) = page();
        let before = p.dress(&set(), &all()).unwrap().clone();
        let spec =
            LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, pad_top: 10.0, pad_bottom: 10.0, ..Default::default() };
        p.layout(&spec);
        let printed = p.dress_of().unwrap().clone();
        assert_ne!(printed.revision, before.revision, "a layout rebuilds the layer");
        let over = p.dress_overflow();
        assert!(over.0 > 0.0 && over.1 > 0.0 && over.2 > 0.0 && over.3 > 0.0, "{over:?}");
        let tall = LayoutSpec { viewport_h: 1500.0, fill_height: true, ..spec };
        p.layout(&tall);
        let filled = p.dress_of().unwrap().clone();
        assert!(filled.view_box[3] > printed.view_box[3], "{:?} -> {:?}", printed.view_box, filled.view_box);
        assert_eq!(filled.view_box[2], printed.view_box[2], "only the height changed");
    }

    #[test]
    fn under_reflow_the_medallion_takes_its_word_s_group() {
        let (mut p, ring, _) = page();
        p.dress(&set(), &all()).unwrap();
        let spec = LayoutSpec {
            viewport_w: 200.0,
            viewport_h: 400.0,
            reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }),
            ..Default::default()
        };
        let l = p.layout(&spec).clone();
        assert!(!l.path_group.is_empty());
        let d = p.dress_of().unwrap();
        let medallion = d.table.iter().find(|t| t.flags & 0xff == OrnamentKind::AyahMark as u32).unwrap();
        assert_eq!(medallion.placement, l.path_group[ring as usize]);
        let band = d.table.iter().find(|t| t.flags & 0xff == OrnamentKind::SurahHeader as u32).unwrap();
        let title = p.data().decorations[0].first_path;
        assert_eq!(band.placement, l.path_group[title as usize]);
    }

    #[test]
    fn refuses_the_wrong_kind_and_an_empty_spec() {
        let (mut p, _, _) = page();
        assert!(p.dress(&set(), &DressSpec { ayah_mark: 1, ..all() }).is_none(), "ornament 1 is a header");
        assert!(p.dress(&set(), &DressSpec { page_frame: 9, ..all() }).is_none());
        assert!(p.dress(&set(), &DressSpec::default()).is_none());
        assert!(p.dress_of().is_none());
    }

    #[test]
    fn refuses_a_band_or_a_border_without_a_slot() {
        let (mut p, _, _) = page();
        for i in [1usize, 2] {
            let mut s = (*set()).clone();
            s.ornaments[i].asset.slot = None;
            let s = Arc::new(s);
            assert!(p.dress(&s, &all()).is_none(), "ornament {i} has no slot");
            assert!(p.dress_of().is_none());
            // the other kinds alone still dress
            let other = if i == 1 {
                DressSpec { surah_header: NONE, ..all() }
            } else {
                DressSpec { page_frame: NONE, ..all() }
            };
            assert!(p.dress(&s, &other).is_some());
        }
        // a medallion is placed by its slot centre, which falls back to the design's centre
        let mut s = (*set()).clone();
        s.ornaments[0].asset.slot = None;
        assert!(p.dress(&Arc::new(s), &all()).is_some());
    }

    #[test]
    fn line_art_keeps_the_strokes_and_colours_override_a_part() {
        let (mut p, _, _) = page();
        let d = p.dress(&set(), &DressSpec { line_art: LINE_ART_ALL, ..all() }).unwrap().clone();
        assert_eq!(d.table.len(), 3);
        assert!(d.table.iter().all(|t| t.flags & ODF_STROKE != 0));
        // line art per kind: the band loses its fill, the medallion and the border keep theirs
        let d = p.dress(&set(), &DressSpec { line_art: LINE_ART_SURAH_HEADER, ..all() }).unwrap().clone();
        assert_eq!(d.table.len(), 5);
        let band: Vec<_> = d.table.iter().filter(|t| t.flags & 0xff == OrnamentKind::SurahHeader as u32).collect();
        assert_eq!(band.len(), 1);
        assert_ne!(band[0].flags & ODF_STROKE, 0);
        assert_eq!(
            d.table.iter().filter(|t| t.flags & ODF_STROKE == 0).count(),
            2,
            "the two other kinds keep a fill each"
        );
        let d = p.dress(&set(), &DressSpec { colors: vec![(2, 0, 0xff0000ff)], ..all() }).unwrap().clone();
        assert_eq!(d.table[0].color, 0xff0000ff, "the border's fill was repainted");
        assert_eq!(d.table[2].color, 0x00649bff, "the band keeps its own");
    }

    #[test]
    fn line_art_outlines_a_design_that_has_no_linework() {
        let (mut p, _, _) = page();
        // a font marker: two fills, light then dark, and no `line` part
        let mut s = OrnamentSet::default();
        let q = s.quant as i32;
        let first = s.paths.len() as u32;
        s.push_path(
            0,
            0,
            0,
            &[
                Cmd::MoveTo(0, 0),
                Cmd::LineTo(80 * q, 0),
                Cmd::LineTo(80 * q, 100 * q),
                Cmd::LineTo(0, 100 * q),
                Cmd::Close,
            ],
        );
        s.push_path(
            1,
            0,
            0,
            &[Cmd::MoveTo(10 * q, 10 * q), Cmd::LineTo(70 * q, 10 * q), Cmd::LineTo(70 * q, 90 * q), Cmd::Close],
        );
        s.ornaments.push(Ornament {
            kind: OrnamentKind::AyahMark,
            style: "font-test".into(),
            lineage: Lineage::Font,
            riwayah: String::new(),
            license: License::default(),
            parts: vec![
                Part { name: "c1".into(), color: 0xfff8e7ff, stroke: false },
                Part { name: "c2".into(), color: 0x083a3aff, stroke: false },
            ],
            asset: Asset {
                view_box: [0.0, 0.0, 80.0, 100.0],
                slot: Some([20.0, 20.0, 40.0, 60.0]),
                first_path: first,
                n_paths: 2,
            },
            slices: None,
        });
        let set = Arc::new(s);
        let spec = DressSpec { ayah_mark: 0, line_art: LINE_ART_AYAH_MARK, ..DressSpec::default() };
        let d = p.dress(&set, &spec).unwrap().clone();
        assert_eq!(d.table.len(), 2, "every fill is kept, as an outline");
        for t in &d.table {
            assert_ne!(t.flags & ODF_STROKE, 0);
            assert!(t.stroke_width > 0.0);
            assert_eq!(t.color, 0x083a3aff, "one ink, the design's darkest");
        }
        // the medallion is 6 units tall for an asset of 100, so the width scales the same way
        assert!((d.table[0].stroke_width - LINE_ART_WIDTH * 0.06).abs() < 1e-4, "{}", d.table[0].stroke_width);
        // in colour the same design keeps its two fills
        let d = p.dress(&set, &DressSpec { ayah_mark: 0, ..DressSpec::default() }).unwrap().clone();
        assert!(d.table.iter().all(|t| t.flags & ODF_STROKE == 0));
        assert_eq!(d.table[0].color, 0xfff8e7ff);
    }

    #[test]
    fn fit_meet_centres_and_keeps_aspect() {
        let t = Tf::fit([0.0, 0.0, 10.0, 100.0], 0.0, 0.0, 100.0, 100.0, true);
        assert_eq!((t.a, t.d, t.e), (1.0, 1.0, 45.0));
        assert_eq!(t.apply(10.0, 100.0), (55.0, 100.0));
    }

    #[test]
    fn fit_none_stretches_and_honours_the_view_box_origin() {
        let t = Tf::fit([2.0, 4.0, 10.0, 20.0], 0.0, 0.0, 20.0, 20.0, false);
        assert_eq!(t.apply(2.0, 4.0), (0.0, 0.0));
        assert_eq!(t.apply(12.0, 24.0), (20.0, 20.0));
    }

    #[test]
    fn stroke_scale_is_the_geometric_mean() {
        assert_eq!(Tf::scale(4.0, 9.0).stroke_scale(), 6.0);
        assert_eq!(Tf::scale(-1.0, 1.0).stroke_scale(), 1.0);
    }

    /// The slot's box mapped through the frame draw's viewBox, as `x, y, w, h`.
    fn mapped_slot(d: &Dress) -> [f32; 4] {
        let (vb, slot, v) = ([0.0f32, 0.0, 70.0, 100.0], [5.0f32, 5.0, 60.0, 90.0], d.view_box);
        [
            v[0] + (slot[0] - vb[0]) / vb[2] * v[2],
            v[1] + (slot[1] - vb[1]) / vb[3] * v[3],
            slot[2] / vb[2] * v[2],
            slot[3] / vb[3] * v[3],
        ]
    }

    fn assert_slot_on_box(p: &Page, d: &Dress, gap: f32) {
        let (left, top, w, h) = p.laid_page_box();
        let want = [left - gap, top - gap, w + 2.0 * gap, h + 2.0 * gap];
        let got = mapped_slot(d);
        assert!(got.iter().zip(want).all(|(a, b)| near(*a, b)), "{got:?} vs {want:?}");
    }

    #[test]
    fn the_stretched_slot_lands_on_the_grown_box() {
        let (mut p, _, _) = page();
        let spec = DressSpec { ayah_mark: NONE, surah_header: NONE, ..all() };
        let d = p.dress(&set(), &spec).unwrap().clone();
        assert!(d.frame_stretched);
        assert_slot_on_box(&p, &d, spec.gap);
        let lay =
            LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, pad_top: 10.0, pad_bottom: 10.0, ..Default::default() };
        p.layout(&lay);
        assert_slot_on_box(&p, p.dress_of().unwrap(), spec.gap);
        let before = p.dress_of().unwrap().view_box[3];
        p.layout(&LayoutSpec { viewport_h: 1500.0, fill_height: true, ..lay });
        let d = p.dress_of().unwrap();
        assert!(d.view_box[3] > before, "the page got taller");
        assert_slot_on_box(&p, d, spec.gap);
    }

    #[test]
    fn under_reflow_the_border_and_band_go_around_the_laid_out_page() {
        let (mut p, _, _) = page();
        p.dress(&set(), &all()).unwrap();
        let spec = LayoutSpec {
            viewport_w: 200.0,
            viewport_h: 400.0,
            reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }),
            ..Default::default()
        };
        let l = p.layout(&spec).clone();
        let d = p.dress_of().unwrap().clone();
        let gap = 5.0;
        assert!(l.laid_width < p.width(), "a reflowed row is narrower than the printed page");
        let band = d.table.iter().position(|t| t.flags & 0xff == OrnamentKind::SurahHeader as u32).unwrap();
        // the band's points are in the title's printed space; where it lands is through the
        // title's placement, which the host applies
        let pl = l.groups[d.table[band].placement as usize];
        let (x0, _, x1, _) = draw_box(&d, band);
        let (lx0, lx1) = (pl.kx * x0 + pl.dx, pl.kx * x1 + pl.dx);
        let band_gap = 5.0;
        let want = l.laid_width + 2.0 * gap - 2.0 * band_gap;
        assert!(near(lx1 - lx0, want), "{} vs {}", lx1 - lx0, want);
        let centre = l.laid_left + l.laid_width / 2.0;
        assert!(near((lx0 + lx1) / 2.0, centre), "{} vs {}", (lx0 + lx1) / 2.0, centre);
        let over = p.dress_overflow();
        let limit = (gap + 20.0) * l.scale;
        assert!(over.0 < limit && over.2 < limit, "{over:?} limit {limit}");
    }

    #[test]
    fn a_band_spans_the_column_when_the_title_shrinks() {
        // banner_zoom 1 at zoom 2 holds the title at half its laid-out size: kx < 1
        let spec = LayoutSpec {
            viewport_w: 200.0,
            viewport_h: 400.0,
            reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }),
            banner_zoom: 1.0,
            ..Default::default()
        };
        let (gap, band_gap) = (5.0, 5.0);
        for (dress, framed) in [(all(), true), (DressSpec { page_frame: NONE, ..all() }, false)] {
            let (mut p, _, _) = page();
            p.dress(&set(), &dress).unwrap();
            let l = p.layout(&spec).clone();
            let d = p.dress_of().unwrap().clone();
            let band = d.table.iter().position(|t| t.flags & 0xff == OrnamentKind::SurahHeader as u32).unwrap();
            let pl = l.groups[d.table[band].placement as usize];
            assert!(pl.kx < 1.0, "the title is shrunk: kx {}", pl.kx);
            let (x0, y0, x1, y1) = draw_box(&d, band);
            let (lx0, lx1) = (pl.kx * x0 + pl.dx, pl.kx * x1 + pl.dx);
            let want = if framed { l.laid_width + 2.0 * gap - 2.0 * band_gap } else { l.laid_width - 2.0 * band_gap };
            assert!(near(lx1 - lx0, want), "framed {framed}: {} vs {}", lx1 - lx0, want);
            let centre = l.laid_left + l.laid_width / 2.0;
            assert!(near((lx0 + lx1) / 2.0, centre), "framed {framed}: {} vs {}", (lx0 + lx1) / 2.0, centre);
            // the slot's vertical centre (50 of the asset's 100) lands on the title's
            let h = p.surah_headers()[0];
            let slot_cy = y0 + (y1 - y0) * 0.5;
            let title_cy = (h.title_y0 + h.title_y1) / 2.0;
            assert!(
                near(pl.ky * slot_cy + pl.dy, pl.ky * title_cy + pl.dy),
                "framed {framed}: {} vs {}",
                pl.ky * slot_cy + pl.dy,
                pl.ky * title_cy + pl.dy
            );
        }
    }

    /// The dressed box, `x0, y0, x1, y1` in layout px: the border's viewBox through the layout.
    fn border_in_layout(p: &Page) -> (f32, f32, f32, f32) {
        let l = p.current_layout().unwrap();
        let vb = p.view_box();
        (
            l.offset_x + vb[0] * l.scale,
            l.offset_y + vb[1] * l.scale,
            l.offset_x + (vb[0] + vb[2]) * l.scale,
            l.offset_y + (vb[1] + vb[3]) * l.scale,
        )
    }

    #[test]
    fn a_dressed_printed_fit_holds_the_whole_border() {
        let (mut p, _, _) = page();
        // tall enough that the undressed page fits as it is, so only the border shrinks it
        let spec = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
        let plain = p.layout(&spec).clone();
        p.dress(&set(), &all()).unwrap();
        let l = p.current_layout().unwrap().clone();
        let (x0, y0, x1, y1) = border_in_layout(&p);
        let to_screen = |x: f32, y: f32| (l.fit_x + l.fit_scale * x, l.fit_y + l.fit_scale * y);
        let (sx0, sy0) = to_screen(x0, y0);
        let (sx1, sy1) = to_screen(x1, y1);
        let e = 1e-3;
        assert!(sx0 >= -e && sy0 >= -e && sx1 <= 200.0 + e && sy1 <= 400.0 + e, "{:?}", (sx0, sy0, sx1, sy1));
        assert!(
            l.fit_scale < plain.fit_scale,
            "the border is wider than the screen: {} vs {}",
            l.fit_scale,
            plain.fit_scale
        );
        // centred: as much air on each side
        assert!(near(sx0, 200.0 - sx1) && near(sy0, 400.0 - sy1), "{:?}", (sx0, sy0, sx1, sy1));
        // a relayout keeps the dressed fit
        assert_eq!(p.layout(&spec).fit_scale, l.fit_scale);
        // undressed, the plain fit comes back
        p.undress();
        let back = p.current_layout().unwrap();
        assert_eq!((back.fit_scale, back.fit_x, back.fit_y), (plain.fit_scale, plain.fit_x, plain.fit_y));
    }

    #[test]
    fn a_dressed_reflowed_fit_starts_at_the_top_of_the_border() {
        let (mut p, _, _) = page();
        p.dress(&set(), &all()).unwrap();
        let spec = LayoutSpec {
            viewport_w: 200.0,
            viewport_h: 400.0,
            reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }),
            ..Default::default()
        };
        let l = p.layout(&spec).clone();
        let over = p.dress_overflow();
        assert_eq!(l.fit_scale, 1.0);
        assert!(near(l.fit_y, over.1), "{} vs {over:?}", l.fit_y);
        let (_, y0, _, _) = border_in_layout(&p);
        assert!(near(l.fit_y + y0, 0.0), "the border's top is the viewport's top: {}", l.fit_y + y0);
    }

    #[test]
    fn the_dress_clamp_reaches_the_border_and_never_passes_it() {
        let (mut p, _, _) = page();
        let spec = LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() };
        p.layout(&spec);
        p.dress(&set(), &all()).unwrap();
        let (x0, y0, x1, y1) = border_in_layout(&p);
        // zoomed in far enough that the dressed box overflows the viewport both ways
        let zoomed = View { scale: 4.0, offset_x: 0.0, offset_y: 0.0 };
        // pushed past the top and the left: the border's top left corner is the viewport's
        let a = p.dress_clamp(View { offset_x: 1e5, offset_y: 1e5, ..zoomed }, 200.0, 400.0);
        assert!(near(a.offset_x + a.scale * x0, 0.0) && near(a.offset_y + a.scale * y0, 0.0), "{a:?}");
        // pushed past the bottom and the right: the bottom right corner
        let b = p.dress_clamp(View { offset_x: -1e5, offset_y: -1e5, ..zoomed }, 200.0, 400.0);
        assert!(near(b.offset_x + b.scale * x1, 200.0) && near(b.offset_y + b.scale * y1, 400.0), "{b:?}");
        // inside the box the view is left where it is
        let mid =
            View { offset_x: (a.offset_x + b.offset_x) / 2.0, offset_y: (a.offset_y + b.offset_y) / 2.0, ..zoomed };
        assert_eq!(p.dress_clamp(mid, 200.0, 400.0), mid);
        // undressed, it is the plain clamp of the content
        p.undress();
        let l = p.current_layout().unwrap().clone();
        let push = View { offset_x: 1e5, offset_y: 1e5, ..zoomed };
        assert_eq!(p.dress_clamp(push, 200.0, 400.0), push.clamp(l.content_w, l.content_h, 200.0, 400.0));
    }

    #[test]
    fn the_dress_view_puts_the_whole_border_at_the_top_of_a_scrolled_canvas() {
        let (mut p, _, _) = page();
        p.layout(&LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, ..Default::default() });
        p.dress(&set(), &all()).unwrap();
        let (x0, y0, x1, y1) = border_in_layout(&p);
        for (scale, vw) in [(1.0, 1000.0), (1.0, 10.0), (2.5, 1000.0)] {
            let (v, w, h) = p.dress_view(scale, vw);
            assert_eq!(v.scale, scale);
            // the border's top is the canvas's top, and the size holds the whole border
            assert!(near(v.offset_y + scale * y0, 0.0), "{v:?}");
            assert!(near(w, scale * (x1 - x0)) && near(h, scale * (y1 - y0)), "{w} {h}");
            let left = v.offset_x + scale * x0;
            if w < vw {
                assert!(near(left, (vw - w) / 2.0), "centred: {left}");
            } else {
                assert!(near(left, 0.0), "at the left edge: {left}");
            }
        }
        // undressed, the content is centred at the top at its own size
        p.undress();
        let l = p.current_layout().unwrap().clone();
        let (v, w, h) = p.dress_view(1.0, 1000.0);
        assert_eq!((w, h), (l.content_w, l.content_h));
        assert_eq!(v, View { scale: 1.0, offset_x: (1000.0 - l.content_w) / 2.0, offset_y: 0.0 });
    }

    #[test]
    fn a_reflowed_dressed_page_keeps_the_border_on_a_phone() {
        let spec = |reflow| LayoutSpec {
            viewport_w: 390.0,
            viewport_h: 844.0,
            pad_top: 24.0,
            pad_bottom: 24.0,
            pad_left: 16.0,
            pad_right: 16.0,
            reflow,
            ..Default::default()
        };
        for dress in [all(), frame_only(), DressSpec { page_frame_scale: 2.0, ..all() }] {
            for set in [set(), tiled_set(false)] {
                let (mut p, _, _) = page();
                p.dress(&set, &dress).unwrap();
                let l = p.layout(&spec(Some(ReflowSpec { zoom: 2.0, ..Default::default() }))).clone();
                assert_eq!(l.fit_scale, 1.0);
                let (x0, _, x1, _) = border_in_layout(&p);
                let (left, right) = (l.fit_x + x0, l.fit_x + x1);
                assert!(left >= -0.5 && right <= 390.5, "border spans {left}..{right}");
                // the row is narrowed only as far as it must be: the border meets an edge
                assert!(near(left, 0.0) || near(right, 390.0) || near(l.offset_x, 16.0), "{left}..{right}");
                assert_eq!(p.dress_overflow().0, 0.0);
            }
        }
        // undressed, the row is the padded viewport as before
        let (mut p, _, _) = page();
        let plain = p.layout(&spec(Some(ReflowSpec { zoom: 2.0, ..Default::default() }))).clone();
        assert!(near(plain.laid_width * plain.scale, 390.0 - 32.0));
    }

    #[test]
    fn a_dress_view_at_scale_zero_fits_the_border_to_the_width() {
        let (mut p, _, _) = page();
        p.layout(&LayoutSpec { viewport_w: 390.0, viewport_h: 844.0, ..Default::default() });
        p.dress(&set(), &all()).unwrap();
        let (x0, _, x1, _) = border_in_layout(&p);
        let full = x1 - x0;
        for vw in [390.0f32, 100.0, 2000.0] {
            let (v, w, _) = p.dress_view(0.0, vw);
            let want = (vw / full).min(1.0);
            assert!(near(v.scale, want), "{} vs {want}", v.scale);
            assert!(w <= vw + 0.5, "{w} in {vw}");
            let (left, right) = (v.offset_x + v.scale * x0, v.offset_x + v.scale * x1);
            assert!(left >= -0.5 && right <= vw + 0.5, "border spans {left}..{right} of {vw}");
            assert_eq!(p.dress_view(-1.0, vw), (v, w, p.dress_view(0.0, vw).2));
        }
    }

    /// A set whose page frame tiles: a 10 x 8 corner and two 6 unit edge strips, each one
    /// filled rectangle on part 0. The corner also holds an outline on the `slot` part.
    fn tiled_set(rotate: bool) -> Arc<OrnamentSet> {
        let base = set();
        let mut set = (*base).clone();
        let q = set.quant as i32;
        let rect = |w: i32, h: i32| {
            vec![Cmd::MoveTo(0, 0), Cmd::LineTo(w * q, 0), Cmd::LineTo(w * q, h * q), Cmd::LineTo(0, h * q), Cmd::Close]
        };
        let slice = |w: i32, h: i32, with_slot: bool, set: &mut OrnamentSet| {
            let first = set.paths.len() as u32;
            set.push_path(0, 0, 0, &rect(w, h));
            if with_slot {
                set.push_path(2, 0, 0, &rect(w, h));
            }
            Asset {
                view_box: [0.0, 0.0, w as f32, h as f32],
                slot: None,
                first_path: first,
                n_paths: 1 + with_slot as u32,
            }
        };
        let corner = slice(10, 8, true, &mut set);
        let edge_h = slice(6, 8, false, &mut set);
        let edge_v = slice(10, 6, false, &mut set);
        let o = &mut set.ornaments[2];
        o.parts.push(Part { name: "slot".into(), color: 0x00ff00ff, stroke: false });
        o.slices = Some(Slices {
            corner_w: 10.0,
            corner_h: 8.0,
            repeat_h: 6.0,
            repeat_v: 6.0,
            rotate,
            corner,
            edge_h,
            edge_v,
        });
        Arc::new(set)
    }

    fn frame_only() -> DressSpec {
        DressSpec { ayah_mark: NONE, surah_header: NONE, ..all() }
    }

    #[test]
    fn under_reflow_the_title_stays_inside_the_band() {
        let (mut p, _, _) = page();
        p.dress(&set(), &DressSpec { ayah_mark: NONE, page_frame: NONE, ..all() }).unwrap();
        let spec = LayoutSpec {
            viewport_w: 200.0,
            viewport_h: 400.0,
            reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }),
            ..Default::default()
        };
        let l = p.layout(&spec).clone();
        let d = p.dress_of().unwrap().clone();
        let band = d.table.iter().find(|t| t.flags & 0xff == OrnamentKind::SurahHeader as u32).unwrap();
        let pl = l.groups[band.placement as usize];
        // the band's slot is 80 of 100 asset units tall, the band is as wide as the laid page
        let slot_h = 80.0 * l.laid_width / 800.0;
        let title_h = 4.0 * pl.ky;
        assert!(title_h <= 0.9 * slot_h + 1e-3, "title {title_h} in a slot of {slot_h}");
        assert!(pl.ky > 0.0);
        // the cap binds: without a band the same layout grows the title more
        p.undress();
        let free = p.layout(&spec).clone();
        let title = p.data().decorations[0].first_path;
        assert!(free.groups[free.path_group[title as usize] as usize].ky > pl.ky);
    }

    #[test]
    fn a_dress_after_a_reflow_caps_the_title_at_once() {
        let (mut p, _, _) = page();
        let spec = LayoutSpec {
            viewport_w: 200.0,
            viewport_h: 400.0,
            reflow: Some(ReflowSpec { zoom: 2.0, ..Default::default() }),
            ..Default::default()
        };
        let title = p.data().decorations[0].first_path;
        let title_ky = |p: &Page| {
            let l = p.current_layout().unwrap();
            l.groups[l.path_group[title as usize] as usize].ky
        };
        p.layout(&spec);
        let free = title_ky(&p);
        // laid out first, dressed after: no relayout from the host
        p.dress(&set(), &DressSpec { ayah_mark: NONE, page_frame: NONE, ..all() }).unwrap();
        let l = p.current_layout().unwrap().clone();
        assert_eq!(l.spec, spec, "the layout keeps the spec it was made from");
        let capped = title_ky(&p);
        let slot_h = 80.0 * l.laid_width / 800.0;
        assert!(4.0 * capped <= 0.9 * slot_h + 1e-3, "title {} in a slot of {slot_h}", 4.0 * capped);
        assert!(capped < free, "the cap binds: {capped} vs {free}");
        // the band is placed under the capped title
        let d = p.dress_of().unwrap();
        let band = d.table.iter().find(|t| t.flags & 0xff == OrnamentKind::SurahHeader as u32).unwrap();
        assert_eq!(l.groups[band.placement as usize].ky, capped);
        // undressed, the title grows again with no relayout from the host
        p.undress();
        assert_eq!(title_ky(&p), free);
    }

    #[test]
    fn the_medallion_and_border_sizes_are_knobs() {
        let (mut p, _, _) = page();
        // the design's own size for a scanned medallion is 1.5 rings; a font marker's is 1
        let d = p.dress(&set(), &DressSpec { ayah_mark_scale: 0.0, ..all() }).unwrap().clone();
        let (_, y0, _, y1) = draw_box(&d, 4);
        assert!(near(y1 - y0, 9.0), "{}", y1 - y0);
        // twice the ring: 12 units tall, still on the ring centre
        let d = p.dress(&set(), &DressSpec { ayah_mark_scale: 2.0, ..all() }).unwrap().clone();
        let (x0, y0, x1, y1) = draw_box(&d, 4);
        assert!(near(y1 - y0, 12.0) && near(x1 - x0, 9.6), "{:?}", (x0, y0, x1, y1));
        assert!(near((x0 + x1) / 2.0, 45.0) && near((y0 + y1) / 2.0, 13.0), "{:?}", (x0, y0, x1, y1));
        // a border twice the size: the pattern scales as a whole, so the margins double,
        // the slot stays on the grown box, a run's motif is twice as big both ways, and
        // fewer of them fit along the edge
        let one = p.dress(&tiled_set(false), &frame_only()).unwrap().clone();
        let two = p.dress(&tiled_set(false), &DressSpec { page_frame_scale: 2.0, ..frame_only() }).unwrap().clone();
        let unit = (p.width() + 10.0) / 60.0;
        assert!(near(one.view_box[2], p.width() + 10.0 + 2.0 * 5.0 * unit), "{:?}", one.view_box);
        assert!(
            near(two.view_box[2], p.width() + 10.0 + 2.0 * 5.0 * unit * 2.0),
            "{:?} vs {:?}",
            one.view_box,
            two.view_box
        );
        assert!(near(two.view_box[0], -5.0 - 5.0 * unit * 2.0), "{:?}", two.view_box);
        assert_eq!(two.table.len() as u32, 4 + two.n_frame_repeats);
        assert!(two.n_frame_repeats < one.n_frame_repeats, "{} vs {}", two.n_frame_repeats, one.n_frame_repeats);
        let (ox0, oy0, ox1, oy1) = draw_box(&one, 4);
        let (tx0, ty0, tx1, ty1) = draw_box(&two, 4);
        assert!(near(ty1 - ty0, 2.0 * (oy1 - oy0)), "{} vs {}", ty1 - ty0, oy1 - oy0);
        let (ow, tw) = (ox1 - ox0, tx1 - tx0);
        assert!(
            (tw / ow - 2.0).abs() < 0.2,
            "a motif twice as wide, within the nudge that fits a whole number: {ow} {tw}"
        );
    }

    #[test]
    fn a_border_whose_corners_fill_the_edge_has_no_runs() {
        let (mut p, _, _) = page();
        let d = p.dress(&tiled_set(false), &DressSpec { page_frame_scale: 100.0, ..frame_only() }).unwrap().clone();
        assert_eq!(d.n_frame_repeats, 0);
        assert_eq!(d.table.len(), 4, "the four corners only");
        assert!(d.pts.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn a_frame_with_slices_tiles_its_edges() {
        let (mut p, _, _) = page();
        let d = p.dress(&tiled_set(false), &frame_only()).unwrap().clone();
        assert!(!d.frame_stretched);
        // the same arithmetic the code does
        let unit = (p.width() + 10.0) / 60.0;
        let (unit_w, unit_h) = (70.0f32, d.view_box[3] / unit);
        let nh = (((unit_w - 20.0) / 6.0).round() as i32).max(1) as u32;
        let nv = (((unit_h - 16.0) / 6.0).round() as i32).max(1) as u32;
        assert_eq!(d.n_frame_repeats, (nh + nv) * 2);
        assert_eq!(d.table.len() as u32, 4 + d.n_frame_repeats);
        assert!(d.table.iter().all(|t| t.flags & 0xff == OrnamentKind::PageFrame as u32));
        assert!(d.table.iter().all(|t| t.placement == NONE));
        assert!(d.table.iter().all(|t| t.part != 2), "the slot outline is dropped");
        let boxes: Vec<_> = (0..d.table.len()).map(|i| draw_box(&d, i)).collect();
        let x0 = boxes.iter().map(|b| b.0).fold(f32::INFINITY, f32::min);
        let y0 = boxes.iter().map(|b| b.1).fold(f32::INFINITY, f32::min);
        let x1 = boxes.iter().map(|b| b.2).fold(f32::NEG_INFINITY, f32::max);
        let y1 = boxes.iter().map(|b| b.3).fold(f32::NEG_INFINITY, f32::max);
        let v = d.view_box;
        assert!(
            near(x0, v[0]) && near(y0, v[1]) && near(x1, v[0] + v[2]) && near(y1, v[1] + v[3]),
            "{:?} vs {v:?}",
            (x0, y0, x1, y1)
        );
    }

    #[test]
    fn rotated_corners_sit_at_the_four_corners() {
        let (mut p, _, _) = page();
        let d = p.dress(&tiled_set(true), &frame_only()).unwrap().clone();
        let v = d.view_box;
        let (r, b) = (v[0] + v[2], v[1] + v[3]);
        let (tl, tr, bl, br) = (draw_box(&d, 0), draw_box(&d, 1), draw_box(&d, 2), draw_box(&d, 3));
        assert!(near(tl.0, v[0]) && near(tl.1, v[1]), "{tl:?}");
        assert!(near(tr.2, r) && near(tr.1, v[1]), "{tr:?}");
        assert!(near(bl.0, v[0]) && near(bl.3, b), "{bl:?}");
        assert!(near(br.2, r) && near(br.3, b), "{br:?}");
    }
}
