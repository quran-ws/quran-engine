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
| raw | 8,262,505 B | 81,218,231 B |
| each file after `gzip -9`, summed | 4,647,264 B | 41,632,217 B |
| each file after `brotli -q 11`, summed | 3,926,524 B | 37,249,927 B |
| smallest file | page 001, 3,690 B | page 001, 37,486 B |
| largest file | page 552, 17,057 B | page 552, 170,465 B |

The layers hold 333,220 letters and 261,953 pieces. A piece is a part of a joined outline
that the converter cut out for one letter. The pieces come from 89,853 page paths. Raw,
the layers are 10.2% of the size of the page files. After `gzip -9` they are 11.2%: gzip
makes a layer 43.8% smaller and a page file 48.7% smaller. After brotli at quality 11 they
are 10.5%: brotli makes a layer 52.5% smaller and a page file 54.1% smaller. The brotli
figures come from the Python `brotli` module, version 1.1.0, one file at a time:

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
Core i9-13900 with 32 threads, the subcommand built all 604 layers in 1.59 s to 1.71 s
wall clock over six runs, with the binary already built.

A layer codes each piece against its source path in the page (`docs/FORMAT.md`). The coded
pieces are lossless: the structure gate decodes every layer and compares each command of
each piece. Before: a first encoding, never released, stored every piece as plain
coordinate deltas, and its 604 files took 37,200,857 B raw and 25,946,365 B after brotli.

Each layer starts with a 72-byte header that holds two 8-byte digests. Over 604 files the
headers take 43,488 B of the raw total, and the digests 9,664 B of it.

Decoding a layer costs more than it did, because the decoder lists the segments of each
source and computes each part of a segment. Over the 604 pages, on one thread of the same
machine, `decode_letters` takes a median 0.484 ms per page (90th percentile 0.547 ms,
largest 0.669 ms). The plain-delta decoder took a median 0.188 ms. Each figure is the
median of 9 decodes of each page, in a release build, with the page already decoded.

The decode time includes the page digest, which the decoder takes from the decoded page
when a layer is attached: a median 0.042 ms per page, and 0.053 ms on page 552, the
largest page (the median of 41 runs of each page). Loading a page does not take it:
`Page::load` costs what it cost before the letter layer, a median 1.03 ms over pages 001,
050, 187, 300, 440, 552 and 604 (300 runs of each).

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
the artwork. The gate asserts four things for each cut path:

- The area of its pieces is within 0.0125 units × L of the area of the path. L is the
  length of the edges that a cut made again near the outline of the path. Near means
  within 0.028 units: 0.0125 units, 0.0141 units for the rounding of the path and of the
  piece, and the error of the measure. The worst path uses 0.491 of this limit (page 381,
  path 810).
- Every other new edge that a piece draws once is a side of a cut. A side of another
  piece of the same path must run along it, within 0.028 units. 116 such sides in 32
  paths all have one. The farthest is 0.0200 units away.
- Where two pieces of the path meet along a cut, they draw the cut in opposite
  directions. All 319,037 such cuts do. A copy of a piece draws its new edges a second
  time in the same direction, so this rule finds a doubled piece.
- A subpath inside another subpath winds the other way: 139,403 do, and none winds the
  same way.

One path is an exception, and the gate expects its exact difference. On page 001, the
piece of word 1:2:1 leaves out a closed subpath of three points. The subpath is 0.05 by
0.01 units, with an area of 0.00025 square units.

A test changes the pieces of each path in turn (`QVP_LETTERS_SWEEP=1`). Of 261,874 lost
pieces, 25 still pass, and the most ink one of them moves is 0.034 square units. No
doubled piece of the 261,953 passes, and no piece replaced by its neighbour passes.

On the Intel Core i9-13900 with 32 threads, the `ink` test alone took 17.7 s to 19.2 s
for 604 pages over three runs. The whole test file, with the default test harness, took
20.2 s to 24.2 s. The harness runs the four tests of the file at the same time. The
sweep test alone, with `QVP_LETTERS_SWEEP=1`, took 11.5 s to 12.5 s.

