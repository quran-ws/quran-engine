# Hafs QCF V1 1405H page data

This is a second Quran Engine data edition. It holds the **Hafs QCF V1 1405H print**. It uses
the QVP1 format of the default V4 data, so the engine reads it with no edition-specific code.
It has its own package name, version sequence, release tag and CDN folder. Do not install it
over the default edition, and do not mix its pages, word sidecars, atlas or surah-name assets
with those of another edition.

**Status:** not published. No GitHub release or CDN object exists yet.

## What the edition holds

- **604 pages** and **77,432 logical words**.
- **60,739 words** (78.44%, on 602 pages) use HQ word outlines. The other **16,693 words**
  keep the QCF V1 font outlines.
- Each page carries one stroke-weight adjustment, so that the HQ words and the font words on a
  page read at one weight. The adjustment is stored geometry, not a viewer filter.
- **4,272** waqf signs in the text. **4,221** are separate glyphs, typed with their waqf mark.
  The other **51** are part of a whole-word glyph and stay untyped, because splitting them
  would invent geometry.
- **240** rubu-al-hizb boundaries. The print draws **199** of their rosettes. The other **41**
  fall where a surah heading stands, and are metadata only.
- **15** sajdah marks.
- One printed glyph holds two words: 13:37:8 and 13:37:9. Both words keep their own key. The
  engine draws, selects and lays out the glyph once (`qvp_word_group`).

## Build

```sh
cargo run -p qvp-convert --release -- batch <qcf-v1-svg>/pages dist/hafs-qcf-v1-1405h <qcf-v1-svg>/index/by-page
node scripts/audit-qcf-v1-semantics.mjs dist/hafs-qcf-v1-1405h
QVP_SOURCE_VERSION=<quran-svg-elements tag> scripts/package-data.sh X.Y.Z hafs-qcf-v1-1405h
```

The audit reads the encoded pages and checks the waqf, division, rosette and sajdah counts
above. The package step writes `VERSION.json` with `"edition": "hafs-qcf-v1-1405h"` and the
digest of every file.

## Release and CDN

```text
tag:    data-hafs-qcf-v1-1405h-vX.Y.Z
asset:  quran-engine-pages-hafs-qcf-v1-1405h.tar.gz
CDN:    https://cdn.quran.ws/qvp/hafs-qcf-v1-1405h/vX.Y.Z/
bundle: hafs-qcf-v1-1405h.tar.br
```

`scripts/publish-cdn.sh` reads the edition from the tag. It refuses a package whose
`VERSION.json` names another edition. The edition has its own `latest.json`. The default
`data-vX.Y.Z` line and `qvp/vX.Y.Z/` stay the V4 data.
