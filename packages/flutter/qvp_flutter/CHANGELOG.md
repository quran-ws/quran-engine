## Unreleased

### Added
- Flutter: `QvpEngine.viewSwipe(dx, dy, [vx, vy])` and `QvpEngine.swipePages(dx, dy, [vx, vy])`
  read a released drag, and `QvpPage.viewAnchor(view, word, nx, ny, toX, toY, viewportW,
  viewportH)` holds a point of a word on the screen across a new layout. The generated
  bindings declared all three, but no Dart call reached them (#165). Both backends bind them.
- `QvpPage.ayahMarksView()`, `hitAreasView([gapBias])` and `wordBandsView(words, height:,
  padX:, padY:)`: the viewport-px twins of `ayahMarks`, `hitAreas` and `wordBands`, in both
  backends. Each answers through the current layout, so an app that draws its own overlay on
  a reflowed page reads where each word or mark is drawn (#82).

### Changed

### Removed

### Fixed

## 0.8.0

### Added
- `QVP_DEFAULT_REFLOW_RELAX`, `QVP_DEFAULT_REFLOW_MAX_STRETCH` and `QVP_DEFAULT_SELECTION_PAD_X`
  in `qvp.h`, mirrored in every wrapper's defaults table (`QvpDefaults` in Kotlin, Swift and
  Dart, `DEFAULTS` in JavaScript and React Native). The page views pass the selection pad
  from the table and the reflow specs take their defaults from it, so no wrapper copies the
  numbers by hand. `scripts/check-parity.py` compares 29 defaults where it compared 26.
- Reflow can keep the print's line breaks with `breaks: 'printed'` in `QvpReflowSpec`. Each
  line stays whole while its ink grows to fill the page width. `reflowMaxZoom` measures the
  widest row in this mode, and larger zooms clamp to it.
- Platforms: iOS, macOS, Windows, Linux and web, alongside Android.
  - iOS and macOS link `QvpEngine.xcframework` through a podspec and resolve symbols with
    `DynamicLibrary.process()`. `scripts/build-engine-ios.sh` copies the framework into both.
  - Windows and Linux bundle `qvp_ffi.dll` and `libqvp_ffi.so` through CMake, and fail the
    build when neither the plugin directory nor the Cargo build holds one.
  - Web runs the engine as WebAssembly over `dart:js_interop`. Call
    `await QvpEngine.initWeb()` before the first engine call.
- `scripts/sync-flutter-web-engine.sh` refreshes the two web assets the package ships.
  `scripts/check-parity.py` fails when the `qvp.js` copy drifts from `web/qvp.js`.
- `QvpPage.batch(fn)` notifies listeners once however many mutations `fn` makes.
- `QvpPage.path(i)` builds one path on demand and keeps it. `QvpPage.buildPaths()`
  returns the whole list as before.

### Changed
- **Breaking.** `QvpPageView.onWordTap` hands the host the `QvpWordInfo` where it
  passed the word's index, and `onDecorationTap` carries the `QvpHit` beside the
  decoration. Android and iOS pass the word and the hit already, so host code reads one
  shape on all three. `QvpWordTapCallback` and `QvpDecorationTapCallback` name the two
  shapes. The index is `word.index`, and a host with no use for the hit takes a second
  parameter and ignores it (issue #139).
- Flutter: `package:ffigen` generates `lib/src/bindings.dart` from `qvp.h`
  (`dart run tool/ffigen.dart`). Before, the file was written by hand.
  `scripts/check-flutter-bindings.sh` fails when the file and the header differ. The Flutter
  workflow runs it, and `scripts/check.sh parity` runs it when Flutter and libclang are
  installed. `scripts/check-parity.py` skips the generated file, so a symbol counts as bound
  in Flutter only when the package's own code names it. `docs/API-PARITY.md` lists as gaps
  the 14 symbols that the old file bound and no backend called.
  The binding object behind `QvpNativeBackend.b` follows the C names everywhere: `hide`,
  `highlight`, `moveHighlight`, `restyleHighlight`, `removeHighlight`, `clearHighlights`
  and `atlasSurahs` are now `styleHide`, `highlightAdd`, `highlightMove`,
  `highlightRestyle`, `highlightRemove`, `highlightClear` and `atlasSurahCount`.
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
- `QvpPage.ayahMarks()` holds its answer until the next mutation.
- Shipped binaries build with the cargo profile `dist`. The Windows and Linux CMake files
  look under `target/dist/` first, then `target/release/`. `scripts/build-engine-windows.ps1`
  and `scripts/sync-flutter-web-engine.sh` build with it.
- Split `engine.dart` into `types.dart`, `page.dart`, `atlas.dart`, `ornaments.dart`,
  `passage.dart` and a backend interface. `QvpEngine` now holds a `QvpBackend`, so the
  Dart API no longer names `dart:ffi` types and the same code compiles for web.
- The native backend marshals input through a bump allocator over one scratch buffer
  rather than an allocation per call. Nested marshalling takes a slice of its own, so
  `text()` can pass two separators safely.
- The package is `dart format` clean, and the Flutter workflow checks the formatting before
  it analyses.
- Android and Flutter: a page view frame where nothing changed asks the engine for the clock
  and nothing else. The views keep the styled paths, the highlight and mask boxes with their
  paths, and the dress readout until a call on the page, a new layout or the clock changes
  them. Small value keys replace the string keys of the base ink, the ornament layer and the
  draw list. On Android such a frame allocates nothing: on page 042 in the API 35 emulator it
  allocated 19 objects, or 1,071 with a highlight and a mask (`docs/MEASURED.md`). The
  numbers the views draw do not change.
- The example's tool-written files are committed, so a clone stays clean after one build.
- The plugin and its example pin `ndkVersion` to the repository's NDK, so Gradle no longer
  downloads Flutter's default NDK beside it.

### Removed
- **Breaking.** `kMinZoom` and `kMaxZoom` are gone, because the engine applies the pinch
  limits. Use `QvpDefaults.maxZoom` and `QvpPageView.maxZoom`. The floor is the fitted page.
  `QvpPageView.minScale` and `maxScale`, which did nothing, are gone.
  `QvpViewController.zoomBy` takes `min` and `max` in place of `minScale` and `maxScale`.
- **Breaking.** `QvpEngine.resolveLibrary`, `QvpEngine.scratchBytes` and `QvpEngine.str`
  are gone. All three named `dart:ffi` types or the native scratch, which cannot sit on a
  platform-neutral class. `QvpNativeBackend` holds them now. Pass a library to
  `QvpEngine(library)` or a path to `QvpEngine.open(path:)` as before.

### Fixed
- The exact hit tests fill their hit with the core's conversion, as the gap-aware hit
  tests do. They are `qvp_hit_test_exact` and `qvp_hit_test_exact_view`, and `hitTestExact`
  and `hitTestExactView` in every wrapper. A hit on a decoration reports the line of the
  decoration's first path. It reported `QVP_NONE`: -1 in the wrappers, and `Int.MAX_VALUE`
  in Kotlin and in React Native on Android. A hit on the box of a word or a decoration,
  outside its ink, reports `is_exact` 0. It reported 1. `conformance/scenarios/hit.json`
  holds one point of each kind, and the web, Swift, Dart and Android tests replay it. The C
  ABI is unchanged.
- Dart: `rgba` and `QvpColor.withAlpha` clamp a NaN alpha to 0, where they threw.
- Every wrapper handles a freed object, a second free and a large array result the same way
  (`docs/API.md`, conventions). A call on a freed page, atlas, ornament set or passage throws
  an error with the object's name: a `StateError` in Dart in every build mode, where a debug
  assert stood before, a fatal error in Swift and a rejection in React Native. The Flutter
  native backend no longer passes a freed handle's null pointer to the engine. A second
  `free`, `close` or `dispose` does nothing, Flutter's `QvpPage.dispose` included, and the
  Kotlin handle is atomic, so two threads that close the same object free it once. Every
  array getter grows its buffer to the total the engine reports, where the Android bridge,
  the web wrapper and the Flutter backend stopped at fixed caps such as 64 line bands, 1,024
  highlight handles and a 256 KB scratch.
- Every pan, zoom and view computation in `QvpPageView` goes through the engine: a tap is
  inverted with `QvpPage.viewToLayout`, a magnifying pinch is placed by
  `QvpEngine.viewZoomAbout` with the fitted page as the floor and `maxZoom` as the ceiling, a
  one-finger drag by `QvpEngine.viewPan` (the view did not drag before), and the view a
  drag or a pinch produces is held with `dressClamp`. A pinch follows the rule in
  `docs/API.md`, and two fingers drag the page. `QvpViewController.zoomBy`, `panBy` and
  `toView` call the engine. `zoomBy` stops at the fitted page, and a controller that a host
  hands to a new view keeps working. `zoomSpec` keeps the host's reflow knobs on the printed
  page. The tests replay `conformance/scenarios/zoom.json`.
- A call on a disposed `QvpEngine` throws `StateError('QvpEngine was disposed')` on a native
  build, where a page, atlas or name call after `engine.dispose()` reached freed scratch
  memory. Freeing a page, atlas, passage or ornament set still works afterwards.
- After a zoom step lays the page out again, `QvpPageView` draws the highlight bands and the
  dress overflow of the new layout. It kept those of the old layout until the next call on the
  page.
- `QvpPageViewStats.basePaths` counts the paths a replayed picture draws, not only the
  paths of a frame that recorded one.
- The example builds from a fresh clone. The root `.gitignore` ignored every PNG, so git
  never tracked the launcher icons and launch images that the Android manifest and the iOS
  and macOS asset catalogs name. The rule now covers the repository root only, and the
  Flutter template's icons are tracked. The example gains the template's `web/` folder, and
  the Flutter workflow builds the example for Android and web, so a missing file fails CI.
- `flutter analyze` no longer warns on a fresh clone that the example's `assets/pages/`
  directory is missing. The directory holds a tracked `README.md`, and `.gitignore` keeps that
  file while it ignores the page data. The rule `pages/` now matches only the root `pages/`.
- A highlight band no longer freezes part way through a transition, and a removed band no
  longer stays on the page. The engine's highlight boxes depend on the clock, so the band
  cache clears on every frame rather than on a revision change.
- Styled paint composites with `srcOver`. With `src`, a styled colour part way through a fade
  wrote its alpha over the paper rect in the same layer and opened a translucent hole.
- A styled path the layout leaves out of the draw list is skipped. The fallback read
  `pathLine()`, which mixed a printed line number into a list of placements and put the glyph
  somewhere else on a reflowed page.

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