The engine gate (`cargo test -p qvp-convert --release --test letters_engine`, with
`QVP_REQUIRE_LETTERS=1 QVP_TEST_ALL=1` for all pages) attaches each layer to its page in the
engine. Its totals on all 604 pages, each asserted:

| check | count |
|---|---|
| pages whose draw list, colours and own geometry rows the attach leaves unchanged | 604 |
| letters whose offset points at their own character in the `qpc` text | 333,220 |
| letters whose body and dots part takes the paths of the character range over their offset | 333,220 |
| marks taken by the nth-mark selector, in text order | 430,453 |
| cut paths swapped for their pieces by a colour on one letter, one path at a time | 12,067 |
| cut paths drawn whole because every piece is of the coloured letter | 13 |
| cut paths swapped by a colour on one letter of every word | 73,793 |
| body members hit at a point inside them and inside no other ink | 335,548 (every one of the 261,953 pieces among them), 0 skipped |
| printed ink points on a 0.5-unit grid, each an exact hit in the ink's word | 39,674,994, of which 23,379 seams |
| the same points reflowed at zoom 2, through each word's placement | 39,674,298, of which 23,333 seams; 696 moved off the ink by the round trip and not tested |
| counters, each a near hit on its own letter | 104,835, 0 with no clear point |

The first run of the counter check failed on 89 counters on 78 pages, where the box of a
neighbouring word reaches over the counter. The hit test now gives a point in a counter
that counter's letter (`docs/API.md`, "The letter under a point"). On the Intel Core
i9-13900, the gate took 24.3 s for 604 pages and about 2 s for the default 8 pages.

What a layer costs the engine on page 003, in a release build on the same machine, pinned
to one core with no other work running: the median of five runs, each the median of 20,000
calls. A draw-list build is forced by a band that changes on every call.

| | |
|---|---|
| a settled `tick`, with or without a layer and letter rules | 13 ns |
| draw-list build, no layer | 1.95 µs |
| draw-list build, a layer and no letter rule | 3.02 µs |
| draw-list build, 400 letter rules, 194 cut paths drawn as pieces | 3.53 µs |
| letter hit on ink | 362 ns |
| letter hit in a gap between words | 340 ns |
| word hit on the same ink point, for scale | 437 ns |

A page with no layer costs what it cost before the letter layer. The table compares page
042 in the engine before the letter layer and in the engine now, built with the settings of
the `dist` profile and pinned to one core: the median of nine runs of each, run in turn,
with the lowest and highest run.

| page 042, no layer | before the letter layer | now |
|---|---|---|
| page load + geometry | 1.18 ms (1.17–1.23) | 1.18 ms (1.17–1.20) |
| colours after one word rule | 4.43 µs (4.34–4.55) | 3.98 µs (3.92–4.17) |
| colours after a theme | 15.1 µs (15.0–15.4) | 15.0 µs (14.8–15.3) |
| first `tick` after an ayah highlight | 39.7 µs (39.6–40.5) | 39.8 µs (39.6–40.1) |
| `styled_paths` | 0.88 µs (0.86–0.90) | 0.82 µs (0.81–0.83) |
| draw-list build, printed | 1.07 µs (1.04–1.17) | 1.05 µs (1.05–1.19) |
| draw-list build, reflowed at zoom 1.6 | 1.02 µs (1.02–1.09) | 1.02 µs (1.02–1.08) |
| idle `tick` | 2.3 ns (2.3–2.5) | 1.9 ns (1.9–1.9) |
| exact hit test | 658 ns (653–710) | 652 ns (647–661) |
| gap-aware hit test | 447 ns (444–469) | 443 ns (441–449) |

The same code can measure up to 18% apart in two binaries, because the compiler lays the
code out differently: `styled_paths` of the old engine measures 0.75 µs in the bench example
and 0.88 µs in the program above.

## Known rough edges

- `surahs()` returns a `bannerDecoration` field that `docs/API.md` does not list; it
  indexes into `page.decorations`.
- npm, crates.io, pub.dev and Maven Central publish from the tag workflow; the first three
  through trusted publishing.
  Every package carries the workspace version in `Cargo.toml`, 0.11.0 today: Android from Maven
  Central, iOS through Swift Package Manager.
