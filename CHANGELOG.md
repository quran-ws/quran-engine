# Changelog

All notable changes to the engine and its packages. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow
`docs/standards/VERSIONING.md`. Every package shares the version listed here.

## [Unreleased]

### Changed (0.2.0, ABI-breaking)
- The header is renamed to the naming standard. The old → new table is the "Renames for
  0.2" section of `docs/API-PARITY.md`; no old name is kept as an alias. Each cluster's
  entry follows here as it lands.
- Hit tests: `qvp_hit_test` and `qvp_hit_test_view` are the gap-aware calls (formerly `_ex`);
  the exact-outline variants are `qvp_hit_test_exact` and `qvp_hit_test_exact_view`. One
  `QvpHit` struct for all four (`word, path, deco, line, distance, is_exact`); the exact
  variants fill `line` and report `is_exact` 1. `QvpHitOptions.exact_first` is `prefer_exact`.
  Wrappers: `hitTest`, `hitTestView`, `hitTestExact`, `hitTestExactView`, `preferExact`, `isExact`.
- Rectangles: `qvp_hit_areas` (was `hit_boxes`, type `QvpHitArea`), `qvp_highlight_boxes_view`,
  `qvp_mask_boxes_view`, `qvp_word_bands` (was `band_boxes`), `qvp_crop_bounds` (type
  `QvpCropBounds`), `qvp_word_bounds_view`. Wrappers: `hitAreas`, `highlightBoxesView`,
  `maskBoxesView`, `wordBands`, `cropBounds`, `wordBoundsView`.
- Verbs: the mask calls pair `mask` with `unmask` (`qvp_mask_word`, `mask_all`, `mask_back`,
  `unmask_word`, `unmask_all`, `unmask_next`; wrappers `maskWord` … `unmaskNext`); the reveal
  readers are `qvp_reveal_position` and `qvp_reveal_step_count`; the style hide is
  `qvp_style_hide` (wrapper `hide` unchanged). Highlights: `qvp_highlight_add`, `_move`,
  `_restyle`, `_remove`, `_clear` (wrappers `highlight`, `moveHighlight`, `restyleHighlight`,
  `removeHighlight`, `clearHighlights`). Lookups: `qvp_target_words` (was `resolve`),
  `qvp_atlas_search_surahs` (was `find_surah`), `qvp_atlas_division_of` (was `_at`; wrappers
  `divisionOf`, `juzOf`). Counts: `qvp_surah_count`, `qvp_atlas_page_count`
  (`atlas.pageCount()`), `qvp_atlas_surah_count`, `qvp_styled_paths` (`styledPaths()`).
  Colours: `qvp_colors` (was `paint`; `colors()`), `qvp_style_recolor` (`recolorStyle`),
  `qvp_style_default_color` (`setDefaultColor`); `unstyle` is `removeStyle`. One `qvp_text`
  taking a target replaces the word-list and target pair.
- Discriminators take their enum's name: `QvpDecoInfo.decoration` (`QVP_DECORATION_*`, was
  `QVP_DECO_*`), `QvpDivision.division` (`QVP_DIVISION_*`, was `QVP_DIV_*`), `QvpTarget.target`,
  `QvpSelector.selector` (`QVP_SELECTOR_*`, was `QVP_SEL_*`); `QVP_HIGHLIGHT_*` was `QVP_HL_*`;
  `QVP_BAND_LINE_SPACING` was `QVP_BAND_PITCH`; `qvp_arabic(op)` takes `QVP_ARABIC_*`. The search
  option is `loose_match` and a match reports `is_loose_match` (wrappers `looseMatch`,
  `isLooseMatch`). `QvpSurahInfo` is `QvpSurah`.
- Full words: `qvp_decoration_info` and `QvpDecorationInfo` (was `deco`); fields `decoration`,
  `n_decorations`, `ayah_mark_decoration`, `banner_decoration`, `line_number`, `ayah_index`,
  `line_index`, `number` (was `n` on `QvpDivision` and `QvpAtlasSurah`); parameters `index`,
  `word_index`, `ayah_index`, `view_x`, `view_y`; `QVP_SELECTOR_DECORATION` and
  `QVP_SELECTOR_DECORATION_INDEX`. Wrappers: `decorations`, `nDecorations`, `decoration`,
  `ayahMarkDecoration`, `bannerDecoration`, `onDecorationTap`, `Sel.decoration`,
  `Sel.decorationIndex`, `index`, `lineNumber`, `ayahIndex`, `lineIndex`, `number`.

### Added
- `QvpViewPolicy` in QvpKit: the zoom limits, zoomed threshold and swipe classifier both iOS
  renderers share. `docs/API-PARITY.md` lists every platform convenience.
- Parity: every declared gap names its kind (issue, native, declarative, list form, not
  applicable) and the platform-level gaps have issues (#40 Flutter iOS, #41 React Native
  iOS, #42 React Native tarball). New bindings: web `engine.version()`, `engineName()`,
  `markFromName()`, `atlas.pages()`, `atlas.json()`; Android `QvpEngine.engineName()` and
  `QvpPageView.layoutSpec()`; React Native `hitTest`, `hitTestEx`, `hitTestView`,
  `layoutGapToFill`, `markCategory`, `engineName`, `nameCount`, `nameId` and a `from`
  field on the `mask` prop.
- `docs/API.md` lists every C symbol with its reference-wrapper spelling and what it does;
  the 32 that no section named are documented.
- Defaults: `QVP_DEFAULT_*` in the header, defined once in `crates/qvp-core/src/defaults.rs`
  (ink, highlight colours, padding and seam, selection band, gap bias, tap distance, nominal
  lines, aspect slack, mask colour, padding and radius, reveal lit and grey, crop padding) and
  mirrored as `QvpDefaults` in every wrapper; the parity check fails when any copy differs.
- The terminology audit passes with zero findings and CI fails on any new one. Code
  comments and documents use the canonical spellings; the shared search-fold
  specification and its fixtures are excluded with a written reason, since their case ids
  belong to the cross-repository spec.
- Names: `qvp_name(table, id)`, `qvp_name_id(table, name)`, `qvp_name_count(table)` over the
  mark, kind, family, category, decoration, division and place tables. Every wrapper reads
  its names from the engine at start-up; the five hand-written mark tables and the place,
  division and decoration lists are gone. React Native's kind and decoration names now
  match the engine's (`ayah_number`, `ayah-mark`); an unknown name resolves to 255, not 0.
- `docs/FORMAT.md` specifies the QVP1 page format and the QVA1 atlas byte by byte;
  `docs/DATA.md` records which mushaf the data is, its source release, how it is built and
  its terms. The format crate's top comment describes the on-disk layout correctly.
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
- Highlight and mask boxes clamp their corner radius to half the box's shorter side in the
  engine, so every renderer draws the same shape.
- Zoom limits are 0.5 to 12 times the fitted scale on every platform (web and Flutter were
  0.2 to 40).
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
