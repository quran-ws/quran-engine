# @quran.ws/qvp-react-native

React Native wrapper for the QVP vector mushaf engine, on Android and iOS. Each platform has
a native view (`<QvpPageView />`) and a promise-based module (`QvpModule`) for queries. On
Android the view hosts the Kotlin library's `QvpPageView` (`ws.quran.qvp`,
`packages/android/qvp`, JNI over `crates/qvp-ffi/include/qvp.h`). On iOS it hosts QvpKit's
`QvpPageView` (`packages/ios/QvpKit`, Swift over the same header). The two modules have the
same method names and give the same result shapes, so the JavaScript side has no platform
branch. The wrapper is *thin*: no hit-testing, layout or styling logic in JS or in the
module — it marshals props/targets/selectors and reconciles declarative arrays against engine
handles. Names follow `docs/API.md`. **The package ships no page data.**

## Install

```sh
npm install @quran.ws/qvp-react-native        # or a file:/git dependency inside the monorepo
```

Android needs the Kotlin library `ws.quran.qvp`. The module finds it in one of three ways,
and the first that applies wins:

1. Your app's `android/settings.gradle` includes the library as the Gradle project `:qvp`.
   The module then depends on `project(':qvp')`. `../example` does this:

   ```groovy
   include ':qvp'
   project(':qvp').projectDir = file('<repo>/packages/android/qvp')   // e.g. '../../../android/qvp'
   ```

2. The Gradle property `qvpAndroidDir` names a checkout of `packages/android/qvp`. The module
   compiles the library's Kotlin and JNI sources from that folder.
3. Otherwise the module depends on `ws.quran:qvp-android` from Maven Central, at the version
   of this package. An app that installs the package from npm needs nothing more. To use a
   different library version, set the Gradle property `qvpVersion`, for example
   `qvpVersion=0.11.0` in `android/gradle.properties`.

The Maven Central library holds the engine. In ways 1 and 2 the engine comes from
`packages/android/qvp/prebuilt/<abi>/libqvp_ffi.a`. Make it once with
`scripts/build-engine-android.sh` from the repository root.

iOS needs CocoaPods. React Native's autolinking finds `qvp-react-native.podspec`, so run
`pod install` in your app's `ios` folder after `npm install`. The pod compiles QvpKit's Swift
sources with the module and links the engine, `QvpEngine.xcframework`. It takes the engine in
one of three ways, and the first that applies wins:

1. Inside this repository, as `../example` does, the pod takes the XCFramework that
   `scripts/build-engine-ios.sh` writes at `packages/ios/QvpKit/QvpEngine.xcframework`. It
   copies QvpKit's sources from `packages/ios/QvpKit` at each `pod install`, so run
   `pod install` again after you change them.
2. The environment variable `QVP_ENGINE_XCFRAMEWORK` names an XCFramework to link, for
   example an engine that you built yourself:

   ```sh
   QVP_ENGINE_XCFRAMEWORK=/path/to/QvpEngine.xcframework pod install
   ```

3. Otherwise `pod install` downloads the XCFramework of this package's release from GitHub,
   the one that SwiftPM takes for QvpKit, and checks its SHA-256 checksum. An app that
   installs the package from npm needs nothing more. The download needs `curl` and `unzip`.

The pod and QvpKit through SwiftPM each link the engine. Do not use both in one app: the
engine's C symbols would be defined twice.

Requirements: RN ≥ 0.76 (legacy view manager / native module through the interop layer, verified on
0.87 with the New Architecture). Android: minSdk 24, and ways 1 and 2 also need NDK 27 and
CMake 3.22. iOS: iOS 15.1, CocoaPods and Xcode 16.

Page data: `cargo run -p qvp-convert --release -- batch pages dist/pages` → `NNN.qvp`,
`NNN.words.json`, `atlas.qva`. The optional letter layers, `NNN.qvl`, come from
`qvp-convert letters` (`docs/DATA.md`). Put them wherever you like (assets, files dir, downloads) and pass
URIs: `asset://pages/042.qvp`, `file:///…`, `/abs/path`, or `base64:…`. On iOS an `asset://`
URI names a file in the app bundle: `asset://pages/042.qvp` is `pages/042.qvp` under the
bundle's resources, for example a folder reference named `pages`.

## The view

