# Changelog

This package shares the engine version. Entries are copied from the root `CHANGELOG.md`.

## [Unreleased]

Nothing yet.

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
