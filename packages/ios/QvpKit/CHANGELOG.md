# Changelog

This package shares the engine version. Entries are copied from the root `CHANGELOG.md`.

## [Unreleased]

### Added
- `qvp_layout_ink_band` picks the band of a laid-out page that a host keeps drawn: the whole
  page when it fits, otherwise two screens around the reader. The band's top moves in steps,
  so a pan inside one step redraws nothing. The band is in layout px, ready for
  `qvp_layout_draw_list`. Its two limits are in viewport px: `max_height` (0 takes the new
  `QVP_DEFAULT_MAX_INK_HEIGHT`, 4096) and `max_pixels`, a pixel budget (0 is no limit). Bound
  as `layoutInkBand` in JavaScript, Kotlin, Dart and Swift. React Native calls it through the
  Kotlin view. `conformance/scenarios/ink-band.json` holds the engine's answers, and the web,
  Kotlin, Dart and Swift tests replay them (#134).

### Changed
- The Android `QvpPageView`, the Flutter `QvpPageView` and the web `CanvasRenderer` ask the
  engine for their band of ink. Each had its own copy of the arithmetic, and the copies did
  not agree (#134):
  - Android drew a printed page as a band. It now draws any page whole when the page fits the
    ceiling and the ink budget, as it already did for a reflowed page. Its band no longer
    passes the ceiling on a screen taller than 4096 px, and the ink budget now limits the band
    as well as the whole page.
  - Flutter measured the 4096 ceiling in layout px. It now measures it in viewport px, so a
    magnified page bands once it is taller than 4096 px on the screen. At a view scale of 1
    nothing changes.
  - The web band now stays under the tallest canvas a browser makes, and under the ink budget.

### Removed

### Fixed

## [0.8.0] - 2026-11-04

### Added
- `QVP_DEFAULT_REFLOW_RELAX`, `QVP_DEFAULT_REFLOW_MAX_STRETCH` and `QVP_DEFAULT_SELECTION_PAD_X`
  in `qvp.h`, mirrored in every wrapper's defaults table (`QvpDefaults` in Kotlin, Swift and
  Dart, `DEFAULTS` in JavaScript and React Native). The page views pass the selection pad
  from the table and the reflow specs take their defaults from it, so no wrapper copies the
  numbers by hand. `scripts/check-parity.py` compares 29 defaults where it compared 26.
- Reflow can keep the print's line breaks with `QvpBreaks.printed`. Each line stays whole
  while its ink grows to fill the page width. `reflowMaxZoom` measures the widest row in this
  mode, and larger zooms clamp to it.

### Changed
- **Breaking.** Swift: `QvpOrnaments.find`, `QvpPage.dress` with a closed set and
  `QvpPassage.layout` on a closed passage stop the program with a fatal error that names the
  object, such as `QvpOrnaments is closed`. They returned nil, the same answer as a refused
  spec or no match. Check `isOpen` first where an object can close before its last call.
- CI builds the release XCFramework. `scripts/prepare-ios-release.sh` starts
  `prepare-ios-release.yml`, which builds it on macOS, attaches it to the draft release and
  pushes the commit that sets its URL, checksum and build commit in `Package.swift`.
  `release.yml` refuses a tag whose engine sources differ from that build commit.

### Removed
- **Breaking.** `QvpViewPolicy.minZoom`, `zoomedThreshold`, `swipeAxisRatio`,
  `swipeDistance`, `swipeVelocity`, `clampZoom(_:)` and `clampZoom(_:fit:)` are gone, because
  the engine decides each of them. Use `QvpZoom.isZoomed(_:fitScale:)`,
  `QvpViewPolicy.swipeDirection(translation:velocity:)`, and `clampZoom(_:fit:ceiling:)` or
  `QvpView.zoomAbout`. The floor of a pinch is the fitted page.

### Fixed
- The exact hit tests fill their hit with the core's conversion, as the gap-aware hit
  tests do. They are `qvp_hit_test_exact` and `qvp_hit_test_exact_view`, and `hitTestExact`
  and `hitTestExactView` in every wrapper. A hit on a decoration reports the line of the
  decoration's first path. It reported `QVP_NONE`: -1 in the wrappers, and `Int.MAX_VALUE`
  in Kotlin and in React Native on Android. A hit on the box of a word or a decoration,
  outside its ink, reports `is_exact` 0. It reported 1. `conformance/scenarios/hit.json`
  holds one point of each kind, and the web, Swift, Dart and Android tests replay it. The C
  ABI is unchanged.
- Swift: `QvpColor.parse` and `QvpColor.withAlpha` clamp a NaN alpha to 0, where they
  trapped.
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
- Every pan, zoom and view computation in `QvpPageView` and `QvpPageCanvas` goes through the
  engine: a tap is inverted with `viewToLayout`, a magnifying pinch is placed by
  `QvpView.zoomAbout` with the fitted page as the floor and `maxZoom` as the ceiling, a drag
  by `QvpView.pan`, the zoomed line by `qvp_zoom_is_zoomed`, and the view a drag or a pinch
  produces is held with `dressClamp`. A pinch follows the rule in `docs/API.md`, and in
  `QvpPageView` two fingers drag the page. A pinch that ends while the reader selects no
  longer leaves the one-finger drag switched off. `zoomSpec` keeps the host's reflow knobs on
  the printed page. The tests replay `conformance/scenarios/zoom.json`.

## [0.7.0] - 2026-10-14

### Added
- Dress: `QvpOrnaments` loads an ornament set, and `QvpPage.dress`, `undress`, `dressInfo`,
  `dressGeometry` and `dressRevision` bind the engine's dress. `QvpPageView` and
  `QvpPageCanvas` paint the ornament layer behind the ink, fit the border and keep it on
  screen when the page scrolls. The demo has a dress sheet.
- `QvpPassageWord.wordX0` to `wordY1`: the box of the word alone, without its signs. The word that
  closes an ayah leaves its medallion out, so an underline under it stops at the word.

### Fixed
- `QvpViewPolicy.clampZoom(_:fit:ceiling:)`, the magnify pinch's ceiling, is applied by the
  engine (`qvp_view_zoom_about`) rather than by the policy's own arithmetic. A ceiling of 0 or
  less is now the engine's own ceiling; a ceiling under the fitted page is still the fitted
  page.
- The engine's passage layout: a cut row with no room for one word, the ellipsis and the
  medallion shrinks the scale rather than placing them outside the layout. The conformance
  replay has 58 cases, from 52.

## [0.6.1] - 2026-10-12

### Fixed
- A passage that runs from page 2 into page 3 draws every word at one size. Page 2 prints its
  words about 20% smaller than page 3, and the passage drew each word at its own page's size, so
  its words came out at two sizes. Its line spacing, the median over its words, also jumped by
  about 20% when the words of page 3 became the majority. Now every word has the size of the
  passage's first word: a word of a page printed at another size is resized (its placement has
  `kx = ky ≠ 1`), and the line spacing is that of the first word's size, however many ayahs
  follow. A passage of pages that print at one size lays out as before. `web/lite-passage.mjs`
  follows the engine, and its `draw` now applies `ky`.

