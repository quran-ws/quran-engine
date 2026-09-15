//! The generator for the engine's shipped zoom steps (`qvp_core::zoom_table`).
//!
//! The rows of a reflowed page only rearrange at certain zooms, so the engine searches a band
//! around each step of a reader's zoom control for the zoom whose rows come out best
//! (`Page::zoom_level_candidates`). The search costs about 40 ms a page and its answer depends
//! only on the page, so it runs here, once, and the engine carries the answer.
//!
//! This also weighs each page against the page before it: two zooms that break a page equally
//! well are not equally good if one of them changes the size of the ink at the page turn. The
//! chain is chosen by a pass over the whole mushaf, so which way the reader turns does not
//! change what they get.
use qvp_core::{LayoutSpec, Page, ReflowSpec};
use std::path::Path;

/// What the table was built from and with. A change to any of it makes the table stale.
pub struct Manifest {
    pub schema: u32,
    pub layout_revision: u32,
    pub artwork: u64,
    pub pages: usize,
    pub nominals: Vec<f32>,
    pub band: f32,
    pub step: f32,
    pub smoothing: f32,
    pub relax_neighbours: usize,
    pub relax: f32,
    pub max_stretch: f32,
    pub word_gap: f32,
}

pub struct Table {
    pub manifest: Manifest,
    /// The zooms of each page, by page number, lowest first. Page 1 is `levels[0]`.
    pub levels: Vec<Vec<f32>>,
}

/// One page's candidates: for each step, every zoom the search weighed and what its rows cost.
type PageCandidates = Vec<Vec<(f32, f32)>>;

/// The version of this generator and of the table it writes.
pub const SCHEMA: u32 = 1;
/// How much a change in the size of the ink at a page turn costs, against a page's own rows.
pub const SMOOTHING: f32 = 8.0;
/// The viewport the search is run at. Its answer does not depend on this — a row is evened
/// against a fixed number of its neighbours — but a spec has to name one.
const VIEWPORT: (f32, f32) = (390.0, 800.0);

