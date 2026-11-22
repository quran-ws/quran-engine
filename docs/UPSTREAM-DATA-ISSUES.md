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

## Fixed in quran-svg-elements v2.0.0

The words archive of v2.0.0 fixes ink that v1.1.2 filed with the wrong word. Upstream
records each item in `docs/defects/reported.json`.

- p232 `11:91:12` drew two letters short: a waqf sign was counted as a letter piece of the
  word before it.
- p540 `57:22:12`: one stroke of its tanwin al-kasr was filed with the word on the line below.
- p470 `40:27:9`, p567 `69:10:5`, p437 `35:29:14`: the tanwin was paired from strokes of
  two different letters.
- p526: three words beside a hizb rosette held ink of a neighbouring word.

In page data built from the words archive of v2.0.0, the paths of these words compare as
follows with data-v0.5.0. Each count is the number of words on the page whose set of path
`d` strings is different:

- p437: 2 words, `35:24:12` and `35:29:14`. Each has a `mark` path with a new outline or
  position.
- p470: 1 word, `40:27:9`.
- p540: 1 word, `57:22:21`, on the line below `57:22:12`. The paths of `57:22:12` do not
  change.
- p567: 1 word, `69:10:5`.
- p232 and p526: 0 words. The built page holds the same paths per word as before. Only the
  order of paths differs.

When the converter builds the pages from the words archive of v2.0.0, 575 of the 604 page
files differ from data-v0.5.0. On 470 of these pages, only the paint order changes: the
page holds the same elements in a different order. On the other 105 pages, some marks have
a new outline or position. The identity gate passes on all 604 pages.

### The `qpc` text changed

The words index of v2.0.0 takes its text from the quran-text release. Its `qpc` form is
different from v1.1.2 in 4159 words on 603 of the 604 pages, in page data built from
v2.0.0. The word keys and the other four text forms do not change. Each count below is the
number of places that have the edit:

- 2945: U+0622 ARABIC LETTER ALEF WITH MADDA ABOVE becomes U+0627 ARABIC LETTER ALEF and <!-- terminology: ignore (Unicode character names) -->
  U+0653 ARABIC MADDAH ABOVE.
- 397 and 393: the text gains a U+0651 ARABIC SHADDA in 397 places and loses one in 393. <!-- terminology: ignore (Unicode character names) -->
  397 words have such an edit. In 385 of them, the old and new text are equal
  without U+0651: the shaddah moves to a different place in the word.
- 277: U+064E ARABIC FATHA becomes U+0653 ARABIC MADDAH ABOVE. <!-- terminology: ignore (Unicode character names) -->
- 206: a U+0640 ARABIC TATWEEL now comes between
  U+06E1 ARABIC SMALL HIGH DOTLESS HEAD OF KHAH and U+0654 ARABIC HAMZA ABOVE. <!-- terminology: ignore (Unicode character names) -->
- 132: a U+0640 ARABIC TATWEEL now comes between U+0650 ARABIC KASRA and <!-- terminology: ignore (Unicode character names) -->
  U+0654 ARABIC HAMZA ABOVE. <!-- terminology: ignore (Unicode character names) -->

This is an upstream change to the text of record. The converter copies the text as it is.
An app that stores `qpc` strings or offsets into them from an older data release must not
mix them with the `qpc` text of a data release built from v2.0.0.
