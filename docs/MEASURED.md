# Measured, and the rough edges

Numbers you can reproduce on your own hardware, and the things that are known to be wrong.
What the engine *is* and how to use it lives on <https://quran.ws/docs/reference/quran-engine>;
this file is for people working on the engine itself.

## Measured

Page 042, release build, Apple silicon — run it yourself and expect different
numbers on different hardware:

```sh
cargo run -p qvp-core --release --example bench -- dist/pages/042.qvp
```

| | |
|---|---|
| page load + geometry | 2.4 ms |
| exact hit-test | 1.4 µs |
| gap-aware hit-test | 0.15 µs |
| full display list, 2 style rules | 0.24 µs |
| search `الله` over the page | 86 µs |
| six-line ayah highlight, bands included | 65 µs |

Android wrapper baseline for page 042 on the API 35 arm64 Android emulator
(`Google sdk_gphone64_arm64`), using debug instrumentation with the release-built Rust core:

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
  Android 0.3.0 is available from Maven Central, and iOS 0.3.0 is available through Swift Package
  Manager.
