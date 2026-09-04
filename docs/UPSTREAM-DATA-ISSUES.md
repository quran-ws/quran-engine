# Upstream SVG data issues (to fix in the exporter, not in the engine)

Found while measuring `pages/*.svg` on 2026-09-04. The converter does **not**
work around any of these; it converts what it is given and prints a warning.

## 1. Pages 001 and 002: ayah markers are duplicated and mis-attributed

Every marker position carries two `ayah-marker` groups: one with the ornament
only, one with ornament + number. Ids are shifted, and later groups have no
`id` / `data-aid` at all. Page 002 has 10 ornaments for 5 ayahs. Normal pages
(e.g. 036) are clean: one group per ayah with ornament + number.

```
<g class="ayah-marker" id="mk-2-1" data-aid="2:1"><g transform="translate(231.135 317.344) …">[ornament]</g></g>
<g class="ayah-marker" id="mk-2-2" data-aid="2:2"><g transform="translate(231.135 317.344) …">[ornament]</g><g …>[number]</g></g>
…
<g class="ayah-marker"><g transform="translate(187.856 274.933) …">[ornament]</g><g …>[number]</g></g>   ← no id, no aid
```

Effect: the ornament is drawn twice (invisible, same ink), the marker for
ayah N is tagged as ayah N+1, and some markers cannot be linked to an ayah.
Expected: exactly one `ayah-marker` group per ayah, with `id="mk-S-A"` and
`data-aid="S:A"`, containing one ornament and one number.

## 2. Pages 001 and 002 use a different viewBox and root transform

```
001/002:  viewBox="-53.3109 -198.4777 345 550"  root g transform="matrix(1.3333 0 0 -1.3333 -136 482)"
others:   viewBox="0 0 345 550"                  root g transform="matrix(1.3333 0 0 -1.3333 -55|-115 640)"
```

Harmless for the converter (it flattens transforms), but every consumer of
the raw SVG has to special-case two pages. Expected: same viewBox everywhere.

## 3. Four `<path>` elements have no `data-kind`

They only carry `d` and `fill`. The converter files them as kind `other`.
Run `qvp-convert batch` and look for `warn: path without data-kind` to locate
them (page and parent group are printed).

## 4. 72 `<path>` elements carry an inline `transform`

```
<path data-kind="mark" data-mark="waqf-awla" transform="matrix(1 0 0 1 -0.376 6.71)" d="…"/>
```

The converter applies them, so no visual issue, but they suggest a manual
nudge step in the exporter that other tooling will miss. Expected: bake the
translation into `d`.

## 5. Coordinate noise

Baked transforms leave values such as `166.17999999999796`. Real precision is
three decimals. Rounding at export time would cut the SVG size by roughly a
third and make diffs readable.

## 6. Per-word text forms repeated on every word group

`data-uthmani`, `data-imlaei`, `data-qpc`, `data-rasm`, `data-search` are
carried on all 77,432 word groups. The engine keeps only `uthmani` inline and
writes the rest to a side file (`NNN.words.json`). Exporting the text index
once, separately from the geometry, would be cleaner.

## 7. No per-letter segmentation

Body paths are per connected stroke, not per letter. Tajweed colouring by
letter (e.g. colouring only the noon of an ikhfa) is not possible from this
data. If that is a product goal, the exporter needs to emit letter boundaries
(or at least glyph ids) per body path.

## 8. Ayah numbers are one merged outline per number

6,230 unique outlines out of 6,236 numbers; each contains 3–4 subpaths (the
digits) but no digit boundaries. Expected: one `<path>` per digit with a
glyph id (e.g. `data-glyph="digit-3"`), so the engine can store 10 digit
glyphs once and place instances. Saves ~2–3 % of the mushaf and lets apps
restyle or replace numerals.

## 9. Surah header frames and basmalah are baked per instance

`surah-name` paths are all unique even after position normalisation, so the
decorative frame and the calligraphic name are merged into the same outlines.
Basmalah repeats only partially (3,370 → 2,962 unique). Expected: emit the
frame and the basmalah as reusable glyphs with a transform (the way the ayah
ornament already is), and keep the surah name as its own paths.

## 10. Ayah ornament is already a shared glyph — keep it that way

One outline placed 6,248 times via `translate(...) scale(0.011 -0.011)`
(`0.0075` on pages 001–002). The engine stores it once per page as a glyph
instance. Please do not bake it into absolute coordinates in future exports.
