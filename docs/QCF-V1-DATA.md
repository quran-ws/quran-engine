# Hafs QCF V1 1405H page data

This is a separate Quran Engine data edition for the **Hafs QCF V1 1405H print**. It uses the
same QVP1 runtime format as the default V4 data, but it has its own source identity, package
name, version sequence, and archive. It must not be installed over the default edition.

**Status:** the source corpus, conversion, deterministic package, consumer source, and separate
release/CDN path are qualified. No GitHub release or CDN object has been published yet.

## Qualified corpus

The package contains all **604 pages** and **77,432 logical words**. It uses **60,739
source-qualified HQ word outlines**, or **78.4417% of words**, on **602 pages**. The other
**16,693 words** retain the verified QCF fallback outlines. The HQ outlines are qualified word vectors. The fallback total includes 62 exact source candidates retained after corpus-wide visual
review of 48 affected pairs; the source summary and exclusion ledger pin that decision.

Admission is source-based: exact canonical key and NFC text, one untouched candidate path, no repair
or review flags, bounded 1406 source-scan fit, verified geometry, deterministic containment in the
real 1405H word bounds, and no typed or standalone semantic owner. Direct vectors retain their
pinned fit-audit and page-scale gates. Exact PDF-recovered vectors must carry the same
`recovered:<sha256>` identity in the SVG and fit report. The 1406 line number is evidence rather than
an admission gate because the 1405H and 1406H prints wrap differently. A candidate does not need to
outscore the fallback against one target-1405 raster scan.

The centre outline is fitted to the complete verified 1405H target word box, after reserving a
page-calibrated optical inset. This removes conspicuous under-width gaps such as
`لَا يُؤْمِنُونَ` without a word-specific exception.

Ayah markers keep their source-owned `qpc-old` boxes. The upstream SVG builder centres visible
marker ink rather than letting non-drawing TrueType contours shift the medallion. Quran Engine
receives the final page-space geometry and preserves it without edition-specific repair.

HQ strokes are intrinsically lighter than the QCF fallback strokes. The pinned optical table selects
one radius per page, shrinks the centre path by that radius, and emits four cardinal copies of the
same source outline. Their union remains exactly inside the verified target box. The final
whole-page mean absolute ink-ratio error is **0.3652%**; p95 is **1.7846%**. The optical
calibration is canonical source/QVP geometry, not a viewer filter, and its SHA-256 is carried through
the package and web manifest.

The canonical text contains **4,272 waqf signs**. The source map gives **4,221** of them an
independent final glyph; those paths are typed as the exact waqf mark and `waqf` family. The
remaining **51** signs are fused into a complete word glyph and deliberately remain untyped until
that outline can be split without inventing geometry.

All **240 rubu-al-hizb boundaries** carry semantic start metadata. This print draws **199** of their
rosettes; the other **41** remain metadata-only because a Surah heading occupies that position. All
**15 sajdah marks** are standalone decorations. Fourteen use glyphs already present in the current
map; the page-454 mark at `38:24` is restored from pinned source evidence.

The package contract is `conformance/qcf-v1-data-package.json`. It pins:

- the exact HQ source summary, 604-page SVG tree, and canonical word-index tree;
- the pinned 1,208-file candidate tree, source-admission manifest, pathless 60,739-record evidence
  ledger, complete mutually exclusive exclusion counts, and the 604-page optical calibration;
- the complete waqf and division/sajdah ownership ledgers and their exact counts;
- the actual semantic geometry, the earlier header-qualified baseline, and canonical path-order
  evidence proving that all 88,472 baseline paths survive and only the reviewed sajdah path is added;
- edition `hafs-qcf-v1`, print year 1405H, and all source counts;
- the exact 1,443-file QVP tree and both atlas digests; and
- the exact `qvp-convert` binary used for conversion.

