//! Loading a page: which line each path belongs to, and which ink gets an outline.
use super::*;

/// A tall sajdah sign on line 1 must not pull the line's centre. An ayah mark stored
/// without a line takes the line of the ayah it closes, not the nearest centre. The
/// sajdah line takes the line of the word under it, not the sign's line.
#[test]
fn decorations_follow_their_line() {
    let mut d = page_data();
    let mut add = |cmds: &[Cmd], kind: PathKind| add_path(&mut d, cmds, kind);
    // the sign spans from above line 1 to the middle of line 2
    let (sajdah, sb) = add(&square(200, 200, 400, 5500), PathKind::Other);
    // the sajdah line sits over word C on line 2, though its sign is on line 1
    let (sajdah_line, ob) = add(&square(1500, 4900, 3500, 4950), PathKind::Mark);
    // the mark sits low: its centre (y=40) is nearer line 2 (55) than line 1 (15)
    let (mark, mb) = add(&square(4200, 3800, 4600, 4200), PathKind::AyahMarkOrnament);
    d.paths[sajdah_line as usize].mark = Mark::SajdahLine;
    let mut sb2 = sb;
    sb2.union(&ob);
    d.decorations = vec![
        DecoRec {
            kind: DecoKind::SajdahMark,
            surah: 1,
            ayah: 1,
            text: NONE_U16,
            first_path: sajdah,
            n_paths: 2,
            line: 0,
            bbox: sb2,
        },
        DecoRec {
            kind: DecoKind::AyahMark,
            surah: 1,
            ayah: 1,
            text: NONE_U16,
            first_path: mark,
            n_paths: 1,
            line: NONE_U16,
            bbox: mb,
        },
    ];
    d.ayahs[0].ayah_mark_decoration = 1;
    let mut p = Page::load(&encode(&d)).unwrap();
    assert_eq!(p.line_centre(0), 15.0);
    assert_eq!(p.line_centre(1), 55.0);
    assert_eq!(p.geometry().table[sajdah as usize].line, 0);
    assert_eq!(p.geometry().table[sajdah_line as usize].line, 1);
    assert_eq!(p.geometry().table[mark as usize].line, 0);
    let l = p
        .layout(&LayoutSpec {
            viewport_w: 100.0,
            viewport_h: 400.0,
            fill_height: true,
            grid_lines: 2,
            ..Default::default()
        })
        .clone();
    assert!(l.line_dy[1] > l.line_dy[0]);
}

/// A page's viewBox is what the page is, and ink outside it gets no outline to draw.
///
/// p17 of the Hafs KFGQPC mushaf is the only page of 604 whose artwork draws anything outside
/// the box: a page number below it and a running head above it, four paths in all. The record
/// and the box stay — a crop still has them — but there is nothing for a host to stroke.
///
/// Needs dist/pages from scripts/sync-test-data.sh, and is skipped without it unless
/// QVP_REQUIRE_DATA is set.
#[test]
fn ink_outside_the_page_box_gets_no_outline() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../dist/pages/017.qvp");
    if !path.exists() {
        assert!(
            std::env::var("QVP_REQUIRE_DATA").is_err(),
            "dist/pages is missing and QVP_REQUIRE_DATA is set; run scripts/sync-test-data.sh"
        );
        return;
    }
    let p = Page::load(&std::fs::read(&path).unwrap()).unwrap();
    let (w, h, q) = (p.width(), p.height(), p.quant());
    let mut off_page = 0;
    for (i, rec) in p.data.paths.iter().enumerate() {
        let bb = rec.bbox;
        let (x0, y0) = (bb.x0 as f32 / q, bb.y0 as f32 / q);
        let (x1, y1) = (bb.x1 as f32 / q, bb.y1 as f32 / q);
        let g = &p.geom.table[i];
        if x1 <= 0.0 || y1 <= 0.0 || x0 >= w || y0 >= h {
            off_page += 1;
            assert_eq!(g.op_count, 0, "path {i} is outside the {w}x{h} box and still has an outline");
            assert_eq!(g.pt_count, 0);
        } else {
            assert!(g.op_count > 0, "path {i} is on the page and lost its outline");
        }
    }
    assert_eq!(off_page, 4, "p17 carries its page number and its running head outside the box");
    // the paths are still there to be counted, cropped and exported
    assert_eq!(p.geom.table.len(), p.data.paths.len());
}
