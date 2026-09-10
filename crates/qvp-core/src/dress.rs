//! Dress a page in another mushaf's ornaments.
//!
//! The medallions, surah bands and page borders of other printed mushafs are not
//! part of a QVP page — they are a separate [`OrnamentSet`], and this module puts
//! them on a page they were never drawn for.
//!
//! EVERY ORNAMENT IS PLACED FROM A MEASUREMENT, never a fixed offset: the
//! medallion from the printed ring's box, the band from the surah name's box and
//! the page's own text column, the border from the page's viewBox. That is why
//! the same rules land correctly on all 604 pages of a print these ornaments were
//! never drawn for.
//!
//! The result is a display list in page units — the same shape as the page's own
//! [`crate::Geometry`], with a colour and a stroke width per outline — that a host
//! paints BEHIND the page ink. Drawing it behind is what keeps the printed ayah
//! numerals on top of whatever replaced the rings around them.
use crate::style::Selector;
use crate::{Handle, Page, Rgba};
use std::sync::Arc;
use qvp_format::ornaments::*;
use qvp_format::*;

/// What to dress a page in.
#[derive(Clone, Debug, PartialEq)]
pub struct DressSpec {
    /// Index into [`OrnamentSet::styles`].
    pub style: usize,
    /// Breathing space between the text and the border, in page units.
    pub gap: f32,
    /// Draw only the constant-width strokes — the same drawing with the fills
    /// taken away. One ink left means one colour.
    pub line_art: bool,
    pub ayah_marks: bool,
    pub surah_headers: bool,
    pub page_frame: bool,
    /// Colour overrides by part index into [`Style::parts`]; the design's own
    /// printed colour is used for every part left out.
    pub colors: Vec<(u16, Rgba)>,
}

impl Default for DressSpec {
    fn default() -> Self {
        DressSpec { style: 0, gap: 5.0, line_art: false, ayah_marks: true, surah_headers: true, page_frame: true, colors: vec![] }
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
    /// `kind | (evenodd?1:0)<<8 | (stroke?2:0)<<8`
    pub flags: u32,
    /// Stroke width in page units (0 when the outline is filled).
    pub stroke_width: f32,
    /// Index into [`Style::parts`], so a host can group by printed colour.
    pub part: u32,
    /// The page line this ornament was measured against, so a host applies the
    /// same `line_dy` the layout gives that line's ink. [`crate::NONE`] for the
    /// border, which is placed from the page and does not move with a line.
    pub line: u32,
}

pub const ODF_EVENODD: u32 = 1 << 8;
pub const ODF_STROKE: u32 = 2 << 8;

/// The ornaments placed on a page, and what placing them did to the page.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Dress {
    pub ops: Vec<u8>,
    pub pts: Vec<f32>,
    pub table: Vec<OrnamentDraw>,
    /// The page's viewBox after the border grew it: `x, y, w, h` in page units.
    /// Without a border this is `0, 0, width, height`.
    pub view_box: [f32; 4],
    pub style: usize,
    pub n_ayah_marks: u32,
    pub n_surah_headers: u32,
    /// Repeat units tiled around the four corners (0 when the frame was scaled whole).
    pub n_frame_repeats: u32,
    /// True when the frame had no slices and had to be stretched whole.
    pub frame_stretched: bool,
    /// Bumped every time the display list is rebuilt — by a new dress, and by a
    /// layout that respaced the page under the border. A host caches the raster
    /// of this layer and this is what tells it the raster is stale.
    pub revision: u32,
    /// The rule hiding the printed rings, so undressing puts them back.
    pub(crate) hidden: Handle,
    /// What it was dressed in, so a new layout can re-place the border without
    /// the host handing the set back.
    pub(crate) set: Option<Arc<OrnamentSet>>,
    pub(crate) spec: DressSpec,
}

/// `[a c e; b d f]`, enough for everything placement needs.
#[derive(Clone, Copy, Debug)]
struct Tf {
    a: f32,
    b: f32,
    c: f32,
    d: f32,
    e: f32,
    f: f32,
}

