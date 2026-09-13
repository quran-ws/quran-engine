# API parity

One row per C symbol in `crates/qvp-ffi/include/qvp.h`, one column per wrapper. `yes`
means the wrapper binds the symbol under the name `docs/standards/NAMING.md` prescribes.
`gap (…)` means the wrapper does not bind it and the gap is declared below with an issue.
`**missing**` means an undeclared gap, which fails CI.

`scripts/check-parity.py` regenerates the table between the markers; `--check` verifies
it. A second table lists platform-level gaps.

## Declared gaps

| symbol | wrapper | issue |
|---|---|---|
| `qvp_alloc` | android | untriaged |
| `qvp_alloc` | ios | untriaged |
| `qvp_alloc` | react-native | untriaged |
| `qvp_atlas_json` | web | untriaged |
| `qvp_atlas_pages` | web | untriaged |
| `qvp_atlas_surah_at` | react-native | untriaged |
| `qvp_band_boxes` | react-native | untriaged |
| `qvp_clear_highlights` | react-native | untriaged |
| `qvp_color_of` | react-native | untriaged |
| `qvp_dealloc` | android | untriaged |
| `qvp_dealloc` | ios | untriaged |
| `qvp_dealloc` | react-native | untriaged |
| `qvp_engine_name` | web | untriaged |
| `qvp_engine_name` | android | untriaged |
| `qvp_engine_name` | react-native | untriaged |
| `qvp_geometry` | react-native | untriaged |
| `qvp_highlight_boxes` | react-native | untriaged |
| `qvp_hit_boxes` | react-native | untriaged |
| `qvp_hit_test_ex` | react-native | untriaged |
| `qvp_hit_test_view` | react-native | untriaged |
| `qvp_mark_category` | react-native | untriaged |
| `qvp_mark_from_name` | web | untriaged |
| `qvp_mask_boxes` | react-native | untriaged |
| `qvp_paint` | react-native | untriaged |
| `qvp_style_add` | react-native | untriaged |
| `qvp_style_add_target` | react-native | untriaged |
| `qvp_style_clear_layer` | react-native | untriaged |
| `qvp_style_repaint` | react-native | untriaged |
| `qvp_styled` | react-native | untriaged |
| `qvp_surah_at` | react-native | untriaged |
| `qvp_surahs_count` | react-native | untriaged |
| `qvp_text_target` | react-native | untriaged |
| `qvp_tick` | react-native | untriaged |
| `qvp_version` | web | untriaged |

`untriaged` marks a gap found by the first parity run. Each one gets an issue or a binding
in the wrapper audit.

## Platform-level gaps

| gap | wrappers | issue |
|---|---|---|
| iOS support (no `ios/` directory, no podspec) | flutter | untriaged |
| iOS support (no podspec) | react-native | untriaged |
| the npm tarball builds only inside this repository (`qvpAndroidDir` default) | react-native | untriaged |

## Matrix

<!-- parity:begin -->
110 symbols in the header, 110 Rust exports. web: 105 bound, android: 107 bound, flutter: 110 bound, ios: 108 bound, react-native: 86 bound.

