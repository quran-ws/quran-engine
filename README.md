# quran-engine — QVP vector mushaf engine

One Rust core, thin wrappers. Word-by-word Uthmani mushaf pages as a compact
lossless binary vector format (**QVP**), rendered by the host canvas of each
platform, with real-time per-word / per-mark styling and exact hit-testing.

```
pages/*.svg ──qvp-convert──▶ dist/pages/NNN.qvp  (+ NNN.words.json text index)
                                     │
                     qvp-core (Rust) ─┤ loader · geometry · hit-test · style state · display list
                                     │
                     qvp-ffi (C ABI) ─┤ libqvp_ffi.{so,a,dylib} / qvp_ffi.wasm  — include/qvp.h
                                     │
        web/qvp.js (Canvas2D) · Swift · Kotlin/JNI · Dart FFI · RN JSI  ← wrappers draw, core decides
```

## Status (2026-09-04)

| Piece | State |
|---|---|
| `crates/qvp-format` | Done. Versioned binary, opcode stream with zigzag-varint deltas, glyph instances. Unit-tested. |
| `crates/qvp-convert` | Done. All 604 pages convert with 0 errors. `svg2qvp`, `qvp2svg`, `batch`, `info`. |
| Identity gate | Done. `tests/identity.rs`: SVG → QVP → SVG, resvg at 4×, pixel diff. All 604 pages pass. |
| `crates/qvp-core` | Done. Load ≈ 1 ms/page, hit-test ≈ 1 µs, display list ≈ 60 µs. Unit-tested. |
| `crates/qvp-ffi` | Done. 24 C functions, `include/qvp.h`. Builds native (.so/.a) and wasm (65 KB). |
| Web wrapper + demo | Done. `web/` — Canvas2D host renderer with cached base layer + overlay. |
| iOS / Android / Flutter / RN wrappers | Not built (no macOS / Android SDK here). They consume `qvp.h` unchanged. |

## Numbers (measured)

| | Whole mushaf | Full page (036) |
|---|---|---|
| Source SVG | 453 MB | 841 KB |
| QVP raw | 111 MB | 210 KB |
| QVP brotli | ≈ 50 MB | ≈ 85 KB |

Lossless: coordinates are quantised to 0.01 page unit (0.04 px at 4× raster);
the identity test shows ≤ 20 sub-tolerance edge pixels per page out of 3 M.

## Build

```sh
curl -sSf https://sh.rustup.rs | sh -s -- -y -t wasm32-unknown-unknown   # once
cargo test --workspace --release                                          # unit tests + identity gate (8 sample pages)
QVP_TEST_ALL=1 cargo test -p qvp-convert --release --test identity        # all 604 pages (~1 min on 32 cores)
cargo run -p qvp-convert --release -- batch pages dist/pages              # convert everything
cargo run -p qvp-core --release --example bench -- dist/pages/036.qvp     # engine numbers
cargo build -p qvp-ffi --release                                          # target/release/libqvp_ffi.{so,a}
cargo build -p qvp-ffi --release --target wasm32-unknown-unknown          # target/wasm32-unknown-unknown/release/qvp_ffi.wasm
```

## Web demo

```sh
cp target/wasm32-unknown-unknown/release/qvp_ffi.wasm web/ && cp dist/pages/* web/pages/
python3 web/build.py dev && (cd web && python3 -m http.server 8765)      # http://127.0.0.1:8765/?p=36
python3 web/build.py embed 1-21,440-445,582,604                           # dist/demo.html, single file, 29 pages inlined
```

The demo drives everything through the engine's style state: word / ayah
selection, mark-family colours, hiding marks, gold ayah markers, path-level
recolouring, word-by-word playback, themes, pan/zoom.

## Format (QVP1)

See `docs/superpowers/specs/2026-09-04-quran-vector-engine-design.md` and the
doc comments in `crates/qvp-format/src/lib.rs`. Upstream data issues found on
the way are listed in `docs/UPSTREAM-DATA-ISSUES.md`.
