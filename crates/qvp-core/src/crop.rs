//! Crop a target to a box, and export it as a standalone SVG.
use crate::{Page, Rgba, Target, NONE};
use qvp_format::*;
use std::fmt::Write;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CropBounds {
    pub x0: f32,
    pub y0: f32,
    pub x1: f32,
    pub y1: f32,
    pub n_words: u32,
    /// ayah_mark deco kept (whole ayah in the crop), or NONE
    pub ayah_mark_deco: u32,
}

impl Page {
    /// Box around a target (page units), `pad` all round. A medallion is kept only when
    /// the whole ayah it closes is inside the target.
    pub fn crop_bounds(&self, target: &Target, pad: f32, keep_ayah_marks: bool) -> Option<CropBounds> {
        let words = self.resolve(target);
        if words.is_empty() {
            return None;
        }
        let q = self.quant();
        let d = self.data();
        let mut bb = IBox::EMPTY;
        for &wi in &words {
            bb.union(&d.words[wi as usize].bbox);
        }
        let mut ayah_mark = NONE;
        if keep_ayah_marks {
            let (s, a) = {
                let w = &d.words[*words.last().unwrap() as usize];
                (w.surah, w.ayah)
            };
            let all: Vec<u32> = self.resolve(&Target::Ayah(s, a));
            let (_, complete) = self.ayah_word_count(s, a);
            if complete && all.iter().all(|w| words.contains(w)) {
                if let Some(m) = self.marker_of(s, a) {
                    ayah_mark = m.deco;
                    bb.union(&d.decos[m.deco as usize].bbox);
                }
            }
        }
        Some(CropBounds {
            x0: bb.x0 as f32 / q - pad,
            y0: bb.y0 as f32 / q - pad,
            x1: bb.x1 as f32 / q + pad,
            y1: bb.y1 as f32 / q + pad,
            n_words: words.len() as u32,
            ayah_mark_deco: ayah_mark,
        })
    }

    /// Standalone SVG of a target with the current colours (mask/reveal/rules applied).
    /// `background` None = transparent.
    pub fn crop_svg(
        &mut self,
        target: &Target,
        pad: f32,
        keep_ayah_marks: bool,
        background: Option<Rgba>,
    ) -> Option<String> {
        let cb = self.crop_bounds(target, pad, keep_ayah_marks)?;
        let words = self.resolve(target);
        let colors: Vec<Rgba> = self.paint().to_vec();
        let d = self.data();
        let quant = d.header.quant;
        let (w, h) = (cb.x1 - cb.x0, cb.y1 - cb.y0);
        let mut s = String::with_capacity(64 * 1024);
        write!(s, "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"{:.2} {:.2} {:.2} {:.2}\" width=\"{:.2}\" height=\"{:.2}\">", cb.x0, cb.y0, w, h, w, h).unwrap();
        if let Some(bg) = background {
            write!(
                s,
                "<rect x=\"{:.2}\" y=\"{:.2}\" width=\"{:.2}\" height=\"{:.2}\" fill=\"{}\"/>",
                cb.x0,
                cb.y0,
                w,
                h,
                css_hex(bg)
            )
            .unwrap();
        }
        let mut path_ids: Vec<u32> = Vec::new();
        for &wi in &words {
            let wr = &d.words[wi as usize];
            path_ids.extend(wr.first_path..wr.first_path + wr.n_paths as u32);
        }
        if cb.ayah_mark_deco != NONE {
            let dc = &d.decos[cb.ayah_mark_deco as usize];
            path_ids.extend(dc.first_path..dc.first_path + dc.n_paths as u32);
        }
        for pi in path_ids {
            let c = colors[pi as usize];
            if c & 0xff == 0 {
                continue;
            }
            let cmds = d.path_cmds(pi as usize).ok()?;
            let p = &d.paths[pi as usize];
            write!(s, "<path d=\"{}\" fill=\"{}\"", svg_path_d(&cmds, quant), css_hex(c)).unwrap();
            if c & 0xff != 0xff {
                write!(s, " fill-opacity=\"{:.3}\"", (c & 0xff) as f32 / 255.0).unwrap();
            }
            if p.flags & PF_EVENODD != 0 {
                s.push_str(" fill-rule=\"evenodd\"");
            }
            s.push_str("/>");
        }
        s.push_str("</svg>");
        Some(s)
    }
}

pub fn css_hex(c: Rgba) -> String {
    format!("#{:02x}{:02x}{:02x}", (c >> 24) & 255, (c >> 16) & 255, (c >> 8) & 255)
}
