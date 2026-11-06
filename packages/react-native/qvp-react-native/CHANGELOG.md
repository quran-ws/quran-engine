# Changelog

This package shares the engine version. Entries are copied from the root `CHANGELOG.md`.

## [Unreleased]

### Added
- `ayahMarksView(tag)` and the `ayahMarksView()` page method: the ayah marks in viewport px
  through the current layout, as `surahHeadersView` gives the surah headings. It runs on the
  UI thread, because it reads the layout (#82).
- `qvp_layout_ink_band` picks the band of a laid-out page that a host keeps drawn: the whole
  page when it fits, otherwise two screens around the reader. The band's top moves in steps,
  so a pan inside one step redraws nothing. The band is in layout px, ready for
  `qvp_layout_draw_list`. Its two limits are in viewport px: `max_height` (0 takes the new
  `QVP_DEFAULT_MAX_INK_HEIGHT`, 4096), which no band passes, and `max_pixels`, a pixel budget
  (0 is no limit), which shrinks the band from two screens to no less than one. With no
  screen (no scale, no viewport height, or a non-finite scale, height or offset) the band is
  the whole page, however tall. Bound as `layoutInkBand` in JavaScript, Kotlin, Dart and
  Swift. React Native calls it through the Kotlin view. `conformance/scenarios/ink-band.json` holds the engine's answers, and the web,
  Kotlin, Dart and Swift tests replay them (#134).

### Changed
- The Android `QvpPageView`, the Flutter `QvpPageView` and the web `CanvasRenderer` ask the
  engine for their band of ink. Each had its own copy of the arithmetic, and the copies did
  not agree (#134):
  - Android drew a printed page as a band. It now draws any page whole when the page fits the
    ceiling and the ink budget, as it already did for a reflowed page. Its band no longer
    passes the ceiling on a screen taller than 4096 px. The ink budget now limits the band as
    well as the whole page, down to one screen and never below.
  - Flutter measured the 4096 ceiling in layout px. It now measures it in viewport px, so a
    magnified page bands once it is taller than 4096 px on the screen. At a view scale of 1
    nothing changes.
  - The web band now stays under the tallest canvas a browser makes, and under the ink budget
    down to one screen.

### Removed

### Fixed
- A band of ink with less than 2 px of slack, from an ink budget of about one screen or a
  screen within 2 px of `max_height`, now follows the reader. Before, its top moved in steps
  of 1 px, more than half its slack, and the band could end up to 2 px short of the screen's
  edge (#134).
- React Native: the npm package builds in an app outside this repository. When the app's
  build has no `:qvp` project and no `qvpAndroidDir` property, the Android module depends
  on `ws.quran:qvp-android` from Maven Central, at the version in the package's
  `package.json`. The Gradle property `qvpVersion` sets a different version. Before, the
  module compiled the Kotlin library from `../../../android/qvp`, a path that exists only
  in this repository (#42). CI resolves the library from Maven Central through the
  example's build with `-PqvpFromMaven`.
- `KIND` and `DECORATION` name every code in `qvp.h`: `KIND.ORNAMENT`, `PAGE_NUMBER` and
  `RUNNING_HEAD` (5 to 7), and `DECORATION.PAGE_NUMBER`, `RUNNING_HEAD` and `OTHER` (5, 6
  and 255). Before, a path or decoration kind could be a value that no constant named.
- The docs of the layout's fit (`fit_scale`, `fit_x`, `fit_y`) say what a reflowed page
  does. On a reflowed page `fit_scale` is 1 and the host scrolls the content. `fit_x`
  centres it across the viewport, and `fit_y` puts its top at the top of the viewport.
  Before, `qvp.h`, the Rust docs, `docs/API.md` and the Kotlin, Swift, Dart and React Native docs
  said that the fit shrinks every page into the viewport and centres it. `qvp.h`, the Rust docs,
  `docs/API.md`, Kotlin and Swift also said that a printed page shrinks to the viewport
  height. It shrinks to fit both the width and the height. The wrapper docs now also say
  that on a dressed page the fit shows the content plus the dress overflow, and that
  `fit_x`, `fit_y` place the content's own origin. The docs of the dress overflow in
  Kotlin, Swift and Dart told a host to fit the overflow itself. The engine's fit holds it
  already. The iOS `resetView()` docs said that the view fits the content height.

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
- Reflow can keep the print's line breaks: the `reflowBreaks` prop takes `'printed'`, and the
  `Breaks` type names the four modes. Each line stays whole while its ink grows to fill the
  page width. `reflowMaxZoom` measures the widest row in this mode, and larger zooms clamp to
  it.

### Changed
- React Native: the Android module runs the page queries that read only the loaded page
  (`words`, `text`, `search`, the metadata calls and others), the engine-wide helpers and the
  atlas on a thread of its own, so a long query does not hold up a frame. `words` builds each
  word's record once per page and reads path names from tables it fills once. `T`, `Sel` and
  the module take the target and selector codes from one table that `scripts/gen-rn-enums.py`
  writes from `qvp.h`, and `scripts/check-parity.py` fails when the table is out of date. The
  example's lock file records the package version, and `scripts/set-version.sh` sets it.

### Fixed
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
- `hitTestView` and `hitTestExactView` invert the Android view's pan and zoom with the
  engine's `viewToLayout` rather than their own arithmetic.
- The example builds from a fresh clone. The root `.gitignore` ignored every PNG, so git
  never tracked the launcher icons that the Android manifest names. The rule now covers
  the repository root only, and the React Native template's icons are tracked. The example
  also gains the template's `debug.keystore`, which its Gradle signing config names and its
  `.gitignore` already allows.
- React Native: the package checks that the native module is linked before it reads the
  module's constants. Without the module, the error now says `QvpModule not linked` instead
  of a `TypeError`.
- React Native: `rgba` clamps a NaN alpha to 0, where it returned a malformed colour.

## [0.7.0] - 2026-10-14

### Added
- `<QvpPassageView />`: a range of complete ayahs, or a surah's basmalah, laid out by the engine on
  rows of the view's width and drawn natively. The layout comes back through `onPassageLayout`,
  and the view takes its height from it. Closes the React Native rows of issue #102.
- Dress: `QvpOrnaments.load(uri)` reads an ornament set (`ornaments.qvo`), and the `dress` prop
  puts its medallions, bands and borders on the page. The hosted `QvpPageView` paints them behind
  the ink and fits the border in. `Qvp.dress`, `undress`, `dressInfo` and `dressRevision` do the
  same by react tag, and `onDressChanged` reports the engine's readout. Closes issue #116.
- `Qvp.dressClamp(tag, view, viewportW, viewportH)` clamps a view over the content plus the dress
  overflow, as the engine does (`qvp_dress_clamp`).
- `Qvp.dressView(tag, scale, viewportW)` returns `{view, width, height}` for a host that scrolls
  the page: the dressed box's top at 0, centred, and its size (`qvp_dress_view`).
- `QvpRnPageView.kt` imports `android.util.Base64`, so the module compiles again.
- `PassageWord.wordX0` to `wordY1` in `onPassageLayout`: the box of the word alone, without its
  signs. The word that closes an ayah leaves its medallion out.

## [0.5.0] - 2026-10-08

- Under `justified`, a row that cannot reach both margins is centred instead of starting at
  the right margin: the last row of a block, a single word, or a row whose gaps would have to
  stretch past `max_stretch`. Hung off the right margin, it read as a ragged column.
- The reflow knobs of `QvpLayoutSpec` are read on a printed page too (`reflow_zoom` 0): they
  are what the zoom control reflows the page with. A C host that zeroes the spec and pinches
  now reflows ragged (fill 0), greedy (breaks 0) and with printed gaps (gaps 0); 255, 255 and 1
  ask for the engine's defaults, which every wrapper already writes.
- On a reflowed row, a medallion is centred between the word it closes and the next one with
  the signs each carries, such as the quarter star (۞) that opens the next ayah. Measured on the
  letters alone, it sat against the star with the space on its other side.
- A pinch keeps the host's fill, breaks and gaps from the first step. The C layer dropped the
  reflow knobs while `reflow_zoom` was 0, so the layout the zoom control made inside a pinch
  always took the engine's defaults.

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