impl Tf {
    const IDENTITY: Tf = Tf { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };
    /// `self` applied after `o` (SVG nesting order).
    fn then(&self, o: &Tf) -> Tf {
        Tf {
            a: self.a * o.a + self.c * o.b,
            b: self.b * o.a + self.d * o.b,
            c: self.a * o.c + self.c * o.d,
            d: self.b * o.c + self.d * o.d,
            e: self.a * o.e + self.c * o.f + self.e,
            f: self.b * o.e + self.d * o.f + self.f,
        }
    }
    fn apply(&self, x: f32, y: f32) -> (f32, f32) {
        (self.a * x + self.c * y + self.e, self.b * x + self.d * y + self.f)
    }
    /// The scale a stroke width is multiplied by (SVG's rule for a non-uniform matrix).
    fn stroke_scale(&self) -> f32 {
        (self.a * self.d - self.b * self.c).abs().sqrt()
    }
    /// Map a viewBox onto a box, as a nested `<svg>` does.
    /// `meet` keeps the aspect and centres; otherwise the drawing is stretched.
    fn fit(vb: [f32; 4], x: f32, y: f32, w: f32, h: f32, meet: bool) -> Tf {
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
    fn translate(tx: f32, ty: f32) -> Tf {
        Tf { e: tx, f: ty, ..Tf::IDENTITY }
    }
    fn scale(sx: f32, sy: f32) -> Tf {
        Tf { a: sx, d: sy, ..Tf::IDENTITY }
    }
}

/// A box in page units.
#[derive(Clone, Copy, Debug)]
struct Box2 {
    x: f32,
    y: f32,
    w: f32,
    h: f32,
}

struct Builder<'a> {
    set: &'a OrnamentSet,
    style: &'a Style,
    spec: &'a DressSpec,
    colors: Vec<Rgba>,
    line_part: Option<u16>,
    slot_part: Option<u16>,
    out: Dress,
}

impl<'a> Builder<'a> {
    /// Emit one asset's outlines through `tf`. `drop_slot` leaves out the
    /// transparent window, which a tiled frame must not keep.
    fn place(&mut self, asset: &Asset, kind: OrnamentKind, tf: &Tf, drop_slot: bool, line: u32) {
        let q = self.set.quant as f32;
        let ss = tf.stroke_scale();
        for i in asset.first_path..asset.first_path + asset.n_paths {
            let p = match self.set.paths.get(i as usize) {
                Some(p) => *p,
                None => continue,
            };
            if self.spec.line_art && Some(p.part) != self.line_part {
                continue;
            }
            if drop_slot && Some(p.part) == self.slot_part {
                continue;
            }
            let color = self.colors.get(p.part as usize).copied().unwrap_or(0);
            if color & 0xff == 0 {
                continue;
            }
            let cmds = match self.set.path_cmds(i as usize) {
                Ok(c) => c,
                Err(_) => continue,
            };
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
                flags: kind as u32 | if p.flags & OPF_EVENODD != 0 { ODF_EVENODD } else { 0 } | if stroke { ODF_STROKE } else { 0 },
                stroke_width: if stroke { p.stroke_width as f32 / q * ss } else { 0.0 },
                part: p.part as u32,
                line,
            });
        }
    }
}

