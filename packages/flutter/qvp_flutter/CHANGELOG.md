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
