# QVP data sources

Quran Engine decodes QVP1 independently of any particular muṣḥaf. The optional
`@quran.ws/engine/data` entry point identifies the page data that an application should load.
It is declarative and performs no network or cache work.

## Editions

| key | edition | print | CDN family |
|---|---|---:|---|
| `hafs-kfgqpc` | KFGQPC V4 | 1441H | `qvp/` |
| `hafs-qcf-v1-1405h` | QCF V1 | 1405H | `qvp/hafs-qcf-v1-1405h/` |

The families have separate version sequences. A QCF V1 version never replaces the default V4
version with the same number.

QCF V1 `v0.1.0` uses `quran-engine/edition-data-release` schema 7. Its release metadata pins
60,739 source-qualified HQ words (78.4417%), 16,693 verified fallbacks, the exact source-candidate
and admission ledgers, 4,221 independently typed waqf paths, all 240 division boundaries, 199
printed rosettes, 41 metadata-only boundaries, 15 sajdah marks, and the exact page-calibrated
optical-weight table. `verifyQvpManifest()` rejects
an older or foreign schema and any contradictory source identity, count, or exclusion ledger.

## One exact asset set

```js
import { loadPage } from '@quran.ws/engine/lite'
import { qvpSource } from '@quran.ws/engine/data'

const data = qvpSource('hafs-qcf-v1-1405h', '0.1.0')
const page = await loadPage(data.pageUrl(42))
```

`qvpSource()` returns immutable URLs for the release manifest, `VERSION.json`, bundle, atlas,
page QVPs, word sidecars, and Surah-name assets. Its version must be explicit. The helper does
not follow `latest.json`, because an application build should not silently change editions or
page bytes.

Treat a source as indivisible. Its pages, sidecars, atlas, and Surah-name assets describe one
edition and version; do not combine them with files from another source.

A custom mirror changes only the origin:

```js
const data = qvpSource('hafs-qcf-v1-1405h', '0.1.0', {
  origin: 'https://static.example.com',
})
```

## Verify before installing

```js
import { verifyQvpManifest } from '@quran.ws/engine/data'

const manifest = await fetch(data.manifestUrl).then(response => {
  if (!response.ok) throw new Error(`manifest: ${response.status}`)
  return response.json()
})
verifyQvpManifest(data, manifest)
```

`verifyQvpManifest()` rejects a different edition, print year, package, version, base URL,
bundle, incomplete page or Surah-name inventory, duplicate path, path traversal, malformed digest,
and unexpected file. It also requires the release's own digest map to cover every required data
asset and agree with the CDN inventory. The downloader must still SHA-256 every response and
compare it with the corresponding `files` record before storing it.

The QCF V1 `v0.1.0` package has been qualified, but its URLs become live only when the matching
GitHub data release is published and mirrored to the CDN.

## Verified local installation

`conformance/qvp-data-editions.json` pins the exact archive tag and outer SHA-256 for each known
edition. `scripts/install-qvp-data.py` verifies that outer digest, safely extracts the one expected
package root, checks the edition's complete inventory and every digest in `VERSION.json`, then moves
the verified tree into place atomically. It never replaces an existing destination.

```sh
# Inspect the pinned QCF release identity without reading the network.
python scripts/install-qvp-data.py resolve hafs-qcf-v1-1405h

# Verify and install the qualified local QCF archive.
python scripts/install-qvp-data.py verify-archive \
  hafs-qcf-v1-1405h /path/to/quran-engine-pages-hafs-qcf-v1-1405h-v0.1.0.tar.gz
python scripts/install-qvp-data.py install \
  hafs-qcf-v1-1405h /path/to/quran-engine-pages-hafs-qcf-v1-1405h-v0.1.0.tar.gz \
  /path/to/qvp/hafs-qcf-v1-1405h/v0.1.0
python scripts/install-qvp-data.py verify-installed \
  hafs-qcf-v1-1405h /path/to/qvp/hafs-qcf-v1-1405h/v0.1.0
```

The published default V4 release may be fetched and installed in one step:

```sh
python scripts/install-qvp-data.py fetch hafs-kfgqpc /path/to/qvp/v0.4.0
```

QCF V1 network fetching remains disabled in the registry until its GitHub release exists. The
qualified local archive can still be verified and installed; the installer refuses to invent or
probe an unpublished URL.
