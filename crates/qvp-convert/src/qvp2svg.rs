//! QVP → SVG, used by the identity test and as a debugging aid.
use qvp_format::*;
use std::fmt::Write;

const INK: &str = "#231f20";

fn fmt_num(v: i32, quant: u16, out: &mut String) {
    // exact decimal for quantised value, trimmed
    let q = quant as i32;
    let neg = v < 0;
    let a = v.abs();
    let (ip, fp) = (a / q, a % q);
    if neg {
        out.push('-');
    }
    write!(out, "{ip}").unwrap();
    if fp != 0 {
        let digits = (quant as f64).log10().ceil() as usize;
        let s = format!("{fp:0digits$}");
        let s = s.trim_end_matches('0');
        out.push('.');
        out.push_str(s);
    }
}

pub fn path_d(cmds: &[Cmd], quant: u16) -> String {
    let mut d = String::new();
    let pt = |d: &mut String, x: i32, y: i32| {
        fmt_num(x, quant, d);
        d.push(' ');
        fmt_num(y, quant, d);
    };
    for c in cmds {
        match *c {
            Cmd::MoveTo(x, y) => {
                d.push('M');
                pt(&mut d, x, y);
            }
            Cmd::LineTo(x, y) => {
                d.push('L');
                pt(&mut d, x, y);
            }
            Cmd::QuadTo(x1, y1, x, y) => {
                d.push('Q');
                pt(&mut d, x1, y1);
                d.push(' ');
                pt(&mut d, x, y);
            }
            Cmd::CubicTo(x1, y1, x2, y2, x, y) => {
                d.push('C');
                pt(&mut d, x1, y1);
                d.push(' ');
                pt(&mut d, x2, y2);
                d.push(' ');
                pt(&mut d, x, y);
            }
            Cmd::Close => d.push('Z'),
        }
    }
    d
}

fn write_path(p: &PageData, i: usize, out: &mut String) -> Result<(), Error> {
    let r = &p.paths[i];
    let cmds = p.path_cmds(i)?;
    write!(out, "<path data-kind=\"{}\"", r.kind.as_str()).unwrap();
    if r.mark != Mark::None {
        write!(out, " data-mark=\"{}\"", r.mark.as_str()).unwrap();
    }
    if r.family != Family::None {
        write!(out, " data-mark-family=\"{}\"", r.family.as_str()).unwrap();
    }
    write!(out, " d=\"{}\" fill=\"{INK}\"", path_d(&cmds, p.header.quant)).unwrap();
    if r.flags & PF_EVENODD != 0 {
        out.push_str(" fill-rule=\"evenodd\"");
    }
    out.push_str("/>");
    Ok(())
}

pub fn to_svg(p: &PageData) -> Result<String, Error> {
    let mut s = String::with_capacity(p.ops.len() * 6);
    write!(
        s,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" data-page=\"{}\">",
        p.header.width, p.header.height, p.header.page
    )
    .unwrap();
    s.push_str("<g class=\"decorations\">");
    for d in &p.decos {
        write!(s, "<g class=\"{}\" data-aid=\"{}:{}\">", d.kind.as_str(), d.sura, d.ayah).unwrap();
        for i in d.first_path..d.first_path + d.n_paths as u32 {
            write_path(p, i as usize, &mut s)?;
        }
        s.push_str("</g>");
    }
    s.push_str("</g>");
    for (li, l) in p.lines.iter().enumerate() {
        write!(s, "<g class=\"line\" data-line=\"{}\">", l.line_no).unwrap();
        let mut cur_ayah = u16::MAX;
        for wi in l.first_word..l.first_word + l.n_words {
            let w = &p.words[wi as usize];
            debug_assert_eq!(w.line_idx as usize, li);
            if w.ayah_idx != cur_ayah {
                if cur_ayah != u16::MAX {
                    s.push_str("</g>");
                }
                cur_ayah = w.ayah_idx;
                let a = &p.ayahs[cur_ayah as usize];
                write!(s, "<g class=\"ayah\" data-aid=\"{}:{}\" data-part=\"{}\" data-ayah-parts=\"{}\">", a.sura, a.ayah, a.part, a.parts).unwrap();
            }
            let text = if w.text == NONE_U16 { "" } else { &p.strings[w.text as usize] };
            write!(s, "<g class=\"word\" data-wid=\"{}:{}:{}\" data-uthmani=\"{}\">", w.sura, w.ayah, w.word, text).unwrap();
            for i in w.first_path..w.first_path + w.n_paths as u32 {
                write_path(p, i as usize, &mut s)?;
            }
            s.push_str("</g>");
        }
        if cur_ayah != u16::MAX {
            s.push_str("</g>");
        }
        s.push_str("</g>");
    }
    s.push_str("</svg>");
    Ok(s)
}
