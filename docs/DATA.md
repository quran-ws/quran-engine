# The page data

The engine ships code only. Page data is published as a release of this repository and
mirrored to a CDN. This page says which mushaf it is, where it comes from, how it is built,
and under what terms an app may use it.

## What is published

| file | what |
|---|---|
| `NNN.qvp` | one printed page in the QVP1 format (`docs/FORMAT.md`): outlines, words, ayah fragments, lines, decorations |
| `NNN.words.json` | that page's five text forms per word key: `rasm_uthmani`, `rasm_imlai`, `qpc`, `rasm`, `search`. Page data built from the v2.0.0 words archive of `quran-svg-elements` also carries `w`, the word's number in the quran-text release. `w` is not a key, because its numbering has gaps. |
| `atlas.qva` | the cross-page index in the QVA1 format: pages, surahs, the 240 `rubu_al_hizb` boundaries |
| `atlas.json` | the same atlas as JSON, for tooling |
| `surah-names/qvp/NNN.qvp` | one compact, title-only engine asset for surah `NNN` |
| `surah-names/svg/NNN.svg` | the same title as a standalone SVG |
| `surah-names/qvp/all.qvp`, `surah-names/svg/all.svg` | all 114 titles on one frame-free sprite sheet |
| `surah-names/surah-names.woff2` | all titles as OpenType/CFF glyphs at private-use code points |
| `surah-names/surah-names.css`, `surah-names/map.json` | browser classes and the code-point and metadata map |
| `VERSION.json` | provenance: data version, source release, engine commit, format version, page and surah-name counts, digests of every generated data file, and, when a set was packaged, an `ornaments` record (source, source commit, redistributable only) |
| `ornaments.qvo` | the ornament set in the QVO1 format: every asset of `quran-ws/quran-assets` whose licence is confirmed, today the 40 font-derived ayah markers |

The data covers 604 pages, 77,432 words, 6,236 ayahs and 114 surahs. The individual QVPs
add about 0.35 MB, the WOFF2 font about 0.13 MB, and the optional SVG and combined forms
bring the complete surah-name set to about 4.4 MB before transport compression. Each release
manifest records exact sizes and digests.

## Where it comes from

The mushaf is the **King Fahd Glorious Qur'an Printing Complex (KFGQPC) Madani mushaf,
version 4, 1441 AH**, in the Hafs reading, as vector artwork. The immediate source is the
`quran-svg-elements` release (`https://github.com/quran-ws/quran-svg-elements`,
`hafs-kfgqpc` bundle): one SVG per page with every word tagged by its key, plus an index of
the text forms. From v2.0.0 that release holds two archives. The page data is built from
the words archive. `docs/UPSTREAM-DATA-ISSUES.md` lists what was found wrong in that source
and fixed there; the converter never patches data.

## The ornament set

`ornaments.qvo` is built by `qvp-convert ornaments <quran-assets checkout> ornaments.qvo
--redistributable` from a pinned commit of `quran-ws/quran-assets`, recorded in
`VERSION.json` under `ornaments.source_commit`. Only assets whose catalogue licence status
is `confirmed` go in. `scripts/package-data.sh` refuses a set that holds any ornament
whose licence is not confirmed. The scanned ornaments of other printed mushafs are traced from
designs that belong to their publishers and stay at CC BY-NC-SA 4.0, provisional, until
each publisher settles permission. They are not in any release. Build a local set without
`--redistributable` to use them, and read each ornament's licence, which the engine
reports, before you publish a page wearing it.

The format has no arc, so the converter writes each arc, circle, ellipse and rounded
corner in an ornament as cubic curves. The cubics follow the source curve to within 0.1
asset units, too close to see, but they are an approximation, not the source curve. In the
released set this applies to four ayah markers, `font-012` in the extralight, light,
regular-black and thin weights.

## The letter layer

`NNN.qvl` is the letter layer of page `NNN`, in the QVL1 format (`docs/FORMAT.md`). It is
optional: a page file is complete without it. For each word of the page it gives the
letters, the page paths that draw each letter, and the marks of each letter. Some page
paths join several letters in one outline. For those, the layer holds pieces: parts of the
outline that the converter cut out, one for each letter. The page file does not change.

The engine loads a layer on request: an app attaches it to a loaded page
(`qvp_attach_letters`, `docs/API.md` "Letters"), and the page then colours, reads and
hit-tests single letters. A page with no layer works as before.