## [0.6.0] - 2026-10-08

### Added
- Justified passages: `Align::Justified` (`QVP_ALIGN_JUSTIFIED`, QvpKit `.justified`, `'justified'`
  in `web/lite-passage.mjs`). Every row but the last opens its word gaps, in equal shares, until
  it reaches both edges, by at most `max_stretch` times the passage's usual air between two words
  (2 by default). Two words the print draws as one piece of calligraphy keep their distance.
  What a row has left over is split between its two sides, so the last row, the cut row with its
  ellipsis, a one-word row and a capped row are centred.
- A surah's basmalah as a passage: `Passage::load_basmalah`, `qvp_passage_load_basmalah`, QvpKit
  `QvpPassage(basmalahOf:pages:)`. It loads from the page that prints it and lays out like any
  passage, on one row, with no ayahs and no words; its line spacing is its page's, so a host sizes
  it by the printed line as it does an ayah. Surahs 1 and 9 fail the load. The other wrappers
  bind it with passages (issue #102).
- `conformance/scenarios/passage.json`: two justified specs on every range, and the basmalahs of
  surahs 2 and 114. QvpKit replays all 52 cases, `web/lite-passage.mjs` the 36 it supports.

### Changed
- C ABI: `QvpPassageSpec` gains a last field, `float max_stretch` (0 = the default, negative = no
  cap), and grows from 28 to 32 bytes. A C host that declares the struct itself must add it.

## [0.5.0] - 2026-10-08

### Added
- `QvpCanvasController.reflowFill` (iOS): how the rows of a page the reader zoomed into fill
  the width, `centred` (the default), `justified` or `ragged`.
- `QvpCanvasController.reflowMaxStretch` (iOS): how far a justified row's gaps may open, as a
  multiple of the page's usual air between two words (2 by default, negative for no cap).

### Changed
- Under `justified`, a row that cannot reach both margins is centred instead of starting at
  the right margin: the last row of a block, a single word, or a row whose gaps would have to
  stretch past `max_stretch`. Hung off the right margin, it read as a ragged column.
- The reflow knobs of `QvpLayoutSpec` are read on a printed page too (`reflow_zoom` 0): they
  are what the zoom control reflows the page with. A C host that zeroes the spec and pinches
  now reflows ragged (fill 0), greedy (breaks 0) and with printed gaps (gaps 0); 255, 255 and 1
  ask for the engine's defaults, which every wrapper already writes.

### Fixed
- On a reflowed row, a medallion is centred between the word it closes and the next one with
  the signs each carries, such as the quarter star (۞) that opens the next ayah. Measured on the
  letters alone, it sat against the star with the space on its other side.
- A pinch keeps the host's fill, breaks and gaps from the first step. The C layer dropped the
  reflow knobs while `reflow_zoom` was 0, so the layout the zoom control made inside a pinch
  always took the engine's defaults.
- `QvpPage.zoomSpec` (iOS) keeps the host's reflow knobs and sets only the zoom, on the printed
  page too. It rebuilt the reflow from the zoom and three spacing knobs, and dropped it at the
  printed size.

## [0.4.0] - 2026-10-07

- `QvpPassage`: a range of complete ayahs laid out on rows away from its page, from the
  outlines of its pages, with `QvpPassageSpec` and `QvpPassageLayout`. A row limit cuts it
  after a whole word, with room for the host's ellipsis and, when asked, the last ayah's
  medallion after it. The pages can close after the passage is made.

## [0.3.1] - 2026-10-05

- `QvpPageCache.setCurrentPage(_:span:)`: a host showing several pages at once says how many,
  and the cache keeps the screen before, the screen itself and the screen after loaded and
  safe from eviction — six pages for a two-page spread — so a swipe never lands on a blank
  half. The default span of 1 is the page and its two neighbors, as before.
- `maxZoom` on `QvpCanvasController` and `QvpPageView`: a host lowers the ceiling the
  magnifying glass is clamped to, for furniture of its own that scales with the glass.
  `QvpViewPolicy.clampZoom(_:fit:ceiling:)` is the clamp. `peekScale` on both: how far the
  glass is over the fitted page — 1 at rest and in every mode but `.magnify`.
- `QvpPageCanvas` adds a reveal to its cached ink instead of drawing the page again, and reads
  the engine clock, the styled paths and the highlight and mask boxes once per frame rather
  than once per canvas.
- Fixed: `QvpPageCanvas` draws a page taller than Core Animation's 8,192-px layer limit as a
  stack of canvases, each under it and cut on a whole pixel, so a reflowed page at its top
  step no longer goes soft.
- Fixed: `QvpPageCanvas` draws its cached ink at the bitmap's own size, so rows down the page
  are no longer blended with their neighbours.

## [0.3.0] - 2026-09-21

- Engine 0.3.0: the reader's zoom control (`zoomPinch`, `zoomToStep`, and the mode, spec,
  carried, at-step and is-zoomed calls), a page reflowed onto rows of the screen's own width
  when the reader zooms in, `layoutDrawList`, `layoutPrintedHeight` and `sidewaysDrag`, and the `surahFrames` and `bannerZoom` layout knobs.
