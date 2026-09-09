//! QVP → SVG, used by the identity test and as a debugging aid.
use qvp_format::*;
use std::fmt::Write;

const INK: &str = "#231f20";

pub fn path_d(cmds: &[Cmd], quant: u16) -> String {
    svg_path_d(cmds, quant)
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
        write!(s, "<g class=\"{}\" data-ayah-key=\"{}:{}\">", d.kind.as_str(), d.surah, d.ayah).unwrap();
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
                write!(s, "<g class=\"ayah-fragment\" data-ayah-key=\"{}:{}\" data-fragment=\"{}\" data-ayah-fragments=\"{}\">", a.surah, a.ayah, a.fragment, a.fragments).unwrap();
            }
            let text = if w.text == NONE_U16 { "" } else { &p.strings[w.text as usize] };
            write!(s, "<g class=\"word\" data-word-key=\"{}:{}:{}\" data-rasm-uthmani=\"{}\">", w.surah, w.ayah, w.word, text).unwrap();
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
