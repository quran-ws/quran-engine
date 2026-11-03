## Unreleased

- The web engine in the package, `assets/wasm/qvp_ffi.wasm` and `assets/js/qvp.js`, is
  built from the release tag, so it matches the native libraries and the crates of the same
  version. The repository no longer tracks the two files. The pubspec declares their
  directories, so a native build of a `git:` or `path:` dependency runs without them. A web
  build from such a dependency builds them first (see the README).
- `QvpPageView` caches the base ink and the ornament layer as `ui.Picture` recordings in
  layout units rather than rasters at the current transform, so a pan or a pinch replays the
  picture instead of redrawing the page. Keys carry the engine layout's offsets and the
  `surahFrames` flag, so a relayout redraws. The ornament layer is keyed on
  `QvpDressInfo.revision` and held per page, which is what a repaint of a dress depends on.
- `QvpPage.batch(fn)` notifies listeners once however many mutations `fn` makes.
- `QvpPage.ayahMarks()` holds its answer until the next mutation.
- `QvpPageViewStats.basePaths` counts the paths a replayed picture draws, not only the
  paths of a frame that recorded one.
- Platforms: iOS, macOS, Windows, Linux and web, alongside Android.
  - iOS and macOS link `QvpEngine.xcframework` through a podspec and resolve symbols with
    `DynamicLibrary.process()`. `scripts/build-engine-ios.sh` copies the framework into both.
  - Windows and Linux bundle `qvp_ffi.dll` and `libqvp_ffi.so` through CMake, and fail the
    build when neither the plugin directory nor the Cargo build holds one.
  - Web runs the engine as WebAssembly over `dart:js_interop`. Call
    `await QvpEngine.initWeb()` before the first engine call.
- Shipped binaries build with the cargo profile `dist`. The Windows and Linux CMake files
  look under `target/dist/` first, then `target/release/`. `scripts/build-engine-windows.ps1`
  and `scripts/sync-flutter-web-engine.sh` build with it.
- `scripts/sync-flutter-web-engine.sh` refreshes the two web assets the package ships.
  `scripts/check-parity.py` fails when the `qvp.js` copy drifts from `web/qvp.js`.
- Split `engine.dart` into `types.dart`, `page.dart`, `atlas.dart`, `ornaments.dart`,
  `passage.dart` and a backend interface. `QvpEngine` now holds a `QvpBackend`, so the
  Dart API no longer names `dart:ffi` types and the same code compiles for web.
- The native backend marshals input through a bump allocator over one scratch buffer
  rather than an allocation per call. Nested marshalling takes a slice of its own, so
  `text()` can pass two separators safely.
- `QvpPage.path(i)` builds one path on demand and keeps it. `QvpPage.buildPaths()`
  returns the whole list as before.
- The package is `dart format` clean, and the Flutter workflow checks the formatting before
  it analyses.
- The example builds from a fresh clone. The root `.gitignore` ignored every PNG, so git
  never tracked the launcher icons and launch images that the Android manifest and the iOS
  and macOS asset catalogs name. The rule now covers the repository root only, and the
  Flutter template's icons are tracked. The example gains the template's `web/` folder, and
  the Flutter workflow builds the example for Android and web, so a missing file fails CI.
