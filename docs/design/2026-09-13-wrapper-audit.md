# Wrapper audit, 2026-09-13

A sweep of every wrapper and example for arithmetic and lookup tables, classified against
the rule in `docs/standards/API-DESIGN.md`: a wrapper may convert units and track gestures,
and nothing else. This page records what the sweep found; `docs/API-PARITY.md` tracks what
has moved into the engine since. Items 1 to 4 moved in the change that introduced this
page (`QvpLayout.fit_*`, `QvpLayoutSpec.max_aspect_slack`, `crop_left`/`crop_right`,
`qvp_layout_gap_to_fill`). Item 5 followed: `qvp_name`, `qvp_name_id` and `qvp_name_count`
cover every table, and no wrapper carries one. Items 6 and 7 followed: the defaults are
`QVP_DEFAULT_*` in the header, defined in `crates/qvp-core/src/defaults.rs`, mirrored as
`QvpDefaults` in every wrapper, and compared by the parity check.

## Computations that belong in the engine

| # | concept | where it is computed today | divergence | engine home |
|---|---|---|---|---|
| 1 | fit scale and centred offsets | `web/example/app.js`, `web/lite.mjs`, `QvpPageView.kt`, `QvpPageView.swift`, `QvpPageCanvas.swift`, `page_view.dart`, `QvpRnPageView.kt` | web and Flutter clamp the scale at 1 and allow negative offsets; iOS and Android clamp offsets at 0; Android skips horizontal centring and React Native patches it back | `QvpLayout.fit_scale`, `fit_x`, `fit_y`, computed by `qvp_layout` |
| 2 | the `1.15` aspect guard on the content width | `app.js`, `page_view.dart` | present in two wrappers, absent in three | `QvpLayoutSpec.max_aspect_slack` |
| 3 | side-margin crop expressed as negative padding | `QvpPageCanvas.swift` | iOS only; re-derives the engine's scale formula | `QvpLayoutSpec.crop_left`, `crop_right` |
| 4 | the inner box handed to `qvp_gap_to_fill` | every example | React Native uses the literal 16 instead of twice the side padding | `qvp_gap_to_fill` takes the layout spec |
| 5 | mark, kind, family, category, decoration, place and division name tables | `qvp.js`, `Types.kt`, `Types.swift`, `engine.dart`, RN `index.tsx` and `Marshal.kt` | RN's Kotlin half says `ayah-ayahMark` where its JavaScript half says `ayah-mark`; RN lists 5 kinds where the header has 8; web, Flutter and RN prefer their table over the engine's name functions | `qvp_mark_name` and friends exist; add `qvp_decoration_name`, `qvp_place_name`, `qvp_division_name`; delete every table |
| 6 | defaults: gap bias 0.6, nominal lines 15, highlight pad 1.2 and seam 0.25, crop pad 2, mask 0.6/0.6/0.8, ink `#231f20`, band `#d6a326` at 0.3, selection band `#2d6fd6` at 0.25 | re-declared in four to eight places each | equal today, by discipline | `QVP_DEFAULT_*` constants in the header; wrappers reference them |
| 7 | default hit distance | view layers use 6, engine layers 0, one web hover path uses 4 | one stray value | `QVP_DEFAULT_TAP_DISTANCE` in the header |
| 8 | zoom limits | web and Flutter 0.2 to 40; iOS and Android 0.5 to 12 | two conventions | view state, but one shared constant per platform family; recorded as a platform convenience |
| 9 | `isZoomed` threshold and the swipe classifier | both iOS renderers | duplicated within one platform | one shared Swift policy value |
| 10 | line-spacing clamp | `QvpPageCanvas.swift` | the engine already clamps | delete |
| 11 | hover box inflation | `app.js` | web only | `qvp_band_boxes` with a pad |
| 12 | corner-radius clamp on highlight boxes | iOS only | boxes render differently per platform | clamp inside `qvp_highlight_boxes` and `qvp_mask_boxes` |
| 13 | nearest page versus clamped page on navigation | four examples pick the nearest available page, iOS clamps | visible to users | the example-app spec fixes it: nearest |
| 14 | reveal slider bound | `steps - 1` in three examples, `max(steps - 1, 1)` on iOS | one divergence | `qvp_reveal_step_count` is the bound; examples stop computing it |
| 15 | zoom spring easing | `QvpZoomSpring.swift` | iOS only; a second clock beside the engine's transitions | declared platform convenience |

## Bugs found on the way

- `Marshal.kt` names the ayah mark decoration `ayah-ayahMark`; the JavaScript side uses
  `ayah-mark`, so a React Native app never matches an ayah-mark tap by name.
- React Native's `named()` maps an unknown selector name to id 0 instead of failing.
- React Native's `KIND_NAMES` and its `(0..4)` enumeration omit the ornament, page-number
  and running-head kinds.

## What is marshalling and stays

Struct offsets, bit unpacking, colour packing to `0xRRGGBBAA`, output-buffer capacities,
byte and time formatting, density conversion, and the pan, pinch and scroll transforms.
`web/lite.mjs` keeps its decoding arithmetic by charter; its `fit()` is item 1.

## Resolution

Items 1 to 7 moved into the engine (fit, aspect slack, crop, gap to fill, name tables,
defaults, tap distance). Items 8 to 15 were resolved as follows:

| # | resolution |
|---|---|
| 8 | one pair of zoom limits, 0.5 to 12, in every page view; listed under platform conveniences in `docs/API-PARITY.md` |
| 9 | `QvpViewPolicy` in QvpKit holds the zoomed threshold and the swipe classifier for both renderers |
| 10 | the clamp in `QvpPageCanvas.swift` is deleted; the engine clamps |
| 11 | the web hover box comes from `qvp_band_boxes` with a pad |
| 12 | `qvp_highlight_boxes` and `qvp_mask_boxes` clamp the corner radius to half the shorter side; the iOS clamp is deleted |
| 13 | `docs/EXAMPLE-APP.md` states the nearest-page rule; the iOS example bundles every page, so its clamp is the same rule |
| 14 | the iOS slider bound is `steps - 1` like the other examples |
| 15 | declared as a platform convenience in `docs/API-PARITY.md` |