| symbol | web | android | flutter | ios | react-native |
|---|---|---|---|---|---|
| `qvp_alloc` | yes | gap (untriaged) | yes | gap (untriaged) | gap (untriaged) |
| `qvp_arabic` | yes | yes | yes | yes | yes |
| `qvp_atlas_division` | yes | yes | yes | yes | yes |
| `qvp_atlas_division_at` | yes | yes | yes | yes | yes |
| `qvp_atlas_find_surah` | yes | yes | yes | yes | yes |
| `qvp_atlas_free` | yes | yes | yes | yes | yes |
| `qvp_atlas_json` | gap (untriaged) | yes | yes | yes | yes |
| `qvp_atlas_load` | yes | yes | yes | yes | yes |
| `qvp_atlas_page_of` | yes | yes | yes | yes | yes |
| `qvp_atlas_page_range` | yes | yes | yes | yes | yes |
| `qvp_atlas_pages` | gap (untriaged) | yes | yes | yes | yes |
| `qvp_atlas_pages_of_juz` | yes | yes | yes | yes | yes |
| `qvp_atlas_surah` | yes | yes | yes | yes | yes |
| `qvp_atlas_surah_at` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_atlas_surahs` | yes | yes | yes | yes | yes |
| `qvp_attach_words` | yes | yes | yes | yes | yes |
| `qvp_ayah_info` | yes | yes | yes | yes | yes |
| `qvp_ayah_keys` | yes | yes | yes | yes | yes |
| `qvp_ayah_label` | yes | yes | yes | yes | yes |
| `qvp_ayah_marks` | yes | yes | yes | yes | yes |
| `qvp_ayah_word_count` | yes | yes | yes | yes | yes |
| `qvp_band_boxes` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_category_name` | yes | yes | yes | yes | yes |
| `qvp_citation` | yes | yes | yes | yes | yes |
| `qvp_clear_highlights` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_color_of` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_crop_box` | yes | yes | yes | yes | yes |
| `qvp_crop_svg` | yes | yes | yes | yes | yes |
| `qvp_dealloc` | yes | gap (untriaged) | yes | gap (untriaged) | gap (untriaged) |
| `qvp_deco_info` | yes | yes | yes | yes | yes |
| `qvp_divisions` | yes | yes | yes | yes | yes |
| `qvp_engine_name` | gap (untriaged) | gap (untriaged) | yes | yes | gap (untriaged) |
| `qvp_family_name` | yes | yes | yes | yes | yes |
| `qvp_find_word` | yes | yes | yes | yes | yes |
| `qvp_gap_to_fill` | yes | yes | yes | yes | yes |
| `qvp_geometry` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_has_form` | yes | yes | yes | yes | yes |
| `qvp_hide` | yes | yes | yes | yes | yes |
| `qvp_hide_all` | yes | yes | yes | yes | yes |
| `qvp_hide_back` | yes | yes | yes | yes | yes |
| `qvp_hide_word` | yes | yes | yes | yes | yes |
| `qvp_highlight` | yes | yes | yes | yes | yes |
| `qvp_highlight_boxes` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_highlight_handles` | yes | yes | yes | yes | yes |
| `qvp_highlight_words` | yes | yes | yes | yes | yes |
| `qvp_hit_boxes` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_hit_test` | yes | yes | yes | yes | yes |
| `qvp_hit_test_ex` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_hit_test_view` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_hit_test_view_ex` | yes | yes | yes | yes | yes |
| `qvp_kind_name` | yes | yes | yes | yes | yes |
| `qvp_layout` | yes | yes | yes | yes | yes |
| `qvp_line_bands` | yes | yes | yes | yes | yes |
| `qvp_line_info` | yes | yes | yes | yes | yes |
| `qvp_mark_category` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_mark_from_name` | gap (untriaged) | yes | yes | yes | yes |
| `qvp_mark_name` | yes | yes | yes | yes | yes |
| `qvp_mask` | yes | yes | yes | yes | yes |
| `qvp_mask_boxes` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_mask_from` | yes | yes | yes | yes | yes |
| `qvp_mask_hidden` | yes | yes | yes | yes | yes |
| `qvp_mask_options` | yes | yes | yes | yes | yes |
| `qvp_mask_words` | yes | yes | yes | yes | yes |
| `qvp_natural_pitch` | yes | yes | yes | yes | yes |
| `qvp_page_free` | yes | yes | yes | yes | yes |
| `qvp_page_info` | yes | yes | yes | yes | yes |
| `qvp_page_load` | yes | yes | yes | yes | yes |
| `qvp_paint` | yes | yes | yes | yes | gap (untriaged) |
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
| `qvp_style_add` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_style_add_target` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_style_clear` | yes | yes | yes | yes | yes |
| `qvp_style_clear_layer` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_style_default` | yes | yes | yes | yes | yes |
| `qvp_style_handles` | yes | yes | yes | yes | yes |
| `qvp_style_remove` | yes | yes | yes | yes | yes |
| `qvp_style_repaint` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_styled` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_surah_at` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_surahs_count` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_text` | yes | yes | yes | yes | yes |
| `qvp_text_target` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_theme` | yes | yes | yes | yes | yes |
| `qvp_tick` | yes | yes | yes | yes | gap (untriaged) |
| `qvp_unhighlight` | yes | yes | yes | yes | yes |
| `qvp_unmask` | yes | yes | yes | yes | yes |
| `qvp_version` | gap (untriaged) | yes | yes | yes | yes |
| `qvp_wasted_fraction` | yes | yes | yes | yes | yes |
| `qvp_word_box_view` | yes | yes | yes | yes | yes |
| `qvp_word_form` | yes | yes | yes | yes | yes |
| `qvp_word_info` | yes | yes | yes | yes | yes |
| `qvp_word_label` | yes | yes | yes | yes | yes |
<!-- parity:end -->
