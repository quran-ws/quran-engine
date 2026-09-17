# Changelog

All notable changes to the engine and its packages. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow
`docs/standards/VERSIONING.md`. Every package shares the version listed here.

## [Unreleased]
### Fixed
- A page draws what is inside its own box and nothing beyond it. The artwork for page 17
  puts that page's printed page number below the box and its running head above it — it is
  the only page of 604 that draws anything outside — and every host drew that ink wherever
  the layout happened to put it. The paths, their boxes and the path numbering are all
  unchanged, so a crop and an SVG export still carry every stroke the artwork drew.

### Changed
- Page data is rebuilt from `quran-svg-elements` v1.1.2, which moves ink that belongs to a
  surah name out of the basmalah beside it, on pages 77, 282 and 428. The other 601 pages
  are byte-identical to the previous data release. The release is `data-v0.3.0`, served as
  `cdn.quran.ws/qvp/v0.3.0/`; `scripts/sync-test-data.sh`, the wrapper page sync scripts and
  CI default to it.
- `@quran.ws/engine/lite` returns `index`, `lineIndex` and `ayahIndex`, the names the
  main entry point already returned. It kept the abbreviations the naming standard
  replaced, because the parity check read only `web/qvp.js` and `web/index.mjs`.
- Page data is rebuilt from `quran-svg-elements` v1.1.1, which redraws the ayah
  medallions to the ones the printed mushaf uses. All 604 pages change. The
  decomposition is the same, 6,236 ayahs and 77,432 words, and every `NNN.words.json`,
  `atlas.qva` and `atlas.json` is byte-identical to the previous data release. Pages 1
  and 2 no longer carry a duplicate ornament, so the ornament count matches the marker
  count on every page. The release is `data-v0.2.0`. `scripts/sync-test-data.sh` and CI
  default to it.
- Page data is published to `cdn.quran.ws/qvp/<version>/` instead of
  `qvp.quran.ws/<version>/`. The old hostname redirects, so existing URLs still
  resolve. One host now carries the releases of the whole stack, each repository
  in its own folder (`docs/CDN.md`).

### Added
- Rust releases use crates.io trusted publishing, and Flutter releases include the Android
  engine and publish `qvp_flutter` to pub.dev.
- Every example app is built by CI: the Android example, the React Native example's
  typecheck, the Flutter example's analysis in its own package, and a parse of the web
  example. An example that stops compiling now fails the build.
- The parity check reads the published JavaScript entry points and rejects an
  abbreviation the naming standard replaced.
- `latest.json` beside each family names its current version, so a consumer can
  resolve the newest release without knowing the tag.
- The wasm, Apple and Android builds are published to `cdn.quran.ws/engine/`.
- `scripts/cdn-put.sh`, the shared upload library, and
  `scripts/migrate-cdn-prefix.sh` for the one-time move of the published data.

### Fixed

- The React Native example typechecks again. It held the ayah word count as
  `complete` where the API returns `isComplete`, and called a `setLineGap` that no
  longer exists. Its lockfile recorded the linked library at 0.1.0.
- The iOS demo builds again. It read `QvpAtlasSurah.n`, which the naming standard
  renamed to `number`, and the surah sheet was one expression larger than the
  Swift type checker would finish. The iOS job piped the build into `tail`, so it
  took `tail`'s exit status and reported a failed build as a pass.
- The CDN publish verifies the bundle by its digest. It required a `br`
  Content-Encoding, which the edge never sets, because brotli is the bundle's
  file format and the client decodes it.
- Publishing documentation now reflects the packages and versions available from Maven Central,
  Swift Package Manager, npm and crates.io.

## [0.2.2] - 2026-09-14

### Added
- QvpKit can be installed from the repository as a remote Swift package. SwiftPM downloads
  the release XCFramework and verifies its checksum.

## [0.2.1] - 2026-09-14

### Added
- Android releases publish `ws.quran:qvp-android` to Maven Central when its credentials are
  configured.

