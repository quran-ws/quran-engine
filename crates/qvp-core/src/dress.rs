//! Dress a page in another mushaf's ornaments.
//!
//! The medallions, surah bands and page borders of other mushafs are not part of a QVP
//! page. They are an [`OrnamentSet`], and this module puts them on a page they were
//! never drawn for.
//!
//! Every ornament is placed from a measurement, never a fixed offset: the medallion
//! from the printed ring's centre and radius, the band from the surah title's box and
//! the text column, the border from the laid-out page box. That is why the same rules
//! land on all 604 pages.
//!
//! The result is a display list in page units. Each draw carries the placement index
//! the page's own draws carry, so a host applies the transform it already applies to
//! the ink. A host paints the list behind the page ink. Only the printed ring is
//! hidden; the numeral inside it is the print's own ink and stays on top.
use crate::style::{Paint, Selector};
use crate::{Handle, Page, Rgba, NONE};
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
    /// Draw only the constant-width strokes: the same drawing with the fills taken
    /// away. One ink left means one colour.
    pub line_art: bool,
    /// Colour overrides as (ornament, part, colour). Every part left out keeps the
    /// design's own printed colour.
    pub colors: Vec<(u32, u16, Rgba)>,
}

impl Default for DressSpec {
    fn default() -> Self {
        DressSpec { ayah_mark: NONE, surah_header: NONE, page_frame: NONE, gap: 5.0, line_art: false, colors: vec![] }
    }
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
    /// Counts up every time the display list is rebuilt: a new dress, and a layout that
    /// moved the page under the border. A host caches the raster of this layer and this
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
}

fn palette(o: &Ornament, index: u32, spec: &DressSpec) -> Palette {
    let mut colors: Vec<Rgba> = o.parts.iter().map(|p| p.color).collect();
    for &(oi, part, rgba) in &spec.colors {
        if oi == index {
            if let Some(slot) = colors.get_mut(part as usize) {
                *slot = rgba;
            }
        }
    }
    Palette { colors, line: o.part_index("line"), slot: o.part_index("slot") }
}

