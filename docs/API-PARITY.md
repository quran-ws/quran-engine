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
| `qvp_highlight_boxes` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_mask_boxes` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_hit_boxes` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_line_bands` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
| `qvp_band_boxes` | react-native | native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript |
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

## Platform conveniences

Members a wrapper adds in its own idiom. Each is view state or a host service; none of them
makes a decision the engine could make (`docs/standards/API-DESIGN.md`, the three tiers).

| convenience | where | note |
|---|---|---|
| pan and pinch transform | every page view | zoom limits 0.5 to 12 times the fitted scale on every platform: `QvpViewPolicy` (iOS), `QvpPageView.MIN_ZOOM` (Android), `kMinZoom` (Flutter), `clampZoom` in the web example |
| zoomed threshold and swipe classifier | iOS, `QvpViewPolicy` | a pinch settled within 2% of the fitted scale is not a zoom; a mostly horizontal drag longer than 40 pt or faster than 500 pt/s is a page swipe |
| zoom spring | iOS, `QvpZoomSpring` | eases a released pinch back to the fitted transform on a display link of its own |
| page cache | iOS, `QvpPageCache` | the policy in `docs/EXAMPLE-APP.md`; the other platforms follow it in their examples |
| long press, double tap, tap callbacks | every page view | gesture recognition only; the hit comes from `qvp_hit_test_ex` |

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
| `qvp_band_boxes` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_category_name` | yes | yes | yes | yes | yes |
| `qvp_citation` | yes | yes | yes | yes | yes |
| `qvp_clear_highlights` | yes | yes | yes | yes | gap (declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript) |
| `qvp_color_of` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_crop_box` | yes | yes | yes | yes | yes |
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
| `qvp_highlight_boxes` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_highlight_handles` | yes | yes | yes | yes | yes |
| `qvp_highlight_words` | yes | yes | yes | yes | gap (declarative: the `styles`, `highlights`, `theme` and `defaultInk` props reconcile handles natively; no handle reaches JavaScript) |
| `qvp_hit_boxes` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_hit_test` | yes | yes | yes | yes | yes |
| `qvp_hit_test_ex` | yes | yes | yes | yes | yes |
| `qvp_hit_test_view` | yes | yes | yes | yes | yes |
| `qvp_hit_test_view_ex` | yes | yes | yes | yes | yes |
| `qvp_kind_name` | yes | yes | yes | yes | yes |
| `qvp_layout` | yes | yes | yes | yes | yes |
| `qvp_layout_gap_to_fill` | yes | yes | yes | yes | yes |
| `qvp_line_bands` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
| `qvp_line_info` | yes | yes | yes | yes | gap (list form: bound as `lines()`) |
| `qvp_mark_category` | yes | yes | yes | yes | yes |
| `qvp_mark_from_name` | yes | yes | yes | yes | yes |
| `qvp_mark_name` | yes | yes | yes | yes | yes |
| `qvp_mask` | yes | yes | yes | yes | yes |
| `qvp_mask_boxes` | yes | yes | yes | yes | gap (native: `QvpRnPageView` calls it while rendering; no drawing happens in JavaScript) |
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
| `qvp_word_box_view` | yes | yes | yes | yes | yes |
| `qvp_word_form` | yes | yes | yes | yes | yes |
| `qvp_word_info` | yes | yes | yes | yes | gap (list form: bound as `word(i)` and `words()`) |
| `qvp_word_label` | yes | yes | yes | yes | yes |
<!-- parity:end -->