### Fixed
- Android release AARs use 16 KiB-aligned 64-bit native libraries and are attached to each
  GitHub release with a SHA-256 checksum.

## [0.2.0] - 2026-09-13
### Changed
- The C header is renamed to the naming standard (`docs/standards/NAMING.md`), and every
  wrapper follows. 0.2.0 is pre-1.0, so no old name is kept as an alias; the old → new table
  at the end of this section is the migration guide. The engine version is `qvp_version`
  and the page format version, unchanged at 1, is `qvp_format_version`.
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
- Layout: one name for one concept. `qvp_page_line_spacing` (was `natural_pitch`;
  `page.lineSpacing`), `QvpLayout.line_spacing` (was `pitch`), `QvpLayout.offset_x` /
  `offset_y` (were `ox` / `oy`), `QVP_BAND_LINE_SPACING` (was `QVP_BAND_PITCH`).
  `QvpLayoutSpec.line_gap` is gone: leading is one multiplier, and
  `qvp_layout_line_spacing_to_fill` (was `layout_gap_to_fill`) returns that multiplier.
  `QvpLayoutSpec.nominal_lines` is `grid_lines` (0 = the page's grid) and `qvp_page_grid`
  reports the grid (`QvpGrid {lines, line_spacing}`); `QVP_DEFAULT_GRID_LINES` was
  `NOMINAL_LINES`. The free `qvp_gap_to_fill` is gone and `qvp_wasted_fraction` is
  `qvp_layout_wasted_fraction(page, spec)`. The page views drop `lineGap`; the conformance
  scenarios carry `lineSpacingToFill` and `wastedFraction`.
- Booleans are one byte: `is_exact`, `prefer_exact`, `fill_height`, `is_loose_match` are
  `uint8_t`, as are the boolean parameters (`normalize`, `loose_match`, `keep_ayah_marks`,
  `by_ayah`, `ayah_marks`, `reverse`) and the boolean returns (`qvp_has_form`, `qvp_tick`,
  `qvp_highlight_move` / `_restyle` / `_remove`, `qvp_mask_word`, `qvp_unmask_word`,
  `qvp_reveal_goto`). `qvp_ayah_word_count` reports `is_complete`; `qvp_selection_text`
  takes `include_citation`.
- `qvp_version` returns the engine version as a string (`0.2.0`); `qvp_format_version`
  returns the page format version. Wrappers: `engine.version()` and `engine.formatVersion()`.
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

### Removed
- Three unreferenced React Native screenshots.

### Old → new

The complete rename table, copied from `docs/API-PARITY.md`.

#### Functions

| cluster | old C | new C | old wrapper | new wrapper | why |
|---|---|---|---|---|---|
| rectangles | `qvp_hit_boxes` | `qvp_hit_areas` | `page.hitBoxes` | `page.hitAreas` | a partition of each line, not a rectangle the app draws |
| rectangles | `qvp_highlight_boxes` | `qvp_highlight_boxes_view` | `page.highlightBoxes` | `page.highlightBoxesView` | viewport pixels, so `_view` |
| rectangles | `qvp_mask_boxes` | `qvp_mask_boxes_view` | `page.maskBoxes` | `page.maskBoxesView` | viewport pixels, so `_view` |
| rectangles | `qvp_band_boxes` | `qvp_word_bands` | `page.bandBoxes` | `page.wordBands` | the band of each word, page units |
| rectangles | `qvp_crop_box` | `qvp_crop_bounds` | `page.cropBox` | `page.cropBounds` | an extent you read |
| rectangles | `qvp_word_box_view` | `qvp_word_bounds_view` | `page.wordBoxView` | `page.wordBoundsView` | an extent you read, viewport pixels |
| hit tests | `qvp_hit_test` | `qvp_hit_test_exact` | `page.hitTest` | `page.hitTestExact` | the exact-outline variant is the special case |
| hit tests | `qvp_hit_test_view` | `qvp_hit_test_exact_view` | `page.hitTestView` | `page.hitTestExactView` |  |
| hit tests | `qvp_hit_test_ex` | `qvp_hit_test` | `page.hitTestEx` | `page.hitTest` | gap-aware is the common case; `_ex` is banned |
| hit tests | `qvp_hit_test_view_ex` | `qvp_hit_test_view` | `page.hitTestViewEx` | `page.hitTestView` |  |
| hide and reveal | `qvp_hide` | `qvp_style_hide` | `page.hide` | `page.hide` | the style prefix, like every subsystem |
| hide and reveal | `qvp_hide_word` | `qvp_mask_word` | `page.hideWord` | `page.maskWord` | the mask prefix pairs `mask` with `unmask` |
| hide and reveal | `qvp_hide_all` | `qvp_mask_all` | `page.hideAll` | `page.maskAll` |  |
| hide and reveal | `qvp_hide_back` | `qvp_mask_back` | `page.hideBack` | `page.maskBack` |  |
| hide and reveal | `qvp_reveal_word` | `qvp_unmask_word` | `page.revealWord` | `page.unmaskWord` | `reveal` is the greyed-page subsystem only |
| hide and reveal | `qvp_reveal_all` | `qvp_unmask_all` | `page.revealAll` | `page.unmaskAll` |  |
| hide and reveal | `qvp_reveal_next` | `qvp_unmask_next` | `page.revealNext` | `page.unmaskNext` |  |
| hide and reveal | `qvp_reveal_at` | `qvp_reveal_position` | `page.revealAt` | `page.revealPosition` | `_at(i)` means by index |
| hide and reveal | `qvp_reveal_steps` | `qvp_reveal_step_count` | `page.revealSteps` | `page.revealStepCount` | a plural is a list; a count is `_count` |
| highlight verbs | `qvp_highlight` | `qvp_highlight_add` | `page.highlight` | `page.highlight` | prefix first, verb second |
| highlight verbs | `qvp_rehighlight` | `qvp_highlight_move` | `page.rehighlight` | `page.moveHighlight` |  |
| highlight verbs | `qvp_restyle_highlight` | `qvp_highlight_restyle` | `page.restyleHighlight` | `page.restyleHighlight` |  |
| highlight verbs | `qvp_unhighlight` | `qvp_highlight_remove` | `page.unhighlight` | `page.removeHighlight` |  |
| highlight verbs | `qvp_clear_highlights` | `qvp_highlight_clear` | `page.clearHighlights` | `page.clearHighlights` |  |
| lookup verbs | `qvp_resolve` | `qvp_target_words` | `page.resolve` | `page.targetWords` | named for what it returns |
| lookup verbs | `qvp_atlas_find_surah` | `qvp_atlas_search_surahs` | `atlas.findSurah` | `atlas.searchSurahs` | `find` is an exact key; this is a text search |
| lookup verbs | `qvp_atlas_division_at` | `qvp_atlas_division_of` | `atlas.divisionAt / juzAt` | `atlas.divisionOf / juzOf` | `_of(surah, ayah)` is containment |
| plural or count | `qvp_surahs_count` | `qvp_surah_count` | `page.surahs().length` | `page.surahs().length` | wrappers keep the list form |
| plural or count | `qvp_atlas_pages` | `qvp_atlas_page_count` | `atlas.pages()` | `atlas.pageCount()` |  |
| plural or count | `qvp_atlas_surahs` | `qvp_atlas_surah_count` | `atlas.surahs()` | `atlas.surahs()` | the C count feeds the wrapper's list |
| plural or count | `qvp_styled` | `qvp_styled_paths` | `page.styled()` | `page.styledPaths()` |  |
| text twins | `qvp_text_target` | `qvp_text` | `page.text(target)` | `page.text(target)` | one `qvp_text` taking a target; the word-list form is `QVP_TARGET_WORDS` |
| colours | `qvp_paint` | `qvp_colors` | `page.paint()` | `page.colors()` | nouns for reads |
| colours | `qvp_style_repaint` | `qvp_style_recolor` | `page.restyle` | `page.recolorStyle` | verbs for writes |
| colours | `qvp_style_default` | `qvp_style_default_color` | `page.setDefaultInk` | `page.setDefaultColor` | `set` writes one value |
| colours | `qvp_style_remove` | `qvp_style_remove` | `page.unstyle` | `page.removeStyle` | wrapper only: `un-` is not a verb in the vocabulary |
| abbreviations | `qvp_deco_info` | `qvp_decoration_info` | `page.decos[i]` | `page.decorations[i]` |  |
| abbreviations | `qvp_natural_pitch` | `qvp_page_line_spacing` | `page.naturalPitch` | `page.lineSpacing` | the owning noun carries the meaning |
| layout | `qvp_gap_to_fill` | (removed) | `engine.gapToFill(pageW, …)` | (removed) | `qvp_layout_line_spacing_to_fill` covers it from a spec |
| layout | `qvp_layout_gap_to_fill` | `qvp_layout_line_spacing_to_fill` | `page.layoutGapToFill` | `page.layoutLineSpacingToFill` | returns the multiplier, not a gap: `line_gap` is gone |
| layout | `qvp_wasted_fraction` | `qvp_layout_wasted_fraction` | `engine.wastedFraction(pageW, …)` | `page.layoutWastedFraction(spec)` | takes the page and a spec |
| layout | (new) | `qvp_page_grid` |  | `page.grid` | `{lines, line_spacing}` of the mushaf's design grid; replaces `QvpLayoutSpec.nominal_lines` |
| version | `qvp_version` | `qvp_format_version` | `engine.version` | `engine.formatVersion` | the page format version |
| version | (new) | `qvp_version` |  | `engine.version` | the engine version as a string, `0.2.0` |

#### Types, fields, enums and parameters

| cluster | old | new | why |
|---|---|---|---|
| rectangles | `QvpHitBox` | `QvpHitArea` |  |
| rectangles | `QvpCropBox` | `QvpCropBounds` |  |
| hit tests | `QvpHitEx` | `QvpHit` | one struct: `word, path, decoration, line, distance, is_exact`; the exact variants fill it too |
| hit tests | `QvpHitOptions.exact_first` | `QvpHitOptions.prefer_exact` | an option is named after the behaviour |
| kind | `QvpDecoInfo.kind` | `QvpDecorationInfo.decoration` | a discriminator takes its enum's name |
| kind | `QvpDivision.kind` | `QvpDivision.division` |  |
| kind | `QvpTarget.kind` | `QvpTarget.target` |  |
| kind | `QvpSelector.kind` | `QvpSelector.selector` |  |
| kind | `qvp_arabic(kind)` | `qvp_arabic(op)` | `QVP_ARABIC_STRIP`, `FOLD`, `NORMALIZE`, `LOOSE`, `SEARCH_KEY` |
| kind | `QVP_DECO_*` | `QVP_DECORATION_*` |  |
| kind | `QVP_SEL_*` | `QVP_SELECTOR_*` |  |
| kind | `QVP_DIV_*` | `QVP_DIVISION_*` |  |
| kind | `QVP_HL_*` | `QVP_HIGHLIGHT_*` |  |
| kind | `QVP_BAND_PITCH` | `QVP_BAND_LINE_SPACING` |  |
| loose | `qvp_search(loose)` | `qvp_search(loose_match)` |  |
| loose | `QvpMatch.loose` | `QvpMatch.is_loose_match` |  |
| abbreviations | `QvpDecoInfo` | `QvpDecorationInfo` |  |
| abbreviations | `QvpPageInfo.n_decos` | `QvpPageInfo.n_decorations` |  |
| abbreviations | `QvpWordInfo.line_no / ayah_idx / line_idx` | `QvpWordInfo.line_number / ayah_index / line_index` |  |
| abbreviations | `QvpAyahInfo.ayah_mark_deco` | `QvpAyahInfo.ayah_mark_decoration` |  |
| abbreviations | `QvpLineInfo.line_no` | `QvpLineInfo.line_number` |  |
| abbreviations | `QvpHit.deco` | `QvpHit.decoration` |  |
| abbreviations | `QvpLineBand.line_no` | `QvpLineBand.line_number` |  |
| abbreviations | `QvpLayout.ox / oy` | `QvpLayout.offset_x / offset_y` |  |
| abbreviations | `QvpSurahInfo.banner_deco` | `QvpSurah.banner_decoration` |  |
| abbreviations | `QvpDivision.n / ayah_idx` | `QvpDivision.number / ayah_index` |  |
| abbreviations | `QvpAyahMark.deco` | `QvpAyahMark.decoration` |  |
| abbreviations | `QvpRosette.deco` | `QvpRosette.decoration` |  |
| abbreviations | `QvpSajdah.deco` | `QvpSajdah.decoration` |  |
| abbreviations | `QvpCropBox.ayah_mark_deco` | `QvpCropBounds.ayah_mark_decoration` |  |
| abbreviations | `QvpAtlasSurah.n` | `QvpAtlasSurah.number` |  |
| abbreviations | parameters `idx`, `wi`, `ai`, `vx`, `vy` | `index`, `word_index`, `ayah_index`, `view_x`, `view_y` |  |
| Info suffix | `QvpSurahInfo` | `QvpSurah` | a metadata record is a plain noun |
| layout | `QvpLayout.pitch` | `QvpLayout.line_spacing` |  |
| layout | `QvpLayoutSpec.line_gap` | (removed) | spacing is one multiplier |
| layout | `QvpLayoutSpec.nominal_lines` | `QvpLayoutSpec.grid_lines` | 0 means the page's own grid (`qvp_page_grid`); pass a count to lay a page out on another grid |
| layout | `QVP_DEFAULT_NOMINAL_LINES` | `QVP_DEFAULT_GRID_LINES` |  |
| booleans | every bool field and parameter | `uint8_t` | `fill_height`, `prefer_exact`, `is_exact`, `is_loose_match`, `normalize`, `loose_match`, `keep_ayah_marks`, `by_ayah`, `ayah_marks`, `reverse` |
| booleans | `qvp_ayah_word_count(complete)` | `qvp_ayah_word_count(is_complete)` |  |
| booleans | `qvp_selection_text(citation)` | `qvp_selection_text(include_citation)` |  |

#### Wrapper fields

The object fields every wrapper exposes follow the C fields:

| old | new |
|---|---|
| `word.lineIdx / ayahIdx / lineNo` | `word.lineIndex / ayahIndex / lineNumber` |
| `line.lineNo` | `line.lineNumber` |
| `hit.deco` | `hit.decoration` |
| `page.decos / nDecos` | `page.decorations / nDecorations` |
| `deco.kind` | `decoration.decoration` |
| `division.kind / n` | `division.division / number` |
| `ayah.ayahMarkDeco` | `ayah.ayahMarkDecoration` |
| `surah.bannerDeco` | `surah.bannerDecoration` |
| `layout.ox / oy / pitch` | `layout.offsetX / offsetY / lineSpacing` |
| `LayoutSpec.lineGap / nominalLines` | (removed) |
| `HitOptions.exactFirst` | `HitOptions.preferExact` |
| `hit.exact` | `hit.isExact` |
| `SearchOptions.loose / match.loose` | `SearchOptions.looseMatch / match.isLooseMatch` |
| `highlight style height 'pitch'` | `'lineSpacing'` |
| `onDecoTap` | `onDecorationTap` |

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

[Unreleased]: https://github.com/quran-ws/quran-engine/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/quran-ws/quran-engine/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/quran-ws/quran-engine/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/quran-ws/quran-engine/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/quran-ws/quran-engine/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/quran-ws/quran-engine/releases/tag/v0.1.0
