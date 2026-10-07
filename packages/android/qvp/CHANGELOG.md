# Changelog

This package shares the engine version. Entries are copied from the root `CHANGELOG.md`.

## [Unreleased]

Nothing yet.

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