impl Page {
    /// The box the page's text occupies, in page units: every line, plus the
    /// surah bands and basmalahs printed inside them. Page furniture (the page
    /// number and the running head, which sit outside the viewBox) is left out,
    /// and so are the medallions, which the print draws in a layer of their own.
    pub fn content_box(&self) -> (f32, f32, f32, f32) {
        let d = self.data();
        let q = self.quant();
        let mut bb = IBox::EMPTY;
        for l in &d.lines {
            bb.union(&l.bbox);
        }
        for dc in &d.decos {
            if matches!(dc.kind, DecoKind::AyahMark | DecoKind::PageNumber | DecoKind::RunningHead) {
                continue;
            }
            bb.union(&dc.bbox);
        }
        if bb.is_empty() {
            return (0.0, 0.0, self.width(), self.height());
        }
        (bb.x0 as f32 / q, bb.y0 as f32 / q, bb.x1 as f32 / q, bb.y1 as f32 / q)
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

    /// Take the ornaments off: the printed rings come back and the viewBox
    /// returns to the page's own.
    pub fn undress(&mut self) {
        if let Some(d) = self.dress.take() {
            if d.hidden != 0 {
                self.styles.remove(d.hidden);
                self.state_dirty = true;
            }
        }
    }

    /// How much bigger the dressed page is than the laid-out content, on each
    /// side, in viewport px through the current layout: `left, top, right, bottom`.
    ///
    /// A DRESSED PAGE IS BIGGER THAN ITS TEXT — the border was drawn around it —
    /// so a host that fits the content box alone crops the border off. This is
    /// the only number it needs; an undressed page answers zeroes.
    pub fn dress_overflow(&self) -> (f32, f32, f32, f32) {
        let vb = self.view_box();
        let (scale, ox, oy, cw, ch) = match &self.layout {
            Some(l) => (l.scale, l.ox, l.oy, l.content_w, l.content_h),
            None => (1.0, 0.0, 0.0, self.width(), self.height()),
        };
        (
            (-(ox + vb[0] * scale)).max(0.0),
            (-(oy + vb[1] * scale)).max(0.0),
            (ox + (vb[0] + vb[2]) * scale - cw).max(0.0),
            (oy + (vb[1] + vb[3]) * scale - ch).max(0.0),
        )
    }

    /// Re-place the ornaments against the current layout. A layout that respaces
    /// the lines makes the page taller than it is printed, and the border is
    /// drawn around what is on the page, so it has to grow with it.
    pub(crate) fn redress(&mut self) {
        let Some(d) = self.dress.as_ref() else { return };
        let (Some(set), spec, hidden) = (d.set.clone(), d.spec.clone(), d.hidden) else { return };
        if let Some(mut next) = self.build_dress(&set, &spec) {
            self.dress_revision = self.dress_revision.wrapping_add(1);
            next.hidden = hidden;
            next.set = Some(set);
            next.spec = spec;
            next.revision = self.dress_revision;
            self.dress = Some(next);
        }
    }

    /// Put another mushaf's ornaments on this page. Returns None when the set
    /// has no such style. Replaces any previous dress.
    pub fn dress(&mut self, set: &Arc<OrnamentSet>, spec: &DressSpec) -> Option<&Dress> {
        self.undress();
        let mut dress = self.build_dress(set, spec)?;
        self.dress_revision = self.dress_revision.wrapping_add(1);
        dress.revision = self.dress_revision;
        if dress.n_ayah_marks > 0 {
            dress.hidden = self.hide(Selector::Kind(PathKind::AyahMarkOrnament));
        }
        dress.set = Some(set.clone());
        dress.spec = spec.clone();
        self.dress = Some(dress);
        self.dress.as_ref()
    }

    /// Place one style's ornaments and return the display list. Pure: it reads
    /// the page and the current layout and changes neither.
    fn build_dress(&self, set: &OrnamentSet, spec: &DressSpec) -> Option<Dress> {
        let style = set.styles.get(spec.style)?;

        // The design's own printed colours, overridden per part by the caller.
        let mut colors: Vec<Rgba> = style.parts.iter().map(|p| p.color).collect();
        for &(part, rgba) in &spec.colors {
            if let Some(slot) = colors.get_mut(part as usize) {
                *slot = rgba;
            }
        }
        let line_part = style.part_index("line");
        let slot_part = style.part_index("slot");

        let pw = self.width();
        let ph = self.height();
        // The text column a surah band spans: the area the border encloses once
        // one is drawn, and the page's own box otherwise. NOT this page's ink —
        // pages 1 and 2 hold one short surah centred in the column, and a band
        // measured from them would come out a third of the width it has on the
        // other 602 pages. The column is the print's, not the page's content.
        let mut text_area = pw;
        let mut b = Builder {
            set,
            style,
            spec,
            colors,
            line_part,
            slot_part,
            out: Dress { view_box: [0.0, 0.0, pw, ph], style: spec.style, ..Dress::default() },
        };

        // ── the border ───────────────────────────────────────────────────
        // A frame publishes its TEXT AREA as its slot. Mapping this page's box
        // onto that slot makes the band thickness follow at the mushaf's own
        // proportions — one uniform scale, never a stretch to the page box.
        if spec.page_frame {
            if let Some((frame, slot)) = b.style.page_frame.as_ref().and_then(|f| f.slot.map(|s| (f, s))) {
                let gap = spec.gap;
                // THE PAGE THE BORDER GOES AROUND IS THE LAID-OUT ONE. A layout
                // that respaces the lines makes the page taller than it is
                // printed, so the printed height would leave the last lines
                // outside their own border.
                let (page_top, page_h) = self.laid_page_box();
                let unit = (pw + 2.0 * gap) / slot[2].max(f32::EPSILON); // page units per frame unit
                let box_x = -gap - slot[0] * unit;
                let box_y = page_top - gap - slot[1] * unit;
                let unit_w = frame.width();
                let unit_h = (page_h + 2.0 * gap) / unit + 2.0 * slot[1];
                let (box_w, box_h) = (unit_w * unit, unit_h * unit);
                // frame units → page units
                let frame_tf = Tf::fit([0.0, 0.0, unit_w, unit_h], box_x, box_y, box_w, box_h, false);
                match b.style.slices.clone() {
                    Some(sl) => b.tile_frame(&sl, &frame_tf, unit_w, unit_h),
                    None => {
                        let frame = frame.clone();
                        b.place(&frame, OrnamentKind::PageFrame, &Tf::fit(frame.view_box, box_x, box_y, box_w, box_h, false), false, crate::NONE);
                        b.out.frame_stretched = true;
                    }
                }
                // THE FRAME IS DRAWN AROUND THE TEXT, so the page grows by the
                // band it added. Nothing is scaled and no word moves.
                b.out.view_box = [box_x, box_y, box_w, box_h];
                // and the area it encloses is the text column every band spans
                text_area = pw + 2.0 * gap;
            }
        }

        // ── the surah bands ──────────────────────────────────────────────
        // The band spans the text column and is CENTRED ON THE PRINTED NAME it
        // dresses. The column gives the width and the name gives the centre, and
        // both are measurements: a band pinned to the left edge of the page's own
        // ink sits visibly off to one side, because that ink is only as symmetric
        // as the longest line happens to be. The name sits in the design's own
        // slot rather than in the middle of the drawing, which is a different
        // point on every design.
        if spec.surah_headers {
            if let Some((header, slot)) = b.style.surah_header.as_ref().and_then(|h| h.slot.map(|s| (h.clone(), s))) {
                let k = text_area.max(f32::EPSILON) / header.width().max(f32::EPSILON);
                let q = self.quant();
                let bands: Vec<(Box2, u32)> = self
                    .data()
                    .decos
                    .iter()
                    .filter(|d| d.kind == DecoKind::SurahName && !d.bbox.is_empty())
                    .map(|d| (Box2 { x: d.bbox.x0 as f32 / q, y: d.bbox.y0 as f32 / q, w: (d.bbox.x1 - d.bbox.x0) as f32 / q, h: (d.bbox.y1 - d.bbox.y0) as f32 / q }, self.geometry().table[d.first_path as usize].line))
                    .collect();
                let (bw, bh) = (header.width() * k, header.height() * k);
                for (band, line) in bands {
                    let x = band.x + band.w / 2.0 - bw / 2.0;
                    let y = band.y + band.h / 2.0 - (slot[1] + slot[3] / 2.0) * k;
                    b.place(&header, OrnamentKind::SurahHeader, &Tf::fit(header.view_box, x, y, bw, bh, false), false, line);
                    b.out.n_surah_headers += 1;
                }
            }
        }

        // ── the medallions ───────────────────────────────────────────────
        // A medallion is a group of two paths: the ring and the numeral. Only the
        // ring is replaced; the numeral is the print's own ink and stays put,
        // because the whole ornament layer is painted behind the page.
        if spec.ayah_marks {
            if let Some(mark) = b.style.ayah_mark.clone() {
                let q = self.quant();
                let rings = self.ring_boxes(q);
                let ratio = mark.width() / mark.height().max(f32::EPSILON);
                for (r, line) in rings {
                    let w = r.h * ratio;
                    b.place(&mark, OrnamentKind::AyahMark, &Tf::fit(mark.view_box, r.x + r.w / 2.0 - w / 2.0, r.y, w, r.h, true), false, line);
                    b.out.n_ayah_marks += 1;
                }
            }
        }

        Some(b.out)
    }

    /// The printed ring of every real ayah medallion, in page units.
    fn ring_boxes(&self, q: f32) -> Vec<(Box2, u32)> {
        let d = self.data();
        let mut out = Vec::new();
        for dc in d.decos.iter().filter(|x| x.kind == DecoKind::AyahMark && x.ayah != 0) {
            let mut bb = IBox::EMPTY;
            for p in dc.first_path..dc.first_path + dc.n_paths as u32 {
                if d.paths[p as usize].kind == PathKind::AyahMarkOrnament {
                    bb.union(&d.paths[p as usize].bbox);
                }
            }
            if bb.is_empty() {
                bb = dc.bbox;
            }
            if bb.is_empty() {
                continue;
            }
            out.push((Box2 { x: bb.x0 as f32 / q, y: bb.y0 as f32 / q, w: (bb.x1 - bb.x0) as f32 / q, h: (bb.y1 - bb.y0) as f32 / q }, self.geometry().table[dc.first_path as usize].line));
        }
        out
    }
}

impl<'a> Builder<'a> {
    /// THE CORNER KEEPS ITS SHAPE; ONLY THE STRAIGHT RUN REPEATS. Stretching one
    /// drawing to a different page is what makes the side borders fatter than the
    /// top and bottom.
    fn tile_frame(&mut self, sl: &Slices, frame_tf: &Tf, unit_w: f32, unit_h: f32) {
        let (cw, ch) = (sl.corner_w, sl.corner_h);
        let turn = sl.rotate;
        let put = |b: &mut Builder<'a>, asset: &Asset, local: Tf| {
            // the slice's own viewBox already sits at the origin at its own size
            let fit = Tf::fit(asset.view_box, 0.0, 0.0, asset.width(), asset.height(), false);
            b.place(asset, OrnamentKind::PageFrame, &frame_tf.then(&local.then(&fit)), true, crate::NONE);
        };
        let corner = sl.corner.clone();
        put(self, &corner, Tf::IDENTITY);
        put(self, &corner, if turn { Tf { a: -1.0, b: 0.0, c: 0.0, d: -1.0, e: unit_w, f: ch } } else { Tf::translate(unit_w, 0.0).then(&Tf::scale(-1.0, 1.0)) });
        put(self, &corner, if turn { Tf { a: -1.0, b: 0.0, c: 0.0, d: -1.0, e: cw, f: unit_h } } else { Tf::translate(0.0, unit_h).then(&Tf::scale(1.0, -1.0)) });
        put(self, &corner, if turn { Tf::translate(unit_w - cw, unit_h - ch) } else { Tf { a: -1.0, b: 0.0, c: 0.0, d: -1.0, e: unit_w, f: unit_h } });

