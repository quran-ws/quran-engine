> **Historical record.** This is the design document the engine was built from on
> 2026-09-04, kept as written. Sizes, function counts and the CI claim are those of the
> first session and no longer match the code. `docs/HOW-IT-WORKS.md` and `docs/API.md`
> describe the engine as it is.

# Quran Vector Engine (QVP) — Design

Date: 2026-09-04. Status: approved in chat, implemented in the same session.

## Goal

Ship the word-by-word Uthmani mushaf pages (604 SVGs, `pages/`) as an SDK for
iOS, Android, Flutter, React Native and Web that is:

- vector, and geometrically identical to the source SVG (proved by a CI pixel diff),
- small (whole mushaf ≈ 20 MB, one page ≈ 30–40 KB),
- interactive: every word, ayah, line, mark family or single path can be styled
  in real time and hit-tested from touch coordinates,
- one core, thin wrappers.

## Source data facts (measured)

- 604 pages, ~750 KB SVG each, ~150 words and ~1,160 paths on a full page,
  ~73 % of paths are marks (diacritics, dots, waqf, sifr, tanween…).
- Hierarchy: `page > line > ayah(part) > word > path(kind=body|mark)`, plus
  `ayah_markers` (ornament + number), `surah-name`, `basmalah`.
- All outlines are unique, coordinates baked absolute with float noise
  (3 significant decimals of real precision). No glyph reuse possible.
- Path commands: M m l h v c s q t z (no arcs). Fills: single ink colour,
  `fill-rule` evenodd on glyphs, nonzero on ornaments.

## Decisions

| Topic | Decision | Why |
|---|---|---|
| Language | Rust | Generated bindings (uniffi / flutter_rust_bridge / wasm), one codec crate shared by encoder, decoder and tests |
| Renderer | Core emits geometry + per-path paint; host canvas draws. Renderer is an interface so a bundled backend (ThorVG/Skia) can be added behind it | No 10 MB Skia per architecture, no double-Skia on Flutter/RN; decision reversible |
| Container | Hand-rolled versioned binary (`QVP1`), fixed-width tables + varint opcode stream | Zero deps, trivially zero-copy, schema versioned by header |
| Coordinates | Page space, origin at viewBox top-left, y down, quantised to 1/100 unit (page is 345×550 units) | 0.01 unit = 0.04 px at 4× raster; verified by pixel diff |
| Path encoding | Opcodes M/L/Q/C/Z, points as zigzag-LEB128 deltas from previous point (MoveTo delta from word origin) | Same idea as Skia/Rive path serialisation |
| Hit testing | Precomputed bboxes; line by y, word by x with ±1 neighbour check, then exact point-in-path on body paths | 150 words/page; an R-tree is over-engineering |
| Text forms | Uthmani text kept inline per word for tooltips; other forms (imlaei, qpc, rasm, search) in a side JSON | Keeps geometry file small |
| Repaint | Host caches unstyled page as a bitmap, redraws only styled paths per frame | Steady 60/120 fps |
| Identity gate | `svg → qvp → svg`, rasterise both with resvg at 4×, pixel-diff in CI | Turns "100 % identical" into a test |

## Components

- `crates/qvp-format` — types, enums (PathKind, Mark, Family), varint, encoder, decoder. No I/O.
- `crates/qvp-convert` — CLI. `svg2qvp` (flatten transforms, normalise paths,
  quantise, build tables), `qvp2svg` (round trip), `batch` (all pages + size report).
- `crates/qvp-core` — `Page` loader, `HitTester`, `StyleState`, display-list
  (`paint()`), `Renderer` trait, and the C ABI (`ffi.rs`) used by wasm and
  native wrappers alike.
- `crates/qvp-ffi` — `cdylib`/`staticlib` build of the C ABI (`include/qvp.h`).
- `web/` — reference wrapper: Canvas2D (host canvas) + Path2D cache, demo app.
- `tests/` — round-trip pixel diff (resvg), hit-test, style resolution.

## C ABI (the wrapper contract)

```
qvp_page_load(bytes,len) -> Page*      qvp_page_free(Page*)
qvp_page_info(Page*, PageInfo*)         width,height,counts
qvp_geometry(Page*, Geometry*)          pointers into decoded f32 point/u8 op arrays + per-path table
qvp_word_info(Page*, idx, WordInfo*)    sura/ayah/word/line/bbox/uthmani text
qvp_hit_test(Page*, x, y, Hit*) -> bool
qvp_style_word/ayah/line/mark/kind/path(Page*, …, rgba)   qvp_style_clear(Page*)
qvp_paint(Page*, u32* colors)           one colour per path (full repaint)
qvp_styled(Page*, u32* out) -> n        (path_idx, colour) pairs for overlay repaint
```

Style precedence: path > word > ayah > line > mark > kind > default.

## Roadmap executed

1. Codec + converter + round-trip pixel test.
2. Core (loader, hit-test, style, paint) + C ABI.
3. Web wrapper (wasm, Canvas2D host renderer) + product demo.
4. Not built here (no macOS / Android SDK on this machine): Swift/Kotlin/Dart
   wrappers. They consume `qvp.h` unchanged.
