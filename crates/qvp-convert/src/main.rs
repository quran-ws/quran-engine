use qvp_convert::*;
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

fn usage() -> ! {
    eprintln!(
        "usage:\n  qvp-convert svg2qvp <in.svg> <out.qvp> [out.words.json]\n  qvp-convert qvp2svg <in.qvp> <out.svg>\n  qvp-convert info <in.qvp>\n  qvp-convert batch <svg_dir> <out_dir>"
    );
    std::process::exit(2)
}

struct Done {
    svg_len: usize,
    qvp_len: usize,
    warnings: Vec<String>,
    page: qvp_format::PageData,
}

fn one(svg_path: &Path, out: &Path, json: Option<&Path>) -> Result<Done, String> {
    let svg = fs::read_to_string(svg_path).map_err(|e| format!("{}: {e}", svg_path.display()))?;
    let c = convert(&svg).map_err(|e| format!("{}: {e}", svg_path.display()))?;
    let bytes = qvp_format::encode(&c.page);
    fs::write(out, &bytes).map_err(|e| e.to_string())?;
    if let Some(j) = json {
        fs::write(j, words_json(&c.words_text)).map_err(|e| e.to_string())?;
    }
    Ok(Done { svg_len: svg.len(), qvp_len: bytes.len(), warnings: c.report.warnings, page: c.page })
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        usage();
    }
    match args[1].as_str() {
        "svg2qvp" if args.len() >= 4 => {
            let json = args.get(4).map(PathBuf::from);
            match one(Path::new(&args[2]), Path::new(&args[3]), json.as_deref()) {
                Ok(d) => {
                    for x in d.warnings {
                        eprintln!("warn: {x}");
                    }
                    println!("{} → {} bytes ({:.1}x smaller)", d.svg_len, d.qvp_len, d.svg_len as f64 / d.qvp_len as f64);
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    std::process::exit(1)
                }
            }
        }
        "qvp2svg" if args.len() >= 4 => {
            let bytes = fs::read(&args[2]).expect("read qvp");
            let page = qvp_format::decode(&bytes).expect("decode");
            fs::write(&args[3], to_svg(&page).expect("to_svg")).expect("write svg");
        }
        "info" => {
            let bytes = fs::read(&args[2]).expect("read qvp");
            let p = qvp_format::decode(&bytes).expect("decode");
            println!(
                "page {} {}x{} quant=1/{} lines={} ayahs={} words={} paths={} decos={} glyphs={} insts={} ops={}B strings={} total={}B",
                p.header.page, p.header.width, p.header.height, p.header.quant,
                p.lines.len(), p.ayahs.len(), p.words.len(), p.paths.len(), p.decos.len(), p.glyphs.len(), p.insts.len(), p.ops.len(), p.strings.len(), bytes.len()
            );
        }
        "batch" if args.len() >= 4 => {
            let out_dir = PathBuf::from(&args[3]);
            fs::create_dir_all(&out_dir).expect("mkdir");
            let mut files: Vec<PathBuf> = fs::read_dir(&args[2])
                .expect("read dir")
                .filter_map(|e| e.ok().map(|e| e.path()))
                .filter(|p| p.extension().map(|x| x == "svg").unwrap_or(false))
                .collect();
            files.sort();
            let results: Vec<_> = files
                .par_iter()
                .map(|f| {
                    let stem = f.file_stem().unwrap().to_string_lossy().to_string();
                    let out = out_dir.join(format!("{stem}.qvp"));
                    let json = out_dir.join(format!("{stem}.words.json"));
                    (stem, one(f, &out, Some(&json)))
                })
                .collect();
            let (mut svg_total, mut qvp_total, mut n, mut errs, mut warns) = (0usize, 0usize, 0usize, 0usize, 0usize);
            let (mut min, mut max) = (usize::MAX, 0usize);
            let mut atlas = qvp_convert::atlas::Builder::default();
            for (stem, r) in results {
                match r {
                    Ok(d) => {
                        n += 1;
                        svg_total += d.svg_len;
                        qvp_total += d.qvp_len;
                        min = min.min(d.qvp_len);
                        max = max.max(d.qvp_len);
                        for x in d.warnings {
                            warns += 1;
                            eprintln!("warn: page {stem}: {x}");
                        }
                        atlas.add_page(&d.page);
                    }
                    Err(e) => {
                        errs += 1;
                        eprintln!("error: {e}");
                    }
                }
            }
            let atlas = atlas.build();
            fs::write(out_dir.join("atlas.qva"), atlas.encode()).expect("write atlas");
            fs::write(out_dir.join("atlas.json"), atlas.to_json()).expect("write atlas json");
            println!("atlas: {} pages, {} surahs, {} rub boundaries → atlas.qva / atlas.json", atlas.pages.len(), atlas.surahs.len(), atlas.rubs.len());
            println!(
                "pages={n} errors={errs} warnings={warns}\nsvg total {:.1} MB → qvp total {:.2} MB ({:.1}x)\nper page: min {} B, avg {} B, max {} B",
                svg_total as f64 / 1e6,
                qvp_total as f64 / 1e6,
                svg_total as f64 / qvp_total.max(1) as f64,
                min,
                qvp_total / n.max(1),
                max
            );
        }
        _ => usage(),
    }
}