        // a whole number of repeats, each nudged to fit exactly — a part-drawn
        // motif at the end of a run is the thing the eye catches
        let run = |total: f32, u: f32| -> (u32, f32) {
            let n = ((total / u.max(f32::EPSILON)).round() as i32).max(1) as u32;
            (n, total / n as f32)
        };
        let edge_h = sl.edge_h.clone();
        let (nh, step_h) = run(unit_w - 2.0 * cw, sl.repeat_h);
        for i in 0..nh {
            let x = cw + i as f32 * step_h;
            let k = step_h / sl.repeat_h.max(f32::EPSILON);
            put(self, &edge_h, Tf::translate(x, 0.0).then(&Tf::scale(k, 1.0)));
            put(self, &edge_h, Tf::translate(x, unit_h).then(&Tf::scale(k, -1.0)));
        }
        let edge_v = sl.edge_v.clone();
        let (nv, step_v) = run(unit_h - 2.0 * ch, sl.repeat_v);
        for i in 0..nv {
            let y = ch + i as f32 * step_v;
            let k = step_v / sl.repeat_v.max(f32::EPSILON);
            put(self, &edge_v, Tf::translate(0.0, y).then(&Tf::scale(1.0, k)));
            put(self, &edge_v, Tf::translate(unit_w, y).then(&Tf::scale(-1.0, k)));
        }
        self.out.n_frame_repeats = (nh + nv) * 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fit_meet_centres_and_keeps_aspect() {
        let t = Tf::fit([0.0, 0.0, 10.0, 100.0], 0.0, 0.0, 100.0, 100.0, true);
        assert_eq!(t.a, 1.0);
        assert_eq!(t.d, 1.0);
        assert_eq!(t.e, 45.0);
        assert_eq!(t.apply(10.0, 100.0), (55.0, 100.0));
    }

    #[test]
    fn fit_none_stretches_and_honours_view_box_origin() {
        let t = Tf::fit([2.0, 4.0, 10.0, 20.0], 0.0, 0.0, 20.0, 20.0, false);
        assert_eq!(t.apply(2.0, 4.0), (0.0, 0.0));
        assert_eq!(t.apply(12.0, 24.0), (20.0, 20.0));
    }

    #[test]
    fn stroke_scale_is_the_geometric_mean() {
        assert_eq!(Tf::scale(4.0, 9.0).stroke_scale(), 6.0);
        assert_eq!(Tf::scale(-1.0, 1.0).stroke_scale(), 1.0);
    }
}

impl Page {
    /// The laid-out page box in page units: `(top, height)`. Without a layout
    /// that is the printed page itself.
    pub(crate) fn laid_page_box(&self) -> (f32, f32) {
        self.laid_page.unwrap_or((0.0, self.height()))
    }
}