/// A content hash of the page data, so a table built from other artwork is caught.
fn hash(bytes: &[u8], seed: u64) -> u64 {
    let mut h = seed;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

fn spec() -> LayoutSpec {
    LayoutSpec {
        viewport_w: VIEWPORT.0,
        viewport_h: VIEWPORT.1,
        reflow: Some(ReflowSpec::default()),
        ..LayoutSpec::default()
    }
}

/// Build the table from a directory of `.qvp` pages.
pub fn generate(pages_dir: &Path) -> Result<Table, String> {
    let mut files: Vec<_> = std::fs::read_dir(pages_dir)
        .map_err(|e| format!("{}: {e}", pages_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "qvp"))
        .collect();
    files.sort();
    if files.is_empty() {
        return Err(format!("{}: no .qvp pages", pages_dir.display()));
    }
    let nominals = qvp_core::defaults::ZOOM_LEVEL_NOMINALS.to_vec();
    let spec = spec();
    // every zoom the search weighs for every page and step, and what its rows cost
    let mut artwork = 0xcbf2_9ce4_8422_2325u64;
    let mut pages: Vec<(u16, PageCandidates)> = Vec::with_capacity(files.len());
    for f in &files {
        let bytes = std::fs::read(f).map_err(|e| format!("{}: {e}", f.display()))?;
        artwork = hash(&bytes, artwork);
        let mut page = Page::load(&bytes).map_err(|e| format!("{}: {e:?}", f.display()))?;
        let mut floor = 0.0;
        let mut per_level = Vec::with_capacity(nominals.len());
        for &nominal in &nominals {
            let cand = page.zoom_level_candidates(&spec, nominal, 0.0, floor);
            let best = cand.iter().fold((f32::INFINITY, cand[0].0), |b, &(z, c)| if c < b.0 { (c, z) } else { b });
            floor = best.1 * 1.08;
            per_level.push(cand);
        }
        pages.push((page.page_number(), per_level));
    }
    pages.sort_by_key(|(n, _)| *n);

    // Each step is chosen for the whole mushaf at once: a page's own cost, plus what the ink
    // changing size at the page turn costs. Ties go to the lower zoom, so the pass is
    // deterministic whichever order the candidates arrive in.
    let n_levels = nominals.len();
    let mut chosen: Vec<Vec<f32>> = vec![Vec::with_capacity(n_levels); pages.len()];
    for level in 0..n_levels {
        let mut cost: Vec<f32> = pages[0].1[level].iter().map(|&(_, c)| c).collect();
        let mut back: Vec<Vec<usize>> = Vec::with_capacity(pages.len().saturating_sub(1));
        for i in 1..pages.len() {
            let (prev, cur) = (&pages[i - 1].1[level], &pages[i].1[level]);
            let (mut next, mut from) = (Vec::with_capacity(cur.len()), Vec::with_capacity(cur.len()));
            for &(z, c) in cur {
                let mut best = (f32::INFINITY, 0usize);
                for (k, &(pz, _)) in prev.iter().enumerate() {
                    let d = (z / pz).ln();
                    let v = cost[k] + SMOOTHING * d * d;
                    if v < best.0 {
                        best = (v, k);
                    }
                }
                next.push(best.0 + c);
                from.push(best.1);
            }
            back.push(from);
            cost = next;
        }
        let mut k =
            cost.iter().enumerate().fold((f32::INFINITY, 0usize), |b, (i, &v)| if v < b.0 { (v, i) } else { b }).1;
        for i in (0..pages.len()).rev() {
            chosen[i].push(pages[i].1[level][k].0);
            if i > 0 {
                k = back[i - 1][k];
            }
        }
    }
    let r = ReflowSpec::default();
    Ok(Table {
        manifest: Manifest {
            schema: SCHEMA,
            layout_revision: qvp_core::defaults::LAYOUT_REVISION,
            artwork,
            pages: pages.len(),
            nominals,
            band: qvp_core::defaults::ZOOM_LEVEL_BAND,
            step: qvp_core::defaults::ZOOM_LEVEL_STEP,
            smoothing: SMOOTHING,
            relax_neighbours: qvp_core::defaults::RELAX_NEIGHBOURS,
            relax: r.relax,
            max_stretch: r.max_stretch,
            word_gap: r.word_gap,
        },
        levels: chosen,
    })
}

/// Every page's steps rise, and none asks for more zoom than its own words allow.
pub fn validate(pages_dir: &Path, table: &Table) -> Result<(), String> {
    let spec = spec();
    let mut files: Vec<_> = std::fs::read_dir(pages_dir)
        .map_err(|e| format!("{}: {e}", pages_dir.display()))?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x == "qvp"))
        .collect();
    files.sort();
    for f in &files {
        let bytes = std::fs::read(f).map_err(|e| format!("{}: {e}", f.display()))?;
        let page = Page::load(&bytes).map_err(|e| format!("{}: {e:?}", f.display()))?;
        let n = page.page_number() as usize;
        let row = table.levels.get(n - 1).ok_or_else(|| format!("page {n} is not in the table"))?;
        let cap = page.reflow_max_zoom(&spec.reflow.unwrap_or_default());
        for (i, &z) in row.iter().enumerate() {
            if z <= 1.0 {
                return Err(format!("page {n} step {} is {z}, which is not above the printed page", i + 2));
            }
            if z > cap + 1e-4 {
                return Err(format!("page {n} step {} is {z}, past its own limit of {cap}", i + 2));
            }
            if i > 0 && z <= row[i - 1] {
                return Err(format!("page {n} step {} is {z}, not above step {}", i + 2, i + 1));
            }
        }
    }
    Ok(())
}