struct Builder<'a> {
    set: &'a OrnamentSet,
    line_art: bool,
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
            if self.line_art && Some(p.part) != pal.line {
                continue;
            }
            if drop_slot && Some(p.part) == pal.slot {
                continue;
            }
            let color = pal.colors.get(p.part as usize).copied().unwrap_or(0);
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
            let stroke = p.flags & OPF_STROKE != 0;
            self.out.table.push(OrnamentDraw {
                op_start,
                op_count: self.out.ops.len() as u32 - op_start,
                pt_start,
                pt_count: self.out.pts.len() as u32 - pt_start,
                color,
                flags: kind as u32
                    | if p.flags & OPF_EVENODD != 0 { ODF_EVENODD } else { 0 }
                    | if stroke { ODF_STROKE } else { 0 },
                stroke_width: if stroke { p.stroke_width as f32 / q * ss } else { 0.0 },
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
        // end of a run is the thing the eye catches
        let run = |total: f32, u: f32| -> (u32, f32) {
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

    /// Take the ornaments off: the printed rings and bands come back and the viewBox
    /// returns to the page's own.
    pub fn undress(&mut self) {
        if let Some(d) = self.dress.take() {
            for h in d.hidden {
                self.styles.remove(h);
            }
            self.state_dirty = true;
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

    /// Re-place the ornaments against the current layout. A layout that respaces the
    /// lines makes the page taller than it is printed, and the border goes around what
    /// is on the page, so it has to follow. A layout that leaves the display list as it
    /// was keeps the revision, so a host keeps its raster.
    pub(crate) fn redress(&mut self) {
        let Some(d) = self.dress.as_ref() else { return };
        let (Some(set), spec) = (d.set.clone(), d.spec.clone()) else { return };
        if let Some(mut next) = self.build_dress(&set, &spec) {
            if next.ops == d.ops && next.pts == d.pts && next.table == d.table && next.view_box == d.view_box {
                return;
            }
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
    /// when an index is out of range, names an ornament of another kind, or every index
    /// is [`NONE`]. Replaces any previous dress.
    pub fn dress(&mut self, set: &Arc<OrnamentSet>, spec: &DressSpec) -> Option<&Dress> {
        self.undress();
        let mut dress = self.build_dress(set, spec)?;
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
                let (asset_w, asset_h) = (o.asset.width(), o.asset.height());
                match &o.slices {
                    Some(sl) => {
                        let box_x = laid_left - gap - slot[0] * unit;
                        let box_y = laid_top - gap - slot[1] * unit;
                        let unit_w = asset_w;
                        // the bottom margin is the frame's own, not a copy of its top
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
            }
        }

        // The band spans the column and is centred on the printed title. The column gives
        // the width and the title gives the centre, and both are measurements. The title
        // sits in the design's own slot, which is a different point on every design.
        if let Some((oi, o)) = header {
            let pal = palette(o, oi, spec);
            let vb = o.asset.view_box;
            let (_, scy) = o.asset.slot_centre();
            let k = column.max(f32::EPSILON) / o.asset.width().max(f32::EPSILON);
            let (bw, bh) = (o.asset.width() * k, o.asset.height() * k);
            for h in self.surah_headers() {
                if h.title_x1 <= h.title_x0 || h.title_y1 <= h.title_y0 {
                    continue;
                }
                let tcx = (h.title_x0 + h.title_x1) / 2.0;
                let tcy = (h.title_y0 + h.title_y1) / 2.0;
                let x = tcx - bw / 2.0;
                let y = tcy - (scy - vb[1]) * k;
                let placement = self.placement_of_path(self.data.decorations[h.decoration as usize].first_path);
                b.place(&o.asset, &pal, OrnamentKind::SurahHeader, &Tf::fit(vb, x, y, bw, bh, false), false, placement);
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
                let s = 2.0 * m.r / o.asset.height().max(f32::EPSILON);
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
        DressSpec { ayah_mark: 0, surah_header: 1, page_frame: 2, gap: 5.0, line_art: false, colors: vec![] }
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
        // the band spans the column the border encloses, centred on the title (x 20..30)
        let (bx0, _, bx1, _) = draw_box(&d, 2);
        assert!(near(bx1 - bx0, p.width() + 10.0) && near((bx0 + bx1) / 2.0, 25.0), "{:?}", (bx0, bx1));
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
        assert!(near(bx1 - bx0, p.width()), "{:?}", (bx0, bx1));
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
    fn a_layout_that_changes_nothing_keeps_the_revision() {
        let (mut p, _, _) = page();
        p.dress(&set(), &DressSpec { surah_header: NONE, page_frame: NONE, ..all() }).unwrap();
        let spec =
            LayoutSpec { viewport_w: 200.0, viewport_h: 400.0, pad_top: 10.0, pad_bottom: 10.0, ..Default::default() };
        p.layout(&spec);
        let first = p.dress_of().unwrap().revision;
        p.layout(&spec);
        assert_eq!(p.dress_of().unwrap().revision, first, "the same layout keeps the raster");
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
    fn line_art_keeps_the_strokes_and_colours_override_a_part() {
        let (mut p, _, _) = page();
        let d = p.dress(&set(), &DressSpec { line_art: true, ..all() }).unwrap().clone();
        assert_eq!(d.table.len(), 3);
        assert!(d.table.iter().all(|t| t.flags & ODF_STROKE != 0));
        let d = p.dress(&set(), &DressSpec { colors: vec![(2, 0, 0xff0000ff)], ..all() }).unwrap().clone();
        assert_eq!(d.table[0].color, 0xff0000ff, "the border's fill was repainted");
        assert_eq!(d.table[2].color, 0x00649bff, "the band keeps its own");
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
        let (x0, _, x1, _) = draw_box(&d, band);
        assert!(near(x1 - x0, l.laid_width + 2.0 * gap), "{} vs {}", x1 - x0, l.laid_width + 2.0 * gap);
        let over = p.dress_overflow();
        let limit = (gap + 20.0) * l.scale;
        assert!(over.0 < limit && over.2 < limit, "{over:?} limit {limit}");
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
