//! Write the cross-wrapper dress scenarios: page 042 dressed in a small synthetic ornament
//! set, laid out at two viewports and two reflow zooms. For each case it records the layout's
//! fit, which holds the border, the border's left and right on screen at that fit, the dress
//! overflow, the dress clamp of a view pushed past each edge, and the dress view for a host
//! that scrolls the page: at scale 1 across the viewport's width and twice it, and at scale 0
//! (fit to the width) across the viewport's width. A reflowed case keeps the border inside
//! the viewport's width. Every wrapper's test suite replays them and compares.
//!
//! The set is written next to the scenarios as hex, so a wrapper loads the same bytes. It is
//! one filled and one stroked rectangle per kind, the shape of the ABI test's set: no
//! licensed artwork.
//!
//! Run: cargo run -p qvp-core --release --example dress_scenarios -- dist/pages/042.qvp \
//!      conformance/scenarios/dress.json conformance/qvo1-dress.hex
use qvp_core::qvp_format::ornaments::*;
use qvp_core::qvp_format::Cmd;
use qvp_core::*;
use std::sync::Arc;

/// Three ornaments, one per kind, each a filled and a stroked rectangle with a slot.
fn ornament_set() -> OrnamentSet {
    let mut set = OrnamentSet::default();
    let q = set.quant as i32;
    let unit = |w: i32, h: i32| {
        vec![Cmd::MoveTo(0, 0), Cmd::LineTo(w * q, 0), Cmd::LineTo(w * q, h * q), Cmd::LineTo(0, h * q), Cmd::Close]
    };
    let parts = vec![
        Part { name: "c1".into(), color: 0x00649bff, stroke: false },
        Part { name: "line".into(), color: 0x303641ff, stroke: true },
    ];
    let asset = |w: i32, h: i32, slot: [f32; 4], set: &mut OrnamentSet| {
        let first = set.paths.len() as u32;
        set.push_path(0, 0, 0, &unit(w, h));
        set.push_path(1, OPF_STROKE, q / 10, &unit(w, h));
        Asset { view_box: [0.0, 0.0, w as f32, h as f32], slot: Some(slot), first_path: first, n_paths: 2 }
    };
    let mark = asset(80, 100, [20.0, 20.0, 40.0, 60.0], &mut set);
    let header = asset(800, 100, [100.0, 10.0, 600.0, 80.0], &mut set);
    let frame = asset(70, 100, [5.0, 5.0, 60.0, 90.0], &mut set);
    let orn = |kind, asset, lineage| Ornament {
        kind,
        style: "test".into(),
        lineage,
        riwayah: "Hafs 'an 'Asim".into(),
        license: License {
            id: "CC0-1.0".into(),
            status: "confirmed".into(),
            redistributable: true,
            attribution: "a synthetic test set".into(),
        },
        parts: parts.clone(),
        asset,
        slices: None,
    };
    set.ornaments.push(orn(OrnamentKind::AyahMark, mark, Lineage::Font));
    set.ornaments.push(orn(OrnamentKind::SurahHeader, header, Lineage::Scan));
    set.ornaments.push(orn(OrnamentKind::PageFrame, frame, Lineage::Scan));
    set
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let page_path = args.get(1).map(String::as_str).unwrap_or("dist/pages/042.qvp");
    let out_path = args.get(2).map(String::as_str).unwrap_or("conformance/scenarios/dress.json");
    let hex_path = args.get(3).map(String::as_str).unwrap_or("conformance/qvo1-dress.hex");
    let bytes = std::fs::read(page_path).expect("read page");
    let mut page = Page::load(&bytes).expect("load page");
    let qvo = ornament_set().encode();
    let set = Arc::new(OrnamentSet::decode(&qvo).expect("decode the set it just wrote"));
    let hex: String = qvo.iter().map(|b| format!("{b:02x}")).collect();
    let hex = hex.as_bytes().chunks(64).map(|c| std::str::from_utf8(c).unwrap()).collect::<Vec<_>>().join("\n");

    let dress = DressSpec {
        ayah_mark: 0,
        surah_header: 1,
        page_frame: 2,
        gap: 5.0,
        band_gap: 5.0,
        ayah_mark_scale: 1.0,
        page_frame_scale: 1.0,
        line_art: 0,
        colors: vec![],
    };
    let cases_in: [(f32, f32, f32); 4] =
        [(390.0, 844.0, 0.0), (1024.0, 768.0, 0.0), (390.0, 844.0, 2.0), (390.0, 844.0, 3.0)];
    let mut cases = Vec::new();
    for (w, h, zoom) in cases_in {
        let spec = LayoutSpec {
            viewport_w: w,
            viewport_h: h,
            pad_top: 24.0,
            pad_bottom: 24.0,
            pad_left: 16.0,
            pad_right: 16.0,
            reflow: (zoom > 0.0).then(|| ReflowSpec { zoom, ..Default::default() }),
            ..Default::default()
        };
        page.undress();
        page.layout(&spec);
        page.dress(&set, &dress).expect("the fixture dresses page 042");
        let l = page.current_layout().unwrap().clone();
        let o = page.dress_overflow();
        // zoomed in, then pushed far past each edge in turn: left, top, right, bottom
        let pushes = [(1e4, 0.0), (0.0, 1e4), (-1e4, 0.0), (0.0, -1e4)];
        let clamps: Vec<String> = pushes
            .iter()
            .map(|&(dx, dy)| {
                let v = View { scale: 3.0, offset_x: dx, offset_y: dy };
                let c = page.dress_clamp(v, w, h);
                format!(
                    "{{\"view\": {{\"scale\": {}, \"offsetX\": {}, \"offsetY\": {}}}, \"held\": {{\"scale\": {}, \"offsetX\": {}, \"offsetY\": {}}}}}",
                    v.scale, v.offset_x, v.offset_y, c.scale, c.offset_x, c.offset_y
                )
            })
            .collect();
        // a host that scrolls the page: at the viewport's width and at twice it, so one case
        // may start at x = 0 and the other is centred; then at scale 0, fitted to the width
        let views: Vec<String> = [(1.0, w), (1.0, 2.0 * w), (0.0, w)]
            .iter()
            .map(|&(scale, vw)| {
                let (v, bw, bh) = page.dress_view(scale, vw);
                format!(
                    "{{\"scale\": {}, \"viewportW\": {}, \"view\": {{\"scale\": {}, \"offsetX\": {}, \"offsetY\": {}}}, \"width\": {}, \"height\": {}}}",
                    scale, vw, v.scale, v.offset_x, v.offset_y, bw, bh
                )
            })
            .collect();
        // the border's left and right on screen at the layout's fit
        let vb = page.view_box();
        let at = |x: f32| l.fit_x + l.fit_scale * (l.offset_x + x * l.scale);
        let border = (at(vb[0]), at(vb[0] + vb[2]));
        let reflow = if zoom > 0.0 { format!(", \"reflow\": {{\"zoom\": {zoom}}}") } else { String::new() };
        cases.push(format!(
            concat!(
                "    {{\"spec\": {{\"viewportW\": {}, \"viewportH\": {}, \"padTop\": 24, \"padBottom\": 24, \"padLeft\": 16, \"padRight\": 16{}}},\n",
                "     \"fit\": {{\"fitScale\": {}, \"fitX\": {}, \"fitY\": {}}},\n",
                "     \"border\": [{}, {}],\n",
                "     \"overflow\": [{}, {}, {}, {}],\n",
                "     \"clamps\": [\n       {}\n     ],\n",
                "     \"dressViews\": [\n       {}\n     ]}}"
            ),
            w, h, reflow, l.fit_scale, l.fit_x, l.fit_y, border.0, border.1, o.0, o.1, o.2, o.3, clamps.join(",\n       "),
            views.join(",\n       ")
        ));
    }
    let json = format!(
        concat!(
            "{{\n  \"page\": \"042.qvp\",\n  \"ornaments\": \"qvo1-dress.hex\",\n  \"tolerance\": 0.001,\n",
            "  \"dress\": {{\"ayahMark\": 0, \"surahHeader\": 1, \"pageFrame\": 2, \"gap\": 5, \"bandGap\": 5, \"ayahMarkScale\": 1, \"pageFrameScale\": 1}},\n",
            "  \"cases\": [\n{}\n  ]\n}}\n"
        ),
        cases.join(",\n")
    );
    std::fs::create_dir_all(std::path::Path::new(out_path).parent().unwrap()).expect("mkdir");
    std::fs::write(out_path, json).expect("write scenarios");
    std::fs::write(hex_path, hex + "\n").expect("write the set");
    println!("{out_path}: {} cases; {hex_path}: {} bytes", cases.len(), qvo.len());
}
