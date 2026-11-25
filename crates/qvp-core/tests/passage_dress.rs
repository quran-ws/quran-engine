//! Passages in another mushaf's ayah medallion, on the real page data.
//!
//! A dressed passage draws the outlines a dressed page draws for the same medallions, each
//! under the placement of the word it goes with. Every design stays inside its layout, the
//! printed ring leaves the draw list and the numeral stays. Line art keeps the row breaks of the
//! colour drawing, undressing gives the printed passage back, and a medallion after a cut or
//! standing alone is dressed too.
//!
//! Every medallion of the released set is tested, scanned and font. A synthetic scan design
//! adds a stroked line around its fill, and a band, which is not a medallion.
//!
//! Needs dist/pages from scripts/sync-test-data.sh, ornaments.qvo included. Skipped when the
//! data is missing unless QVP_REQUIRE_DATA is set.
mod common;

use qvp_core::qvp_format::ornaments::*;
use qvp_core::qvp_format::{Cmd, DecoKind, PathKind, OP_CLOSE, OP_CUBIC, OP_LINE, OP_MOVE, OP_QUAD};
use qvp_core::*;
use std::sync::Arc;

/// The released set and the index of its first medallion.
fn released() -> Option<(Arc<OrnamentSet>, u32)> {
    let bytes = common::data("pages/ornaments.qvo")?;
    let set = Arc::new(OrnamentSet::decode(&bytes).unwrap());
    let m = (0..set.ornaments.len() as u32).find(|&i| set.ornaments[i as usize].kind == OrnamentKind::AyahMark)?;
    Some((set, m))
}

/// Every medallion of a set of one lineage.
fn medallions(set: &OrnamentSet, lineage: Lineage) -> Vec<u32> {
    (0..set.ornaments.len() as u32)
        .filter(|&i| {
            set.ornaments[i as usize].kind == OrnamentKind::AyahMark && set.ornaments[i as usize].lineage == lineage
        })
        .collect()
}

/// A scanned medallion, drawn larger than its ring: a filled square with a stroked line part
/// around it and a slot in the middle. Then a band, which is not a medallion.
fn scanned() -> Arc<OrnamentSet> {
    let mut set = OrnamentSet::default();
    let q = set.quant as i32;
    let square = |x0: i32, y0: i32, x1: i32, y1: i32| {
        vec![
            Cmd::MoveTo(x0 * q, y0 * q),
            Cmd::LineTo(x1 * q, y0 * q),
            Cmd::LineTo(x1 * q, y1 * q),
            Cmd::LineTo(x0 * q, y1 * q),
            Cmd::Close,
        ]
    };
    let first = set.paths.len() as u32;
    set.push_path(0, 0, 0, &square(0, 0, 120, 100));
    set.push_path(1, OPF_STROKE, 4 * q, &square(2, 2, 118, 98));
    set.push_path(2, 0, 0, &square(40, 30, 80, 70));
    let parts = vec![
        Part { name: "fill".into(), color: 0x00649bff, stroke: false },
        Part { name: "line".into(), color: 0x203040ff, stroke: true },
        Part { name: "slot".into(), color: 0x00000000, stroke: false },
    ];
    let mark = Asset {
        view_box: [0.0, 0.0, 120.0, 100.0],
        slot: Some([40.0, 30.0, 40.0, 40.0]),
        first_path: first,
        n_paths: 3,
    };
    let band_first = set.paths.len() as u32;
    set.push_path(0, 0, 0, &square(0, 0, 850, 100));
    let band = Asset {
        view_box: [0.0, 0.0, 850.0, 100.0],
        slot: Some([235.0, 10.0, 380.0, 80.0]),
        first_path: band_first,
        n_paths: 1,
    };
    let orn = |kind, asset, parts| Ornament {
        kind,
        style: "scan".into(),
        lineage: Lineage::Scan,
        riwayah: String::new(),
        license: License::default(),
        parts,
        asset,
        slices: None,
    };
    set.ornaments.push(orn(OrnamentKind::AyahMark, mark, parts.clone()));
    set.ornaments.push(orn(OrnamentKind::SurahHeader, band, parts));
    Arc::new(set)
}

