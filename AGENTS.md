# Agent notes for quran-engine

Read `CONTRIBUTING.md` first for the rules. This file holds only what an agent needs
beyond them: exact commands, invariants that are easy to break, and where to look.

## Commands

```sh
scripts/check.sh                      # everything CI runs
scripts/check.sh test                 # unit tests, no data
scripts/sync-test-data.sh             # pages/, index/ (quran-svg-elements) and dist/pages/ (data release)
scripts/check.sh gates                # identity gate (8 pages), line-shift gate, ABI test, conformance
QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test identity   # all 604 pages
cargo run -p qvp-convert --release -- batch pages dist/pages          # rebuild page data
cargo build -p qvp-ffi --release --target wasm32-unknown-unknown      # the wasm engine
scripts/build-engine-android.sh       # native libs for Android and Flutter
scripts/build-engine-ios.sh           # QvpEngine.xcframework
scripts/check-parity.py               # header vs Rust vs every wrapper
```

## Invariants that are easy to break

- The engine computes; wrappers marshal calls and render results. No formula in a wrapper
  (`docs/standards/API-DESIGN.md`). Its documented exception is the optional pure-JavaScript lite passage layout; keep it out of the default lite import.
- `crates/qvp-ffi/include/qvp.h` is hand-written and must match the `#[no_mangle]` set in
  `crates/qvp-ffi/src/lib.rs` exactly. `scripts/build-engine-android.sh` generates the Android copy at
  `packages/android/qvp/src/main/cpp/qvp.h`.
- The codec (`crates/qvp-format/src/codec.rs`) derives boxes and offsets at load and needs
  ops in canonical order (`PageData::canonicalize_ops`). Two boxes are stored, not derived:
  a glyph instance's and a glyph outline's. Changing the encoding means rerunning the full
  identity gate and cutting a new data release.
- Lossless only. Never simplify a curve.
- Never commit page data. Regenerating it would add about 92 MB to history each time.
- The optional `@quran.ws/engine/data` web helper owns verified release fetching and caches; see `docs/WEB-DATA.md`. The core and lite decoder stay fetch-free.
- The engine is URL-agnostic: every wrapper takes bytes or a path, never a base URL. The
  CDN (`docs/CDN.md`) mirrors the signed releases of this repo, quran-svg and
  quran-svg-elements, each under its own folder on `cdn.quran.ws`.
- Naming: `docs/standards/NAMING.md`. Quranic words: the Quran.ws terminology standard and
  `.terminology.json`. Run the audit before adding a word.
- Writing: docs, comments and PR text follow the plain-writing rules in
  `CONTRIBUTING.md`. Lint with the `ste-plain-writing` linter.
- Git: no tool attribution or session footers in commits or PRs.

## Where to look

| for | see |
|---|---|
| how the engine works | `docs/HOW-IT-WORKS.md` |
| the API | `docs/API.md`, `crates/qvp-ffi/include/qvp.h` |
| which wrapper binds what | `docs/API-PARITY.md` |
| source data problems | `docs/UPSTREAM-DATA-ISSUES.md` |
| platform toolchains | `docs/MACOS.md`, `packages/ios/AGENTS.md`, `packages/android/AGENTS.md` |
