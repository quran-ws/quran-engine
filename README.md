# quran-engine

A lossless vector engine for the word-by-word Uthmani mushaf. One Rust core, thin
wrappers per platform, rendered by each platform's own canvas.

- **Lossless.** Every page is converted from the source SVG with a pixel-diff gate
  (resvg, 4×) that all 604 pages pass. Coordinates are exact to 0.01 page unit.
- **Small.** A full page is ~210 KB raw / ~85 KB compressed (4× smaller than the SVG);
  the whole mushaf is ~50 MB compressed. Pages, atlas and text sidecars are data your app
  loads — no package bundles them.
- **The engine decides, the host draws.** Hit-testing (gap-aware, exact outlines),
  layout (line spacing, fill-height, padding), layered styles with handles and
  transitions down to *one diacritic of one word*, animated highlight bands, selection,
  masking and reveal for memorisation, search with Arabic normalisation, crop-to-SVG,
  metadata and the cross-page atlas all live in the core. A wrapper marshals and paints.
- **One API everywhere.** `docs/API.md` documents it once; `web/qvp.js` is the reference
  wrapper, the Kotlin, Dart, React Native and Swift wrappers mirror its names.

```
pages/*.svg ──qvp-convert──▶ NNN.qvp · atlas.qva · NNN.words.json (data, shipped by the app)
                                      │
                      qvp-core (Rust) ─┤  loader · geometry · hit-test · layout · styles · highlights
                                      │  selection · mask/reveal · text/search · crop · atlas
                      qvp-ffi (C ABI) ─┤  libqvp_ffi.{so,a,dylib} · qvp_ffi.wasm · include/qvp.h
                                      │
   web/qvp.js (Canvas2D) · packages/android (Kotlin, JNI) · packages/flutter (Dart FFI) ·
   packages/react-native · packages/ios (Swift)  ← thin wrappers + demos
```

## Repository

| path | what |
|---|---|
| `crates/qvp-format` | QVP1 page format, QVA1 atlas format, codec, mark taxonomy |
| `crates/qvp-convert` | `svg2qvp`, `qvp2svg`, `batch` (all pages + atlas + sidecars), identity test |
| `crates/qvp-core` | the engine (see `docs/API.md`) |
| `crates/qvp-ffi` | C ABI: `include/qvp.h`, native + wasm builds, ABI smoke test |
| `web/` | reference wrapper `qvp.js`, Canvas2D renderer, demo app, single-file build |
| `packages/android` | Kotlin library (JNI over `qvp.h`) + demo app |
| `packages/flutter` | Dart FFI plugin + example app |
| `packages/react-native` | native view module + example app |
| `docs/` | `API.md`, design spec, `UPSTREAM-DATA-ISSUES.md` (for the exporter team), `MACOS.md` |
| `scripts/` | `build-engine-android.sh` |

## Build the engine

```sh
curl -sSf https://sh.rustup.rs | sh -s -- -y -t wasm32-unknown-unknown      # once
cargo test --workspace --release                    # unit tests + ABI test + identity gate (8 pages)
QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test identity        # all 604 pages
cargo run -p qvp-convert --release -- batch pages dist/pages              # NNN.qvp, NNN.words.json, atlas.qva/json
cargo build -p qvp-ffi --release                                          # target/release/libqvp_ffi.{so,a}
cargo build -p qvp-ffi --release --target wasm32-unknown-unknown          # target/wasm32-unknown-unknown/release/qvp_ffi.wasm
scripts/build-engine-android.sh                                           # arm64-v8a / x86_64 / armeabi-v7a
```

## Web demo

```sh
cp target/wasm32-unknown-unknown/release/qvp_ffi.wasm web/ && cp dist/pages/* web/pages/
python3 web/build.py dev && (cd web && python3 -m http.server 8765)      # http://127.0.0.1:8765/?p=42
python3 web/build.py embed 1-21,440-445,582,604                           # dist/demo.html, single file
```

## Measured (page 042, release build)

| | |
|---|---|
| page load + geometry | 1 ms |
| exact hit-test / gap-aware hit-test | 0.7 µs / 0.07 µs |
| full display list, 2 style rules | 0.2 µs |
| search "الله" over the page | 17 µs |
| six-line ayah highlight incl. band boxes | 48 µs |
| wasm engine | 265 KB |

## Data pipeline notes

Source SVGs come from the exporter; issues found in them are listed in
`docs/UPSTREAM-DATA-ISSUES.md` and are fixed upstream, never patched here. Production
pages will carry only the uthmani text; derived forms (imlaei, qpc, rasm, search) are
attached at runtime from `NNN.words.json` via `page.attachWords(...)`.
