# Changelog

All notable changes to the engine and its packages. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/). Versions follow
`docs/standards/VERSIONING.md`. Every package shares the version listed here.

## [Unreleased]

### Added
- Flutter: `QvpEngine.viewSwipe(dx, dy, [vx, vy])` and `QvpEngine.swipePages(dx, dy, [vx, vy])`
  read a released drag, and `QvpPage.viewAnchor(view, word, nx, ny, toX, toY, viewportW,
  viewportH)` holds a point of a word on the screen across a new layout. The generated
  bindings declared all three, but no Dart call reached them (#165). Both backends bind them.
- The viewport-px twins `ayahMarksView`, `hitAreasView(gapBias)` and `wordBandsView(words,
  height, padX, padY)` on Android (`QvpPage`) and Flutter (`QvpPage`, both backends), and
  `ayahMarksView` on React Native. Each answers its page-unit call through the current
  layout, so an app that draws its own overlay on a reflowed page reads where each word or
  mark is drawn (#82). iOS bound all three already.

### Changed
- `docs/API-PARITY.md` declares no gap for these six symbols on Android and Flutter, and none for the three viewport-px twins on iOS.
  React Native keeps `qvp_hit_areas_view` and `qvp_word_bands_view` as gaps, with the reason
  that JavaScript hit-tests and places overlays through `hitTestView` and `wordBoundsView`.

### Removed

### Fixed
- React Native: the npm package builds in an app outside this repository. When the app's
  build has no `:qvp` project and no `qvpAndroidDir` property, the Android module depends
  on `ws.quran:qvp-android` from Maven Central, at the version in the package's
  `package.json`. The Gradle property `qvpVersion` sets a different version. Before, the
  module compiled the Kotlin library from `../../../android/qvp`, a path that exists only
  in this repository (#42). CI resolves the library from Maven Central through the
  example's build with `-PqvpFromMaven`.

## [0.8.0] - 2026-11-04

### Added
- `QVP_DEFAULT_REFLOW_RELAX`, `QVP_DEFAULT_REFLOW_MAX_STRETCH` and `QVP_DEFAULT_SELECTION_PAD_X`
  in `qvp.h`, mirrored in every wrapper's defaults table (`QvpDefaults` in Kotlin, Swift and
  Dart, `DEFAULTS` in JavaScript and React Native). The page views pass the selection pad
  from the table and the reflow specs take their defaults from it, so no wrapper copies the
  numbers by hand. `scripts/check-parity.py` compares 29 defaults where it compared 26.
- React Native: `DEFAULTS` holds every engine default under its header name, read from the
  native module, which sends the whole `QvpDefaults` table. A key the native module lacks is
  an error that names the key and says the native module is older than the package.
- Reflow can keep the print's line breaks with `breaks: 'printed'` (`QvpBreaks.printed`
  in Swift). Each line stays whole while its ink grows to fill the page width.
  `reflowMaxZoom` measures the widest row in this mode, and larger zooms clamp to it.
  Android's `QvpPageView.reflowBreaks` and the React Native `reflowBreaks` prop pick the
  breaks a pinch reflows with.
- Flutter: the package builds for iOS, macOS, Windows, Linux and web, alongside Android.
  iOS and macOS link `QvpEngine.xcframework` through a podspec and resolve symbols with
  `DynamicLibrary.process()`. `scripts/build-engine-ios.sh` copies the framework into both.
  Windows and Linux bundle `qvp_ffi.dll` and `libqvp_ffi.so` through CMake, and fail the
  build when neither the plugin directory nor the Cargo build holds one. Web runs the
  engine as WebAssembly over `dart:js_interop`, so call `await QvpEngine.initWeb()` before
  the first engine call.
- `scripts/sync-flutter-web-engine.sh` refreshes the two web assets the Flutter package
  ships. `scripts/check-parity.py` fails when its `qvp.js` copy drifts from `web/qvp.js`.
- Flutter: `QvpPage.batch(fn)` notifies listeners once however many mutations `fn` makes, and
  `QvpPage.path(i)` builds one path on demand and keeps it. `QvpPage.buildPaths()` returns the
  whole list as before. `QvpPage.ayahMarks()` holds its answer until the next mutation.

### Changed
- **Breaking.** Flutter: `QvpPageView.onWordTap` hands the host the `QvpWordInfo` where it
  passed the word's index, and `onDecorationTap` carries the `QvpHit` beside the
  decoration. Android and iOS pass the word and the hit already, so host code reads one
  shape on all three. `QvpWordTapCallback` and `QvpDecorationTapCallback` name the two
  shapes. The index is `word.index`, and a host with no use for the hit takes a second
  parameter and ignores it (issue #139).
- **Breaking.** Swift: `QvpOrnaments.find`, `QvpPage.dress` with a closed set and
  `QvpPassage.layout` on a closed passage stop the program with a fatal error that names the
  object, such as `QvpOrnaments is closed`. They returned nil, the same answer as a refused
  spec or no match. Check `isOpen` first where an object can close before its last call.
- **Breaking.** Rust: the `qvp-core` fields `Page.styles` and `Page.selection` are
  crate-private. Read them with `styles()` and `selection()`, and change them with
  `styles_mut()` and `selection_mut()`.
- CI builds the release XCFramework. `scripts/prepare-ios-release.sh` starts
  `prepare-ios-release.yml`, which builds it on macOS, attaches it to the draft release and
  pushes the commit that sets its URL, checksum and build commit in `Package.swift`.
  `release.yml` refuses a tag whose engine sources differ from that build commit.
- Flutter: `package:ffigen` generates `lib/src/bindings.dart` from `qvp.h`
  (`dart run tool/ffigen.dart`). Before, the file was written by hand.
  `scripts/check-flutter-bindings.sh` fails when the file and the header differ. The Flutter
  workflow runs it, and `scripts/check.sh parity` runs it when Flutter and libclang are
  installed. `scripts/check-parity.py` skips the generated file, so a symbol counts as bound
  in Flutter only when the package's own code names it. `docs/API-PARITY.md` lists as gaps
  the 14 symbols that the old file bound and no backend called.
  The binding object behind `QvpNativeBackend.b` follows the C names everywhere: `hide`,
  `highlight`, `moveHighlight`, `restyleHighlight`, `removeHighlight`, `clearHighlights`
  and `atlasSurahs` are now `styleHide`, `highlightAdd`, `highlightMove`,
  `highlightRestyle`, `highlightRemove`, `highlightClear` and `atlasSurahCount`.
- React Native: the Android module runs the page queries that read only the loaded page
  (`words`, `text`, `search`, the metadata calls and others), the engine-wide helpers and the
  atlas on a thread of its own, so a long query does not hold up a frame. `words` builds each
  word's record once per page and reads path names from tables it fills once. `T`, `Sel` and
  the module take the target and selector codes from one table that `scripts/gen-rn-enums.py`
  writes from `qvp.h`, and `scripts/check-parity.py` fails when the table is out of date. The
  example's lock file records the package version, and `scripts/set-version.sh` sets it.
- One composite action, `.github/actions/android-toolchain`, installs Java 17, the Android
  SDK, the pinned NDK and cmake, and cargo-ndk on request. The Android, Flutter, React Native
  and release workflows use it. It reads the NDK version from
  `scripts/build-engine-android.sh` and retries `sdkmanager` three times.
- Dependabot also updates the actions that `.github/actions/*` uses.
- The React Native workflow runs when the Android toolchain action or the NDK pin changes.
- `scripts/check-versions.sh` fails when an `ndkVersion` pin or an NDK mention in the Android
  docs and READMEs differs from `QVP_NDK_VERSION`.
- The iOS workflow runs the Demo's UI smoke test on the simulator. `docs/standards/TESTING.md`
  says which examples have one.
- The Flutter example's tool-written files are committed, so a clone stays clean after one
  build.
- The Flutter workflow runs `flutter pub get` once, before `dart format`.
- The Flutter workflow's format step passes `--language-version=latest`, so the style does
  not depend on the package's Dart floor.
- `scripts/sync-test-data.sh` takes `QVP_SYNC=pages` to fetch the data release and leave
  the SVG bundle alone. The Flutter and iOS jobs use it and cache `dist/pages` only.
  `scripts/build-engine-android.sh` and `scripts/sync-flutter-web-engine.sh` take
  `QVP_PROFILE`, `dist` by default. The Flutter example job builds its engine under
  `release`, frees the runner's unused toolchains first and caches the cargo registry
  without `target/`, after the Android build ran out of disk on main. The Flutter plugin
  and its example pin `ndkVersion` to the repository's NDK, so Gradle no longer
  downloads Flutter's default NDK beside it.
- The Flutter package's web engine assets, `assets/wasm/qvp_ffi.wasm` and `assets/js/qvp.js`,
  are build outputs and no longer tracked. The release workflow builds them from the tagged
  commit into the pub.dev package, as it does the Android libraries.
  `scripts/sync-flutter-web-engine.sh` produces them locally and the Flutter workflow runs it
  before the analyzer. `scripts/check-structure.sh` fails when either is tracked. The pubspec
  declares their directories, so a native build of a `git:` or `path:` dependency runs without
  them. A web build from such a dependency builds them first (see the package README).
- Shipped binaries build with a new cargo profile, `dist`, which holds the LTO, single
  codegen unit and `strip` settings that `release` held. `release` keeps `opt-level = 3`
  only, so `scripts/check.sh` rebuilds in seconds after one edit. The build scripts,
  `npm run build:wasm` and the Flutter desktop CMake files read `target/<triple>/dist/`.
  `Cargo.toml` declares the internal crate versions once, under `[workspace.dependencies]`,
  and `scripts/check-versions.sh` fails when a crate manifest declares its own.
- Reflow sizes a surah banner's row from its ink and normal line spacing, not its printed slot.
- The engine measures a page for reflow once and keeps the measure: the row unit of every
  word, the air between each word and the one before it, the sajdah lines, the surah frames
  and the group each path is drawn under. A reflow layout, `reflowMaxZoom` and every frame of
  a continuous pinch read it. The clock visits only the paths with a transition running, and
  `colors()` returns the kept display list without a copy. No layout or colour changes: the
  conformance scenarios are byte for byte the same. On page 042 a reflow layout takes about
  a fifth of the time it took, and an idle `tick` about a fortieth.
- `scripts/check.sh` and CI run the same commands once each: `gates` runs the web tests that
  read pages with `QVP_REQUIRE_DATA=1`, so `smoke.mjs`, `scenarios.test.mjs` and
  `passage.test.mjs` fail there without data instead of skipping; `parity` checks the
  table without rewriting it; a `shell` check runs shellcheck over every script; the
  `standards` job calls `scripts/check.sh`; `npm test` builds the wasm first and runs the
  replay tests; `scripts/check-terminology.sh` uses the pinned audit unless
  `QVP_TERMINOLOGY_SCRIPT` names another; Dependabot covers Gradle, pub and the React
  Native package.
- The prose linter that the writing rules name is in the repository, at
  `scripts/ste_lint.py`, under its own MIT notice. `scripts/check.sh docs` prints its report
  for every tracked document. The counts guide a writer and fail no check.
- The pull request template carries the lite passage layout tier, and a line for a change
  that fits no tier, such as CI. `CONTRIBUTING.md` lists four tiers. The template listed
  three.
- The test and example scripts and CI fetch `data-v0.5.0` by default. Its pages are
  byte-identical to `data-v0.4.0`. It adds `ornaments.qvo`, which holds ayah medallions only.
  The web smoke test skips its dress-by-style checks when no style in the set has all three
  ornament kinds.
- The React Native workflow compiles the Android module's Kotlin in release as well as in
  debug. It also runs when the Kotlin library's build file changes. Such a change broke the
  module once (#83) and now fails CI. The job installs the Android SDK, NDK and CMake the
  library's build reads. Gradle no longer downloads them mid-build.
- Flutter: `QvpPageView` caches the base ink and the ornament layer as `ui.Picture`
  recordings in layout units, where it recorded rasters at the current transform before. A
  pan or a pinch now replays the picture instead of redrawing the page. The ink and the
  draw-list keys carry the engine layout's offsets and the `surahFrames` flag, and the
  ornament layer is keyed on `QvpDressInfo.revision` and held per page, because a dress
  changes the colour and stroke width the picture bakes in.
- Flutter: `engine.dart` splits into `types.dart`, `page.dart`, `atlas.dart`,
  `ornaments.dart`, `passage.dart` and a backend interface. `QvpEngine` holds a
  `QvpBackend`, so the Dart API no longer names `dart:ffi` types and the same code compiles
  for web.
- Flutter: the native backend marshals input through a bump allocator over one scratch
  buffer, in place of an allocation per call. A nested call takes a slice of its own, so
  `text()` can pass two separators without the second overwriting the first.
- The Flutter package is `dart format` clean, and its workflow checks the formatting before
  it analyses. `docs/standards/CODE-STYLE.md` names the tool for Dart, and its section order
  matches the header it describes.
- `scripts/set-version.sh` stamps the default `qvpVersion` in `packages/android/build.gradle.kts`
  and `CFBundleShortVersionString` in `packages/ios/example/project.yml`.
  `scripts/check-versions.sh` checks both, and `docs/standards/VERSIONING.md` names the two
  files. The Android default was `0.1.0-SNAPSHOT` and is now the plain workspace version, so
  `publishToMavenLocal` needs no `-PqvpVersion` flag.
- The documents say what is true today. The README, the package READMEs and `docs/MEASURED.md`
  name the published version 0.7.0 and the measured wasm size. `docs/HOW-IT-WORKS.md` names the
  header as the function list. `docs/LITE-PASSAGES.md` names the npm test command. `docs/API.md`
  documents seven header symbols it left out: `qvp_layout_omitted_paths`,
  `qvp_layout_path_groups`, `qvp_sideways_drag`, `qvp_swipe_pages`, `qvp_zoom_at_step`,
  `qvp_zoom_carried` and `qvp_zoom_is_zoomed`. `docs/standards/CODE-STYLE.md` says review
  enforces the JavaScript, Kotlin and Swift doc blocks until the three linters it named are set
  up (issue #141).
- The FFI crate is one module per header section (`crates/qvp-ffi/src/page.rs`, `text.rs`,
  `hit.rs`, `view.rs`, `layout.rs`, `style.rs`, `highlight.rs`, `crop.rs`, `dress.rs`,
  `atlas.rs`, `passage.rs`, `names.rs`, with the shared structs and helpers in `types.rs`,
  `convert.rs` and `abi.rs`) instead of one file. The header and the 173 exported symbols are
  unchanged. `scripts/check-parity.py` reads every file under `crates/qvp-ffi/src/`. One
  behaviour note: `qvp_layout_groups`, `qvp_layout_repeats`, `qvp_layout_path_groups`,
  `qvp_layout_omitted_paths` and `qvp_layout_row_words` now write a truncated copy when the
  buffer is short and return the total count, as the header says every array output does.
  They wrote nothing in that case before. `cargo run -p qvp-ffi --release --example bench`
  times the getters a renderer calls every frame through the C ABI.
  `scripts/build-engine-android.sh` copies the header to the Android library, as its docs say.
- The getters a renderer calls every frame copy less and return the same numbers. A layout
  keeps the draw list it built last and the band it was for, so the fill call after a count
  call copies that list. `qvp_layout_placements` reads the groups in place.
  `qvp_highlight_boxes_view` and `qvp_mask_boxes_view` reuse one buffer and no longer copy the
  line shifts or the boxes of a highlight that is not moving. A `gap_bias` below 0 in
  `QvpHitOptions` and in `qvp_hit_areas` and `qvp_hit_areas_view` now means the engine
  default, 0.6, as a `max_distance` of 0 or less already means no limit.
- The core crate's reflow is a `reflow/` module with one file per step: row units, breaks,
  horizontal and vertical placement, banners and the sajdah line. `Page::from_data` runs
  named load steps in `load.rs`, and the words sidecar parser is in `sidecar.rs`. The unit
  tests share one fixtures module and sit in one file per area. Layouts, hits and the C ABI
  do not change.
- Android and Flutter: a page view frame where nothing changed asks the engine for the clock
  and nothing else. The views keep the styled paths, the highlight and mask boxes with their
  paths, and the dress readout until a call on the page, a new layout or the clock changes
  them. Small value keys replace the string keys of the base ink, the ornament layer and the
  draw list. On Android such a frame allocates nothing: on page 042 in the API 35 emulator it
  allocated 19 objects, or 1,071 with a highlight and a mask (`docs/MEASURED.md`). The
  numbers the views draw do not change.

### Removed
- **Breaking.** Flutter: `QvpEngine.resolveLibrary`, `QvpEngine.scratchBytes` and
  `QvpEngine.str` are gone. All three named `dart:ffi` types or the native scratch, which
  cannot sit on a platform-neutral class. `QvpNativeBackend` holds them now. Pass a library
  to `QvpEngine(library)` or a path to `QvpEngine.open(path:)` as before. Before 1.0 a
  removal takes a minor bump (`docs/standards/VERSIONING.md`), so this release is 0.8.0.
- **Breaking.** The pinch limits that each page view kept for itself are gone, because the
  engine applies them. The floor is the fitted page. Flutter: `kMinZoom` and `kMaxZoom` are
  gone. Use `QvpDefaults.maxZoom` and `QvpPageView.maxZoom`. `QvpPageView.minScale` and
  `maxScale`, which did nothing, are gone. `QvpViewController.zoomBy` takes `min` and `max`
  in place of `minScale` and `maxScale`. Android: `QvpPageView.MIN_ZOOM` is gone.
  `QvpPageView.MAX_ZOOM` stays and is `QvpDefaults.MAX_ZOOM`. iOS: `QvpViewPolicy.minZoom`,
  `zoomedThreshold`, `swipeAxisRatio`, `swipeDistance`, `swipeVelocity`, `clampZoom(_:)` and
  `clampZoom(_:fit:)` are gone. Use `QvpZoom.isZoomed(_:fitScale:)`,
  `QvpViewPolicy.swipeDirection(translation:velocity:)`, and `clampZoom(_:fit:ceiling:)` or
  `QvpView.zoomAbout`.

### Fixed
- Flutter: `flutter analyze` no longer warns on a fresh clone that the example's
  `assets/pages/` directory is missing. The directory holds a tracked `README.md`, and
  `.gitignore` keeps that file while it ignores the page data. The rule `pages/` now matches
  only the root `pages/`.
- Flutter: a call on a disposed `QvpEngine` throws `StateError('QvpEngine was disposed')` on a
  native build. A page, atlas or name call after `engine.dispose()` reached freed scratch
  memory. Freeing a page, atlas, passage or ornament set still works afterwards.
- `docs/API.md`: the "Every symbol" table has a row for each of the 173 header functions, in
  the header's order. It had 139 rows and left out the view, zoom and reflow functions and
  five layout getters. `scripts/check-parity.py` fails when a header function has no row.
- `scripts/sync-android-test-data.sh` runs under the bash 3.2 that macOS ships. It used
  `mapfile`, which that bash lacks, so the script failed on a Mac.
- The exact hit tests fill their hit with the core's conversion, as the gap-aware hit
  tests do. They are `qvp_hit_test_exact` and `qvp_hit_test_exact_view`, and `hitTestExact`
  and `hitTestExactView` in every wrapper. A hit on a decoration reports the line of the
  decoration's first path. It reported `QVP_NONE`: -1 in the wrappers, and `Int.MAX_VALUE`
  in Kotlin and in React Native on Android. A hit on the box of a word or a decoration,
  outside its ink, reports `is_exact` 0. It reported 1. `conformance/scenarios/hit.json`
  holds one point of each kind, and the web, Swift, Dart and Android tests replay it. The C
  ABI is unchanged.
- React Native: the Android module reads the camelCase names of the TypeScript types, so
  `wordForm`, `text`, `search` and `selectionText` with `'rasmImlai'` use that form, not the
  default. `rubuAlHizb` and the highlight height `'lineSpacing'` now resolve too. The snake
  and upper spellings still work. A word's `forms` and the page's `forms` use the same
  camelCase names as the `Form` type.
- Android: `QvpColor.parse` and `QvpColor.withAlpha` round alpha to the nearest of 255
  steps, as the Swift, Dart and web wrappers do. They truncated, so alpha 0.5 gave `0x7f` on
  Android and `0x80` everywhere else. A NaN alpha clamps to 0 in every wrapper, where
  Swift trapped, Dart threw and React Native returned a malformed colour.
- `scripts/check-parity.py` reads a hex colour whole. It stripped every trailing `f` as a
  suffix, so `0x231f20ffu` compared as `0x231f20` and a wrapper colour without its alpha
  byte passed. A failed default now also stops the summary from reporting agreement.
- Every wrapper handles a freed object, a second free and a large array result the same way
  (`docs/API.md`, conventions). A call on a freed page, atlas, ornament set or passage throws
  an error with the object's name: a `StateError` in Dart in every build mode, where a debug
  assert stood before, a fatal error in Swift and a rejection in React Native. The Flutter
  native backend no longer passes a freed handle's null pointer to the engine. A second
  `free`, `close` or `dispose` does nothing, Flutter's `QvpPage.dispose` included, and the
  Kotlin handle is atomic, so two threads that close the same object free it once. Every
  array getter grows its buffer to the total the engine reports, where the Android bridge,
  the web wrapper and the Flutter backend stopped at fixed caps such as 64 line bands, 1,024
  highlight handles and a 256 KB scratch.
- Every pan, zoom and view computation in the page views goes through the engine. The views on
  iOS, Android and Flutter no longer invert the view transform for a tap, place a magnifying
  pinch or draw the zoomed line themselves: they call `viewToLayout`, `viewZoomAbout` (the
  fitted page as the floor, the host's `maxZoom` as the ceiling) and `zoomIsZoomed`, pan a
  drag with `viewPan`, and each holds the view a drag or a pinch produces with `dressClamp`.
  React Native's `hitTestView` and `hitTestExactView` invert the Android view's pan and zoom
  with `viewToLayout`. When the fingers of a stepped pinch come back to the step it began on,
  the engine lays the page out at that step again. Before, it kept the step the pinch passed
  through. Every page view and the web example follow one pinch rule, stated in `docs/API.md`:
  the engine zooms about the focal point the gesture began on, by the factor of the whole
  gesture, and the host pans the answer by the drift of the focal point. Two fingers drag the
  page on every platform again, and a step committed in the middle of a gesture anchors on the
  view the last frame returned. The Flutter view drags with one finger, which it did not do
  before, and its `QvpViewController.zoomBy`, `panBy` and `toView` call the engine. On iOS, a
  pinch that ends while the reader selects no longer leaves the one-finger drag switched off.
  `zoomSpec` keeps the host's reflow knobs on the printed page on every wrapper. Flutter's
  `QvpViewController.zoomBy` stops at the fitted page, and a controller that a host hands to a
  new view keeps working. `conformance/scenarios/zoom.json` holds the engine's answers, and
  every wrapper test replays them.
- A malformed page, atlas or ornament file is an error from the decoder, not a panic or
  an abort. On wasm32 a panic is a trap, so a damaged download could take a host app down.
  The page decoder checks the magic, the version, `quant`, every count against its
  section and every cross reference before the engine indexes by one, and `Error::Bounds`
  names the table. It bounds each allocation by the bytes that could encode it and checks
  its arithmetic. `docs/FORMAT.md` lists what a reader rejects. The words sidecar reader
  rejects a lone surrogate and brackets nested past 64 deep. In Rust, `Page::from_data`
  now returns `Result` and runs `PageData::validate` first. The C ABI is unchanged.
- Flutter: a highlight band no longer freezes part way through a transition, and a removed
  band no longer stays on the page. The engine's highlight boxes depend on the clock, so the
  band cache clears on every frame rather than on a revision change.
- The Flutter and React Native examples build from a fresh clone. The root `.gitignore`
  ignored every PNG, so git never tracked the launcher icons and launch images that their
  Android manifests and the Flutter example's iOS and macOS asset catalogs name. The rule
  now covers the repository root only, and the templates' icons are tracked. The React
  Native example also gains the template's `debug.keystore`, which its Gradle signing
  config names and its `.gitignore` already allows. The Flutter example gains the
  template's `web/` folder, and the Flutter workflow builds the example for Android and
  web, so a missing file fails CI.
- Flutter: styled paint composites with `srcOver`. With `src`, a styled colour part way
  through a fade wrote its alpha over the paper rect in the same layer and opened a
  translucent hole.
- Flutter: a styled path the layout leaves out of the draw list is skipped. The fallback
  read `pathLine()`, which mixed a printed line number into a list of placements and put the
  glyph somewhere else on a reflowed page.
- Flutter: `QvpPageViewStats.basePaths` counts the paths a replayed picture draws, not only
  the paths of a frame that recorded one.
- Web: `free()` on a page, atlas, ornament set or passage is safe to call twice, and any
  call that reaches the engine on a freed one throws `QvpPage was freed` (or the class's
  name) instead of handing the engine a null handle. Cached records such as `page.words`
  stay readable.
- Web: an input longer than its scratch region, such as a target of over 768 words, a
  separator of over 256 bytes, or a long search string, goes in a block from the engine's
  allocator for the length of the call. Before, it overwrote the struct or the separator
  behind it.
- Web: `page.layout(spec)` and the other layout calls refuse a spec with a field that is
  not a finite number, with a `TypeError` that names the field, and keep the layout in hand.
  A page or ornament set the engine loads but gives no record for fails to load instead of
  leaving a null in `page.words` or `set.list`.
- Web: the wrapper frees a read buffer it outgrows. `engine.free()` frees every page, atlas,
  ornament set and passage still open, then the scratch buffers; `free()` on one of them
  afterwards does nothing, and every call on a freed engine throws `QvpEngine was freed`.
- React Native: the package checks that the native module is linked before it reads the
  module's constants. Without the module, the error now says `QvpModule not linked` instead
  of a `TypeError`.
- Flutter: after a zoom step lays the page out again, `QvpPageView` draws the highlight
  bands and the dress overflow of the new layout. It kept those of the old layout until the
  next call on the page.
- `scripts/check-flutter-bindings.sh` runs `flutter pub get` before it generates the
  bindings, so it no longer fails on a checkout whose `.dart_tool` predates the ffigen
  dependency. It restores `example/pubspec.lock` afterwards, and its failure message says
  what to run by hand.

## [0.7.0] - 2026-10-14

### Added
- `dress`: put another mushaf's or a font's ornaments on a page. A new QVO1 ornament set
  (`docs/FORMAT.md`), built by `qvp-convert ornaments` from `quran-ws/quran-assets`, holds
  ayah medallions, surah bands and page frames. The engine places each one from the page's
  own measurements and hands the host a display list to paint behind the ink; each draw
  carries a layout placement, so a dressed page reflows. Twelve C symbols. Every wrapper
  binds them and paints the ornament layer behind the ink: web, Android, Flutter, iOS and
  React Native. A dress names a `style`, which takes that mushaf's medallion, band and border
  together, and a kind key overrides it. Line art is chosen per kind. `bandGap` sets the air
  between a band and the border, `ayahMarkScale` sizes the medallion against the printed ring,
  and `pageFrameScale` sizes the border, which scales its pattern as a whole. Under reflow a
  band holds its title inside its slot. The layout's fit includes the dress, so the border is
  in view with no host arithmetic, and `qvp_dress_clamp` keeps a pan inside the dressed box.
  A host that scrolls the page itself places it with `qvp_dress_view` (`dressView`), which
  returns the view and the size that hold the whole border. The iOS canvas places a page under
  `hostScrolls` with it and reports the size as `hostPageSize`.
  A band or a border with no slot is refused. The data release ships the redistributable
  subset as `ornaments.qvo`.
- Passages reach every wrapper: `qvp.js` (`engine.loadPassage`,
  `engine.loadBasmalah`, `QvpPassage`, `CanvasRenderer.drawPassage`), the Android SDK
  (`QvpPassage`) and the Flutter plugin (`QvpPassage`, `paintPassage`). Each replays
  `conformance/scenarios/passage.json`, and the web replay runs in the gates. React Native shows
  one as `<QvpPassageView />`, laid out and drawn natively, which closes issue #102.
- A conformance scenario for a cut row too narrow for one word, the ellipsis and the medallion:
  58 passage cases, from 52.
- The Flutter page view takes `maxZoom` and reports `peekScale`, as the iOS and Android page
  views do, and `setZoomMode` switches the pinch to magnify.
- A passage word gives the box of the word alone, without its signs: `QvpPassageWord.word_x0`,
  `word_y0`, `word_x1`, `word_y1`, and `wordX0` to `wordY1` in `qvp.js`, the Android SDK, the
  Flutter plugin, QvpKit and React Native (`wordBox` in `web/lite-passage.mjs`). The word that
  closes an ayah carries the ayah medallion, so its box with signs runs under the medallion; an
  app that underlines a word reads the word's own box instead. `QvpPassageWord` grows from 28
  to 44 bytes, so every caller must use the new header. `conformance/scenarios/passage.json`
  gives each word both boxes, and every wrapper's replay checks the second.

### Fixed
- `surah_headers_view` on a reflowed page returned the printed text block scaled by the banner's
  growth as the frame box, which was wider than the row, and the screen, as soon as the reader
  zoomed in. The frame box is now the row the heading is on; the banner's growth still sets its
  height.
- A surah name whose every path is an ornament has a title box: the decoration's, rather than
  the empty box's sentinel corners. The headings are also measured once per page, where a host
  overlay asked on every pan and zoom frame.
- A passage cut to a row with no room for one word, the ellipsis and the medallion of its last
  ayah placed them outside the layout, left of the padding. The scale now shrinks, once, so that
  every word has room for what follows the cut, and the rows are broken again.
- `zoomSpec` on web, Android and Flutter dropped the host's reflow knobs on the printed page, so
  the first pinch off it reflowed with the engine's defaults. They now keep the knobs with zoom
  0, as QvpKit has since 0.5.0.
- The magnify pinch's ceiling (`maxZoom`) is clamped by the engine, in `qvp_view_zoom_about`,
  on iOS, Android and Flutter, rather than by each platform's own arithmetic. A ceiling of 0 or
  less is now the engine's own ceiling; a ceiling under the fitted page is still the fitted
  page, which `qvp_view_zoom_about` now holds to itself: a `max` under `min` is `min`.

## [0.6.1] - 2026-10-12

### Fixed
- A passage that runs from page 2 into page 3 draws every word at one size. Page 2 prints its
  words about 20% smaller than page 3, and the passage drew each word at its own page's size, so
  its words came out at two sizes. Its line spacing, the median over its words, also jumped by
  about 20% when the words of page 3 became the majority. Now every word has the size of the
  passage's first word: a word of a page printed at another size is resized (its placement has
  `kx = ky ≠ 1`), and the line spacing is that of the first word's size, however many ayahs
  follow. A passage of pages that print at one size lays out as before. `web/lite-passage.mjs`
  follows the engine, and its `draw` now applies `ky`.

## [0.6.0] - 2026-10-08

### Added
- Justified passages: `Align::Justified` (`QVP_ALIGN_JUSTIFIED`, QvpKit `.justified`, `'justified'`
  in `web/lite-passage.mjs`). Every row but the last opens its word gaps, in equal shares, until
  it reaches both edges, by at most `max_stretch` times the passage's usual air between two words
  (2 by default). Two words the print draws as one piece of calligraphy keep their distance.
  What a row has left over is split between its two sides, so the last row, the cut row with its
  ellipsis, a one-word row and a capped row are centred.
- A surah's basmalah as a passage: `Passage::load_basmalah`, `qvp_passage_load_basmalah`, QvpKit
  `QvpPassage(basmalahOf:pages:)`. It loads from the page that prints it and lays out like any
  passage, on one row, with no ayahs and no words; its line spacing is its page's, so a host sizes
  it by the printed line as it does an ayah. Surahs 1 and 9 fail the load. The other wrappers
  bind it with passages (issue #102).
- `conformance/scenarios/passage.json`: two justified specs on every range, and the basmalahs of
  surahs 2 and 114. QvpKit replays all 52 cases, `web/lite-passage.mjs` the 36 it supports.

### Changed
- C ABI: `QvpPassageSpec` gains a last field, `float max_stretch` (0 = the default, negative = no
  cap), and grows from 28 to 32 bytes. A C host that declares the struct itself must add it.

## [0.5.0] - 2026-10-08

### Added
- `QvpCanvasController.reflowFill` (iOS): how the rows of a page the reader zoomed into fill
  the width, `centred` (the default), `justified` or `ragged`.
- `QvpCanvasController.reflowMaxStretch` (iOS): how far a justified row's gaps may open, as a
  multiple of the page's usual air between two words (2 by default, negative for no cap).

### Changed
- Under `justified`, a row that cannot reach both margins is centred instead of starting at
  the right margin: the last row of a block, a single word, or a row whose gaps would have to
  stretch past `max_stretch`. Hung off the right margin, it read as a ragged column.
- The reflow knobs of `QvpLayoutSpec` are read on a printed page too (`reflow_zoom` 0): they
  are what the zoom control reflows the page with. A C host that zeroes the spec and pinches
  now reflows ragged (fill 0), greedy (breaks 0) and with printed gaps (gaps 0); 255, 255 and 1
  ask for the engine's defaults, which every wrapper already writes.

### Fixed
- On a reflowed row, a medallion is centred between the word it closes and the next one with
  the signs each carries, such as the quarter star (۞) that opens the next ayah. Measured on the
  letters alone, it sat against the star with the space on its other side.
- A pinch keeps the host's fill, breaks and gaps from the first step. The C layer dropped the
  reflow knobs while `reflow_zoom` was 0, so the layout the zoom control made inside a pinch
  always took the engine's defaults.
- `QvpPage.zoomSpec` (iOS) keeps the host's reflow knobs and sets only the zoom, on the printed
  page too. It rebuilt the reflow from the zoom and three spacing knobs, and dropped it at the
  printed size.

## [0.4.0] - 2026-10-07

### Added
- Passages in the core: `qvp_passage_*` lays out a range of complete ayahs on rows of a given
  width, from the outlines of their pages. Each word goes with its signs: a medallion and a
  sajdah sign with the word that closes their ayah, a division mark with the word that it opens,
  and a sajdah line with the words under it. A range can span two pages. `max_rows` cuts the
  passage after a whole word, keeps room for the host's ellipsis and, with `keep_ayah_mark`,
  places the last ayah's medallion after it. A width of zero or less puts the passage on one
  row. The passage copies what it needs, so the pages can be freed after it is loaded.
- `QvpPassage` (iOS): the passage over the C ABI, with `QvpPassageSpec` and `QvpPassageLayout`.
  The other wrappers bind it next (`docs/API-PARITY.md`).
- `conformance/scenarios/passage.json`: the core's layouts of six ranges at six specs. QvpKit
  replays all of them, and `web/lite-passage.mjs` replays the ones it supports.

### Changed
- `web/lite-passage.mjs` is now a port of the core's passage layout, not a JavaScript-only
  layout. A change to the layout starts in `crates/qvp-core/src/passage.rs`.

## [0.3.1] - 2026-10-05

### Added
- `QvpPageCache.setCurrentPage(_:span:)` (iOS): a host showing several pages at once says how
  many, and the cache keeps the screen before, the screen itself and the screen after loaded
  and safe from eviction — six pages for a two-page spread — so a swipe never lands on a blank
  half. The default span of 1 is the page and its two neighbors, as before.
- `maxZoom` on `QvpCanvasController` and `QvpPageView` (iOS) and on `QvpPageView` (Android):
  a host lowers the ceiling the magnifying glass is clamped to, from the policy's twelve, for
  furniture of its own that scales with the glass. A ceiling under the fitted page is the
  fitted page — a pinch never shrinks the page inside the screen.
  `QvpViewPolicy.clampZoom(_:fit:ceiling:)` is the iOS clamp.
- `peekScale` on the same three: how far the glass is over the fitted page — 1 at rest and in
  every mode but `.magnify`. A host overlay divides it out of the view transform to draw its
  furniture once at the fitted size and let the glass scale that one raster, instead of
  drawing it again at every size a pinch passes through.

### Changed
- iOS `QvpPageCanvas` adds a reveal to its cached ink instead of drawing the page again. A
  reveal only takes paths out of the styled set, so the paths it frees are filled onto the band
  already cached, and a band the revealed word is not on is kept as it is. A memorization review
  reveals a word at a time: on a page reflowed to its top zoom step a reveal cost ~15 ms and now
  costs 4–9 ms, measured on macOS.
- iOS `QvpPageCanvas` reads the engine clock, the styled paths and the highlight and mask boxes
  once per frame and shares them between the canvases a tall page is drawn as, rather than once
  per canvas.
- npm releases use GitHub trusted publishing, as crates.io and pub.dev already did, so the
  release workflow needs no `NPM_TOKEN`.

### Fixed
- iOS `QvpPageCanvas` draws a page taller than Core Animation's 8,192-px layer limit as a stack
  of canvases, each under it and cut on a whole pixel, with the cached ink band and the draw
  list kept per slice. A reflowed page at its top step on a phone is ~8,500 px; as one layer it
  was rasterized to fit and stretched back, so every glyph went soft and a surah name read as
  bold. A page that fits is one canvas, as before.
- iOS `QvpPageCanvas` draws its cached ink at the bitmap's own size. Stretched to a band a
  fraction of a pixel shorter, the bitmap was resampled, and rows down the page came out
  blended with their neighbours.
- Code releases now fail when registry credentials are missing, skip npm and CDN artifacts
  already published during a retry, and publish verified engine artifacts to the CDN even when
  a registry fails.

## [0.3.0] - 2026-09-21

### Added
- Optional `@quran.ws/engine/lite/passage` lays out complete ayah ranges in JavaScript without Wasm, including ranges across pages. It keeps the original word outlines, every ayah medallion, and the relevant division and sajdah signs. Synthetic, Rust-fixture and 604-page checks cover it.
- The lite geometry decoder exposes line, ayah, decoration and text metadata already present in QVP files. The wire format is unchanged.
- The page-data build emits frame-free surah titles as individual and combined QVP and SVG
  assets, plus a lossless OpenType/CFF WOFF2 font, CSS and a metadata map. Frames remain in
  the page files; `atlas.qva` remains the cross-Mushaf lookup and search index.
- `qvp_surah_headers` and `qvp_surah_headers_view`: the two boxes a host needs to draw its
  own surah frame. The first is the box a frame fills, the page's text block wide and the
  heading's row tall. The second is the title ink inside it. Both leave out a native frame,
  and the view call has already applied the layout's move of the heading's line. Every
  wrapper binds them.
- `surah_frames` on the layout spec controls source-native frames around surah names. It
  defaults on for the printed page; a host that supplies its own frame can turn it off.
  Reflowed titles and the reusable title assets remain frameless.
- `qvp_layout_printed_height`: the page's height laid out for a spec at the printed pitch, in
  viewport px — `content_h` before fill-height adds any leading. A host deciding whether a page
  fits a box, or how far to shrink it to keep it whole, asks this before it sizes the canvas,
  instead of re-deriving the crop and the grid a short page sits on. The iOS controller
  answers it for a width with its own knobs (`printedHeight(forWidth:)`).
- `banner_zoom` on the layout spec caps how big a surah name or a basmalah gets as the reader
  zooms in, as a multiple of its printed size. Left at 0 a banner grows with the words around
  it until it fills the row — about five times the print for a surah name — which a host that
  draws its own frame around the printed name cannot follow. 1 holds it at the print. It is a
  layout knob, not a reflow one: the zoom control fills the reflow knobs in itself, so a host
  that pinches never gets to set one there.
- `QvpCanvasController.printedPitch` (iOS): what a PRINTED row is worth in the canvas's box,
  in page units — the print's pitch opened up by `fillHeight`. A host that sizes its own
  furniture to a printed row needs one answer whichever layout is in force, and
  `currentLayout.lineSpacing` is not it: that reports the spread pitch on a printed page and
  the printed pitch once the page has reflowed onto rows of its own.
- A reader who zooms in gets the page reflowed onto rows of the screen's own width: the ink
  grows, fewer words fit a row, the rest move down, and the page scrolls. Each page carries
  its own zoom steps, chosen so the rows break best near each nominal size.
- The reader's zoom control, which the engine owns: `qvp_zoom_pinch`, `qvp_zoom_to_step`,
  `qvp_zoom_mode`, `qvp_zoom_spec`, `qvp_zoom_carried`, `qvp_zoom_at_step` and
  `qvp_zoom_is_zoomed`. A pinch reflows onto the page's own steps by default; a free reflow
  zoom and a plain magnify are the other two modes. A step takes the same small reach to
  leave either way, and a pinch never shrinks the page inside the screen.
- `qvp_layout_draw_list` gives the whole of what a layout draws as `{path, placement}`
  pairs, so one loop draws any page, reflowed or not.
- `qvp_sideways_drag` says whether a sideways drag pans the page or turns it, and
  `qvp_swipe_pages` says how many pages a released drag turns, in reading order: a mushaf
  is read right to left, so a flick to the right turns to the next page.
- iOS and Android draw a reflowed page, cache the ink as a band of the page rather than
  redrawing on every scroll event, and turn one page per swipe. The Android demo is built
  on Material 3.
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

### Changed
- Page data release `data-v0.4.0` adds the surah-name assets under `surah-names/`: the 114
  titles as individual and combined QVP and SVG, and the WOFF2 font with its CSS and metadata
  map. Every page, word sidecar and atlas is byte-identical to `data-v0.3.0`. It is served as
  `cdn.quran.ws/qvp/v0.4.0/`; `scripts/sync-test-data.sh`, the wrapper page sync scripts and
  CI default to it.
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

### Fixed
- iOS `QvpCanvasController` under `hostScrolls` measures the layout in `hostViewportHeight`,
  the box the host looks through, never in its own canvas — which, once the host sizes it to
  the page, is the layout's output: fill-height read it back as the viewport and grew the
  page by the host's padding on every pass. A printed page too tall for that box is no longer
  shrunk to it either, nor centred as if it had been — a printed layout's `fitX` goes with its
  `fitScale`, and applied at scale 1 it pushed the page off to one side by half the width the
  shrink would have freed. That page is what the host's scroll view is for, and `fitScale`
  stays the engine's answer for a host that would rather shrink. Both knobs lay the page out again
  when set, and the band a pinch paints follows the view transform, so a magnify peek on a
  host-scrolled page no longer leaves a blank box under the fingers.
- A native surah frame stored as `PathKind::Ornament` stays with the printed page but no
  longer freezes the title at the frame's width when the page reflows. The reflow omits the
  frame from drawing, sizing and hit testing; the title keeps growing with the reader. On the
  printed page, exact hit testing follows the visible frame ink instead of treating its large
  transparent centre as part of the title.
- `qvp_layout_line_spacing_to_fill` answers the spacing the page is really laid out at. It
  measured the full printed width and the page's own height against the viewport, while
  `qvp_layout` fits the CROPPED width and sits a short page on the grid a full page fills —
  so a host that crops the printed side margins was told a multiplier several percent above
  the one in force, and a quarter above it on a page whose margins are most of its width.
  Both read one answer now, so they cannot drift again. `conformance/scenarios/layout.json`
  carries the corrected value for every cropped case.
- The Flutter `QvpPage.zoomSpec` carries `bannerZoom` through. It rebuilds the spec field by
  field and left that one out, dropping the cap on the only layout it applies to.
- `QvpCanvasController.zoomSteps` (iOS) is empty until the canvas has a size, as
  `QvpPageView.zoomSteps` already was: `zoomToStep` refuses without one, so a host drawing a
  size control off it showed the control live over a page it could not move.
- A page draws what is inside its own box and nothing beyond it. The artwork for page 17
  puts that page's printed page number below the box and its running head above it — it is
  the only page of 604 that draws anything outside — and every host drew that ink wherever
  the layout happened to put it. The paths, their boxes and the path numbering are all
  unchanged, so a crop and an SVG export still carry every stroke the artwork drew.
- The Kotlin library module (`packages/android/qvp`) no longer applies the Maven publishing
  plugin itself; the Android SDK's root build applies and configures it. A root build that
  only needs the module to compile — the React Native example, which includes it as
  `:qvp` — configured with `Plugin [id: 'com.vanniktech.maven.publish'] was not found`
  since the plugin arrived, so the React Native Android module could not be built in this
  repository (#83). `:qvp:publishAndReleaseToMavenCentral` is unchanged.
- The React Native Android module compiles: its view manager was missing the import of
  `QvpDefaults`, and its page view called a `centre()` that does not exist. Both had
  never been through a compiler, which is what #84 is for.
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

[Unreleased]: https://github.com/quran-ws/quran-engine/compare/v0.7.0...HEAD
[0.7.0]: https://github.com/quran-ws/quran-engine/compare/v0.6.1...v0.7.0
[0.6.1]: https://github.com/quran-ws/quran-engine/compare/v0.6.0...v0.6.1
[0.6.0]: https://github.com/quran-ws/quran-engine/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/quran-ws/quran-engine/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/quran-ws/quran-engine/compare/v0.3.1...v0.4.0
[0.3.1]: https://github.com/quran-ws/quran-engine/compare/v0.3.0...v0.3.1
[0.3.0]: https://github.com/quran-ws/quran-engine/compare/v0.2.2...v0.3.0
[0.2.2]: https://github.com/quran-ws/quran-engine/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/quran-ws/quran-engine/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/quran-ws/quran-engine/compare/v0.1.1...v0.2.0
[0.1.1]: https://github.com/quran-ws/quran-engine/compare/v0.1.0...v0.1.1
[0.1.0]: https://github.com/quran-ws/quran-engine/releases/tag/v0.1.0
