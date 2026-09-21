# Changelog

This package shares the engine version. Entries are copied from the root `CHANGELOG.md`.

## [Unreleased]

Nothing yet.

## [0.3.0] - 2026-09-21

- Engine 0.3.0: `layoutPrintedHeight`, and `surahFrames` on the layout spec.
- `surahHeaders` and `surahHeadersView`: the box a surah frame fills and the title ink
  inside it, for a host that draws its own frame.
- Fixed: the Android module compiles. Its view manager was missing the import of
  `QvpDefaults`, and its page view called a `centre()` that does not exist (#84).
- Fixed: the example typechecks again (`isComplete`, and a `setLineGap` that no longer exists).

## [0.2.0] - 2026-09-13

- ABI 0.2: the header and every wrapper follow the naming standard (hit tests, rectangles,
  verbs, discriminators, full words, layout, one-byte booleans, the engine/format version split).
  No alias is kept; the old → new table is in the root `CHANGELOG.md` and `docs/API-PARITY.md`.

## [0.1.1] - 2026-09-13

- Layout: leading only opens up; the printed line spacing is the floor.

## [0.1.0] - 2026-09-09

- First release.
