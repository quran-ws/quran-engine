# The page data

The engine ships code only. Page data is published as a release of this repository and
mirrored to a CDN. This page says which mushaf it is, where it comes from, how it is built,
and under what terms an app may use it.

## What is published

| file | what |
|---|---|
| `NNN.qvp` | one printed page in the QVP1 format (`docs/FORMAT.md`): outlines, words, ayah fragments, lines, decorations |
| `NNN.words.json` | that page's five text forms per word key: `rasm_uthmani`, `rasm_imlai`, `qpc`, `rasm`, `search` |
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
the text forms. `docs/UPSTREAM-DATA-ISSUES.md` lists what was found wrong in that source
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