```tsx
import { QvpPageView, useQvp, Sel, T, LAYER, KIND, DECORATION, rgba } from '@quran.ws/qvp-react-native';

const qvp = useQvp();
<QvpPageView
  ref={qvp.ref}
  style={{ flex: 1 }}
  pageUri="asset://pages/042.qvp"
  wordsUri="asset://pages/042.words.json"                       // optional sidecar (rasm_imlai/qpc/rasm/search forms)
  lettersUri="asset://pages/042.qvl"                             // optional letter layer (see Letters)
  padTop={12} padBottom={12} padSide={8}                        // dp
  lineSpacing={1} fillHeight={false}                            // engine layout knobs
  // spacing only opens up: lineSpacing < 1 is clamped to "as printed"
  paperColor="#fffdf7" defaultInk="#231f20"
  theme={{ diacritics: '#1a73e8', dots: '#c62828', waqf: '#0a7d32', ms: 200 }}     // page.theme(...) — one handle
  styles={[
    { id: 'hide-marks', selector: Sel.kind(KIND.MARK), hide: true },                  // page.hide(sel)
    { id: 'gold', selector: Sel.decoration(DECORATION.AYAH_MARK), color: '#b8860b', ms: 300, layer: LAYER.THEME + 1 },
    { id: 'mark', selector: Sel.wordMark(12, 1), color: '#ef6c00', ms: 200, layer: LAYER.TOP },  // 2nd diacritic of word 12
    { id: 'ayah', target: '2:255', color: '#0a7d32' },                                 // page.styleTarget(...)
  ]}
  highlights={[
    { id: 'sel', target: T.word(12), style: { mode: 'both', ink: '#1a73e8', band: rgba('#1a73e8', 0.18), radius: 1.5, ms: 250, layer: LAYER.SELECTION } },
    { id: 'search', target: T.words([3, 9, 41]), style: { mode: 'both', ink: '#c62828', band: rgba('#c62828', 0.12), height: 'ink' } },
  ]}
  mask={{ target: '2:255', mode: 'hide' }}                       // or 'block' | 'blur' (+ blockColor, padX, padY, radius)
  reveal={{ lit: 2, grey: '#c9c4b8', ink: '#231f20', ms: 150, at: 3 }}   // greyed page; `at` drives revealGoto
  onWordTap={({ word, hit }) => …}      // word: {index, surah, ayah, word, line, …, text, wordKey, aid, forms, label, paths[]}
  onDecorationTap={({ decoration, hit }) => …}      // ayah markers, banners, basmalah, rosettes, sajdah signs
  onEmptyTap={() => …}
  onSelectionChanged={({ words, text, citation, textWithCitation }) => …}   // long-press-drag selection
  onPageLoad={info => …}                // {page, width, height, nLines, nAyahs, nWords, nPaths, nDecorations, forms, loadMs, bytes}
  onRevealChanged={({ steps, at }) => …}
  onError={({ message }) => …}
/>
```

Reconciliation: each `highlights` entry maps to one engine handle. A new id → `highlight`; a changed
`target` → `moveHighlight` (the band slides); a changed `style` → `restyleHighlight`; a removed id →
`removeHighlight` (fades out). `styles` works the same over `style` / `styleLetters` / `styleTarget` / `hide` / `recolorStyle` /
`removeStyle`; `theme` is one handle; `mask` re-applies `maskOptions` + `mask` when the object changes
(`null` → `unmask`); `reveal` calls `revealStart` when its config changes and `revealGoto` when `at`
changes (`null` → `revealStop`). When `pageUri` changes, the old page is freed and every declarative
prop is re-applied to the new page.

Gestures are the native view's (Kotlin or QvpKit): tap → gap-aware `hitTestView` → `onWordTap` / `onDecorationTap` /
`onEmptyTap`; long-press-drag → engine `select` with a band in `LAYER.SELECTION`; pinch / pan;
double-tap resets. Props `selectionEnabled`, `zoomEnabled`, `hitMaxDistance`, `selectionBand`.

Colours: `'#rgb' | '#rrggbb' | '#rrggbbaa' | 0xRRGGBBAA` (`css()` / `rgba(c, alpha)` helpers).
Targets: `'page' | '2:255' | '2:255:3' | '2:255-257' | 'line:7' | 'surah:2' | wordIdx | [index…] | T.*`.
Selectors: `Sel.page/path/wordPath/wordMark/wordMarkNamed/wordBody/wordMarks/word/ayah/line/mark/category/family/kind/decoration/decorationIndex`,
and with a letter layer `Sel.letter/letterMark/letterMarkNamed/wordChars` (see Letters).

## The passage view

A passage is a range of complete ayahs shown away from its page, or a surah's basmalah. The
engine lays it out on rows of the view's width and the view draws it; the layout comes back
through `onPassageLayout`, and the view takes its height from it.