- `surahHeaders` and `surahHeadersView`: the box a surah frame fills and the title ink
  inside it, for a host that draws its own frame.
- `QvpCanvasController.printedPitch`, what a printed row is worth in the canvas's box, and
  `printedHeight(forWidth:)`, the page's printed height for a width with the controller's own
  knobs.
- The demo draws a reflowed page, caches the ink as a band of the page rather than redrawing
  on every scroll event, and turns one page per swipe.
- Fixed: `QvpCanvasController` under `hostScrolls` measures the layout in `hostViewportHeight`,
  never in its own canvas, and no longer shrinks or centres a printed page too tall for that
  box. Both knobs lay the page out again when set, and the band a pinch paints follows the
  view transform.
- Fixed: `QvpCanvasController.zoomSteps` is empty until the canvas has a size, as
  `QvpPageView.zoomSteps` already was.
- Fixed: the demo builds again after the naming standard's renames.

## [0.2.2] - 2026-09-14

- QvpKit can be installed from the repository as a remote Swift package. SwiftPM downloads
  the release XCFramework and verifies its checksum.

## [0.2.0] - 2026-09-13

- ABI 0.2: the header and every wrapper follow the naming standard (hit tests, rectangles,
  verbs, discriminators, full words, layout, one-byte booleans, the engine/format version split).
  No alias is kept; the old → new table is in the root `CHANGELOG.md` and `docs/API-PARITY.md`.

## [0.1.1] - 2026-09-13

- Layout: leading only opens up; the printed line spacing is the floor.

## [0.1.0] - 2026-09-09

- First release.
