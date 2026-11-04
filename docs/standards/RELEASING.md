# Releasing

## A code release (`vX.Y.Z`)

1. On a release branch, run `scripts/set-version.sh X.Y.Z`, move the `Unreleased` section of
   `CHANGELOG.md` under the new version, and copy per-package entries into each package's
   changelog. Push the branch and open one PR.
2. When the code is final, run `scripts/prepare-ios-release.sh X.Y.Z` on the release branch.
   It starts `prepare-ios-release.yml` and waits for it. On a macOS runner, the workflow
   builds the XCFramework with the commands `ios.yml` runs and runs the Swift tests. Then it
   pushes one commit to the release branch. That commit sets the URL, the checksum and the
   build commit in the root `Package.swift`. Then it attaches the zip and its checksum to
   the draft release `vX.Y.Z`. Last, it starts `ci.yml` for the new commit. The script
   then pulls that commit, also after a failed run. No release artifact is built on a
   developer's machine.
3. Review the PR, merge it, then `git tag vX.Y.Z && git push --tags` on the merged commit.
   If the engine sources change after step 2, run step 2 again. `release.yml` refuses a
   tag whose engine sources differ from the build commit.
4. `release.yml` runs `scripts/check.sh` in full, verifies the prepared Apple binary, builds
   the other release artifacts, and publishes:
   - npm: `@quran.ws/engine`, `@quran.ws/qvp-react-native`
   - crates.io: `qvp-format`, `qvp-core`, `qvp-convert`, `qvp-ffi`
   - Maven Central: `ws.quran:qvp-android`
   - pub.dev: `qvp_flutter`
   - the Android AAR, XCFramework zip, and their checksums as release assets. `Package.swift`
     points at the prepared XCFramework URL and checksum
   The release notes are the changelog section.
5. Nobody publishes by hand. If a step fails, fix and re-run the workflow for the same
   tag. The workflow skips and reports any registry that refuses the same version twice.

SwiftPM reads the XCFramework URL and checksum from the commit that the tag points at. For
this reason the XCFramework is built before the tag, and the commit that sets the checksum
is part of the release PR. `scripts/package-ios.sh` writes the zip with sorted entries, one
fixed date and fixed file modes. The same XCFramework files then give the same checksum.
To repeat step 2, run the script again. The new run builds the current branch head, pushes a
new commit and replaces the assets of the draft release. The workflow refuses a run whose
commit is no longer the branch head. "Re-run jobs" therefore fails after the run pushed its
commit or after any other push. A push made with the workflow token does not start other
workflows, so `prepare-ios-release.yml` starts `ci.yml` for its commit. The required checks
of the PR then report on that commit.

crates.io, npm and pub.dev use GitHub trusted publishing with short-lived OpenID Connect
tokens. Each crate and each npm package trusts `.github/workflows/release.yml` of this
repository. The pub.dev package trusts this repository and the `v{{version}}` tag pattern.
None of the three needs a repository secret. The crates.io, npm and pub.dev steps are safe
to rerun for the same tag.

A registry failure keeps the workflow red, but does not block verified Wasm, Apple and Android
artifacts from reaching the CDN. CDN publishing verifies and skips an identical artifact family
on a retry.

Maven Central requires the `ws.quran` namespace to be verified in the Central Portal and four
repository secrets: `MAVEN_CENTRAL_USERNAME`, `MAVEN_CENTRAL_PASSWORD`, `MAVEN_SIGNING_KEY`, and
`MAVEN_SIGNING_PASSWORD`. The username and password are a Central Portal user token, and the
signing key is an ASCII-armored private PGP key whose public key is available from a key server.
The release key fingerprint is `5215 4C15 0932 3295 218D C5A3 C51E AFC2 1281 52F4`; it expires
on 2029-09-13 and must be replaced in GitHub before then.

## A data release (`data-vX.Y.Z`)

1. `scripts/sync-test-data.sh` fetches the `quran-svg-elements` bundle (`pages/`, `index/`); note its tag.
2. `QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test identity`: all 604 pages
   must pass the pixel gate.
3. `cargo run -p qvp-convert --release -- batch pages dist/pages`; this also writes the
   individual and combined QVP and SVG surah names, WOFF2 font, CSS and metadata map.
4. `scripts/package-data.sh X.Y.Z`: writes `VERSION.json` (data version, source bundle
   version, engine commit, format version, page and surah-name counts, and SHA-256 digests
   of every generated page, sidecar, atlas and surah-name asset), the upstream rights notice,
   and `dist/quran-engine-pages-hafs-kfgqpc.tar.gz` with its `.sha256`. With an ornament
   set at `dist/pages/ornaments.qvo`, set `QVP_ASSETS_COMMIT` to the quran-assets commit
   it was built from. The tarball then carries the set, and `scripts/sync-test-data.sh`
   uses that copy. In step 5, upload the same file as a separate asset beside the
   tarball, for users who fetch the set on its own.
5. `gh release create data-vX.Y.Z` with the tarball, the checksum, a sample page and, when
   there is one, `dist/pages/ornaments.qvo`.
   `publish-cdn.yml` mirrors it to `cdn.quran.ws/qvp/` under an immutable folder.

A published data version is never rewritten. Cut a new one.

## Before either

`scripts/check.sh` green locally, `CHANGELOG.md` has the section, no open issue carries the
`release-blocker` label.

## How it is checked

`release.yml` is the only publisher and refuses a tag that does not match `Cargo.toml`.
`publish-cdn.yml` refuses an existing prefix.

`prepare-ios-release.yml` refuses `main`, a version that does not match `Cargo.toml`, an
existing tag and a published release. Before it publishes the XCFramework, the
`xcframework` job of `release.yml` checks these conditions:

- the checksum in `Package.swift`, the checksum asset and the SHA-256 of the zip are equal
- the URL in `Package.swift` names the tag
- the zip is a valid archive
- `crates/`, `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `scripts/build-engine-ios.sh`
  and `scripts/package-ios.sh` at the tag are equal to those at the build commit that
  `Package.swift` records
