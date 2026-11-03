# Measured, and the rough edges

Numbers you can reproduce on your own hardware, and the things that are known to be wrong.
What the engine *is* and how to use it lives on <https://quran.ws/docs/reference/quran-engine>;
this file is for people working on the engine itself.

## Measured

Page 042, the `dist` build (the shipped profile), Apple silicon — run it yourself and expect different
numbers on different hardware:

```sh
cargo run -p qvp-core --profile dist --example bench -- dist/pages/042.qvp
```

| | |
|---|---|
| page load + geometry | 2.4 ms |
| exact hit-test | 1.4 µs |
| gap-aware hit-test | 0.15 µs |
| full display list, 2 style rules | 0.24 µs |
| search `الله` over the page | 86 µs |
| six-line ayah highlight, bands included | 65 µs |
| printed layout | 0.60 µs |
| reflow layout, zoom 1.6 | 35 µs |
| continuous pinch, one frame of ten, page laid out again each frame | 58 µs |
| idle `tick`, nothing animating | 9 ns |
| `colors()` after the page settled | 3 ns |

The last five rows are the median of five runs. The reflow and pinch rows read the page's
reflow measure (`reflow::Prepared`), which the first reflow of a page builds once; the bench
excludes that first call.

The wasm engine, built with `--profile dist` (`opt-level = 3`, LTO, one codegen unit,
stripped) for `wasm32-unknown-unknown`, is 527,468 bytes (515 KiB). `opt-level = "s"`
gives 506,111 bytes and `"z"` 455,183 bytes, and both cost more time than they save in
bytes. The table is page 042 under Node 20 on Apple silicon, through `web/qvp.js`: the
median of seven runs, best of three rounds. Every `s` and `z` number is more than ten
percent slower than `3`, so the wasm stays at `opt-level = 3`.

| | `3` | `s` | `z` |
|---|---|---|---|
| size | 527,468 B | 506,111 B | 455,183 B |
| page load | 4.6 ms | 8.3 ms | 8.7 ms |
| 10,000 gap-aware hit tests | 25 ms | 43 ms | 46 ms |
| 1,000 draw-list calls | 25 ms | 38 ms | 84 ms |
| one six-line ayah highlight | 0.08 ms | 0.13 ms | 0.18 ms |

Cargo has no per-target profile setting, so a smaller wasm would need
`CARGO_PROFILE_DIST_OPT_LEVEL=s` in the wasm build commands, or a `rustflags` entry
for the target in `.cargo/config.toml`.

Android wrapper baseline for page 042 on the API 35 arm64 Android emulator
(`Google sdk_gphone64_arm64`), using debug instrumentation with the Rust core built with the `dist` profile:

| | |
|---|---|
| `QvpPage` load + metadata copy | 1.36–1.90 ms median |
| Android `Path` construction | 2.62–3.71 ms median |
| first 1080 × 1920 software Canvas draw | 14.37–17.29 ms median |
| cached Canvas draw | 0.57–0.59 ms median |

Run `scripts/sync-android-test-data.sh`, then
`./gradlew :qvp:connectedDebugAndroidTest` from `packages/android` to reproduce it.
`QvpAndroidBenchmarkTest` emits the raw JSON under the `QvpBenchmark` log tag. These are a
development baseline, not physical-device numbers; comparisons need the same device and build.

The reason for the engine is not this table. It is that a fully split page is
hundreds of kilobytes of vector paths, and a phone cannot hold 604 of them in a
DOM and stay responsive.

## Known rough edges

- `surahs()` returns a `bannerDecoration` field that `docs/API.md` does not list; it
  indexes into `page.decorations`.
- npm, crates.io, pub.dev and Maven Central publish from the tag workflow; the first three
  through trusted publishing.
  Every package carries the workspace version in `Cargo.toml`, 0.7.0 today: Android from Maven
  Central, iOS through Swift Package Manager.
