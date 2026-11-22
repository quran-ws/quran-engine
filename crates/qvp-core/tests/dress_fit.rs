//! A dressed page in a reader that scrolls the page itself: the printed margins cropped to the
//! ink, fill-height on, the canvas as wide as the screen. The medallions stay inside the
//! cropped page, a band stays inside its column, and a border fills the box exactly.
//!
//! Needs dist/pages from scripts/sync-test-data.sh, ornaments.qvo included. Skipped when the
//! data is missing unless QVP_REQUIRE_DATA is set. The released set holds medallions only, so
//! the border and the band are synthetic.
mod common;

use qvp_core::qvp_format::ornaments::*;
use qvp_core::qvp_format::{Cmd, OP_CUBIC, OP_LINE, OP_MOVE, OP_QUAD};
use qvp_core::*;
use std::sync::Arc;

/// Pages with medallions near both edges, an opening page and a page with a surah name.
const PAGES: [u32; 5] = [1, 2, 50, 187, 604];

/// The box a phone reader lays a page out in, its printed margins cut to the ink as the reader
/// crops them: the words' box, one unit out.
fn reader_spec(p: &Page) -> LayoutSpec {
    let ink = p.crop_bounds(&Target::Page, 1.0, true).expect("the page has words");
    LayoutSpec {
        viewport_w: 393.0,
        viewport_h: 700.0,
        fill_height: true,
        crop_left: ink.x0,
        crop_right: p.width() - ink.x1,
        ..Default::default()
    }
}

/// The left and right ends of one draw's ink, its curves walked in small steps.
fn ink_x(d: &Dress, t: &OrnamentDraw) -> (f32, f32) {
    let ops = &d.ops[t.op_start as usize..(t.op_start + t.op_count) as usize];
    let pts = &d.pts[t.pt_start as usize..(t.pt_start + t.pt_count) as usize];
    let (mut lo, mut hi, mut x, mut k) = (f32::INFINITY, f32::NEG_INFINITY, 0.0f32, 0usize);
    let mut add = |v: f32| {
        lo = lo.min(v);
        hi = hi.max(v);
    };
    for &op in ops {
        let n = match op {
            OP_MOVE | OP_LINE => 1,
            OP_QUAD => 2,
            OP_CUBIC => 3,
            _ => 0,
        };
        let xs: Vec<f32> = (0..n).map(|i| pts[k + 2 * i]).collect();
        for s in 0..=64 {
            let t = s as f32 / 64.0;
            let u = 1.0 - t;
            match xs[..] {
                [a] => add(a),
                [a, b] => add(u * u * x + 2.0 * u * t * a + t * t * b),
                [a, b, c] => add(u * u * u * x + 3.0 * u * u * t * a + 3.0 * u * t * t * b + t * t * t * c),
                _ => {}
            }
        }
        if let Some(&last) = xs.last() {
            x = last;
        }
        k += 2 * n;
    }
    let half = if t.flags & ODF_STROKE != 0 { t.stroke_width / 2.0 } else { 0.0 };
    (lo - half, hi + half)
}

