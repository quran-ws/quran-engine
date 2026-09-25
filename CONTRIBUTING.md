# Contributing

This page describes how the repository is organised, how to build and test it, and what
a change must include. The rules live in `docs/standards/`. This
page links them and gives the short version.

## The architecture

One Rust core computes everything: hit testing, layout, styles, highlights, selection,
masks, search, crop, atlas. The C header `crates/qvp-ffi/include/qvp.h` is the only
contract. The five wrappers (web, Android, Flutter, React Native, iOS) marshal calls and
render results, and compute nothing themselves. The optional JavaScript lite passage layout is a narrow exception for apps that must not load Wasm; `docs/LITE-PASSAGES.md` states its scope and checks. Read `docs/HOW-IT-WORKS.md` first if the engine is new to you.

## First ten minutes

```sh
git clone https://github.com/quran-ws/quran-engine.git && cd quran-engine
scripts/check.sh test          # unit tests, no data needed
scripts/sync-test-data.sh      # the quran-svg-elements bundle + the page data release
scripts/check.sh gates         # identity gate, ABI test, conformance
```

`scripts/check.sh` with no argument runs everything CI runs. If it is green locally, it is
green in CI. Platform builds need more tools. See `docs/MACOS.md` and each package's
`AGENTS.md` for the exact versions.

## Where things live

`docs/standards/STRUCTURE.md` has the full map. In short:

| path | what |
|---|---|
| `crates/qvp-format` | the page and atlas formats and their codec |
| `crates/qvp-core` | the engine |
| `crates/qvp-convert` | SVG to QVP converter and the pixel-identity gate |
| `crates/qvp-ffi` | the C ABI and `include/qvp.h` |
| `web/` | the reference wrapper `qvp.js`, the lite decoder, the web example |
| `packages/<platform>/` | one SDK and one example per platform |
| `conformance/` | fixtures and scenarios every wrapper must match |
| `docs/` | reference, standards, design history |
| `scripts/` | build, package, publish, sync and check scripts |

## The standards

| document | covers |
|---|---|
| `docs/standards/NAMING.md` | every name, in every language |
| `docs/standards/API-DESIGN.md` | the shape of the public surface, the three tiers, no formulas in wrappers |
| `docs/standards/CODE-STYLE.md` | comments, doc blocks, file order |
| `docs/standards/STRUCTURE.md` | where each thing lives |
| `docs/standards/TESTING.md` | what a change ships with |
| `docs/standards/VERSIONING.md`, `RELEASING.md` | one version, two tag families, how a release happens |

Quranic terms follow the Quran.ws terminology standard. `.terminology.json` records what
this repository has adopted. Run the terminology audit before you add a word.

## Adding an engine function

Bind every engine function in every wrapper, in this order, in one pull request:

1. `crates/qvp-core`: the logic and a unit test on a synthesised page.
2. `crates/qvp-ffi/src/lib.rs` and `include/qvp.h`: the symbol, with a doc block above it.
3. `crates/qvp-ffi/tests/abi.rs`: a call on a real page.
4. `web/qvp.js`: the reference binding. Its name is the name.
5. Kotlin: `packages/android/qvp/src/main/cpp/qvp_jni.c`, `QvpNative.kt`, `QvpPage.kt`.
6. Dart: `packages/flutter/qvp_flutter/lib/src/bindings.dart` and `engine.dart`.
7. Swift: `packages/ios/QvpKit/Sources/QvpKit/QvpPage.swift`, then
   `scripts/build-engine-ios.sh`.
8. React Native: the Kotlin bridge and `src/index.tsx`.
9. `docs/API.md` and a line in `CHANGELOG.md` under Unreleased.

If a platform must lag, add the symbol to `docs/API-PARITY.md` as a declared gap with an
issue number. CI fails on an undeclared gap.

## Three tiers of change

State the tier in the pull request:

- **Core behaviour**: changed in Rust once. Every platform receives it with the next
  native build.
- **Engine API**: a C symbol. Bound everywhere or declared as a gap.
- **Lite passage layout**: JavaScript-only excerpt layout in `web/lite-passage.mjs`, with no C ABI change. Keep it separately importable and test it against the codec fixtures, Rust measurements and complete page data. Do not duplicate unrelated engine systems.
- **Platform convenience**: one platform's idiom. Documented in that package's README.
  Must not compute anything the engine can compute. A fit scale, a clamp or a table of
  names is such a computation.

## Lossless, always

The converter and the format never simplify a curve or drop a point. Every page passes a
pixel-diff gate against the source SVG. A source problem goes in
`docs/UPSTREAM-DATA-ISSUES.md`. The converter does not patch data.

The optional web data helper (`docs/WEB-DATA.md`) owns release URLs, integrity checks and caches outside the engine. Its small release descriptor and canonical surah metadata are packaged; Quran text and artwork are fetched, never committed. This web-only convenience has no C ABI or parity changes.

## Data is not code

Packages ship code only. Apps load page data (`NNN.qvp`, `atlas.qva`,
`NNN.words.json`, and the generated `surah-names/` tree).
The built data is published as a GitHub release with a CDN mirror and is not committed.
Examples bundle a sample as gitignored assets.

## Pull requests

- One topic per PR. Mechanical moves and renames in their own commits.
- `scripts/check.sh` green.
- The PR template asks for the tier, the tests, and the changelog line.
- Commit messages say what changed and why, in plain sentences. No tool attribution
  footers.
- Changes to the header, the FFI crate or the standards need a maintainer review
  (`CODEOWNERS`).

## Writing

Docs, comments, changelog entries and PR text use short sentences, active voice, one
meaning per word, and define a project or domain term at first use. The
`ste-plain-writing` linter (`python3 scripts/ste_lint.py <file>`) is the check.

## Reporting

Bugs and API proposals: GitHub issues, using the templates. Security: `SECURITY.md`.
Conduct: `CODE_OF_CONDUCT.md`.