A layer is built for one exact page file. It codes each piece against the page path it was
cut from, and its header holds a digest of what it depends on in that page: the word runs
and each path's kind, mark, family, flags and geometry. The engine refuses a layer whose page
digest differs from the digest of its page.

The layers built from `quran-svg-elements` v2.0.0 match the pages built from v2.0.0 on all
604 pages. They match the released `data-v0.5.0` pages, built from v1.1.2, on 29 pages only.
So a data release that carries the layers must carry pages built from the same source
release, and the layers are built again whenever the page data is built again.

A data release carries the layers only when its `VERSION.json` lists them. `data-v0.5.0`
does not. Until a release carries them, build the layers and the pages from the same source:

```sh
QVP_SYNC=letters scripts/sync-test-data.sh   # what the default sync fetches, then pages-letters/
cargo run -p qvp-convert --release -- batch pages dist/pages
cargo run -p qvp-convert --release -- letters pages-letters pages dist/letters
```

The source is the letters archive of the same `quran-svg-elements` release, which holds
the same pages as the words archive with every word divided into letters. The page ink
still comes from the words archive. The layer only says which part of that ink draws
which letter.

Three gates check the layers. `scripts/check.sh gates` runs them on 8 pages when
`pages-letters/` is present, and the nightly workflow runs them on all 604 pages. All 604
pages pass each gate. `docs/MEASURED.md` gives their figures.

The structure gate (`crates/qvp-convert/tests/letters.rs`) checks that every path of every
word has exactly one role and that the letter count is the count of the archive's index.
It also decodes every layer and compares each command of each piece with the converter's.

The ink gate (the same file) draws the page from the pieces and compares it with the page.
It asserts these things:

- The page drawn from its pieces passes the pixel threshold of the identity gate.
- For each cut path, the area of its pieces is within 0.0125 units × L of the area of the
  path. 0.0125 units is the edge limit that `quran-svg-elements` states. L is the length
  of the edges that a cut made again, near the outline of the path. One path is a known
  exception, with its exact difference (page 001, word 1:2:1).
- Every side of a cut has a partner: a side of another piece of the same path runs along
  it within 0.028 units.
- Where two pieces meet along a cut, they draw the cut in opposite directions.
- A subpath inside another winds the other way, so that the areas count ink.

It reports, and does not assert, the page drawn with the pieces of each path as one path,
and the result of losing, doubling or replacing each piece. It cannot see these:

- A lost piece whose cut sides lie within 0.028 units of the outline of its path (a sliver
  thinner than about 0.056 units), or whose neighbours' sides each find a partner in a
  third piece. Of all single lost pieces, 25 pass, up to 0.034 square units. No doubled or
  replaced piece passes.
- A cut side whose partner is an outline edge. The partner rule takes any side of another
  piece, so near the outline the end of a cut side can find its partner in a neighbour's
  outline edge.
- A subpath whose nesting it cannot decide. It counts 954 such subpaths and asserts nothing
  about them.
- A gap or an overlap between the pieces of two different paths.
- Where ink moved inside one path. The pixel threshold sees this at page level only.
- Whether a cut falls where a calligrapher would put it, and whether a reader can see a
  seam.

The engine gate (`crates/qvp-convert/tests/letters_engine.rs`) attaches each layer to its
page in the engine and asserts what an app relies on:

- The attach changes no drawing. With no letter rule, the draw list, the colours and every
  geometry row of the page's own paths are those of the page with no layer.
- Every letter's offset points at its own character in the `qpc` text, and the offsets that
  the engine keeps are the ones the layer stores.
- The selectors take the letter's paths. For each letter, the body and dots part takes the
  same paths as the character range over the letter's offset, and the nth mark takes the
  nth mark in text order.
- A colour on one letter draws each cut path with a piece of another letter as its pieces,
  in place, and the plain draw list comes back when the rule goes.
- The letter hit test names the right letter: at a point inside each body member, on every
  point of ink on a 0.5-unit grid, printed and reflowed, and inside each counter, the hole of
  a letter's body.

The counter check does not see a counter that a cut divides between two pieces, because
no single member holds it: of the 88,914 counters of cut paths on the 604
pages, 883 are divided, 863 of them the loop of a kaf joined to the alif or lam after it,
and a point in such a loop names the nearer of the two letters (the alif or lam for 192,775
of 555,399 points sampled). For 412 points of word 76:20:6 the answer is another word: the
word that `qvp_hit_test` gives there, with or without a layer.

