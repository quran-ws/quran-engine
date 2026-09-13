# API parity

One row per C symbol in `crates/qvp-ffi/include/qvp.h`, one column per wrapper. `yes`
means the wrapper binds the symbol under the name `docs/standards/NAMING.md` prescribes.
`gap (…)` means the wrapper does not bind it and the gap is declared below with an issue.
`**missing**` means an undeclared gap, which fails CI.

`scripts/check-parity.py` regenerates the table between the markers; `--check` verifies
it. A second table lists platform-level gaps.

## Declared gaps

A gap is one of four kinds. Each row says which, so the reader can tell whether anything
is owed.

- `issue #N`: a binding is owed; the issue tracks it.
- `native: …`: the symbol is a rendering or lifetime call the wrapper's own view makes;
  the app never needs it.
- `declarative: …`: the wrapper exposes the behaviour through props that reconcile
  handles natively.
- `list form: …` and `not applicable: …`: the same operation under another shape, or a
  symbol that exists for one host only.

| symbol | wrapper | why |
|---|---|---|
| `qvp_alloc` | android | not applicable: the host allocates; `qvp_alloc` serves the wasm host |
| `qvp_alloc` | ios | not applicable: the host allocates; `qvp_alloc` serves the wasm host |
| `qvp_alloc` | react-native | not applicable: the host allocates; `qvp_alloc` serves the wasm host |
| `qvp_dealloc` | android | not applicable: the host allocates; `qvp_alloc` serves the wasm host |
| `qvp_dealloc` | ios | not applicable: the host allocates; `qvp_alloc` serves the wasm host |
| `qvp_dealloc` | react-native | not applicable: the host allocates; `qvp_alloc` serves the wasm host |
| `qvp_page_load` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript (`pageUri` prop) |
| `qvp_page_free` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_geometry` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_tick` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_paint` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_styled` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_color_of` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_highlight_boxes_view` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_mask_boxes_view` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_hit_areas` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_line_bands` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_word_bands` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_style_add` | react-native | declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript |
| `qvp_style_add_target` | react-native | declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript |
| `qvp_style_remove` | react-native | declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript |
| `qvp_style_repaint` | react-native | declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript |
| `qvp_style_clear` | react-native | declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript |
| `qvp_style_clear_layer` | react-native | declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript |
| `qvp_style_default` | react-native | declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript |
| `qvp_clear_highlights` | react-native | declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript |
| `qvp_highlight_words` | react-native | declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript |
| `qvp_surah_at` | react-native | list form: bound as `surahs()` |
| `qvp_surahs_count` | react-native | list form: bound as `surahs().length` |
| `qvp_atlas_surah_at` | react-native | list form: bound as `atlasSurahs()` |
| `qvp_word_info` | react-native | list form: bound as `word(i)` and `words()` |
| `qvp_ayah_info` | react-native | list form: bound as `ayahs()` |
| `qvp_line_info` | react-native | list form: bound as `lines()` |
| `qvp_deco_info` | react-native | list form: bound as `decos()` |
| `qvp_text_target` | react-native | list form: `text(target)` takes a target |

## Platform-level gaps

| gap | wrappers | issue |
|---|---|---|
| iOS support (no `ios/` directory, no podspec) | flutter | #40 |
| iOS support (no podspec) | react-native | #41 |
| the npm tarball builds only inside this repository (`qvpAndroidDir` default) | react-native | #42 |

## Renames for 0.2

The header cleanup `docs/standards/NAMING.md` calls for. 0.2.0 is pre-1.0, so the old
names go without aliases; this table is the migration guide and the changelog repeats it.
Wrapper spellings are the reference wrapper's (`web/qvp.js`); the others use the same names
in their own casing. The `_view` suffix, `_count`, `_of`, `is_` and the full-word rule apply
to every language.

### Functions

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
| colours | `qvp_style_default` | `qvp_style_default_color` | `page.setDefaultInk` | `page.defaultColor` |  |
| colours | `qvp_style_remove` | `qvp_style_remove` | `page.unstyle` | `page.removeStyle` | wrapper only: `un-` is not a verb in the vocabulary |
| abbreviations | `qvp_deco_info` | `qvp_decoration_info` | `page.decos[i]` | `page.decorations[i]` |  |
| abbreviations | `qvp_natural_pitch` | `qvp_page_line_spacing` | `page.naturalPitch` | `page.lineSpacing` | the owning noun carries the meaning |
| layout | `qvp_gap_to_fill` | (removed) | `engine.gapToFill(pageW, …)` | (removed) | `qvp_layout_line_spacing_to_fill` covers it from a spec |
| layout | `qvp_layout_gap_to_fill` | `qvp_layout_line_spacing_to_fill` | `page.layoutGapToFill` | `page.layoutLineSpacingToFill` | returns the multiplier, not a gap: `line_gap` is gone |
| layout | `qvp_wasted_fraction` | `qvp_layout_wasted_fraction` | `engine.wastedFraction(pageW, …)` | `page.layoutWastedFraction(spec)` | takes the page and a spec |
| layout | (new) | `qvp_page_grid` |  | `page.grid` | `{lines, line_spacing}` of the mushaf's design grid; replaces `QvpLayoutSpec.nominal_lines` |
| version | `qvp_version` | `qvp_format_version` | `engine.version` | `engine.formatVersion` | the page format version |
| version | (new) | `qvp_version` |  | `engine.version` | the engine version as a string, `0.2.0` |

### Types, fields, enums and parameters

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
| layout | `QvpLayoutSpec.nominal_lines` | (removed) | the engine reads the grid from the page (`qvp_page_grid`) |
| layout | `QVP_DEFAULT_NOMINAL_LINES` | `QVP_DEFAULT_GRID_LINES` |  |
| booleans | every bool field and parameter | `uint8_t` | `fill_height`, `prefer_exact`, `is_exact`, `is_loose_match`, `normalize`, `loose_match`, `keep_ayah_marks`, `by_ayah`, `ayah_marks`, `reverse` |
| booleans | `qvp_ayah_word_count(complete)` | `qvp_ayah_word_count(is_complete)` |  |
| booleans | `qvp_selection_text(citation)` | `qvp_selection_text(include_citation)` |  |

### Wrapper fields

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

## Platform conveniences

Members a wrapper adds in its own idiom. Each is view state or a host service; none of them
makes a decision the engine could make (`docs/standards/API-DESIGN.md`, the three tiers).

| convenience | where | note |
|---|---|---|
| pan and pinch transform | every page view | zoom limits 0.5 to 12 times the fitted scale on every platform: `QvpViewPolicy` (iOS), `QvpPageView.MIN_ZOOM` (Android), `kMinZoom` (Flutter), `clampZoom` in the web example |
| zoomed threshold and swipe classifier | iOS, `QvpViewPolicy` | a pinch settled within 2% of the fitted scale is not a zoom; a mostly horizontal drag longer than 40 pt or faster than 500 pt/s is a page swipe |
| zoom spring | iOS, `QvpZoomSpring` | eases a released pinch back to the fitted transform on a display link of its own |
| page cache | iOS, `QvpPageCache` | the policy in `docs/EXAMPLE-APP.md`; the other platforms follow it in their examples |
| long press, double tap, tap callbacks | every page view | gesture recognition only; the hit comes from `qvp_hit_test` |

## Matrix

<!-- parity:begin -->
114 symbols in the header, 114 Rust exports. web: 114 bound, android: 112 bound, flutter: 114 bound, ios: 112 bound, react-native: 83 bound.

| symbol | web | android | flutter | ios | react-native |
|---|---|---|---|---|---|
| `qvp_alloc` | yes | gap (not applicable: the host allocates; `qvp_alloc` serves the wasm host) | yes | gap (not applicable: the host allocates; `qvp_alloc` serves the wasm host) | gap (not applicable: the host allocates; `qvp_alloc` serves the wasm host) |
| `qvp_arabic` | yes | yes | yes | yes | yes |
| `qvp_atlas_division` | yes | yes | yes | yes | yes |
| `qvp_atlas_division_at` | yes | yes | yes | yes | yes |
| `qvp_atlas_find_surah` | yes | yes | yes | yes | yes |
| `qvp_atlas_free` | yes | yes | yes | yes | yes |
| `qvp_atlas_json` | yes | yes | yes | yes | yes |
| `qvp_atlas_load` | yes | yes | yes | yes | yes |
| `qvp_atlas_page_of` | yes | yes | yes | yes | yes |
| `qvp_atlas_page_range` | yes | yes | yes | yes | yes |
| `qvp_atlas_pages` | yes | yes | yes | yes | yes |
| `qvp_atlas_pages_of_juz` | yes | yes | yes | yes | yes |
| `qvp_atlas_surah` | yes | yes | yes | yes | yes |
| `qvp_atlas_surah_at` | yes | yes | yes | yes | gap (list form: bound as `atlasSurahs()`) |
| `qvp_atlas_surahs` | yes | yes | yes | yes | yes |
| `qvp_attach_words` | yes | yes | yes | yes | yes |
| `qvp_ayah_info` | yes | yes | yes | yes | gap (list form: bound as `ayahs()`) |
| `qvp_ayah_keys` | yes | yes | yes | yes | yes |
| `qvp_ayah_label` | yes | yes | yes | yes | yes |
| `qvp_ayah_marks` | yes | yes | yes | yes | yes |
| `qvp_ayah_word_count` | yes | yes | yes | yes | yes |
| `qvp_category_name` | yes | yes | yes | yes | yes |
| `qvp_citation` | yes | yes | yes | yes | yes |
| `qvp_clear_highlights` | yes | yes | yes | yes | gap (declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript) |
| `qvp_color_of` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_crop_bounds` | yes | yes | yes | yes | yes |
| `qvp_crop_svg` | yes | yes | yes | yes | yes |
| `qvp_dealloc` | yes | gap (not applicable: the host allocates; `qvp_alloc` serves the wasm host) | yes | gap (not applicable: the host allocates; `qvp_alloc` serves the wasm host) | gap (not applicable: the host allocates; `qvp_alloc` serves the wasm host) |
| `qvp_deco_info` | yes | yes | yes | yes | gap (list form: bound as `decos()`) |
| `qvp_divisions` | yes | yes | yes | yes | yes |
| `qvp_engine_name` | yes | yes | yes | yes | yes |
| `qvp_family_name` | yes | yes | yes | yes | yes |
| `qvp_find_word` | yes | yes | yes | yes | yes |
| `qvp_gap_to_fill` | yes | yes | yes | yes | yes |
| `qvp_geometry` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_has_form` | yes | yes | yes | yes | yes |
| `qvp_hide` | yes | yes | yes | yes | yes |
| `qvp_hide_all` | yes | yes | yes | yes | yes |
| `qvp_hide_back` | yes | yes | yes | yes | yes |
| `qvp_hide_word` | yes | yes | yes | yes | yes |
| `qvp_highlight` | yes | yes | yes | yes | yes |
| `qvp_highlight_boxes_view` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_highlight_handles` | yes | yes | yes | yes | yes |
| `qvp_highlight_words` | yes | yes | yes | yes | gap (declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript) |
| `qvp_hit_areas` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_hit_test` | yes | yes | yes | yes | yes |
| `qvp_hit_test_exact` | yes | yes | yes | yes | yes |
| `qvp_hit_test_exact_view` | yes | yes | yes | yes | yes |
| `qvp_hit_test_view` | yes | yes | yes | yes | yes |
| `qvp_kind_name` | yes | yes | yes | yes | yes |
| `qvp_layout` | yes | yes | yes | yes | yes |
| `qvp_layout_gap_to_fill` | yes | yes | yes | yes | yes |
| `qvp_line_bands` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_line_info` | yes | yes | yes | yes | gap (list form: bound as `lines()`) |
| `qvp_mark_category` | yes | yes | yes | yes | yes |
| `qvp_mark_from_name` | yes | yes | yes | yes | yes |
| `qvp_mark_name` | yes | yes | yes | yes | yes |
| `qvp_mask` | yes | yes | yes | yes | yes |
| `qvp_mask_boxes_view` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_mask_from` | yes | yes | yes | yes | yes |
| `qvp_mask_hidden` | yes | yes | yes | yes | yes |
| `qvp_mask_options` | yes | yes | yes | yes | yes |
| `qvp_mask_words` | yes | yes | yes | yes | yes |
| `qvp_name` | yes | yes | yes | yes | yes |
| `qvp_name_count` | yes | yes | yes | yes | yes |
| `qvp_name_id` | yes | yes | yes | yes | yes |
| `qvp_natural_pitch` | yes | yes | yes | yes | yes |
| `qvp_page_free` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_page_info` | yes | yes | yes | yes | yes |
| `qvp_page_load` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript (`pageUri` prop)) |
| `qvp_paint` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_recite_map` | yes | yes | yes | yes | yes |
| `qvp_rehighlight` | yes | yes | yes | yes | yes |
| `qvp_resolve` | yes | yes | yes | yes | yes |
| `qvp_restyle_highlight` | yes | yes | yes | yes | yes |
| `qvp_reveal_all` | yes | yes | yes | yes | yes |
| `qvp_reveal_at` | yes | yes | yes | yes | yes |
| `qvp_reveal_goto` | yes | yes | yes | yes | yes |
| `qvp_reveal_next` | yes | yes | yes | yes | yes |
| `qvp_reveal_start` | yes | yes | yes | yes | yes |
| `qvp_reveal_step_of` | yes | yes | yes | yes | yes |
| `qvp_reveal_steps` | yes | yes | yes | yes | yes |
| `qvp_reveal_stop` | yes | yes | yes | yes | yes |
| `qvp_reveal_word` | yes | yes | yes | yes | yes |
| `qvp_rosettes` | yes | yes | yes | yes | yes |
| `qvp_sajdahs` | yes | yes | yes | yes | yes |
| `qvp_search` | yes | yes | yes | yes | yes |
| `qvp_select` | yes | yes | yes | yes | yes |
| `qvp_selection` | yes | yes | yes | yes | yes |
| `qvp_selection_text` | yes | yes | yes | yes | yes |
| `qvp_style_add` | yes | yes | yes | yes | gap (declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript) |
| `qvp_style_add_target` | yes | yes | yes | yes | gap (declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript) |
| `qvp_style_clear` | yes | yes | yes | yes | gap (declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript) |
| `qvp_style_clear_layer` | yes | yes | yes | yes | gap (declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript) |
| `qvp_style_default` | yes | yes | yes | yes | gap (declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript) |
| `qvp_style_handles` | yes | yes | yes | yes | yes |
| `qvp_style_remove` | yes | yes | yes | yes | gap (declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript) |
| `qvp_style_repaint` | yes | yes | yes | yes | gap (declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript) |
| `qvp_styled` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_surah_at` | yes | yes | yes | yes | gap (list form: bound as `surahs()`) |
| `qvp_surahs_count` | yes | yes | yes | yes | gap (list form: bound as `surahs().length`) |
| `qvp_text` | yes | yes | yes | yes | yes |
| `qvp_text_target` | yes | yes | yes | yes | gap (list form: `text(target)` takes a target) |
| `qvp_theme` | yes | yes | yes | yes | yes |
| `qvp_tick` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_unhighlight` | yes | yes | yes | yes | yes |
| `qvp_unmask` | yes | yes | yes | yes | yes |
| `qvp_version` | yes | yes | yes | yes | yes |
| `qvp_wasted_fraction` | yes | yes | yes | yes | yes |
| `qvp_word_bands` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_word_bounds_view` | yes | yes | yes | yes | yes |
| `qvp_word_form` | yes | yes | yes | yes | yes |
| `qvp_word_info` | yes | yes | yes | yes | gap (list form: bound as `word(i)` and `words()`) |
| `qvp_word_label` | yes | yes | yes | yes | yes |
<!-- parity:end -->
