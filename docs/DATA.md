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
| `VERSION.json` | provenance: data version, source release, engine commit, format version, page count, a digest of every file |

Sizes, for the whole mushaf: 604 pages, 77,432 words, 6,236 ayahs, 114 surahs; about 82 MB
raw, 42 MB as the release tarball, 37 MB brotli-compressed on the CDN.

## Where it comes from

The mushaf is the **King Fahd Glorious Qur'an Printing Complex (KFGQPC) Madani mushaf,
version 4, 1441 AH**, in the Hafs reading, as vector artwork. The immediate source is the
`quran-svg-elements` release (`https://github.com/quran-ws/quran-svg-elements`,
`hafs-kfgqpc` bundle): one SVG per page with every word tagged by its key, plus an index of
the text forms. `docs/UPSTREAM-DATA-ISSUES.md` lists what was found wrong in that source
and fixed there; the converter never patches data.

## How it is built

```sh
scripts/sync-test-data.sh                                    # the source bundle by tag, checksum verified
QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test identity   # every page passes the pixel gate
cargo run -p qvp-convert --release -- batch pages dist/pages  # NNN.qvp, NNN.words.json, atlas.qva, atlas.json
scripts/package-data.sh X.Y.Z                                # VERSION.json, the tarball, its checksum
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
  `https://cdn.quran.ws/qvp/<version>/NNN.qvp`, immutable per version, with a
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