/// A band and two borders, one that tiles and one stretched, each a filled rectangle with a
/// slot: the shapes of the catalogue's, without its artwork.
fn furniture() -> Arc<OrnamentSet> {
    let mut set = OrnamentSet::default();
    let q = set.quant as i32;
    let rect = |w: i32, h: i32| {
        vec![Cmd::MoveTo(0, 0), Cmd::LineTo(w * q, 0), Cmd::LineTo(w * q, h * q), Cmd::LineTo(0, h * q), Cmd::Close]
    };
    let mut asset = |w: i32, h: i32, slot: Option<[f32; 4]>| {
        let first = set.paths.len() as u32;
        set.push_path(0, 0, 0, &rect(w, h));
        Asset { view_box: [0.0, 0.0, w as f32, h as f32], slot, first_path: first, n_paths: 1 }
    };
    let band = asset(850, 100, Some([235.0, 10.0, 380.0, 80.0]));
    let tiled = asset(66, 100, Some([5.0, 5.0, 56.0, 90.0]));
    let stretched = asset(64, 100, Some([4.5, 6.0, 55.0, 88.5]));
    let (corner, edge_h, edge_v) = (asset(10, 9, None), asset(6, 9, None), asset(10, 6, None));
    let orn = |kind, asset, slices| Ornament {
        kind,
        style: "test".into(),
        lineage: Lineage::Scan,
        riwayah: String::new(),
        license: License::default(),
        parts: vec![Part { name: "c1".into(), color: 0x00649bff, stroke: false }],
        asset,
        slices,
    };
    let slices =
        Slices { corner_w: 10.0, corner_h: 9.0, repeat_h: 6.0, repeat_v: 6.0, rotate: false, corner, edge_h, edge_v };
    set.ornaments.push(orn(OrnamentKind::SurahHeader, band, None));
    set.ornaments.push(orn(OrnamentKind::PageFrame, tiled, Some(slices)));
    set.ornaments.push(orn(OrnamentKind::PageFrame, stretched, None));
    Arc::new(set)
}

#[test]
fn every_medallion_stays_inside_the_cropped_page() {
    let Some(bytes) = common::data("pages/ornaments.qvo") else { return };
    let set = Arc::new(OrnamentSet::decode(&bytes).unwrap());
    let marks: Vec<u32> =
        (0..set.ornaments.len() as u32).filter(|&i| set.ornaments[i as usize].kind == OrnamentKind::AyahMark).collect();
    assert!(!marks.is_empty());
    for n in PAGES {
        let Some(mut p) = common::page(n) else { return };
        let spec = reader_spec(&p);
        let asked = p.width() - spec.crop_left - spec.crop_right;
        for &m in &marks {
            let style = &set.ornaments[m as usize].style;
            p.dress(&set, &DressSpec { ayah_mark: m, ..Default::default() }).unwrap();
            let l = p.layout(&spec).clone();
            let d = p.dress_of().unwrap().clone();
            let (left, right) = (l.laid_left, l.laid_left + l.laid_width);
            for t in &d.table {
                let (x0, x1) = ink_x(&d, t);
                assert!(x0 >= left - 1e-3 && x1 <= right + 1e-3, "page {n} {style}: ink {x0}..{x1} in {left}..{right}");
            }
            // the crop gives way by the medallion's overhang, never more than a few units
            assert!(
                l.laid_width >= asked - 1e-3 && l.laid_width <= asked + 4.0,
                "page {n} {style}: {} vs {asked}",
                l.laid_width
            );
            // the dressed box is the laid-out page: nothing grows the canvas or shrinks the ink
            let (v, w, h) = p.dress_view(0.0, spec.viewport_w);
            assert!((v.scale - 1.0).abs() < 1e-5 && (w - 393.0).abs() < 1e-2, "page {n} {style}: {v:?} {w}");
            assert!((h - l.content_h).abs() < 1e-2, "page {n} {style}: {h} vs {}", l.content_h);
        }
        p.undress();
    }
}

#[test]
fn a_band_stays_inside_the_cropped_column() {
    let set = furniture();
    for n in PAGES {
        let Some(mut p) = common::page(n) else { return };
        if p.surah_headers().is_empty() {
            continue;
        }
        let spec = reader_spec(&p);
        for band_gap in [0.0, 5.0] {
            p.dress(&set, &DressSpec { surah_header: 0, band_gap, ..Default::default() }).unwrap();
            let l = p.layout(&spec).clone();
            let d = p.dress_of().unwrap().clone();
            assert!(d.n_surah_headers > 0);
            for t in &d.table {
                let (x0, x1) = ink_x(&d, t);
                let (left, right) = (l.laid_left + band_gap, l.laid_left + l.laid_width - band_gap);
                assert!(
                    x0 >= left - 1e-3 && x1 <= right + 1e-3,
                    "page {n} gap {band_gap}: {x0}..{x1} in {left}..{right}"
                );
            }
        }
        p.undress();
    }
}

