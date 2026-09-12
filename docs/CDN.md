# Page data on qvp.quran.ws

The engine ships code; an app loads page data. For the web that data has to come over HTTP
with CORS, so each data release is mirrored to a CDN under versioned, immutable URLs:

```
https://qvp.quran.ws/v0.1.0/manifest.json
https://qvp.quran.ws/v0.1.0/001.qvp
https://qvp.quran.ws/v0.1.0/001.words.json
https://qvp.quran.ws/v0.1.0/atlas.qva
https://qvp.quran.ws/v0.1.0/hafs-kfgqpc.tar.br
```

A reader fetches the pages near its position and caches them; a service worker can prefetch
a juz from the manifest. Nothing needs the whole 92 MB up front.

## The contract

- **A version is written once.** `vX.Y.Z/` is never rewritten — a new build is a new version.
  That is what lets everything be served `Cache-Control: public, max-age=31536000, immutable`,
  and why `publish-cdn.sh` refuses a prefix that already exists. Rewriting a published object
  needs a manual cache purge to take effect.
- **Pages are stored raw and compressed at the edge**, negotiating zstd, brotli or gzip per
  client: about 67 KB per page, 41.8 MB for the whole mushaf fetched page by page. Storing
  raw means the bytes a client verifies are the bytes the release signed.
- **`manifest.json`** lists every file with its size and sha256, the bundle, and the data
  release's own `VERSION.json`. It is how a client prefetches a page range and verifies what
  it got without 604 HEAD requests. It is the one object **not** cached as immutable — five
  minutes, because an index cached for a year cannot be corrected without a purge.
- **Two shapes of the same data.** The per-page objects are what a reader fetches. The bundle
  is for downloading the whole mushaf: one request instead of 1,212, and 26 MB instead of
  41.8 MB, because one brotli stream over all 604 pages finds the repetition between them.
  gzip cannot — its 32 KB window never holds two pages at once — which is why the release
  `.tar.gz` is 42 MB and saves nothing.
- **The bundle is an opaque brotli file.** No `Content-Encoding`: every client receives the
  same 26 MB and decodes it. iOS uses `COMPRESSION_BROTLI` (iOS 15+), which takes 0.31 s for
  the whole mushaf. Browsers have no brotli decoder in JavaScript — `DecompressionStream`
  supports only gzip, deflate and deflate-raw — so a web app should fetch pages individually
  instead; 20 pages arrive in 36 ms.
- **Digests cover both ends of the decode.** `bundle.sha256` is the bytes as downloaded;
  `bundle.decoded` is the size and digest of the `ustar` archive inside.

## Publishing

The GitHub release is the canonical artefact. The CDN is a mirror of it, built from the
signed tarball, never from a working tree:

```
scripts/publish-cdn.sh v0.1.0 --from-release    # verify sha256, extract, stage, upload
scripts/publish-cdn.sh v0.1.0 --stage-only      # build dist/cdn/v0.1.0, upload nothing
```

`.github/workflows/publish-cdn.yml` runs the first form when a release is published, then
checks a page against its manifest digest before the run is allowed to pass.

## Infrastructure

Cloudflare R2 bucket `qvp-pages`, with `qvp.quran.ws` as its custom domain. Serving this data
needs control of compression and of cache headers, and a per-version copy of a 92 MB dataset
needs room to grow.

Zone settings these URLs depend on, all on `quran.ws`:

| setting | value | why |
|---|---|---|
| Compression Rule `qvp page data` | host, excluding `.tar.br` → zstd, brotli, gzip | Cloudflare does not compress `application/octet-stream` by default. The bundle is excluded because it is already brotli-11; re-encoding it at the edge spends CPU and makes it bigger. |
| Cache Rule `qvp page data` | same match → eligible for cache, edge and browser TTL respect origin | the objects carry `immutable`; this stops a zone-wide rule overriding them |
| Tiered Cache | Smart topology | POPs fill from a regional parent instead of each hitting R2 |
| HTTP/3, 0-RTT | on | a reader fetches many small files at once, and returns often |
| WAF skip rule for this hostname | bot checks, browser integrity, hotlink protection, security level, rate limiting | these challenge or block non-browser clients. Without the skip, `URLSession`, OkHttp and Dart break while a browser keeps working, and a service worker prefetching a juz looks like abuse. |
| CORS policy on the bucket | `*`, `GET`/`HEAD`, `Range` | R2 sends no CORS headers otherwise, which is the reason this CDN exists |

The credentials are an R2 API token scoped to **Object Read & Write on this bucket only**,
held as secrets on the `cdn` GitHub environment (`R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`,
`R2_SECRET_ACCESS_KEY`) rather than as repository secrets, so only an approved,
tag-triggered run can read them.

The upstream rights notice travels with the data: `README.md` and `VERSION.json` are
published alongside the pages at every version prefix.