```tsx
<QvpPassageView
  style={{ width: '100%' }}
  pageUris={['asset://pages/042.qvp']}                 // the pages that print the range, in any order
  surah={2} from={255}                                 // to={…} for a range; basmalah for the surah's basmalah
  lineHeight={22}                                      // dp per printed line (or scale: dp per page unit)
  align="justified" maxStretch={2}                     // right (the default), center or justified
  maxRows={2} ellipsisWidth={12} keepAyahMark          // cut after two rows, ellipsis, then the medallion
                                                       // omitAyahMarks: no medallions; ayahMark: ayah `from`'s medallion alone
  dress={{ ornaments: set, ayahMark: medallion }}      // the page view's dress: its medallion on every ring
  ink="#231f20" paperColor="#fffdf7"
  onPassageLayout={l => console.log(l.rows.length, l.words.length, l.isTruncated)}
/>
```

## Dress

A page can wear another mushaf's ornaments: an ayah medallion, a surah band and a page border
from an ornament set (`ornaments.qvo`, in the data release). The engine places each ornament.
The native view paints the layer behind the ink and fits the border into the view.

```tsx
const set = await QvpOrnaments.load('asset://pages/ornaments.qvo');
const medallion = await set.find('ayah_mark', 'font-003-regular');
<QvpPageView
  pageUri="asset://pages/604.qvp"
  dress={{ ornaments: set, style: 'mushaf-qalon', ayahMark: medallion, gap: 5, lineArt: ['surah_header'] }}
  onDressChanged={({ info }) => console.log(info?.ayahMarks, info?.frameRepeats)}   // null: refused
/>
```

- `style` takes a mushaf's medallion, band and border together. A kind key (`ayahMark`,
  `surahHeader`, `pageFrame`) overrides the style for that kind. It takes an ornament, an
  index or a style name. `null` is none.
