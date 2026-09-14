# Releasing

## A code release (`vX.Y.Z`)

1. `scripts/set-version.sh X.Y.Z`, move the `Unreleased` section of `CHANGELOG.md` under the
   new version, copy per-package entries into each package's changelog. One PR.
2. Merge, then `git tag vX.Y.Z && git push --tags`.
3. `release.yml` runs `scripts/check.sh` in full, builds the native libraries for every
   target, and publishes:
   - npm: `@quran.ws/engine`, `@quran.ws/qvp-react-native`
   - crates.io: `qvp-format`, `qvp-core`, `qvp-convert`, `qvp-ffi`
   - the Android AAR, XCFramework zip, and their checksums as release assets. `Package.swift`
     points at the XCFramework URL and checksum
   The release notes are the changelog section.
4. Nobody publishes by hand. If a step fails, fix and re-run the workflow for the same
   tag. The workflow skips and reports any registry that refuses the same version twice.

Maven Central and pub.dev are not wired yet. Until a registry is live, its release step is
a no-op that says so.

## A data release (`data-vX.Y.Z`)

1. `scripts/sync-test-data.sh` fetches the `quran-svg-elements` bundle (`pages/`, `index/`); note its tag.
2. `QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test identity`: all 604 pages
   must pass the pixel gate.
3. `cargo run -p qvp-convert --release -- batch pages dist/pages`
4. `scripts/package-data.sh X.Y.Z`: writes `VERSION.json` (data version, source bundle
   version, engine commit, format version, page count, sha256 of every file), the upstream
   rights notice, and `dist/quran-engine-pages-hafs-kfgqpc.tar.gz` with its `.sha256`.
5. `gh release create data-vX.Y.Z` with the tarball, the checksum and a sample page.
   `publish-cdn.yml` mirrors it to `qvp.quran.ws` under an immutable prefix.

A published data version is never rewritten. Cut a new one.

## Before either

`scripts/check.sh` green locally, `CHANGELOG.md` has the section, no open issue carries the
`release-blocker` label.

## How it is checked

`release.yml` is the only publisher and refuses a tag that does not match `Cargo.toml`.
`publish-cdn.yml` refuses an existing prefix.