- **Breaking.** `QvpPageView.onWordTap` hands the host the `QvpWordInfo` where it
  passed the word's index, and `onDecorationTap` carries the `QvpHit` beside the
  decoration. Android and iOS pass the word and the hit already, so host code reads one
  shape on all three. `QvpWordTapCallback` and `QvpDecorationTapCallback` name the two
  shapes. The index is `word.index`, and a host with no use for the hit takes a second
  parameter and ignores it (issue #139).
- **Breaking.** `QvpEngine.resolveLibrary`, `QvpEngine.scratchBytes` and `QvpEngine.str`
  are gone. All three named `dart:ffi` types or the native scratch, which cannot sit on a
  platform-neutral class. `QvpNativeBackend` holds them now. Pass a library to
  `QvpEngine(library)` or a path to `QvpEngine.open(path:)` as before.

## 0.7.0

- Dress: `QvpEngine.loadOrnaments` and `QvpOrnaments` (`list`, `find`, `ofKind`, `styles`), and
  `QvpPage.dress`, `undress`, `dressInfo`, `dressGeometry` and `dressRevision`. `QvpPageView`
  paints the ornament layer behind the ink and fits the border. The example has a dress panel.
- Passages: `QvpPassage.load` and `QvpPassage.basmalah`, `layout`, and `paintPassage` to draw a
  layout; the plugin replays `conformance/scenarios/passage.json` (issue #102).
- `QvpPageView.maxZoom`, `peekScale` and `setZoomMode`, as on iOS and Android. The magnify pinch
  is clamped by the engine (`QvpEngine.viewZoomAbout`).
- `zoomSpec` keeps the host's reflow knobs on the printed page, with zoom 0, so the first pinch
  off it reflows with the host's fill rather than the engine's defaults.
- Engine: a passage cut to a row with no room for one word, the ellipsis and the medallion
  shrinks the scale rather than placing them outside the layout.
- `QvpPassageWord.wordX0` to `wordY1`: the box of the word alone, without its signs. The word that
  closes an ayah leaves its medallion out, so an underline under it stops at the word.

## 0.6.1

- Engine 0.6.1: a passage that runs from page 2 into page 3 draws every word at one size
  in QvpKit and `web/lite-passage.mjs`; this package binds passages next (issue #102). Nothing
  changes here.

## 0.6.0

- Engine 0.6.0: justified passages and a surah's basmalah as a passage reach QvpKit first;
  this package binds passages next (issue #102). Nothing changes here.

## 0.5.0

- Under `justified`, a row that cannot reach both margins is centred instead of starting at
  the right margin: the last row of a block, a single word, or a row whose gaps would have to
  stretch past `max_stretch`. Hung off the right margin, it read as a ragged column.
- The reflow knobs of `QvpLayoutSpec` are read on a printed page too (`reflow_zoom` 0): they
  are what the zoom control reflows the page with. A C host that zeroes the spec and pinches
  now reflows ragged (fill 0), greedy (breaks 0) and with printed gaps (gaps 0); 255, 255 and 1
  ask for the engine's defaults, which every wrapper already writes.
- On a reflowed row, a medallion is centred between the word it closes and the next one with
  the signs each carries, such as the quarter star (۞) that opens the next ayah. Measured on the
  letters alone, it sat against the star with the space on its other side.
- A pinch keeps the host's fill, breaks and gaps from the first step. The C layer dropped the
  reflow knobs while `reflow_zoom` was 0, so the layout the zoom control made inside a pinch
  always took the engine's defaults.

## 0.3.0

- Engine 0.3.0: the reader's zoom control (`zoomPinch`, `zoomToStep`, and the mode, spec,
  carried, at-step and is-zoomed calls), a page reflowed onto rows of the screen's own width
  when the reader zooms in, `layoutDrawList`, `layoutPrintedHeight`, `sidewaysDrag` and
  `swipePages`, and the `surahFrames` and `bannerZoom` layout knobs.
- `surahHeaders` and `surahHeadersView`: the box a surah frame fills and the title ink
  inside it, for a host that draws its own frame.
- Fixed: `QvpPage.zoomSpec` carries `bannerZoom` through.

## 0.2.2

- The package includes the Android engine and is available from pub.dev.

## 0.2.0

- ABI 0.2: the header and every wrapper follow the naming standard (hit tests, rectangles,
  verbs, discriminators, full words, layout, one-byte booleans, the engine/format version split).
  No alias is kept; the old → new table is in the root `CHANGELOG.md` and `docs/API-PARITY.md`.

## 0.1.1

- Line spacing only ever opens up. The printed pitch is the floor: `lineSpacing` below 1,
  a negative `lineGap`, or a `fillHeight` that would need to tighten now lay the page out
  exactly as printed, so the lines can never collapse into one another. The text width was
  never adjustable and stays fitted to the viewport.

## 0.1.0

- First release: page loading, hit-testing, layout, styles, highlights, selection, masks,
  search and crop over the Rust engine.
