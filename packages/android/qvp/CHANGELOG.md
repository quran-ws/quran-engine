# Changelog

This package shares the engine version. Entries are copied from the root `CHANGELOG.md`.

## [Unreleased]

Nothing yet.

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
