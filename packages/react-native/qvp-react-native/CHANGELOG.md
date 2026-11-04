# Changelog

This package shares the engine version. Entries are copied from the root `CHANGELOG.md`.

## [Unreleased]

### Added
- `QVP_DEFAULT_REFLOW_RELAX`, `QVP_DEFAULT_REFLOW_MAX_STRETCH` and `QVP_DEFAULT_SELECTION_PAD_X`
  in `qvp.h`, mirrored in every wrapper's defaults table (`QvpDefaults` in Kotlin, Swift and
  Dart, `DEFAULTS` in JavaScript and React Native). The page views pass the selection pad
  from the table and the reflow specs take their defaults from it, so no wrapper copies the
  numbers by hand. `scripts/check-parity.py` compares 29 defaults where it compared 26.
- React Native: `DEFAULTS` holds every engine default under its header name, read from the
  native module, which sends the whole `QvpDefaults` table. A key the native module lacks is
  an error that names the key and says the native module is older than the package.

### Fixed
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
