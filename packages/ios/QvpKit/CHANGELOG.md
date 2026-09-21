# Changelog

This package shares the engine version. Entries are copied from the root `CHANGELOG.md`.

## [Unreleased]

Nothing yet.

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
