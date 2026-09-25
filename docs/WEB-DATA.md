# Optional web data loader

`@quran.ws/engine/data` loads verified Hafs text and artwork. It is a separate web
convenience layer, not part of the byte-oriented engine or lite decoder. Importing it
starts no requests and loads no Wasm. It imports passage layout only when asked for a
passage. No C ABI or other platform wrapper changes.

```js
import { QvpData } from '@quran.ws/engine/data'
import { surahs } from '@quran.ws/engine/metadata'

const source = new QvpData({ pageCacheSize: 6, responseCacheSize: 24 })
const text = await source.loadText()
const passage = await source.loadPassage({ surah: 2, from: 255 })
const layout = passage.layout({ width: 320 })
passage.draw(context, layout)
console.log(surahs[1].name, text[1][254])
```

## Surface

- `surahs`: immutable Arabic names and ayah counts for 114 surahs. Zero-based array.
- `new QvpData(options)`: creates an independent in-memory cache. Options are
  `pageCacheSize` (6), `responseCacheSize` (24), `cachePrefix` (`qvp`), `fetch`
  (the host's Fetch function), and `cacheStorage` (the host's CacheStorage).
  Pass `cacheStorage: null` to disable persistent caching. Zero page limits disable
  their respective caches. Negative or fractional limits throw.
- `pageOf(surah, ayah)`: returns the one-based printed page. Invalid references throw.
- `loadPage(number)`: returns a decoded, digest-checked page. Treat its geometry as immutable.
- `loadPassage({surah, from, to = from})`: returns a complete-ayah `QvpPassage`.
  Pages load sequentially, not as an unbounded burst of parallel requests.
- `loadText()`: returns immutable Unicode strings in zero-based surah/ayah arrays.
- `hafs`: immutable release descriptor with `pageUrl`, `pageVersion`, `pages`,
  `textUrl`, `textVersion` and `textDigest`. Each page row is
  `[firstSurah, firstAyah, sha256]`. Hosts can use it to provision test fixtures.

Use one loader instance per shared cache policy. Successful and in-flight text requests
are shared. Page promises use an LRU map bounded by `pageCacheSize`; evicting a pending
request does not cancel it. The map bounds retained entries, not the number of calls a
host can make at once. Callers should bound visible passages and concurrent requests.

## Integrity and offline use

The loader verifies SHA-256 before decoding both network and cached bytes. A corrupt
cached response is removed and fetched again. Failed requests can be retried; they do
not poison the in-memory cache. A response is cached only after decoding succeeds.

Page responses use `<cachePrefix>-qvp-v0.3.0`, with at most `responseCacheSize` entries
after a successful write. Text uses `<cachePrefix>-quran-text-v1`, with one entry.
CacheStorage failures do not prevent network use. First-use offline requests fail;
the host keeps its source text and supplies a retry UI. There is no boot fetch, full
artwork download, service-worker dependency or bundled Quran corpus.

Fetch responses and cached assets use the host's memory while being verified. Only
fetch trusted release URLs. This helper does not replace the host's transport limits.

## Release metadata and provenance

The release descriptor (`web/data-hafs.json`) and surah metadata (`web/surahs.json`)
are plain JSON. Their public JavaScript exports freeze them once on import.
Node requires 18.20 or later for JSON import attributes.

The small release descriptor and canonical surah metadata are packaged inputs;
page binaries and Quran text remain external. They are not another copy of the data
release. Changing the pinned edition requires updating the descriptor, cache versions
and tests together.

- Pages: `https://cdn.quran.ws/qvp/v0.3.0/`. The page starts and digests come from its
  `atlas.json` and `manifest.json`. Atlas SHA-256:
  `0e09a775b5aa66d8f0cf1218fc0265a7936be780a77f4d94d720d5724d6b1614`.
- Text: `https://fonts.nuqayah.com/quran.txt`. The URL is not versioned; the pinned
  SHA-256 defines the accepted version:
  `b8436b6bba887eab7635a3d524ebdd9ba62e6dec3e329c1400516abbc82159ed`.
  Each of its 6,236 rows has two comma-separated columns. The second is Unicode
  Arabic; the first is font-specific. Extract the second without normalization.
- Surah names follow Dalail's Arabic names. Counts come from the maximum ayah number
  per surah in `https://fonts.nuqayah.com/qcf4-data.txt`, and sum to 6,236.

The engine code is MIT-licensed. Quran.ws decomposition and indexes have their own
CC-BY-4.0 terms and in-product attribution waiver. Original KFGQPC artwork retains its
own rights. Fetching text or artwork does not relicense it under the engine's license.
See `DATA.md` and the data release's license.

## Checks

`node web/data.test.mjs` checks metadata, reference bounds, shared loads, retry,
integrity rejection, corrupt-cache recovery, offline reuse and cache limits with
synthetic text and the codec fixture. It does not need a network connection or ship
Quran text. The lite passage suite separately checks real artwork and layout.
