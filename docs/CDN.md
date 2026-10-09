# cdn.quran.ws

The engine ships code; an app loads page data. For the web that data has to come over HTTP
with CORS, so each release is mirrored to a CDN under versioned, immutable URLs:

```
https://cdn.quran.ws/qvp/<version>/manifest.json
https://cdn.quran.ws/qvp/<version>/001.qvp
https://cdn.quran.ws/qvp/<version>/001.words.json
https://cdn.quran.ws/qvp/<version>/atlas.qva
https://cdn.quran.ws/qvp/<version>/surah-names/qvp/001.qvp
https://cdn.quran.ws/qvp/<version>/surah-names/qvp/all.qvp
https://cdn.quran.ws/qvp/<version>/surah-names/svg/001.svg
https://cdn.quran.ws/qvp/<version>/surah-names/svg/all.svg
https://cdn.quran.ws/qvp/<version>/surah-names/surah-names.woff2
https://cdn.quran.ws/qvp/<version>/surah-names/map.json
https://cdn.quran.ws/qvp/<version>/hafs-kfgqpc.tar.br

https://cdn.quran.ws/qvp/hafs-qcf-v1-1405h/<version>/manifest.json
https://cdn.quran.ws/qvp/hafs-qcf-v1-1405h/<version>/001.qvp
https://cdn.quran.ws/qvp/hafs-qcf-v1-1405h/<version>/hafs-qcf-v1-1405h.tar.br
```

A reader fetches the pages near its position and caches them. A surah list can choose
individual QVP or SVG titles, either combined sheet, or the WOFF2 font without loading a
full page. The atlas supplies aggregate lookup and search metadata. A service worker can prefetch a juz from the manifest. Nothing needs the
whole data set up front.

## The layout

One host carries every published artefact of the stack. Each family has its own folder and
its own version line, so a page-data release and an SVG release never collide:

```
cdn.quran.ws/
├── qvp/                    page data, from quran-engine
│   ├── v0.4.0/             default KFGQPC V4 line
│   ├── latest.json
│   └── hafs-qcf-v1-1405h/  separate QCF V1 1405H line
│       ├── v0.1.0/
│       └── latest.json
├── svg/
│   ├── pages/              full-page mushaf SVG, from quran-svg
│   │   ├── v1.2.0/
│   │   └── latest.json
│   └── elements/           element SVG and index, from quran-svg-elements
│       ├── v0.4.0/
│       └── latest.json
└── engine/
    ├── wasm/v0.2.1/        qvp_ffi.wasm
    ├── apple/v0.2.1/       QvpEngine.xcframework.zip and its checksum
    └── android/v0.2.1/     the AAR
```

`latest.json` sits beside the version folders, not inside one. It names the current version
and the URL of its manifest, and it only moves forward, so republishing an older release
does not send consumers back to it:

```json
{ "version": "v0.4.0",
  "manifest": "https://cdn.quran.ws/qvp/v0.4.0/manifest.json",
  "updated": "2026-09-15T00:00:00Z" }
```

The default data line is tagged `data-vX.Y.Z` and publishes to `qvp/vX.Y.Z/`. QCF V1 1405H
is tagged `data-hafs-qcf-v1-1405h-vX.Y.Z` and publishes to
`qvp/hafs-qcf-v1-1405h/vX.Y.Z/`. Their version sequences and `latest.json` pointers are
independent. The engine line is tagged `vX.Y.Z` and publishes to
`engine/<family>/vX.Y.Z/`. The first default data release, `v0.1.0`, predates the prefix.

Before this layout the page data was served from `qvp.quran.ws/<version>/`. That hostname
now returns a 301 to `cdn.quran.ws/qvp/<version>/`, so URLs published before the move still
resolve.

## The contract

- **A version is written once.** `vX.Y.Z/` is never rewritten — a new build is a new version.
  That is what lets everything be served `Cache-Control: public, max-age=31536000, immutable`,
  and why `cdn-put.sh` refuses a folder that already exists. Rewriting a published object
  needs a manual cache purge to take effect.
- **Pages and surah-name assets are stored raw and compressed at the edge**, negotiating
  zstd, brotli or gzip per client. The bytes a client verifies are the bytes the release
  signed.
- **`manifest.json`** lists every file with its size and sha256, the bundle, and the data
  release's own `VERSION.json`. It is how a client prefetches a page range and verifies what
  it got without 604 HEAD requests. It is the one object **not** cached as immutable — five
  minutes, because an index cached for a year cannot be corrected without a purge.
- **Two shapes of the same data.** The individual objects are what a reader fetches. The
  bundle carries all 604 pages, 604 word sidecars, both atlas files, release metadata and
  114 surah-title assets in one request. One brotli stream finds repetition across files;
  gzip's 32 KB window cannot exploit repetition across distant archive members.