/// The paths of a page's printed rings.
fn rings(p: &Page) -> Vec<u32> {
    let d = p.data();
    (0..d.paths.len() as u32).filter(|&i| d.paths[i as usize].kind == PathKind::AyahMarkOrnament).collect()
}

/// The paths of a page's ayah numerals.
fn numerals(p: &Page) -> Vec<u32> {
    let d = p.data();
    (0..d.paths.len() as u32).filter(|&i| d.paths[i as usize].kind == PathKind::AyahNumber).collect()
}

/// One outline as its ops, its points' bits, its colour, flags and stroke width.
type Outline = (Vec<u8>, Vec<u32>, u32, u32, u32);

/// Each outline of a list, sorted: two lists of the same outlines in any order compare equal.
fn outlines(ops: &[u8], pts: &[f32], table: &[OrnamentDraw]) -> Vec<Outline> {
    let mut out: Vec<_> = table
        .iter()
        .map(|t| {
            let o = ops[t.op_start as usize..(t.op_start + t.op_count) as usize].to_vec();
            let p = pts[t.pt_start as usize..(t.pt_start + t.pt_count) as usize].iter().map(|v| v.to_bits()).collect();
            (o, p, t.color, t.flags, t.stroke_width.to_bits())
        })
        .collect();
    out.sort();
    out
}

/// The medallion outlines of a page dressed in `spec`.
fn page_outlines(p: &mut Page, set: &Arc<OrnamentSet>, spec: &DressSpec) -> Vec<Outline> {
    let d = p.dress(set, spec).expect("the page takes the medallion").clone();
    p.undress();
    let table: Vec<OrnamentDraw> =
        d.table.iter().copied().filter(|t| t.flags & 0xff == OrnamentKind::AyahMark as u32).collect();
    outlines(&d.ops, &d.pts, &table)
}

/// The box of every point a dressed outline puts down, in layout px: its curves walked in small
/// steps under its placement and the layout's scale, grown by half its stroke.
fn dress_box(l: &PassageLayout, d: &PassageDress, t: &OrnamentDraw) -> [f32; 4] {
    let q = l.placements[t.placement as usize];
    let ops = &d.ops[t.op_start as usize..(t.op_start + t.op_count) as usize];
    let pts = &d.pts[t.pt_start as usize..(t.pt_start + t.pt_count) as usize];
    let mut b = [f32::INFINITY, f32::INFINITY, f32::NEG_INFINITY, f32::NEG_INFINITY];
    let mut add = |x: f32, y: f32| {
        let (x, y) = ((q.kx * x + q.dx) * l.scale, (q.ky * y + q.dy) * l.scale);
        b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
    };
    let (mut here, mut k) = ((0.0f32, 0.0f32), 0usize);
    for &op in ops {
        let n = match op {
            OP_MOVE | OP_LINE => 1,
            OP_QUAD => 2,
            OP_CUBIC => 3,
            _ => 0,
        };
        let p: Vec<(f32, f32)> = (0..n).map(|i| (pts[k + 2 * i], pts[k + 2 * i + 1])).collect();
        for s in 0..=32 {
            let t = s as f32 / 32.0;
            let u = 1.0 - t;
            match p[..] {
                [a] => add(a.0, a.1),
                [a, c] => add(
                    u * u * here.0 + 2.0 * u * t * a.0 + t * t * c.0,
                    u * u * here.1 + 2.0 * u * t * a.1 + t * t * c.1,
                ),
                [a, c, e] => add(
                    u * u * u * here.0 + 3.0 * u * u * t * a.0 + 3.0 * u * t * t * c.0 + t * t * t * e.0,
                    u * u * u * here.1 + 3.0 * u * u * t * a.1 + 3.0 * u * t * t * c.1 + t * t * t * e.1,
                ),
                _ => {}
            }
        }
        if let Some(&last) = p.last() {
            here = last;
        }
        debug_assert!(op != OP_CLOSE || n == 0);
        k += 2 * n;
    }
    let half = if t.flags & ODF_STROKE != 0 { t.stroke_width / 2.0 * q.kx * l.scale } else { 0.0 };
    [b[0] - half, b[1] - half, b[2] + half, b[3] + half]
}

