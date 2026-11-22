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
| cached Canvas draw, one ayah highlighted and three words blocked | 4.25–5.14 ms median |
| objects allocated per cached draw, with or without the highlight | 0 |

A cached draw is a frame where nothing changed. Before the views kept what they read from
the engine, the same frame allocated 19 objects, or 1,071 with the highlight and the blocks.
Three runs each on one emulator, while other work shared the machine, gave a highlighted
frame 5.09–8.72 ms before and 4.25–5.14 ms after. The bitmap blit and the highlighted ink
take most of a frame, so the plain cached draw moved less than the runs varied
(0.92–1.17 ms before, 0.82–1.15 ms after).

Run `scripts/sync-android-test-data.sh`, then
`./gradlew :qvp:connectedDebugAndroidTest` from `packages/android` to reproduce it.
`QvpAndroidBenchmarkTest` emits the raw JSON under the `QvpBenchmark` log tag. These are a
development baseline, not physical-device numbers; comparisons need the same device and build.

The reason for the engine is not this table. It is that a fully split page is
hundreds of kilobytes of vector paths, and a phone cannot hold 604 of them in a
DOM and stay responsive.

## The letter layer

The letter layer (`NNN.qvl`, `docs/FORMAT.md`) is an optional file per page. It gives the
letters of each word and the paths that draw each letter. `docs/DATA.md` says where it
comes from and what its limits are. The figures below are for the 604 layers that
`qvp-convert letters` builds from the letters archive of `quran-svg-elements` v2.0.0. For
scale, the table also gives the 604 page files that `qvp-convert batch` builds from the
words archive of the same release.

| | `NNN.qvl`, 604 files | `NNN.qvp`, 604 files |
|---|---|---|
| raw | 37,200,857 B | 81,218,231 B |
| each file after `gzip -9`, summed | 30,001,948 B | 41,632,217 B |
| each file after `brotli -q 11`, summed | 25,946,365 B | 37,249,927 B |
| smallest file | page 001, 16,045 B | page 001, 37,486 B |
| largest file | page 552, 81,582 B | page 552, 170,465 B |

The layers hold 333,220 letters and 261,953 pieces. A piece is a part of a joined outline
that the converter cut out for one letter. The pieces come from 89,853 page paths. Raw,
the layers are 45.8% of the size of the page files. After `gzip -9` they are 72.1%,
because gzip makes a layer only 19.4% smaller and a page file 48.7% smaller. After brotli
at quality 11 they are 69.7%: brotli makes a layer 30.3% smaller and a page file 54.1%
smaller. The brotli figures come from the Python `brotli` module, version 1.1.0, one file
at a time:

```sh
python3 -c 'import brotli,glob,sys; print(sum(len(brotli.compress(open(f,"rb").read(), quality=11)) for f in glob.glob(sys.argv[1])))' 'dist/letters/[0-9][0-9][0-9].qvl'
```

```sh
cargo run -p qvp-convert --release -- letters pages-letters pages dist/letters
cargo run -p qvp-convert --release -- batch pages <pages_dir>
cat dist/letters/*.qvl | wc -c
t=0; for f in dist/letters/*.qvl; do t=$((t + $(gzip -9 -n -c "$f" | wc -c))); done; echo $t
stat -c '%s %n' dist/letters/*.qvl | sort -n | sed -n '1p;$p'
```

Repeat the last three commands with `<pages_dir>/*.qvp` for the page column. On an Intel
Core i9-13900 with 32 threads, the subcommand built all 604 layers in 1.00 s to 2.58 s
wall clock over six runs, with the binary already built.

The ink gate (`cargo test -p qvp-convert --release --test letters ink`, with
`QVP_REQUIRE_LETTERS=1 QVP_TEST_ALL=1` for all pages) draws each page twice from its
pieces and compares it with the page. It uses the metric of the identity gate: a 4×
render, a pixel differs when its alpha differs by more than 48, and a page fails when more
than 0.05% of its pixels differ. That is 1,518 of 3,036,000 pixels.

| measure | pages over the limit | differing pixels, 604 pages | worst page |
|---|---|---|---|
| each piece as its own path (asserted) | 0 | 350,588 | 516 and 552, 765 px (0.0252%) |
| the pieces of one path as one path (reported) | 0 | 4,196 | 022, 20 px |

When each piece is its own path, 347,129 differing pixels (99.0%) are within 1.5 px of a
cut. The other 3,459 are near an edge that a cut made again, and none are elsewhere. The
worst page uses half of what the gate allows. When the pieces of one path are one path,
only 737 differing pixels are near a cut, because the seam at the cuts is gone. The other
3,459 are near an edge that a cut made again.

The gate also compares areas. For each of the 89,853 cut paths, it takes the absolute
difference between the area of the path and the area of its pieces. The sum of these
differences is 1,466.9 square units, 0.0244% of the 6,003,600 square units of the paths.
`quran-svg-elements` states that the edge of a cut piece is less than 0.0125 units from
the artwork. The gate asserts three things for each cut path:

- The area of its pieces is within 0.0125 units × L of the area of the path. L is the
  length of the edges that a cut made again near the outline of the path. Near means
  within 0.028 units: 0.0125 units, 0.0141 units for the rounding of the path and of the
  piece, and the error of the measure. The worst path uses 0.491 of this limit (page 381,
  path 810).
- Every other new edge that a piece draws once is a side of a cut. A side of another
  piece of the same path must run along it, within 0.028 units. 116 such sides in 32
  paths all have one. The farthest is 0.0200 units away.
- A subpath inside another subpath winds the other way: 139,403 do, and none winds the
  same way.

One path is an exception, and the gate expects its exact difference. On page 001, the
piece of word 1:2:1 leaves out a closed subpath of three points. The subpath is 0.05 by
0.01 units, with an area of 0.00025 square units.

A test changes the pieces of each path in turn (`QVP_LETTERS_SWEEP=1`). Of 261,874 lost
pieces, 25 still pass, and the most ink one of them moves is 0.034 square units. Of
261,953 doubled pieces, 16 pass, up to 0.135 square units. No piece replaced by its
neighbour passes. With 32 threads the gate took 16.2 s for 604 pages.

## Known rough edges

- `surahs()` returns a `bannerDecoration` field that `docs/API.md` does not list; it
  indexes into `page.decorations`.
- npm, crates.io, pub.dev and Maven Central publish from the tag workflow; the first three
  through trusted publishing.
  Every package carries the workspace version in `Cargo.toml`, 0.8.0 today: Android from Maven
  Central, iOS through Swift Package Manager.
