# Changelog

All notable changes to the engine and its packages. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow
`docs/standards/VERSIONING.md`. Every package shares the version listed here.

## [Unreleased]

### Added
- `docs/standards/`: naming, API design, code style, structure, versioning, releasing and
  testing standards. `CONTRIBUTING.md`, `AGENTS.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`.
- iOS: six page-cache lifecycle tests.

### Changed
- iOS: a closed page is inert. A released cache leaves a retained controller's page open.
  The cache no longer traps when pages load before `setCurrentPage`.
- Repository: one `.gitignore`; xcodegen generates `Demo.xcodeproj` and git no longer tracks it. The iOS demo
  zip's README lives beside its script. `gen-mark-table.py` writes the table and checks it.

### Removed
- Three unreferenced React Native screenshots.

## [0.1.1] - 2026-09-13

### Changed
- Layout: leading only opens up. The printed line spacing is the floor for `lineSpacing`,
  `lineGap` and `fillHeight`. A short page under `fillHeight` takes the rows a full page
  gets. Slot boundaries stop half a pitch from a header line.
- iOS: `onDoubleTap` receives the hit under the finger (signature change). Page cache,
  zoom spring-back, long-press, side-margin crop.
- Every package version string reads 0.1.1.

## [0.1.0] - 2026-09-09

First release: the QVP1 page format and QVA1 atlas, the converter with its pixel-identity
gate, the engine and its 110-function C ABI, the web reference wrapper, and the Android,
Flutter, React Native and iOS packages with demos. Page data published as the `v0.1.0`
data release.

[Unreleased]: https://github.com/quran-ws/quran-engine/compare/v0.1.0...HEAD
[0.1.1]: https://github.com/quran-ws/quran-engine/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/quran-ws/quran-engine/releases/tag/v0.1.0
