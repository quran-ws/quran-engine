# Upstream SVG data issues (to fix in the pipeline, not in the engine)

Measured against the `quran-svg hafs-kfgqpc v1.0.0` bundle (production profile,
schema `quran-svg/version` 1.0.0, artwork `b91d39e1`, pipeline `6588ce9`). The
converter does **not** work around any of these; it converts what it is given
and prints a warning.

`cargo run -p qvp-convert --release -- batch pages dist/pages` on this bundle
reports **0 errors and 0 warnings** over all 604 pages.

## Fixed in v1.0.0

Everything the 2026-09-04 revision of this file listed as items 1–6 is fixed,
and each fix is verified here rather than taken on trust:

| was | now |
|---|---|
| pages 1–2 drew every ayah ornament twice, with shifted ids and groups carrying no `id` / `data-aid` | 6,236 `g.ayah-mark` groups, every one with `id="mk-S-A"` and `data-ayah-key`; the 12 doubled ornaments sit inside their own ayah's group as `data-duplicate="1"` and are kept deliberately (two fills composite differently at the edge) |
| pages 1–2 used `viewBox="-53.3109 -198.4777 345 550"` | every page is `viewBox="0 0 345 550"`; the offset is folded into the page matrix |
| four `<path>` elements had no `data-kind` | every ink path carries `data-kind` |
| 72 `<path>` elements carried an inline `transform` | no `<path>` carries a transform; the translation is baked into the absolute movetos |
| coordinate noise such as `166.17999999999796` | absolute movetos are written to at most three decimals |
| all five text forms repeated on all 77,432 word groups | the production profile carries `data-word-key` and `data-rasm-uthmani` only; the four derived forms ship once in `index/by-page/NNN.json` |

## Still open

### 1. No per-letter segmentation

Body paths are per connected stroke, not per letter. Tajwid colouring by letter
(e.g. colouring only the noon of an ikhfa) is not possible from this data. The
dev profile's `<g class="ligature">` is a rendering run, not a spelling — the
bundle's own `schema/FORMAT.md` §10.3 says so — and the production profile drops
it. If letter colouring is a product goal, the pipeline needs to emit letter
boundaries (or at least glyph ids) per body path.

### 2. Ayah numbers are one merged outline per number

Sampled 87 pages: 891 `data-kind="ayah_number"` paths, **891 distinct outlines**,
each holding 1–5 subpaths (the digits) with no digit boundaries. Expected: one
`<path>` per digit with a glyph id (e.g. `data-glyph="digit-3"`), so the engine
can store ten digit glyphs once and place instances. Saves ~2–3 % of the mushaf
and lets apps recolorStyle or replace the numerals.

### 3. Surah header and basmalah ink is baked per instance

Sampled 87 pages: 33 `data-kind="header_ink"` paths, **33 distinct outlines**.
v1.0.0 merged each banner's ink into a single compound path (226 paths for 226
banners, down from 5,670), which is smaller but moves further from reuse: the
decorative frame, the calligraphic surah name and the basmalah are all one
outline. Expected: emit the frame and the basmalah as reusable glyphs placed by
a transform — the way the ayah ornament already is — and keep the surah name as
its own paths.

### 4. Ayah ornament is a shared glyph — please keep it that way

Sampled 87 pages: 898 `data-kind="ayah_mark_ornament"` paths from **one**
outline, placed by `translate(…) scale(0.011 -0.011)` (`0.0075` on pages 1–2).
The engine stores it once per page as a glyph instance. Do not bake it into
absolute coordinates in future exports.

## Not a defect

Two things that look wrong and are not, both documented in the bundle's
`schema/FORMAT.md`, recorded here so nobody "fixes" them in the converter:

- **`data-rasm-uthmani` disagrees with `data-rasm-imlai` at all 609 iqlab sites**
  (§9.7) and around the open tanwin U+08F0–U+08F2 (§9.8). That is the print's
  orthography, not corruption.
- **A mark can be drawn outside its own word** (§9.9) — a word-final tanwin
  floats into the gap toward the next word. Word bounding boxes therefore
  overlap. Ownership is the enclosing `g.word`, never proximity.