A changed source identity, candidate tree, admission ledger, exclusion count, page, word index,
semantic ledger, path-order proof, word sidecar, atlas, Surah-name asset, converter, or unexpected
file fails before packaging.

## Build

First convert the qualified HQ SVG corpus:

```sh
cargo build -p qvp-convert --release

target/release/qvp-convert batch \
  /path/to/qcf-v1-hq-svg/pages \
  /tmp/qcf-v1-qvp \
  /path/to/qcf-v1-hq-svg/index/by-page
```

Audit the encoded semantics over all 604 pages:

```sh
node scripts/audit-qcf-v1-semantics.mjs /tmp/qcf-v1-qvp
```

Then create the edition archive:

```sh
python scripts/package-edition-data.py X.Y.Z \
  --source-dir /path/to/qcf-v1-hq-svg \
  --qvp-dir /tmp/qcf-v1-qvp \
  --converter target/release/qvp-convert \
  --out-dir dist/qcf-v1
```

The packer accepts no quality, edition, or inventory overrides. The committed contract owns all
source and QVP identities.

## Archive

The archive is named:

```text
quran-engine-pages-hafs-qcf-v1-1405h-vX.Y.Z.tar.gz
```

Its root is `quran-engine-pages-hafs-qcf-v1-1405h/`. It contains:

- 604 `NNN.qvp` pages and 604 canonical `NNN.words.json` sidecars;
- `atlas.qva` and `atlas.json`;
- all 114 Surah-name QVP, SVG, combined, font, CSS, and map assets;
- `LICENSE.txt` and an edition-specific `README.md`; and
- schema-7 `VERSION.json`, with the complete source semantic evidence, converter, QVP tree,
  package contract, and SHA-256 digest of every payload file.

The archive is deterministic: members are sorted; tar timestamps, owners, groups, and modes are
normalized; the gzip timestamp and filename are empty. Independently produced archives from the
pinned inputs must be byte-identical.

The current `v0.1.0` qualification has **1,446 archive members**, **1,445 payload hashes**, and
is **62,475,245 bytes** compressed. Two independent builds produced SHA-256
`7ec6052d35175c1be893484e88004ce754b2fdffc2c752c3c64632c63036d9f1`. Its 604 QVP pages were
independently audited to the exact waqf, division-boundary, rosette, and sajdah inventories. This is
qualification evidence, not a published release.

## Release and CDN

QCF V1 uses its own release and CDN family:

```text
tag:    data-hafs-qcf-v1-1405h-vX.Y.Z
asset:  quran-engine-pages-hafs-qcf-v1-1405h-vX.Y.Z.tar.gz
CDN:    https://cdn.quran.ws/qvp/hafs-qcf-v1-1405h/vX.Y.Z/
bundle: hafs-qcf-v1-1405h.tar.br
```

`scripts/qvp-data-release.py` verifies release schema 7, the source semantic evidence, archive
root, edition, print year, complete 1,443-file QVP inventory, notice files, and all declared digests.
`scripts/publish-cdn.sh`
then creates the manifest and opaque brotli bundle; `--stage-only` performs the complete process
without uploading.
The default `data-vX.Y.Z` and `/qvp/vX.Y.Z/` line remains the V4 corpus.

The qualified `v0.1.0` archive is ready for this path, but the URL is not live until the matching
GitHub release is created. Publishing is an explicit release operation, not part of qualification.

## Consumer use

The decoder needs no QCF-specific branch: pages remain QVP1. The optional data entry point keeps
all edition assets together:

```js
import { qvpSource, verifyQvpManifest } from '@quran.ws/engine/data'

const data = qvpSource('hafs-qcf-v1-1405h', '0.1.0')
const manifest = await fetch(data.manifestUrl).then(response => response.json())
verifyQvpManifest(data, manifest)
```

Do not mix this source's pages, word sidecars, atlas, or Surah-name assets with another edition.
The downloader must hash each response against the verified manifest before storing it.