/// Every dressed outline lies inside its layout, and inside the box of the row it is on.
fn inside(l: &PassageLayout, d: &PassageDress, at: &str) {
    let slack = 0.05;
    for t in &d.table {
        let [x0, y0, x1, y1] = dress_box(l, d, t);
        assert!(
            x0 >= -slack && y0 >= -slack && x1 <= l.width + slack && y1 <= l.height + slack,
            "{at}: a design at {x0},{y0}..{x1},{y1} outside {}×{}",
            l.width,
            l.height
        );
        assert!(
            l.rows.iter().any(|r| x0 >= r.x0 - slack && x1 <= r.x1 + slack && y0 >= r.y0 - slack && y1 <= r.y1 + slack),
            "{at}: a design at {x0},{y0}..{x1},{y1} outside every row"
        );
    }
}

/// The ring and the numeral of every drawn medallion: no ring path is drawn, every numeral is.
fn rings_out_numerals_in(l: &PassageLayout, pages: &[&Page], at: &str) {
    for p in pages {
        let n = p.page_number();
        let drawn: Vec<u32> = l.draws.iter().filter(|d| d.page == n).map(|d| d.path).collect();
        for r in rings(p) {
            assert!(!drawn.contains(&r), "{at}: page {n} draws ring path {r}");
        }
        for num in numerals(p) {
            let shown = l.words.iter().any(|w| w.page == n);
            if shown && p.data().paths[num as usize].kind == PathKind::AyahNumber {
                // a numeral of an ayah the layout shows is drawn
                let deco = p
                    .data()
                    .decorations
                    .iter()
                    .find(|d| (d.first_path..d.first_path + d.n_paths as u32).contains(&num));
                if let Some(deco) = deco {
                    if l.words.iter().any(|w| w.page == n && w.surah == deco.surah && w.ayah == deco.ayah) {
                        assert!(drawn.contains(&num), "{at}: page {n} leaves out numeral {num}");
                    }
                }
            }
        }
    }
}

/// The medallions of the ayahs a page's words fill completely.
fn medallions_on(p: &Page, surah: u16) -> usize {
    p.data().decorations.iter().filter(|d| d.kind == DecoKind::AyahMark && d.surah == surah && d.ayah != 0).count()
}

#[test]
fn a_dressed_passage_draws_the_outlines_a_dressed_page_draws() {
    let Some((set, m)) = released() else { return };
    let (Some(mut p2), Some(mut p3)) = (common::page(2), common::page(3)) else { return };
    let scan = scanned();
    let mut specs = vec![
        (set.clone(), DressSpec { ayah_mark: m, colors: vec![(m, 0, 0x336699ff)], ..Default::default() }),
        (scan.clone(), DressSpec { ayah_mark: 0, ..Default::default() }),
        (scan.clone(), DressSpec { ayah_mark: 0, ayah_mark_scale: 0.8, line_art: LINE_ART_ALL, ..Default::default() }),
    ];
    // every medallion the release holds, in colour and as line art
    for i in medallions(&set, Lineage::Scan).into_iter().chain(medallions(&set, Lineage::Font)) {
        specs.push((set.clone(), DressSpec { ayah_mark: i, ..Default::default() }));
        specs.push((set.clone(), DressSpec { ayah_mark: i, line_art: LINE_ART_AYAH_MARK, ..Default::default() }));
    }
    for (set, spec) in &specs {
        let mut want = page_outlines(&mut p2, set, spec);
        want.extend(page_outlines(&mut p3, set, spec));
        want.sort();
        // 2:1-16 runs from page 2, which prints smaller, onto page 3: the words of page 3 are
        // resized, and their designs stay in page 3's units under their placement
        let mut passage = Passage::load(&[&p2, &p3], 2, 1, 16).unwrap();
        assert!(passage.dress(set, spec));
        for width in [0.0, 300.0] {
            let l = passage.layout(&PassageSpec { width, scale: 0.9, ..Default::default() }).unwrap().clone();
            let d = passage.dress_of().unwrap().clone();
            assert_eq!(d.ayah_mark, spec.ayah_mark);
            assert_eq!(d.n_ayah_marks, 16);
            assert_eq!(outlines(&d.ops, &d.pts, &d.table), want, "width {width}, {spec:?}");
            inside(&l, &d, &format!("2:1-16 at {width}"));
            rings_out_numerals_in(&l, &[&p2, &p3], &format!("2:1-16 at {width}"));
            // each design goes under the placement of the word that closes its ayah
            for t in &d.table {
                assert!(l.draws.iter().any(|dr| dr.placement == t.placement), "a design under a placement no word has");
            }
        }
    }
}