#[test]
fn a_bordered_page_fills_the_box_and_its_height_plans_it() {
    let set = furniture();
    let mut filled = 0;
    for n in PAGES {
        let Some(mut p) = common::page(n) else { return };
        let spec = reader_spec(&p);
        for frame in [1, 2] {
            p.dress(&set, &DressSpec { page_frame: frame, ..Default::default() }).unwrap();
            // the height a host plans the box with, asked before the page is laid out for it
            let planned = p.printed_height(&spec);
            let half = p.printed_height(&LayoutSpec { viewport_w: spec.viewport_w / 2.0, ..spec });
            assert!((2.0 * half - planned).abs() < 1e-2, "page {n} frame {frame}: in proportion to the width");
            // laid out at the printed pitch, the canvas is the planned height
            p.layout(&LayoutSpec { fill_height: false, ..spec });
            let (_, w, h) = p.dress_view(0.0, spec.viewport_w);
            if p.data().lines.len() as u32 >= p.grid().lines {
                assert!((h - planned).abs() < 1e-2, "page {n} frame {frame}: {h} vs planned {planned}");
            }
            assert!((w - spec.viewport_w).abs() < 1e-2);
            // a page that fits fills the box exactly, border included
            if planned <= spec.viewport_h {
                p.layout(&spec);
                let (v, w, h) = p.dress_view(0.0, spec.viewport_w);
                assert!((w - 393.0).abs() < 1e-2 && (h - 700.0).abs() < 1e-2, "page {n} frame {frame}: {w} x {h}");
                assert!((p.current_layout().unwrap().fit_scale - v.scale).abs() < 1e-5);
                filled += 1;
            }
        }
        p.undress();
    }
    assert!(filled >= 6, "only {filled} pages fitted their box");
}

/// A page dressed in a medallion and no border is placed as the undressed page, printed or
/// reflowed: the same fit, dress view and dress clamp at every zoom step. Page 264 in a phone
/// reader, the case where a reflowed page was pushed hundreds of px to the left.
#[test]
fn a_page_dressed_without_a_border_is_placed_as_the_undressed_page() {
    let Some(bytes) = common::data("pages/ornaments.qvo") else { return };
    let set = Arc::new(OrnamentSet::decode(&bytes).unwrap());
    let Some(mark) = set.find(OrnamentKind::AyahMark, "font-003-regular") else { return };
    let Some(mut p) = common::page(264) else { return };
    let base = LayoutSpec {
        viewport_w: 370.0,
        viewport_h: 700.0,
        // the reader's fill knobs; zoom 0 is the printed page
        reflow: Some(ReflowSpec { zoom: 0.0, fill: Fill::Justified, ..Default::default() }),
        ..reader_spec(&p)
    };
    let steps = p.zoom_steps(&base).len() as u32;
    assert!(steps >= 3);
    let push = View { scale: 3.0, offset_x: 1e4, offset_y: -1e4 };
    for step in 0..=3 {
        let spec = p.zoom_spec(&base, Zoom { mode: ZoomMode::Stepped, step, zoom: 1.0 });
        let placed = |p: &mut Page| {
            let l = p.layout(&spec).clone();
            (l.scale, l.fit_scale, l.fit_x, l.fit_y, p.dress_view(0.0, 370.0), p.dress_clamp(push, 370.0, 700.0))
        };
        p.undress();
        let bare = placed(&mut p);
        p.dress(&set, &DressSpec { ayah_mark: mark as u32, ..Default::default() }).unwrap();
        let dressed = placed(&mut p);
        let o = p.dress_overflow();
        assert!([o.0, o.1, o.2, o.3].iter().all(|v| v.abs() < 1e-3), "step {step}: overflow {o:?}");
        let near = |a: f32, b: f32| (a - b).abs() < 1e-3 * a.abs().max(1.0);
        let (bv, dv) = (bare.4, dressed.4);
        assert!(near(bare.1, dressed.1) && near(bare.2, dressed.2) && near(bare.3, dressed.3), "step {step}: fit");
        assert!(
            near(bv.0.scale, dv.0.scale) && near(bv.0.offset_x, dv.0.offset_x) && near(bv.0.offset_y, dv.0.offset_y)
        );
        assert!(near(bv.1, dv.1) && near(bv.2, dv.2), "step {step}: dress view size {bv:?} vs {dv:?}");
        assert!(
            near(bare.5.offset_x, dressed.5.offset_x) && near(bare.5.offset_y, dressed.5.offset_y),
            "step {step}: clamp"
        );
        if step == 0 {
            // printed: the crop gives way to a medallion that reaches past it, by a hair at most
            assert!(dressed.0 <= bare.0 && dressed.0 > 0.99 * bare.0, "step {step}: scale {} vs {}", dressed.0, bare.0);
        } else {
            // reflowed: the rows are broken exactly as undressed
            assert_eq!(bare.0, dressed.0, "step {step}: scale");
        }
    }
}

