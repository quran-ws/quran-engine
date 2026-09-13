# @quran.ws/qvp-react-native

React Native wrapper for the QVP vector mushaf engine. Android-first: a native view
(`<QvpPageView />`) hosting the Kotlin library's `QvpPageView` (`ws.quran.qvp`,
`packages/android/qvp`, JNI over `crates/qvp-ffi/include/qvp.h`) plus a promise-based module
(`QvpModule`) for queries. The wrapper is *thin*: no hit-testing, layout or styling logic in JS or
in the module — it marshals props/targets/selectors and reconciles declarative arrays against engine
handles. Names follow `docs/API.md`. **The package ships no page data.**

## Install

```sh
npm install @quran.ws/qvp-react-native        # or a file:/git dependency inside the monorepo
```

Android needs the Kotlin library. Preferred: include it as a Gradle project in your app's
`android/settings.gradle` (this is what `../example` does):

```groovy
include ':qvp'
project(':qvp').projectDir = file('<repo>/packages/android/qvp')   // e.g. '../../../android/qvp'
```

The module then depends on `project(':qvp')`. If `:qvp` is not included, the module compiles the
library's Kotlin + JNI sources itself from `qvpAndroidDir` (Gradle property; default
`../../../android/qvp`, right for the monorepo). Either way the engine comes from
`packages/android/qvp/prebuilt/<abi>/libqvp_ffi.a` — produce it once with
`scripts/build-engine-android.sh` from the repo root.

Requirements: RN ≥ 0.76 (legacy view manager / native module through the interop layer, verified on
0.87 with the New Architecture), minSdk 24, NDK 27, CMake 3.22.

Page data: `cargo run -p qvp-convert --release -- batch pages dist/pages` → `NNN.qvp`,
`NNN.words.json`, `atlas.qva`. Put them wherever you like (assets, files dir, downloads) and pass
URIs: `asset://pages/042.qvp`, `file:///…`, `/abs/path`, or `base64:…`.

## The view