#[test]
fn every_dressed_medallion_stays_inside_its_layout() {
    let Some((set, m)) = released() else { return };
    let scan = scanned();
    // a font marker as line art, the synthetic scan, and every scanned medallion of the release,
    // each drawn half as big again as its ring
    let mut dresses = vec![
        (set.clone(), DressSpec { ayah_mark: m, line_art: LINE_ART_AYAH_MARK, ..Default::default() }),
        (scan.clone(), DressSpec { ayah_mark: 0, ..Default::default() }),
    ];
    for i in medallions(&set, Lineage::Scan) {
        dresses.push((set.clone(), DressSpec { ayah_mark: i, ..Default::default() }));
    }
    for n in 1..=604 {
        let page = common::page(n).unwrap();
        let data = page.data();
        let mut surahs: Vec<u16> = data.words.iter().map(|w| w.surah).collect();
        surahs.dedup();
        for surah in surahs {
            let ayahs: Vec<u16> = data.ayahs.iter().filter(|a| a.surah == surah).map(|a| a.ayah).collect();
            let (from, to) = (ayahs[0], ayahs[ayahs.len() - 1]);
            let mut passage = Passage::load(&[&page], surah, from, to).unwrap();
            for (set, spec) in &dresses {
                assert!(passage.dress(set, spec), "page {n} {surah}: refused");
                for (width, align) in [(0.0, Align::Right), (320.0, Align::Justified)] {
                    let at = format!("page {n}, {surah}:{from}-{to} at {width}");
                    let l = passage
                        .layout(&PassageSpec { width, scale: 0.9, align, ..Default::default() })
                        .unwrap()
                        .clone();
                    let d = passage.dress_of().unwrap().clone();
                    assert_eq!(d.n_ayah_marks as usize, medallions_on(&page, surah), "{at}");
                    inside(&l, &d, &at);
                    rings_out_numerals_in(&l, &[&page], &at);
                }
                // cut to one row, the last medallion still closes it, after the ellipsis
                let cut = PassageSpec {
                    width: 200.0,
                    scale: 0.9,
                    max_rows: 1,
                    keep_ayah_mark: true,
                    ellipsis_width: 9.0,
                    ..Default::default()
                };
                let at = format!("page {n}, {surah}:{from}-{to} cut");
                let l = passage.layout(&cut).unwrap().clone();
                let d = passage.dress_of().unwrap().clone();
                inside(&l, &d, &at);
                if l.is_truncated {
                    let e = l.ellipsis.unwrap();
                    let tail = l.placements.len() as u32 - 1;
                    let tail = (0..l.placements.len() as u32)
                        .rev()
                        .find(|&i| d.table.iter().any(|t| t.placement == i))
                        .unwrap_or(tail);
                    for t in d.table.iter().filter(|t| t.placement == tail) {
                        let b = dress_box(&l, &d, t);
                        assert!(
                            b[2] <= e[0] + 0.05,
                            "{at}: the kept medallion at {b:?} is not after the ellipsis {e:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn line_art_keeps_the_row_breaks_and_a_bigger_design_takes_more_room() {
    let Some((set, _)) = released() else { return };
    let Some(page) = common::page(50) else { return };
    let mut passage = Passage::load(&[&page], 3, 1, 9).unwrap_or_else(|_| {
        let surah = page.data().words[0].surah;
        let ayah = page.data().words.last().unwrap().ayah;
        Passage::load(&[&page], surah, ayah, ayah).unwrap()
    });
    let spec = PassageSpec { width: 260.0, scale: 0.9, align: Align::Justified, ..Default::default() };
    let undressed = passage.layout(&spec).unwrap().clone();
    for i in medallions(&set, Lineage::Scan).into_iter().chain(medallions(&set, Lineage::Font)) {
        let style = &set.ornaments[i as usize].style;
        let colour = DressSpec { ayah_mark: i, ..Default::default() };
        let lines = DressSpec { ayah_mark: i, line_art: LINE_ART_AYAH_MARK, ..Default::default() };
        assert!(passage.dress(&set, &colour));
        let a = passage.layout(&spec).unwrap().clone();
        let da = passage.dress_of().unwrap().clone();
        assert!(passage.dress(&set, &lines));
        let b = passage.layout(&spec).unwrap().clone();
        let db = passage.dress_of().unwrap().clone();
        // the same rows, words and draws: dark mode moves no break
        assert_eq!(a, b, "{style}");
        assert_eq!(da.n_ayah_marks, db.n_ayah_marks, "{style}");
        assert!(db.table.iter().all(|t| t.flags & ODF_STROKE != 0), "{style}: line art draws strokes only");
        assert_ne!(outlines(&da.ops, &da.pts, &da.table), outlines(&db.ops, &db.pts, &db.table), "{style}");
    }
    // a design half as big again as its ring makes its word's box wider than the printed one
    let scan = scanned();
    assert!(passage.dress(&scan, &DressSpec { ayah_mark: 0, ..Default::default() }));
    let c = passage.layout(&spec).unwrap().clone();
    let closing = |l: &PassageLayout| {
        let w = l.words.iter().find(|w| w.x1 - w.x0 > w.word_x1 - w.word_x0 + 1.0).unwrap();
        (w.x1 - w.x0) - (w.word_x1 - w.word_x0)
    };
    assert!(closing(&c) > closing(&undressed) * 1.2, "{} vs {}", closing(&c), closing(&undressed));
}

#[test]
fn undressing_gives_the_printed_passage_back() {
    let Some((set, m)) = released() else { return };
    let Some(page) = common::page(3) else { return };
    let spec = PassageSpec { width: 300.0, scale: 0.9, ..Default::default() };
    let mut never = Passage::load(&[&page], 2, 6, 16).unwrap();
    let want = never.layout(&spec).unwrap().clone();
    let want_styled = never.styled_draw_list(&[&page]);
    let mut passage = Passage::load(&[&page], 2, 6, 16).unwrap();
    passage.layout(&spec).unwrap();
    assert!(passage.dress(&set, &DressSpec { ayah_mark: m, ..Default::default() }));
    // the dress drops the layout: the host lays the passage out again
    assert!(passage.current_layout().is_none());
    assert!(passage.dress_of().is_none());
    assert!(passage.styled_draw_list(&[&page]).is_empty());
    let dressed = passage.layout(&spec).unwrap().clone();
    assert_ne!(dressed, want);
    // the styled list is the draw list when no page colours a letter on its own
    let styled = passage.styled_draw_list(&[&page]);
    assert_eq!(styled, dressed.draws);
    passage.undress();
    assert!(passage.current_layout().is_none());
    assert_eq!(passage.layout(&spec).unwrap(), &want);
    assert!(passage.dress_of().is_none());
    assert_eq!(passage.styled_draw_list(&[&page]), want_styled);
    // undressing an undressed passage keeps its layout
    passage.undress();
    assert!(passage.current_layout().is_some());
}

#[test]
fn a_spec_with_no_medallion_is_refused_and_undresses() {
    let Some((set, m)) = released() else { return };
    let Some(page) = common::page(3) else { return };
    let spec = PassageSpec { width: 300.0, scale: 0.9, ..Default::default() };
    let mut passage = Passage::load(&[&page], 2, 6, 16).unwrap();
    let printed = passage.layout(&spec).unwrap().clone();
    assert!(!passage.dress(&set, &DressSpec::default()));
    // a refusal on an undressed passage keeps its layout
    assert_eq!(passage.current_layout(), Some(&printed));
    assert!(passage.dress(&set, &DressSpec { ayah_mark: m, ..Default::default() }));
    passage.layout(&spec).unwrap();
    for refused in [
        DressSpec::default(),
        DressSpec { ayah_mark: set.ornaments.len() as u32, ..Default::default() },
        DressSpec { ayah_mark: 1, ..Default::default() },
    ] {
        let s = if refused.ayah_mark == 1 { scanned() } else { set.clone() };
        assert!(passage.dress(&set, &DressSpec { ayah_mark: m, ..Default::default() }));
        passage.layout(&spec).unwrap();
        assert!(!passage.dress(&s, &refused), "{refused:?}");
        // refused, the passage is undressed and its layout dropped
        assert!(passage.current_layout().is_none());
        assert_eq!(passage.layout(&spec).unwrap(), &printed);
        assert!(passage.dress_of().is_none());
    }
    // a page's whole spec works: the band and the border are ignored
    let whole = DressSpec { ayah_mark: m, surah_header: 7, page_frame: 9, gap: 3.0, ..Default::default() };
    assert!(passage.dress(&set, &whole));
}

#[test]
fn a_medallion_alone_and_a_passage_without_medallions() {
    let Some((set, m)) = released() else { return };
    let Some(page) = common::page(3) else { return };
    let dress = DressSpec { ayah_mark: m, ..Default::default() };
    // with no medallions, there is nothing to dress
    let omit = PassageSpec { width: 300.0, scale: 0.9, omit_ayah_marks: true, ..Default::default() };
    let mut plain = Passage::load(&[&page], 2, 6, 16).unwrap();
    let want = plain.layout(&omit).unwrap().clone();
    let mut passage = Passage::load(&[&page], 2, 6, 16).unwrap();
    assert!(passage.dress(&set, &dress));
    assert_eq!(passage.layout(&omit).unwrap(), &want);
    assert_eq!(passage.dress_of().unwrap().n_ayah_marks, 0);
    // the medallion alone wears the design, sized for the ayah's own layout
    let words = passage.layout(&omit).unwrap().clone();
    let mut alone = Passage::load_ayah_mark(&[&page], 2, 7).unwrap();
    let printed = alone.layout(&PassageSpec { scale: words.scale, ..Default::default() }).unwrap().clone();
    assert!(alone.dress(&set, &dress));
    for spec in [
        PassageSpec { scale: words.scale, ..Default::default() },
        PassageSpec { scale: words.scale, omit_ayah_marks: true, ..Default::default() },
    ] {
        let l = alone.layout(&spec).unwrap().clone();
        let d = alone.dress_of().unwrap().clone();
        assert_eq!(d.n_ayah_marks, 1);
        assert!(!d.table.is_empty());
        inside(&l, &d, "2:7 alone");
        rings_out_numerals_in(&l, &[&page], "2:7 alone");
        assert!(l.draws.len() < printed.draws.len(), "the ring leaves the draw list");
        assert!(!l.draws.is_empty(), "the numeral stays");
    }
    // the page's own outline for that medallion
    let mut p = common::page(3).unwrap();
    let all = page_outlines(&mut p, &set, &dress);
    let d = alone.dress_of().unwrap();
    for o in outlines(&d.ops, &d.pts, &d.table) {
        assert!(all.contains(&o), "the alone medallion is not one the page draws");
    }
}