/// A pinch or a step on a page dressed in a border lands on the new layout's resting scale.
/// The printed page rests shrunk so its border fits across the width, and a reflowed page
/// rests at scale 1 because its rows are narrowed instead. Page 584 in a phone reader that
/// scrolls the page itself, where a pinch carried one layout's scale into the other.
#[test]
fn a_pinch_on_a_bordered_page_lands_on_the_new_layout_s_rest() {
    let set = furniture();
    let Some(mut p) = common::page(584) else { return };
    let spec = LayoutSpec {
        viewport_w: 370.0,
        reflow: Some(ReflowSpec { zoom: 0.0, fill: Fill::Justified, ..Default::default() }),
        ..reader_spec(&p)
    };
    let rest = |p: &Page| p.dress_view(0.0, spec.viewport_w).0;
    let printed = Zoom { mode: ZoomMode::Stepped, step: 0, zoom: 1.0 };
    let focal = (185.0, 350.0);
    for dressed in [true, false] {
        p.undress();
        if dressed {
            p.dress(&set, &DressSpec { page_frame: 1, ..Default::default() }).unwrap();
        }
        let case = if dressed { "bordered" } else { "bare" };
        let at = p.zoom_spec(&spec, printed);
        p.layout(&at);
        let at_print = rest(&p);
        if dressed {
            assert!(at_print.scale < 0.9, "the border shrinks the printed page: {}", at_print.scale);
        }
        // pinch in from the printed page's rest
        let c = p.zoom_pinch(&spec, printed, at_print, 1.6, focal);
        assert!(c.relaid && c.zoom.step >= 1, "{case}: {:?}", c.zoom);
        let at_zoom = rest(&p);
        assert!((c.view.scale - at_zoom.scale).abs() < 1e-4, "{case} in: {} vs rest {}", c.view.scale, at_zoom.scale);
        // and back out from the reflowed page's rest
        let back = p.zoom_pinch(&spec, c.zoom, at_zoom, 0.4, focal);
        assert!(back.relaid && back.zoom.step == 0, "{case}: {:?}", back.zoom);
        assert!(
            (back.view.scale - at_print.scale).abs() < 1e-4,
            "{case} out: {} vs {}",
            back.view.scale,
            at_print.scale
        );
        // the step button lands on the same scale
        let step = p.zoom_to_step(&spec, back.zoom, 2, back.view);
        assert!(
            (step.view.scale - rest(&p).scale).abs() < 1e-4,
            "{case} step: {} vs {}",
            step.view.scale,
            rest(&p).scale
        );
        if !dressed {
            assert_eq!((c.view.scale, back.view.scale, step.view.scale), (1.0, 1.0, 1.0), "a bare page keeps scale 1");
        }
    }
}
