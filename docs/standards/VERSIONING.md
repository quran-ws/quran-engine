# Versioning

## One version

The engine and every SDK share one version, `X.Y.Z`, whose source of truth is
`[workspace.package] version` in `Cargo.toml`. `scripts/set-version.sh X.Y.Z` stamps it into:

- the root `package.json` (`@quran.ws/engine`)
- `packages/react-native/qvp-react-native/package.json`
- `packages/flutter/qvp_flutter/pubspec.yaml` and its `android/build.gradle`
- the Android library's default `qvpVersion`
- `MARKETING_VERSION` in `packages/ios/example/project.yml`

`scripts/check-versions.sh` fails when any of them differ. A wrapper with no changes still
takes the new number: its version means "binds header version X.Y.Z".

## What bumps what

| change | bump |
|---|---|
| a symbol added to the header | minor |
| a symbol renamed or removed | major (pre-1.0: minor, no alias) |
| behaviour change with no header change | minor |
| a fix | patch |

## Code and data tag families

- `vX.Y.Z` releases the code: every package at once.
- `data-vX.Y.Z` releases the default KFGQPC V4 page data.
- `data-hafs-qcf-v1-1405h-vX.Y.Z` releases the QCF V1 1405H page data.

Each data edition has its own cadence, version sequence, `VERSION.json`, CDN family,
and `latest.json`. The first default data release used the tag `v0.1.0` before the data prefix
existed. It is the only exception.

A wrapper declares the page format version it reads (`qvp_format_version()`), so an app
with old data keeps working after an engine update.

## Changelogs

`CHANGELOG.md` at the root follows Keep a Changelog, one section per `vX.Y.Z`, with a
per-package subsection when a package has something of its own to say. Each SDK package
also carries a `CHANGELOG.md` because its registry requires one. Copy those entries
from the root file. Do not write them separately.

## How it is checked

`scripts/check-versions.sh` in the `standards` CI job; `release.yml` refuses a tag whose
version does not match `Cargo.toml`.
