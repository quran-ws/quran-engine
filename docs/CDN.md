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

- **A version is written once.** `vX.Y.Z/` is never rewritten — a new build is a new
  version. That is what lets everything be served `Cache-Control: public, max-age=31536000,
  immutable`, and why `publish-cdn.sh` refuses a prefix that already exists.
- **Objects are stored raw; the edge compresses.** A Cloudflare Compression Rule on
  `qvp.quran.ws` compresses `application/octet-stream`, negotiating zstd → brotli → gzip per
  client — roughly 37 MB brotli against 92 MB raw for the whole mushaf. Storing raw means the
  bytes a client verifies are the bytes the release signed, and losing the rule degrades to
  larger-but-correct responses rather than breaking decoding.
- **`manifest.json`** lists every file with its size and sha256, the bundle, and the data
  release's own `VERSION.json`. It is how a client prefetches a page range and verifies what
  it got without 604 HEAD requests. It is the one object **not** cached as immutable — five
  minutes, because an index cached for a year is unfixable without a cache purge.
- **Two shapes of the same data.** The per-page objects are what a reader fetches; it wants
  page 42, not 92 MB. The bundle is for "download the whole mushaf": one request instead of
  1,212. Solid brotli sees the redundancy between 604 pages of one mushaf; gzip's 32 KB
  window never has two pages in scope at once, which is why the release `.tar.gz` is 42 MB
  and saves nothing over fetching pages individually.
- **The bundle is an opaque brotli file.** No `Content-Encoding` is involved: every client
  receives the same 26 MB and decodes it itself. iOS does this with `COMPRESSION_BROTLI`
  (iOS 15+, 0.31 s for the whole mushaf measured on arm64).

  Serving it as `Content-Encoding: br` was tried and reverted. It looked ideal — 26 MB to
  browsers and iOS with no client decoder — but the edge caches one normalised body, so
  **whichever client filled the cache first decided what everyone got**:

  | first request | subsequent br clients |
  |---|---|
  | br | 26 MB, correct |
  | gzip | **89 MB, uncompressed**, and it stays that way until a purge |

  That is not shippable: it would have failed randomly per Cloudflare POP.

  Browsers have no brotli decoder in JavaScript — `DecompressionStream` supports only
  gzip/deflate/deflate-raw — so a web app should fetch pages individually rather than use
  the bundle. 20 pages arrive in 36 ms, which is what a reader wants anyway.

- The manifest's `bundle.sha256` covers the bytes as downloaded; `bundle.decoded` carries
  the size and digest of the tar inside, so a client can check both ends of the decode.

## Publishing

The GitHub release is still the canonical artefact — the CDN is a mirror of it, built from
the signed tarball, never from a working tree:

```
scripts/publish-cdn.sh v0.1.0 --from-release    # verify sha256, extract, stage, upload
scripts/publish-cdn.sh v0.1.0 --stage-only      # build dist/cdn/v0.1.0, upload nothing
```

`.github/workflows/publish-cdn.yml` runs the first form when a release is published.

## Infrastructure

Cloudflare R2 bucket `qvp-pages`, with `qvp.quran.ws` as its custom domain. R2 was chosen over
GitHub Pages because the page format is deliberately compressor-friendly and uncompressed on
disk — serving it needs control of compression, which Pages does not give — and because
Pages' 1 GB site and 100 GB/month bandwidth limits do not fit a per-version copy of a 92 MB
dataset.

Zone settings this depends on, all on `quran.ws`:

| setting | value | why |
|---|---|---|
| Compression Rule `qvp page data` | host, **excluding `.tar`** → zstd, brotli, gzip | Cloudflare does not compress `application/octet-stream` by default. The `.tar` exclusion stops the edge re-encoding the bundle's brotli-11 at its own faster, worse level — Chrome advertises zstd, and without the exclusion it was served 37 MB of zstd instead of 26 MB of br. |
| Compression Rule `qvp bundle` | host, `.tar`, and request does not accept br | **obsolete** — it existed for the `Content-Encoding: br` design and now matches nothing, since the bundle ends in `.tar.br`. Safe to delete. |
| Cache Rule `qvp page data` | same match → eligible for cache, edge and browser TTL respect origin | the objects carry `immutable`; this stops a zone-wide rule overriding them |
| Tiered Cache | Smart topology | POPs fill from a regional parent instead of each hitting R2 |
| HTTP/3, 0-RTT | on | a reader fetches many small files at once, and returns often |
| Bot Fight Mode | **off** | it challenges non-browser clients — it would break `URLSession`, OkHttp and Dart while the web demo kept working. If it is ever switched on, add a WAF skip rule for this hostname. |

The credentials are an R2 API token scoped to **Object Read & Write on this bucket only**,
held as secrets on the `cdn` GitHub environment (`R2_ACCOUNT_ID`, `R2_ACCESS_KEY_ID`,
`R2_SECRET_ACCESS_KEY`) rather than as repository secrets, so only an approved,
tag-triggered run can read them.

The upstream rights notice travels with the data: `README.md` and `VERSION.json` are
published alongside the pages at every version prefix.