- The knobs are `gap`, `bandGap`, `ayahMarkScale` (0 takes the design's own size),
  `pageFrameScale`, `lineArt` (`true` or a list of kinds) and `colors` (`[ornament, part, colour]`).
- `set.list` gives each ornament with its `licence` and `parts`. `set.styles()` groups the
  ornaments by mushaf, and `set.ofKind(kind)` filters them. Read the licence before you
  publish a dressed page.
- The prop dresses every page the view loads. `qvp.dress(set, spec)`, `qvp.undress()`,
  `qvp.dressInfo()` and `qvp.dressRevision()` act on the page in the view now. Do not mix them with the `dress` prop on one view.

## Letters

A page can take a letter layer (`NNN.qvl`, `docs/DATA.md`). The layer names the letters
of each word, the paths that draw each letter, and the pieces cut from an outline that joins
letters. With a layer, you can read, colour and hit-test one letter or one mark.

```tsx
const qvp = useQvp();
<QvpPageView
  ref={qvp.ref}
  pageUri="asset://pages/042.qvp"
  wordsUri="asset://pages/042.words.json"
  lettersUri="asset://pages/042.qvl"                                   // optional letter layer
  styles={[
    { id: 'lam', letters: { scope: '2:255', text: 'ل' }, color: '#c62828' },                // every lam of the ayah
    { id: 'marks', letters: { text: 'ب', part: LETTER_PART.MARKS }, color: '#1a73e8' },     // the marks of every ba
    { id: 'one', selector: Sel.letter(12, 0, LETTER_PART.BODY_AND_DOTS), color: '#0a7d32' },
    { id: 'fathah', selector: Sel.letterMarkNamed(12, 1, 'fathah'), color: '#ef6c00' },
    { id: 'chars', selector: Sel.wordChars(12, 0, 2), color: '#6a1b9a' },                  // qpc characters 0 to 2
  ]}
/>

await qvp.hasLetters();                              // true once a layer is attached
const letters = await qvp.letters(12);               // [{index, text, codePoint, at, moreAt, x0…y1, hasInk, members, marks}]
await qvp.letterAtOffset(12, 3);                     // the letter that draws qpc offset 3, or null
await qvp.pathLetter(path);                          // {letter, at}, or null
await qvp.findLetters({ scope: 'page', text: 'ل', mark: 'shaddah' });     // [{word, letter}]
const hit = await qvp.hitTestLetterView(x, y);       // view dp: {word, letter, path, at, mark, markName, isExact, isBody, …}
await qvp.attachLetters('asset://pages/042.qvl');    // when you do not use the lettersUri prop
```

- The view attaches `lettersUri` after the page loads and after `wordsUri`. A new value
  attaches that layer in place of the one before. The engine has no detach: if you remove the
  prop, the layer stays until the next page loads. The view logs a layer that does not load,
  or that the engine refuses (the layer of another page). `hasLetters()` tells you.
- `attachLetters` resolves `false` when the engine refuses the layer. Then nothing changes.
  It rejects when the URI does not load.
- An attach adds the pieces as paths after the page's own paths, so `info().nPaths` grows.
  No other path index changes. After a successful attach, a value that you got from the page
  before it and that counts or indexes paths, such as `info().nPaths`, is stale. Read it again.
- A letter's `text` is its character and `codePoint` the same as a number. A member and a hit
  give the mark as its id (`mark`) and its name (`markName`).
- `members` holds the bodies first, then the marks in text order. `marks[n]` is the path that
  `Sel.letterMark(word, index, n)` takes. An absent offset or letter is -1.
- A letter query takes `scope` (a target, the page when left out), `text` (one character, or
  a code point as a number), `mark` (a name or an id) and `part` (`LETTER_PART`). A field left
  out, `''` or 0 matches any letter. `part` does not filter `findLetters`. A `text` of more
  than one character rejects the call. In a style rule, it adds no rule, as a bad selector
  does.
- `hitTestLetter` and `hitTestLetterView` take the options of `hitTest`. As in `hitTest`, a
  `maxDistance` that you leave out is `DEFAULTS.TAP_DISTANCE`, so a tap far from every word
  finds nothing. The other wrappers have no limit by default. Pass 0 for no limit.
- A rule with `letters` takes the layer that is attached when the rules run. A rule added
  before the layer selects nothing until the layer comes.
- The native view keys its draw list on the engine's draw revision. A page with no layer
  draws as before.

## The module

Everything takes the view's react tag (`findNodeHandle`) — or use the bound forms:
`useQvp()` returns `{ ref, tag, ...api }`, and the component ref exposes the same `api`.

```ts
const qvp = useQvp();
await qvp.info(); qvp.words(); qvp.word(i); qvp.ayahs(); qvp.lines(); qvp.decorations();
await qvp.search('الرحمان', { mode: 'includes' });     // [{word, wordKey, text, index, loose}]
await qvp.text('2:255', { form: 'search', wordSep: ' ' });
await qvp.targetWords('line:7'); qvp.findWord(2, 255, 3); qvp.wordForm(i, 'rasm_imlai'); qvp.hasForm('qpc');
await qvp.attachWords(jsonString);                      // when you do not use the wordsUri prop
await qvp.surahs(); qvp.divisions(); qvp.ayahMarks(); qvp.ayahMarksView(); qvp.rosettes(); qvp.sajdahs(); qvp.ayahKeys();
await qvp.ayahWordCount(2, 255); qvp.reciteMap(2, 255, 4); qvp.wordLabel(i); qvp.ayahLabel(ayahIndex);
await qvp.citation([12, 13, 14]);
await qvp.cropSvg('2:255', { pad: 3, keepMarkers: true, background: '#fffdf7' }); qvp.cropBounds(target);
await qvp.select(anchor, focus); qvp.clearSelection(); qvp.selection(); qvp.selectionText('rasm_uthmani', true);
await qvp.unmaskNext(1); qvp.maskBack(1); qvp.unmaskWord(i); qvp.maskWord(i); qvp.unmaskAll(); qvp.maskAll();
await qvp.maskHidden(); qvp.maskWords(); qvp.revealStepCount(); qvp.revealPosition(); qvp.revealStepOf(i);
await qvp.hitTestView(x, y, { maxDistance: 6 }); qvp.hitTestExactView(x, y); qvp.hitTest(px, py); qvp.hitTestExact(px, py);   // view dp or page units
await qvp.wordBoundsView(i); qvp.currentLayout(); qvp.layoutLineSpacingToFill(); qvp.relayout(); qvp.resetView(); qvp.stats();

// engine-wide (Qvp.*)
await Qvp.strip(s); Qvp.fold(s); Qvp.normalize(s); Qvp.looseKey(s);       // = Qvp.arabic(kind, s)
await qvp.layoutLineSpacingToFill(); qvp.layoutWastedFraction(); qvp.grid()
Qvp.markName(7); Qvp.kindName(1); Qvp.categoryName(1); Qvp.familyName(3); Qvp.nameId('marks', 'fathah'); Qvp.version; await Qvp.markCategory(7); Qvp.engineName()

// atlas (cross-page)
const atlas = await QvpAtlas.load('asset://pages/atlas.qva');
await atlas.pageOf(2, 255); atlas.pageRange(42); atlas.surah(36); atlas.surahs(); atlas.pageOfSurah(36);
await atlas.juz(30); atlas.hizb(3); atlas.rubuAlHizb(7); atlas.juzOf(2, 255); atlas.divisionOf('hizb', 2, 255);
await atlas.pagesOfJuz(30); atlas.searchSurahs('cow' | 'البقرة' | '2'); atlas.free();
// tag-level equivalents: Qvp.atlasPageOf(id, s, a), Qvp.atlasSearchSurahs(id, text), Qvp.atlasPagesOfJuz(id, n), Qvp.atlasJuzOf(id, s, a), …
```

On iOS, a view method takes and returns points, where Android takes dp.

`layoutLineSpacingToFill`, `layoutWastedFraction` and `layoutPrintedHeight` measure the
printed page. Both platforms pass the engine the view's spec before the reader's zoom.

The module runs a call on one of two threads (on iOS, two queues). Its own query thread runs each call that reads only
what a page fixed when it loaded: `info`, `words`, `word`, `ayahs`, `lines`, `decorations`, `findWord`,
`targetWords`, `wordForm`, `hasForm`, the metadata calls except `surahHeaders`, `surahHeadersView`
and `ayahMarksView`, `text`, `search`, `citation`, `cropBounds`, `grid`, the engine-wide helpers and
the atlas. A long call there does not hold up a frame. The view keeps its page open until such a call returns. Every other
call reads or changes the layout, the view, the styles or the page state, and runs on the UI thread
(the main thread on iOS), where the view draws. `attachWords` runs on the UI thread after the queries queued before it, and the
queries queued after it wait for it. The letter calls run on the UI thread in the same order.

A page with a letter layer changes as the view draws it: the engine brings its style state up
to date for the draw list. The engine allows one thread at a time per page, so on such a page
the query thread hands each query to the UI thread, in order. A page with no layer keeps the
query thread.

`words` builds each word's record once per page: its forms, its label and the names of its paths.
A later call sends the same records again, and `attachWords` drops them.

## Layout of this package

```
android/build.gradle                       library module; depends on project(':qvp') or compiles its sources
android/src/main/java/ws/quran/qvp/rn/
  QvpRnPageView.kt      QvpPageView subclass: page loading, prop → handle reconciliation, events
  QvpPageViewManager.kt SimpleViewManager<QvpRnPageView> (props, events, commit after each prop batch)
  QvpRnPassageView.kt   View: loads the pages, lays the passage out at the view's width, draws it, reports the layout
  QvpPassageViewManager.kt SimpleViewManager<QvpRnPassageView>
  QvpRnBytes.kt         the bytes a URI names (asset, file, path, base64)
  QvpModule.kt          ReactContextBaseJavaModule: page queries by react tag, engine helpers, atlas, ornament sets
  Marshal.kt            JS shapes ↔ Kotlin types (targets, selectors, styles, records)
  QvpRnWords.kt         each word's record, built once per page
  Codes.kt              QVP_TARGET_* and QVP_SELECTOR_* by name (scripts/gen-rn-enums.py writes it)
  QvpPackage.kt
qvp-react-native.podspec                   the iOS pod: the module, QvpKit's sources and QvpEngine.xcframework
ios/
  QvpModule.m           what React Native sees: the module's methods, the two view managers' props and events
  QvpModule.swift       the module over QvpKit, with the method names of QvpModule.kt
  QvpRnPageView.swift   hosts QvpKit's QvpPageView: page loading, prop → handle reconciliation, events
  QvpRnPassageView.swift loads the pages, lays the passage out at the view's width, draws it, reports the layout
  QvpRnMarshal.swift    JS shapes ↔ QvpKit types, key for key as Marshal.kt
  QvpRnBytes.swift      the bytes a URI names, and each word's record, built once per page
  QvpRnCodes.swift      QVP_TARGET_* and QVP_SELECTOR_* by name (scripts/gen-rn-enums.py writes it)
  vendor-qvpkit.sh      copies QvpKit's sources and the XCFramework's URL and checksum into ios/vendor (prepack)
src/index.tsx                              typed API: <QvpPageView/>, <QvpPassageView/>, useQvp(), Qvp, QvpAtlas, QvpOrnaments, Sel, T, constants
src/codes.ts                               the same codes for T and Sel (scripts/gen-rn-enums.py writes it)
```
