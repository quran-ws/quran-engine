# Web data loader

Optional fetching and caching, separate from the engine. No import-time requests or Wasm.

```js
import { QvpData } from '@quran.ws/engine/data'
import surahs from '@quran.ws/engine/metadata' with { type: 'json' }

const source = new QvpData()
const text = await source.loadText() // text[surah - 1][ayah - 1]
const passage = await source.loadPassage({ surah: 2, from: 255 })
passage.draw(context, passage.layout({ width: 320 }))
```

## API

- `pageOf(surah, ayah)`: printed page number; invalid references throw.
- `loadPage(number)`: verified decoded page. Treat its geometry as read-only.
- `loadPassage({surah, from, to = from})`: complete-ayah passage; pages load sequentially.
- `loadText()`: immutable Unicode arrays indexed by surah and ayah, both zero-based.

Constructor options: `pageCacheSize` (6), `responseCacheSize` (24), `cachePrefix`
(`qvp`), `fetch` (host Fetch), `cacheStorage` (host CacheStorage, or `null` to disable).
Zero disables a page cache; negative or fractional limits throw. Use one instance to
share text requests and the page LRU. Eviction does not cancel pending requests;
hosts still bound visible ranges and concurrent calls.

SHA-256 is checked before decoding network or cached bytes. Corrupt cache entries are
removed; failures can be retried. Cache writes happen only after decoding succeeds.
CacheStorage failures do not block network use. First-use offline failure needs a host
fallback. Fetch trusted release URLs; transport limits remain the host's responsibility.

Cache buckets are `<cachePrefix>-qvp-v0.3.0` (bounded page responses) and
`<cachePrefix>-quran-text-v1` (one text response). There is no service-worker dependency.

## Data and provenance

`web/surahs.json` is exported directly as `@quran.ws/engine/metadata`; treat it as
read-only. `web/data-hafs.json` is exposed as the frozen `hafs` export from `/data`.
Its page rows are `[firstSurah, firstAyah, sha256]`. Node requires 18.20+ for JSON imports.
Quran text and artwork are fetched, not packaged. Update pins and cache versions together.

- Page starts/digests: `https://cdn.quran.ws/qvp/v0.3.0/atlas.json` and `manifest.json`.
  Atlas SHA-256: `0e09a775b5aa66d8f0cf1218fc0265a7936be780a77f4d94d720d5724d6b1614`.
- Text: `https://fonts.nuqayah.com/quran.txt`; its digest in the descriptor pins the
  otherwise unversioned URL. Extract the second comma-separated column of 6,236 rows
  without normalization; the first column is font-specific.
- Surah names follow Dalail; counts come from `https://fonts.nuqayah.com/qcf4-data.txt`.

Fetching does not relicense assets under the engine's MIT license. Quran.ws indexes
have CC-BY-4.0 terms with an in-product attribution waiver; KFGQPC artwork retains its
own rights. See `DATA.md` and the data release license.

`node web/data.test.mjs` checks validation, integrity, retry, offline reuse and cache
limits with synthetic text and a codec fixture, without network access.