```tsx
import { QvpPageView, useQvp, Sel, T, LAYER, KIND, DECORATION, rgba } from '@quran.ws/qvp-react-native';

const qvp = useQvp();
<QvpPageView
  ref={qvp.ref}
  style={{ flex: 1 }}
  pageUri="asset://pages/042.qvp"
  wordsUri="asset://pages/042.words.json"                       // optional sidecar (rasm_imlai/qpc/rasm/search forms)
  padTop={12} padBottom={12} padSide={8}                        // dp
  lineSpacing={1} lineGap={0} fillHeight={false}                // engine layout knobs (page units for lineGap)
  // spacing only opens up: lineSpacing < 1 and a negative lineGap are clamped to "as printed"
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
`removeHighlight` (fades out). `styles` works the same over `style` / `styleTarget` / `hide` / `recolorStyle` /
`removeStyle`; `theme` is one handle; `mask` re-applies `maskOptions` + `mask` when the object changes
(`null` → `unmask`); `reveal` calls `revealStart` when its config changes and `revealGoto` when `at`
changes (`null` → `revealStop`). When `pageUri` changes, the old page is freed and every declarative
prop is re-applied to the new page.

Gestures are the Kotlin view's: tap → gap-aware `hitTestView` → `onWordTap` / `onDecorationTap` /
`onEmptyTap`; long-press-drag → engine `select` with a band in `LAYER.SELECTION`; pinch / pan;
double-tap resets. Props `selectionEnabled`, `zoomEnabled`, `hitMaxDistance`, `selectionBand`.

Colours: `'#rgb' | '#rrggbb' | '#rrggbbaa' | 0xRRGGBBAA` (`css()` / `rgba(c, alpha)` helpers).
Targets: `'page' | '2:255' | '2:255:3' | '2:255-257' | 'line:7' | 'surah:2' | wordIdx | [index…] | T.*`.
Selectors: `Sel.page/path/wordPath/wordMark/wordMarkNamed/wordBody/wordMarks/word/ayah/line/mark/category/family/kind/decoration/decorationIndex`.

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
await qvp.surahs(); qvp.divisions(); qvp.ayahMarks(); qvp.rosettes(); qvp.sajdahs(); qvp.ayahKeys();
await qvp.ayahWordCount(2, 255); qvp.reciteMap(2, 255, 4); qvp.wordLabel(i); qvp.ayahLabel(ayahIndex);
await qvp.citation([12, 13, 14]);
await qvp.cropSvg('2:255', { pad: 3, keepMarkers: true, background: '#fffdf7' }); qvp.cropBounds(target);
await qvp.select(anchor, focus); qvp.clearSelection(); qvp.selection(); qvp.selectionText('rasm_uthmani', true);
await qvp.unmaskNext(1); qvp.maskBack(1); qvp.unmaskWord(i); qvp.maskWord(i); qvp.unmaskAll(); qvp.maskAll();
await qvp.maskHidden(); qvp.maskWords(); qvp.revealStepCount(); qvp.revealPosition(); qvp.revealStepOf(i);
await qvp.hitTestView(x, y, { maxDistance: 6 }); qvp.hitTestExactView(x, y); qvp.hitTest(px, py); qvp.hitTestExact(px, py);   // view dp or page units
await qvp.wordBoundsView(i); qvp.currentLayout(); qvp.layoutGapToFill(); qvp.relayout(); qvp.resetView(); qvp.stats();

// engine-wide (Qvp.*)
await Qvp.strip(s); Qvp.fold(s); Qvp.normalize(s); Qvp.looseKey(s);       // = Qvp.arabic(kind, s)
await Qvp.gapToFill(pageW, pageH, lines, viewW, viewH); Qvp.wastedFraction(...)
Qvp.markName(7); Qvp.kindName(1); Qvp.categoryName(1); Qvp.familyName(3); Qvp.nameId('marks', 'fathah'); Qvp.version; await Qvp.markCategory(7); Qvp.engineName()

// atlas (cross-page)
const atlas = await QvpAtlas.load('asset://pages/atlas.qva');
await atlas.pageOf(2, 255); atlas.pageRange(42); atlas.surah(36); atlas.surahs(); atlas.pageOfSurah(36);
await atlas.juz(30); atlas.hizb(3); atlas.rubuAlHizb(7); atlas.juzOf(2, 255); atlas.divisionOf('hizb', 2, 255);
await atlas.pagesOfJuz(30); atlas.searchSurahs('cow' | 'البقرة' | '2'); atlas.free();
// tag-level equivalents: Qvp.atlasPageOf(id, s, a), Qvp.atlasSearchSurahs(id, text), Qvp.atlasPagesOfJuz(id, n), Qvp.atlasJuzOf(id, s, a), …
```

All page calls run on the UI thread (where the view draws), so the engine is never touched from two
threads. Only `maskHidden`, `unmask` (via the `mask` prop), `unmaskNext` etc. mutate state; queries are
microseconds.

## Layout of this package

```
android/build.gradle                       library module; depends on project(':qvp') or compiles its sources
android/src/main/java/ws/quran/qvp/rn/
  QvpRnPageView.kt      QvpPageView subclass: page loading, prop → handle reconciliation, events
  QvpPageViewManager.kt SimpleViewManager<QvpRnPageView> (props, events, commit after each prop batch)
  QvpModule.kt          ReactContextBaseJavaModule: page queries by react tag, engine helpers, atlas
  Marshal.kt            JS shapes ↔ Kotlin types (targets, selectors, styles, records)
  QvpPackage.kt
src/index.tsx                              typed API: <QvpPageView/>, useQvp(), Qvp, QvpAtlas, Sel, T, constants
```

## iOS — TODO

Not implemented (no macOS in the build environment). The iOS side mirrors the Android module over the
same `qvp.h`: a Swift `QvpPage`/`QvpAtlas` over `libqvp_ffi.a` (build with
`cargo build -p qvp-ffi --release --target aarch64-apple-ios[-sim]`), a `UIView` renderer with the
same draw order (highlight bands → cached base ink → styled ink → mask boxes, `tick(now)` per
`CADisplayLink` frame), an `RCTViewManager` exposing the props above with the same reconciliation, and
an `RCTBridgeModule` with the same method names. `src/index.tsx` needs no changes.
