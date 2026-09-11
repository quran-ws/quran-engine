# Quran Engine

**SVG does not perform on mobile. This does.** The same interactive muṣḥaf page —
one printed page of the Qur'an — as a compact binary, with hit-testing, layout,
styling and search inside the engine, drawn by each platform's own canvas.

| Package | Version | Whole muṣḥaf | Engine |
|---|---|---|---|
| not published yet — build from source | 0.1.0 | 604 pages · 92.5 MB raw · **38.9 MB brotli** | 280 KB wasm |

<sub>Sizes measured over the `v0.1.0` page data (`node` brotli-11, quality 11) and over
`qvp_ffi.wasm` built from this repository at `3b17fc7`.</sub>

A *muṣḥaf* is a printed copy of the Qur'an, and its page breaks belong to that
edition, not to the Qur'an itself —
[glossary](https://quran.ws/docs/concepts/glossary/#mushaf).

## What it provides

- **604 printed pages as `NNN.qvp`**, converted from the split SVG artwork with a
  pixel-diff gate (resvg, 4×) that every page passes; coordinates stay exact to
  0.01 page unit.
- **A Rust core, one C ABI, thin wrappers.** Hit-testing, layout, layered styles,
  highlight bands, selection, masking, search, crop-to-SVG and the cross-page
  atlas all live in the core. A wrapper marshals and paints.
- **Word-level addressing that costs microseconds.** Exact-outline hit-testing
  with a gap-aware fallback, so a tap between two words resolves the way a reader
  expects rather than landing nowhere.
- **Styling down to one diacritic of one word**, in layers, with handles — undo
  exactly the call you made, never a global clear.
- **Text without a database.** Five text forms per word from the sidecars,
  Arabic-normalised search, citations, surah and juzʾ metadata, screen-reader
  labels.

## Use it when you need

- A mobile or native app, where shipping and rendering 604 split SVG pages is too
  large and too slow.
- The same word-level behaviour across Web, Android, Flutter and React Native
  from one API and one set of names.
- Memorisation tools that hide and reveal words in place, on the page itself.
- Following recitation word by word, where every frame costs a hit-test and a
  highlight move.

## Not for

| If you want | Use |
|---|---|
| A page in a browser, where the split SVGs already render fast enough | [Quran SVG Elements](https://github.com/quran-ws/quran-svg-elements) |
| A muṣḥaf that has not been split into elements — most of the archive | [Quran SVG](https://github.com/quran-ws/quran-svg) |
| Showing a page and tapping whole ayat, with no library at all | [Quran SVG](https://github.com/quran-ws/quran-svg) |
| The Qur'anic text itself, as text you can query | [Quran Text](https://github.com/quran-ws/quran-text) |

Three blocks render printed pages, and they are not a queue. **Coverage narrows
as addressing deepens:**

| | Covers | You can address |
|---|---|---|
| Quran SVG | every vectorised muṣḥaf, and growing by contribution | an ayah |
| Quran SVG Elements | only the muṣḥafs that have been split | a word, a mark |
| **Quran Engine** | those same split muṣḥafs, on mobile | the same, fast |

Quran SVG is the archive, not a stepping stone — splitting a muṣḥaf is hard work
and not every one of them will ever be split. This engine can only serve those
that have.

It is also **not the artwork** — the shapes come from Elements — and **not a text
source**: word text is attached at runtime from the `NNN.words.json` sidecars,
which the converter folds in from the source bundle's own per-page index.

## See it work

<https://quran.ws/blocks/quran-engine/> runs this engine in the browser: the real
wasm built from these crates, the real converted page, and the repository's own
`web/qvp.js`. Tap a word and the timing shown is measured in your browser.

The repository's own demo is `web/` — see **Quick start**.

## Supported riwayat

A *riwayah* is one transmitted reading of the Qur'an; different riwayat print
different muṣḥafs — [glossary](https://quran.ws/docs/concepts/glossary/#riwayah).

| Riwayah | Print | Status |
|---|---|---|
| Ḥafṣ ʿan ʿĀṣim | KFGQPC Madani muṣḥaf, V4 1441H | published as `v0.1.0` page data |

The converter reads the `quran-svg` bundle format (schema `quran-svg/version`
1.0.0), so any muṣḥaf published in that shape can be converted. One is published
today.

## Provenance

| | |
|---|---|
| Release | `v0.1.0` — *page data only*, no engine build (see Quick start) |
| Source bundle | `quran-svg hafs-kfgqpc` v1.0.0 |
| Built by engine commit | `16d16e7606e1` |
| Contents | 604 pages · 77,432 words · 6,236 ayat · 114 surahs · 240 rubʿ al-ḥizb boundaries |
| Sizes | 81.5 MB `.qvp` · 11.0 MB sidecars · 10 KB atlas |
| Digest | `sha256:bbc05984b9a5a2aa4e2bb655d95677825bc01ae679ee5892ada2335980209666` |

```sh
shasum -a 256 -c quran-engine-pages-hafs-kfgqpc.tar.gz.sha256
```

Every count above is read from `VERSION.json` inside the bundle; the sizes were
recomputed from the unpacked files.

## Quick start

**There is no install line.** Nothing is published to a package registry, and the
release carries the page data but no engine build — no wasm, no native library,
no `qvp.js`. Publishing those as release assets is planned. Until then the engine
is built from source, which means a Rust toolchain.

```sh
git clone https://github.com/quran-ws/quran-engine && cd quran-engine

# 1 — the page data (44 MB download, 92.5 MB unpacked, 604 pages)
gh release download v0.1.0 -R quran-ws/quran-engine -p '*.tar.gz'
tar xzf quran-engine-pages-hafs-kfgqpc.tar.gz

# 2 — the engine (~40 s once the toolchain is there)
curl -sSf https://sh.rustup.rs | sh -s -- -y -t wasm32-unknown-unknown
cargo build -p qvp-ffi --release --target wasm32-unknown-unknown

# 3 — the reference web demo
mkdir -p web/pages
cp target/wasm32-unknown-unknown/release/qvp_ffi.wasm web/
cp quran-engine-pages-hafs-kfgqpc/{042.qvp,042.words.json,atlas.qva} web/pages/
python3 web/build.py dev && (cd web && python3 -m http.server 8765)
# http://127.0.0.1:8765/?p=42
```

Then, in your own code, through `web/qvp.js`:

```js
const engine = await QvpEngine.init(wasmBytes);
const page   = engine.loadPage(qvpBytes);
page.attachWords(wordsJson);                             // optional text forms

page.layout({ viewportW, viewportH, nominalLines: page.nLines });
page.hitTestEx(x, y, { maxDistance: 6, gapBias: 0.6 });  // → {word, wordKey: "1:2:3", exact, …}
page.highlight("1:2", { mode: "both", ink: "#0a7d32" }); // → a handle; unhighlight(h) undoes it
page.search("الرحمان");                                   // normalised: finds the printed form
```

Two things that are easy to get wrong:

- **Pass `nominalLines: page.nLines`.** It defaults to 15 so that consecutive
  full pages share a grid. Lay out a short page without it and the engine
  reserves room for 15 lines — al-Fātiḥa has 8, and renders half-size inside an
  empty box.
- **`page.buildPaths()` is a renderer concern.** It caches `Path2D` per path and
  is only needed before drawing, so it needs a DOM. Every query API — geometry,
  hit-testing, layout, styles, text, search, crop, metadata — works headless
  without it.

## Platforms

| | In the tree |
|---|---|
| Web (Canvas2D over wasm) | `web/qvp.js` — the reference wrapper |
| Android (Kotlin, JNI) | `packages/android` + demo app |
| Flutter (Dart FFI) | `packages/flutter` |
| React Native | `packages/react-native` |
| iOS (Swift) | **not written yet** — see `docs/MACOS.md` |

`docs/API.md` documents the API once; each wrapper exposes the same names in its
own casing, and `crates/qvp-ffi/include/qvp.h` is the source of truth.

## Measured

Page 042, release build, Apple silicon — run it yourself and expect different
numbers on different hardware:

```sh
cargo run -p qvp-core --release --example bench -- quran-engine-pages-hafs-kfgqpc/042.qvp
```

| | |
|---|---|
| page load + geometry | 2.4 ms |
| exact hit-test | 1.4 µs |
| gap-aware hit-test | 0.15 µs |
| full display list, 2 style rules | 0.24 µs |
| search `الله` over the page | 86 µs |
| six-line ayah highlight, bands included | 65 µs |

The reason for the engine is not this table. It is that a fully split page is
hundreds of kilobytes of vector paths, and a phone cannot hold 604 of them in a
DOM and stay responsive.

## Known rough edges

- `revealStart()` in `web/qvp.js` throws `ReferenceError: markers is not defined`
  on every call — the greyed-page reveal is unreachable from the web wrapper. The
  rest of the memorisation surface (`mask`, `revealNext`, `revealWord`, `unmask`,
  `maskHidden`, `maskBoxes`) works.
- `surahs()` returns a `bannerDeco` field that `docs/API.md` does not list; it
  indexes into `page.decos`.
- No package is published, on any registry.

## Works with

| | |
|---|---|
| [Quran SVG Elements](https://github.com/quran-ws/quran-svg-elements) | the split artwork this engine converts and draws |
| [Quran Text](https://github.com/quran-ws/quran-text) | the Qur'anic text as queryable data, when you need more than the words on the page |
| [Quran Tajweed](https://github.com/quran-ws/quran-tajweed) | recitation-rule spans fed into layered styles — [what tajwīd is](https://quran.ws/docs/concepts/glossary/#tajweed) |
| [Qiraat Ayah Map](https://github.com/quran-ws/qiraat-ayah-map) | reconcile ayah numbers when your source counts differently |

## Documentation

- `docs/API.md` — the whole API, once, for every platform.
- `crates/qvp-ffi/include/qvp.h` — the C ABI, which the wrappers mirror.
- `docs/UPSTREAM-DATA-ISSUES.md` — issues found in the source data, fixed
  upstream and never patched here.
- <https://quran.ws/docs/reference/quran-engine/> — the long form, with the
  running demo.

## Licence

Code is **MIT** — `crates/`, `packages/`, `scripts/`, `web/`. Documentation is
**CC BY 4.0**, with attribution waived for use inside a product.

The page data is a separate release with its own terms: CC BY 4.0 covers this
project's decomposition, labels and indexes. The page artwork and the Qur'anic
text belong to the King Fahd Glorious Qur'an Printing Complex and are **not**
licensed by us — the Complex's own usage rights are reproduced in full in
`LICENSE`.

Anyone publishing Qur'anic text to readers is responsible for verifying it
against an authorised printed muṣḥaf. Corrections: corrections@quran.ws.