Quran.ws did not measure where a joined outline divides between two letters. That
division is the output of DigitalKhatt's `medinamushafanalyzer`
(<https://github.com/DigitalKhatt/medinamushafanalyzer>) at commit `056191f`, the work of
DigitalKhatt (Amine Anane), published under the MIT licence, Copyright (c) 2023-2026
Digitalkhatt. `quran-svg-elements` uses it with the author's agreement. Its `NOTICE.md`
asks whoever republishes the letters archive to carry this line next to the Quran.ws
attribution:

```
Letter division by DigitalKhatt medinamushafanalyzer (commit 056191f),
MIT licence. https://github.com/DigitalKhatt/medinamushafanalyzer
```

`quran-svg-elements` states these limits of the division in section 10.10 of its
`schema/FORMAT.md`:

- It quotes no accuracy figure, because it measured none. Where a cut falls inside a
  connecting stroke is the analyzer's convention, not a fact of the print.
- The edge of a cut letter can differ from the artwork by less than 0.0125 units. `quran-svg-elements`
  says to use the words archive for a print-exact outline.
- Two pieces of one outline meet edge to edge. When a renderer draws them apart, the antialiased
  pixels on the cut can be lighter than on the word page. The ink is the same.

## How it is built

```sh
scripts/sync-test-data.sh                                    # the source bundle by tag, checksum verified
QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test identity   # every page passes the pixel gate
cargo run -p qvp-convert --release -- batch pages dist/pages  # pages, sidecars, atlas, surah-names/
scripts/package-data.sh X.Y.Z                                # VERSION.json, the tarball, its checksum
```

The batch projects each converted `SurahName` decoration into individual QVP and SVG
assets, matching combined sprite sheets, and one WOFF2 font. Every form contains only the
calligraphic `header_ink`; the page file keeps its frame. The combined QVP has one
`SurahName` decoration per row, in surah order. The combined SVG exposes matching
`surah-001` through `surah-114` views. The font maps those titles to `U+E001` through
`U+E072`; `map.json` supplies the source page, names and ayah count, and its Arabic name must
also be used as accessible text. CFF keeps the source cubic curves rather than approximating
them as TrueType quadratics. `atlas.qva` remains the aggregate lookup and search index.

The font needs no JavaScript:

```html
<link rel="stylesheet" href="surah-names/surah-names.css">
<span class="qvp-surah-name" data-surah="1" role="img" aria-label="سورة الفاتحة"></span>
```

The identity gate renders each source page and its converted page at four times the page
size and compares them pixel by pixel; the conversion keeps every curve and point, and
every page passes. A data release is tagged `data-vX.Y.Z` and is served as `qvp/vX.Y.Z/` (the first one,
`v0.1.0`, predates
that naming) and is never rewritten; a correction is a new version.

## Where to get it

- The GitHub release: `https://github.com/quran-ws/quran-engine/releases`, the
  `quran-engine-pages-hafs-kfgqpc.tar.gz` asset and its `.sha256`.
- The CDN, for apps that load pages over HTTP:
  `https://cdn.quran.ws/qvp/<version>/NNN.qvp` and
  `https://cdn.quran.ws/qvp/<version>/surah-names/qvp/NNN.qvp` (or the matching SVG,
  combined asset or font), immutable per version, with a
  `manifest.json` of digests (`docs/CDN.md`).
- `scripts/sync-test-data.sh` fetches the release into `dist/pages/` for development and
  tests; `scripts/sync-example-data.sh` copies the 29-page example set into an example app.

## Terms

Quran.ws publishes its data under CC BY 4.0 with a permanent attribution waiver for use
inside an application; attribution is required when the data is republished as data. The
underlying artwork is the Complex's, published by it for free use in personal, institutional,
digital and software works. `LICENSE` at the repository root carries the full notice,
including the Complex's own terms and the note on printed copies in the Kingdom of Saudi
Arabia. GitHub reports the repository licence as "Other" because that notice is a
composite; this is deliberate.

Errors in the text or the rendering are corrected over time: report one to
corrections@quran.ws. An app that shows Qur'anic text to readers is responsible for
verifying it against an authorised printed mushaf and for following corrections.
