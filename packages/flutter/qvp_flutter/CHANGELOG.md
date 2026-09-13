## 0.1.1

- Line spacing only ever opens up. The printed pitch is the floor: `lineSpacing` below 1,
  a negative `lineGap`, or a `fillHeight` that would need to tighten now lay the page out
  exactly as printed, so the lines can never collapse into one another. The text width was
  never adjustable and stays fitted to the viewport.

## 0.1.0

- First release: page loading, hit-testing, layout, styles, highlights, selection, masks,
  search and crop over the Rust engine.