/// The table as the Rust source the engine carries.
pub fn render(t: &Table) -> String {
    let mut s = String::new();
    s.push_str("//! The zoom each step of a reader's zoom control lands on, one row a page.\n//!\n");
    s.push_str("//! Generated by `qvp-convert zoom-levels`; `scripts/check.sh gates` rebuilds it and fails\n");
    s.push_str("//! if it has gone stale. Do not edit by hand. The steps are chosen for the engine's own\n");
    s.push_str("//! breaking and spacing: a reader who changes those keeps these steps and gets a page\n");
    s.push_str("//! laid out with the settings they asked for, so a spacing knob never resizes the text.\n");
    let m = &t.manifest;
    s.push_str("\n#![allow(dead_code)] // the manifest below is the record of how the table was built\n");
    s.push_str(&format!(
        "\n/// The generator and layout this table was built with.\npub(crate) const SCHEMA: u32 = {};\n/// The revision of the layout the steps were searched against.\npub(crate) const LAYOUT_REVISION: u32 = {};\n",
        m.schema, m.layout_revision
    ));
    s.push_str(&format!(
        "/// A content hash of the page data it was built from.\npub(crate) const ARTWORK: u64 = 0x{:016x};\n",
        m.artwork
    ));
    s.push_str(&format!(
        "/// The zoom each step aims at, before the search.\npub(crate) const NOMINALS: [f32; {}] = [{}];\n",
        m.nominals.len(),
        m.nominals.iter().map(|v| format!("{v:?}")).collect::<Vec<_>>().join(", ")
    ));
    s.push_str(&format!("/// How far either side of a nominal the search went, and in what steps.\npub(crate) const BAND: f32 = {:?};\npub(crate) const STEP: f32 = {:?};\n", m.band, m.step));
    s.push_str(&format!("/// What a change in the size of the ink at a page turn cost against a page's own rows.\npub(crate) const SMOOTHING: f32 = {:?};\n", m.smoothing));
    s.push_str(&format!("/// The spacing the rows were measured with: rows compared either side, how far a short\n/// row was opened, the cap on a gap, and the word gap.\npub(crate) const RELAX_NEIGHBOURS: usize = {};\npub(crate) const RELAX: f32 = {:?};\npub(crate) const MAX_STRETCH: f32 = {:?};\npub(crate) const WORD_GAP: f32 = {:?};\n", m.relax_neighbours, m.relax, m.max_stretch, m.word_gap));
    s.push_str(&format!("\n/// The steps of each page, by page number: page 1 is the first row. The printed page, zoom\n/// 1, is the step every control starts from and is not in here.\npub(crate) const LEVELS: [[f32; {}]; {}] = [\n", m.nominals.len(), t.levels.len()));
    for row in &t.levels {
        s.push_str(&format!("    [{}],\n", row.iter().map(|v| format!("{v:?}")).collect::<Vec<_>>().join(", ")));
    }
    s.push_str("];\n");
    s.push_str(
        r#"
/// The steps of one page, by page number, or `None` for a page this table does not cover.
pub(crate) fn steps(page: u16) -> Option<&'static [f32; NOMINALS.len()]> {
    LEVELS.get((page as usize).checked_sub(1)?)
}
/// The layout the steps were chosen for. A change to any of it makes the table stale.
///
/// This is a test, not a compile-time assertion: the generator that rewrites this file is built
/// against this crate, so a table that refused to compile would take the tool that fixes it
/// down with it.
#[cfg(test)]
mod manifest {
    #[test]
    fn the_table_was_built_with_the_layout_the_engine_has() {
        assert_eq!(super::LAYOUT_REVISION, crate::defaults::LAYOUT_REVISION);
        assert_eq!(super::BAND, crate::defaults::ZOOM_LEVEL_BAND);
        assert_eq!(super::STEP, crate::defaults::ZOOM_LEVEL_STEP);
        assert_eq!(super::RELAX_NEIGHBOURS, crate::defaults::RELAX_NEIGHBOURS);
        assert_eq!(super::RELAX, crate::defaults::REFLOW_RELAX);
        assert_eq!(super::MAX_STRETCH, crate::defaults::REFLOW_MAX_STRETCH);
        assert_eq!(super::WORD_GAP, crate::reflow::ReflowSpec::default().word_gap);
        assert_eq!(super::NOMINALS, crate::defaults::ZOOM_LEVEL_NOMINALS);
    }
}
"#,
    );
    s
}
