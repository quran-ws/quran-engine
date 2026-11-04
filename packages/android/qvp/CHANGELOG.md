# Changelog

This package shares the engine version. Entries are copied from the root `CHANGELOG.md`.

## [Unreleased]

### Added
- `QVP_DEFAULT_REFLOW_RELAX`, `QVP_DEFAULT_REFLOW_MAX_STRETCH` and `QVP_DEFAULT_SELECTION_PAD_X`
  in `qvp.h`, mirrored in every wrapper's defaults table (`QvpDefaults` in Kotlin, Swift and
  Dart, `DEFAULTS` in JavaScript and React Native). The page views pass the selection pad
  from the table and the reflow specs take their defaults from it, so no wrapper copies the
  numbers by hand. `scripts/check-parity.py` compares 29 defaults where it compared 26.

### Changed
- The build file's default `qvpVersion` is the workspace version, 0.7.0, where it was
  `0.1.0-SNAPSHOT`. `publishToMavenLocal` needs no `-PqvpVersion` flag.
- Android and Flutter: a page view frame where nothing changed asks the engine for the clock
  and nothing else. The views keep the styled paths, the highlight and mask boxes with their
  paths, and the dress readout until a call on the page, a new layout or the clock changes
  them. Small value keys replace the string keys of the base ink, the ornament layer and the
  draw list. On Android such a frame allocates nothing: on page 042 in the API 35 emulator it
  allocated 19 objects, or 1,071 with a highlight and a mask (`docs/MEASURED.md`). The
  numbers the views draw do not change.

### Removed
- **Breaking.** `QvpPageView.MIN_ZOOM` is gone, because the engine applies the pinch limits.
  The floor is the fitted page. `QvpPageView.MAX_ZOOM` stays and is `QvpDefaults.MAX_ZOOM`.

### Fixed
- The exact hit tests fill their hit with the core's conversion, as the gap-aware hit
  tests do. They are `qvp_hit_test_exact` and `qvp_hit_test_exact_view`, and `hitTestExact`
  and `hitTestExactView` in every wrapper. A hit on a decoration reports the line of the
  decoration's first path. It reported `QVP_NONE`: -1 in the wrappers, and `Int.MAX_VALUE`
  in Kotlin and in React Native on Android. A hit on the box of a word or a decoration,
  outside its ink, reports `is_exact` 0. It reported 1. `conformance/scenarios/hit.json`
  holds one point of each kind, and the web, Swift, Dart and Android tests replay it. The C
  ABI is unchanged.
- Android: `QvpColor.parse` and `QvpColor.withAlpha` round alpha to the nearest of 255
  steps, as the Swift, Dart and web wrappers do. They truncated, so alpha 0.5 gave `0x7f` on
  Android and `0x80` everywhere else. A NaN alpha clamps to 0 in every wrapper, where
  Swift trapped, Dart threw and React Native returned a malformed colour.
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
  drag by `QvpEngine.viewPan`, and the view a drag or a pinch produces is held with
  `dressClamp`. A pinch follows the rule in `docs/API.md`: the engine zooms about the focal
  point the gesture began on, by the factor of the whole gesture, and the view pans the
  answer by the drift of the focal point, so two fingers drag the page. `zoomSpec` keeps the
  host's reflow knobs on the printed page. The instrumented tests replay
  `conformance/scenarios/zoom.json`.

## [0.7.0] - 2026-10-14

### Added
- Dress: `QvpOrnaments` loads an ornament set, and `QvpPage.dress`, `undress`, `dressInfo`,
  `dressGeometry` and `dressRevision` bind the dress section of the ABI. `QvpPageView` paints the
  ornament layer behind the ink, cached per dress revision, and fits the border on screen. The
  example app has a dress panel.
- Passages: `QvpPassage.load` and `QvpPassage.basmalah`, `layout`, and `QvpPassageLayout.draw`
  to draw a layout on a `Canvas`. The instrumented tests replay
  `conformance/scenarios/passage.json` (issue #102), and `scripts/sync-android-test-data.sh`
  copies the pages and the scenario file they read.
- `QvpPassageWord.wordX0` to `wordY1`: the box of the word alone, without its signs. The word that
  closes an ayah leaves its medallion out, so an underline under it stops at the word.

### Fixed
- `QvpPage.zoomSpec` keeps the host's reflow knobs on the printed page, with zoom 0, so the first
  pinch off it reflows with the host's fill rather than the engine's defaults, as QvpKit does.
- The magnify pinch's ceiling (`QvpPageView.maxZoom`) is clamped by the engine
  (`qvp_view_zoom_about`) rather than by the view's own arithmetic. A ceiling of 0 or less is the
  engine's own ceiling.

## [0.5.0] - 2026-10-08

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

## [0.3.1] - 2026-10-05

- `maxZoom` on `QvpPageView`: a host lowers the ceiling the magnifying glass is clamped to,
  for furniture of its own that scales with the glass. A ceiling under the fitted page is the
  fitted page — a pinch never shrinks the page inside the screen.
- `peekScale` on `QvpPageView`: how far the glass is over the fitted page — 1 at rest and in
  every mode but magnify.

## [0.3.0] - 2026-09-21

- Engine 0.3.0: the reader's zoom control (`zoomPinch`, `zoomToStep`, and the mode, spec,
  carried, at-step and is-zoomed calls), a page reflowed onto rows of the screen's own width
  when the reader zooms in, `layoutDrawList`, `layoutPrintedHeight`, `sidewaysDrag` and
  `swipePages`, and the `surahFrames` and `bannerZoom` layout knobs.
- `surahHeaders` and `surahHeadersView`: the box a surah frame fills and the title ink
  inside it, for a host that draws its own frame.
- The demo draws a reflowed page, caches the ink as a band of the page rather than redrawing
  on every scroll event, turns one page per swipe, and is built on Material 3.
- The library module no longer applies the Maven publishing plugin itself; the Android SDK's
  root build applies and configures it, so another root can include `:qvp` (#83).
  `:qvp:publishAndReleaseToMavenCentral` is unchanged.

## [0.2.1] - 2026-09-14

- Releases publish `ws.quran:qvp-android` to Maven Central when its credentials are configured.
- Release AARs use 16 KiB-aligned 64-bit native libraries and are attached to each GitHub
  release with a SHA-256 checksum.

## [0.2.0] - 2026-09-13

- ABI 0.2: the header and every wrapper follow the naming standard (hit tests, rectangles,
  verbs, discriminators, full words, layout, one-byte booleans, the engine/format version split).
  No alias is kept; the old → new table is in the root `CHANGELOG.md` and `docs/API-PARITY.md`.

## [0.1.1] - 2026-09-13

- Layout: leading only opens up; the printed line spacing is the floor.

## [0.1.0] - 2026-09-09

- First release.
