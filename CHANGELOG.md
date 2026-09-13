# Changelog

All notable changes to the engine and its packages. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow
`docs/standards/VERSIONING.md`. Every package shares the version listed here.

## [Unreleased]

### Added
- Mask: `qvp_mask_transition(page, ms)` (iOS `maskTransition`) fades `hide` words in and out
  on the engine clock instead of switching at once; the ink keeps its colour and only its
  alpha moves, on an ease-in-out curve (other colour transitions keep easing out). 0 (the
  default) keeps the instant switch; `unmask` resets it. Bound on iOS; declared as a gap for
  web, Android, Flutter and React Native in `docs/API-PARITY.md`. iOS: `QvpPageCanvas` starts
  a transition's frames in the update that begins it, and its renderer reads the frame's date
  so every timeline tick redraws.
- Names: `qvp_name(table, id)`, `qvp_name_id(table, name)`, `qvp_name_count(table)` over the
  mark, kind, family, category, decoration, division and place tables. Every wrapper reads
  its names from the engine at start-up; the five hand-written mark tables and the place,
  division and decoration lists are gone. React Native's kind and decoration names now
  match the engine's (`ayah_number`, `ayah-mark`); an unknown name resolves to 255, not 0.
- Layout: `QvpLayout.fit_scale`, `fit_x`, `fit_y` (the view transform that shows the whole
  content), `QvpLayoutSpec.crop_left`, `crop_right` (cut printed side margins) and
  `max_aspect_slack` (bound the content width by the page aspect), and
  `qvp_layout_gap_to_fill(page, spec, max)`. Every wrapper reads the fit from the engine
  instead of computing it; `conformance/scenarios/layout.json` records the engine's answers
  for 40 viewport cases and the wrapper tests replay them.
- CI: `ios.yml`, `flutter.yml`, `react-native.yml` per platform; `nightly.yml` (all 604 pages,
  benchmark); `release.yml` on `vX.Y.Z` tags; `scripts/set-version.sh`, `scripts/package-data.sh`.
- CI: `ci.yml` with the `engine`, `gates`, `standards` and `web` jobs; `scripts/check.sh`
  runs the same checks locally; `scripts/sync-test-data.sh`, `check-parity.py`,
  `check-versions.sh`, `check-structure.sh`, `check-terminology.sh`; `docs/API-PARITY.md`.
- `rust-toolchain.toml`, `rustfmt.toml`, `.editorconfig`, `.mailmap`, `CODEOWNERS`, pull
  request and issue templates, Dependabot.
- `docs/standards/`: naming, API design, code style, structure, versioning, releasing and
  testing standards. `CONTRIBUTING.md`, `AGENTS.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`.
- iOS: six page-cache lifecycle tests.

### Changed
- FFI: every `qvp_*` entry point catches a panic in the engine and returns its error value
  (0, -1 or null) instead of aborting the host; the release profile no longer sets
  `panic = "abort"`.
- Every platform's demo app lives in `example/`: `packages/ios/example`,
  `packages/android/example` (Gradle module `:example`), `web/example/`. The web build
  writes `dist/web/` and reads pages from `dist/pages/`. The iOS README lives inside
  `QvpKit/`; the design spec lives in `docs/design/` with a historical banner.
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