- **The bundle is an opaque brotli file.** No `Content-Encoding`: every client receives the
  same bytes and decodes them. Its inner ustar follows the verified inventory order and uses
  zero timestamps, numeric owner/group zero, and mode `0644`, so extraction time and host file
  metadata cannot change an immutable bundle's identity. iOS uses `COMPRESSION_BROTLI` (iOS 15+),
  which takes 0.31 s for the whole mushaf. Browsers have no brotli decoder in JavaScript —
  `DecompressionStream` supports only gzip, deflate and deflate-raw — so a web app should fetch
  pages individually instead; 20 pages arrive in 36 ms.
- **Digests cover both ends of the decode.** `bundle.sha256` is the bytes as downloaded;
  `bundle.decoded` is the size and digest of the `ustar` archive inside.

## Publishing

The GitHub release is the canonical artefact. The CDN is a mirror of it, built from the
checksummed tarball, never from a working tree:

```
scripts/publish-cdn.sh data-v0.4.0 --from-release
scripts/publish-cdn.sh data-hafs-qcf-v1-1405h-v0.1.0 --from-release
scripts/publish-cdn.sh data-hafs-qcf-v1-1405h-v0.1.0 --from-release --stage-only
```

`scripts/qvp-data-release.py` resolves the tag to its package and CDN family, safely extracts
the checksummed archive, and verifies edition identity, complete inventory, and every declared digest.
`.github/workflows/publish-cdn.yml` runs the matching publish command when a data release is
published, then checks a page, bundle, family-specific `latest.json`, and manifest over HTTPS.

`scripts/cdn-put.sh` holds what every publisher needs: the signed upload, the content types,
the cache headers, the already-published guard and the `latest.json` update. It is sourced,
not run.

### Publishing from another repository

Each repository publishes its own folder. To add one:

1. Create a `cdn` environment in the repository and give it the three R2 secrets.
2. Copy `scripts/cdn-put.sh` into the repository. It is small and has no dependencies beyond
   curl and jq, which is why each repository carries a copy instead of calling a shared
   action.
3. Write a publisher that stages the files, writes a `manifest.json` listing each file with
   its size and sha256, then calls `cdn_guard`, `cdn_put` and `cdn_latest`.
4. Trigger it from `release: [published]`, and check one file against its manifest digest
   over HTTPS before the run passes.

| repository | folder |
|---|---|
| quran-engine | `qvp/`, `engine/wasm/`, `engine/apple/`, `engine/android/` |
| quran-svg | `svg/pages/` |
| quran-svg-elements | `svg/elements/` |

## Infrastructure

Cloudflare R2 bucket `cdn-quran-ws`, with `cdn.quran.ws` as its custom domain. Serving this
data needs control of compression and cache headers, and every immutable data version needs
room to grow.

Zone settings these URLs depend on, all on `quran.ws`:

| setting | value | why |
|---|---|---|
| Compression Rule `cdn` | host, excluding `.tar.br` → zstd, brotli, gzip | Cloudflare does not compress `application/octet-stream` by default. The bundle is excluded because it is already brotli-11; re-encoding it at the edge spends CPU and makes it bigger. |
| Cache Rule `cdn` | same match → eligible for cache, edge and browser TTL respect origin | the objects carry `immutable`; this stops a zone-wide rule overriding them |
| Tiered Cache | Smart topology | POPs fill from a regional parent instead of each hitting R2 |
| HTTP/3, 0-RTT | on | a reader fetches many small files at once, and returns often |
| WAF skip rule for this hostname | bot checks, browser integrity, hotlink protection, security level, rate limiting | these challenge or block non-browser clients. Without the skip, `URLSession`, OkHttp and Dart break while a browser keeps working, and a service worker prefetching a juz looks like abuse. |
| CORS policy on the bucket | `*`, `GET`/`HEAD`, `Range` | R2 sends no CORS headers otherwise, which is the reason this CDN exists |

Two rules remain on the old hostname. A Redirect Rule sends `qvp.quran.ws/<path>` to
`https://cdn.quran.ws/qvp/<path>` with a 301, preserving the query string. The WAF skip rule
stays with it: the firewall phase runs before the redirect phase, so a challenged client
never reaches the 301, and `URLSession`, OkHttp and Dart would break on the old URLs while a
browser kept working. The compression and cache rules for that hostname were removed, since
it no longer serves a body.

The credentials are an R2 API token scoped to **Object Read & Write on this bucket only**,
held as secrets on the `cdn` GitHub environment (`R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`,
`R2_SECRET_ACCESS_KEY`) rather than as repository secrets, so only an approved,
tag-triggered run can read them. The three publishing repositories share one token. The
token does not restrict which folder a run may write, so that discipline lives in the
publishers.

The upstream rights notice travels with the data: `README.md` and `VERSION.json` are
published alongside the pages at every version prefix.
