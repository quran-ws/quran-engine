# quran-engine

A lossless vector engine for the word-by-word Uthmani mushaf. One Rust core, thin
wrappers per platform, rendered by each platform's own canvas.

- **Lossless.** Every page is converted from the source SVG with a pixel-diff gate
  (resvg, 4×) that all 604 pages pass. Coordinates are exact to 0.01 page unit.
- **Small.** A full page is ~135 KB raw / ~62 KB brotli (5.3× smaller than the SVG);
  the whole mushaf is 92.5 MB raw / **38.9 MB brotli**, sidecars and atlas included.
  Pages, atlas and text sidecars are data your app loads — no package bundles them.
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
| `packages/android` | Kotlin library (JNI over `qvp.h`) + demo app — built and verified on the emulator |
| `packages/flutter` | Dart FFI plugin + example app — 13 FFI tests, verified on the emulator |
| `packages/react-native` | `@quranpedia/qvp-react-native` (declarative props over the Kotlin library) + example — verified on the emulator |
| `packages/ios` | Swift package + demo — not yet built (needs a Mac; see `docs/MACOS.md` for the prompt) |
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
| page load + geometry | 1.6 ms |
| exact hit-test / gap-aware hit-test | 0.7 µs / 0.07 µs |
| full display list, 2 style rules | 0.2 µs |
| search "الله" over the page | 17 µs |
| six-line ayah highlight incl. band boxes | 48 µs |
| wasm engine | 273 KB |

## Page format

`NNN.qvp` stores only what cannot be worked out again. Bboxes, path origins and opcode
offsets are all derived at load; the opcode stream is split into packed opcodes and
separate x and y delta streams, which costs nothing raw and gives a compressor
homogeneous streams. Against storing the records as they sit in memory that is **27%
smaller raw and 19.6% smaller compressed** — 111.6 → 81.5 MB raw, 46.3 → 37.3 MB
brotli over the mushaf — paid for at load by rebuilding what was dropped, about
0.28 ms per page once decompression is counted.

## Data pipeline notes

The source data is the **`quran-svg hafs-kfgqpc` release bundle** (KFGQPC Madani
mushaf V4 1441H, production profile, schema `quran-svg/version` 1.0.0). Unpack it so
that `pages/` and `index/` sit side by side at the repo root — `batch` finds
`index/by-page` next to `pages/` on its own.

Issues found in the source are listed in `docs/UPSTREAM-DATA-ISSUES.md` and are fixed
upstream, never patched here. Production pages carry only `data-word-key` and
`data-rasm-uthmani`; the derived forms (`rasm_imlai`, `qpc`, `rasm`, `search`) come from
`index/by-page/NNN.json`, which the converter folds into `NNN.words.json` for apps to
attach at runtime via `page.attachWords(...)`.

Mark, family and category names are the bundle's own `mark-taxonomy` v2 vocabulary
(`schema/mark-taxonomy.json`), which follows the [Quran.ws terminology
standard](https://github.com/quran-ws/guidelines): `fathah`, `hamzat_al_wasl`,
`omitted_alif`, `rounded_zero`, `small_meem`, `waqf_jaiz_mustawi_al_tarafayn`. The same
names appear in the C ABI, every wrapper and `docs/API.md`, and
`crates/qvp-ffi/tests/abi.rs` gates them.
