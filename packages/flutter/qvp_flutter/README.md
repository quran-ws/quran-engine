# qvp_flutter — Flutter wrapper for the QVP vector mushaf engine

A dart:ffi plugin (no method channels, no Kotlin) over `libqvp_ffi` — the one
Rust core in `crates/qvp-core`. The engine computes everything (layout,
hit-testing, style resolution, highlights, masks, search, text); the plugin
only marshals the C ABI in `crates/qvp-ffi/include/qvp.h` and draws with a
`CustomPainter`. Method names are identical to `web/qvp.js`, so the web docs
apply one to one. The package ships **no page data**.

## Depend on it

```yaml
dependencies:
  qvp_flutter: ^0.2.2
```

The pub.dev package supports Android, iOS, macOS, Windows, Linux, and Web.
When developing this repository, produce the native engines with:

```sh
# Android: arm64-v8a, armeabi-v7a, x86_64
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/27.2.12479018
bash scripts/build-engine-android.sh

# iOS & macOS: produces QvpEngine.xcframework
bash scripts/build-engine-ios.sh

# Windows: target/release/qvp_ffi.dll
powershell scripts/build-engine-windows.ps1    # or: cargo build -p qvp-ffi --release

# Host lib for `flutter test` on Linux: target/release/libqvp_ffi.so
cargo build -p qvp-ffi --release
```

Page data comes from the converter: `cargo run -p qvp-convert --release -- batch pages dist/pages`
→ `dist/pages/NNN.qvp`, `NNN.words.json` (text forms sidecar) and `atlas.qva`.

## API (mirrors web/qvp.js)

```dart
import 'package:qvp_flutter/qvp_flutter.dart';

final engine = QvpEngine.open();                        // Android: libqvp_ffi.so from the plugin; host: $QVP_LIB or path:
engine.markName(7); engine.kindName(QvpKind.mark); engine.categoryName(1); engine.familyName(1);
engine.strip(s); engine.fold(s); engine.normalize(s); engine.looseKey(s);          // Arabic text tools
page.layoutLineSpacingToFill(spec); page.layoutWastedFraction(spec); page.grid; page.lineSpacing;

final page = engine.loadPage(bytes);                    // geometry copied once: page.ops / page.pts / page.table (stride 8)
page.words / ayahs / lines / decorations;  page.wordForm(i, 'rasm_imlai');  page.findWord(2, 255, 3);
page.targetWords('2:255');                                  // targets: 'page' | '2:255' | '2:255:3' | '2:255-257' | 'line:7' | 'surah:2' | T.word(i) | [w0, w1]
page.surahs(); page.divisions(); page.ayahMarks(); page.rosettes(); page.sajdahs(); page.ayahKeys();
page.ayahWordCount(2, 255); page.reciteMap(2, 255, 4); page.wordLabel(i); page.ayahLabel(ayahIndex);
page.text('2:255'); page.search('الله', mode: 'includes'); page.citation(words); page.attachWords(json); page.hasForm('qpc');
page.hitTestExact(x, y); page.hitTest(x, y, QvpHitOptions(maxDistance: 6));         // page units, exact / gap-aware
page.hitTestExactView(viewX, viewY); page.hitTestView(viewX, viewY);                             // viewport px through the layout
page.lineBands(); page.hitAreas();
final l = page.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100, padTop: 50, padBottom: 50, fillHeight: true)); // l.scale, l.lineDy[line]
page.wordBoundsView(i);
final h = page.style(Sel.wordMark(w, 1), '#ef6c00', ms: 200, layer: QvpLayer.top);  // Sel.path/word/ayah/line/mark/category/family/kind/decoration…
page.styleTarget('2:255', color); page.recolorStyle(h, color); page.removeStyle(h); page.hide(Sel.kind(QvpKind.mark));
page.theme(QvpTheme(diacritics: '#1a73e8', marks: {'shaddah': '#0a7d32'})); page.setDefaultColor('#231f20'); page.clearStyles(); page.clearLayer(QvpLayer.theme);
page.tick(nowMs);                                       // true while animating — keep drawing frames
page.colors(); page.styledPaths(); page.colorOf(i);            // display list (per-path colours)
final hl = page.highlight('2:255', QvpHighlightStyle(mode: 'both', ms: 200)); page.moveHighlight(hl, T.word(3)); page.removeHighlight(hl);
page.highlightBoxesView(); page.wordBands(words);            // viewport px; draw each id as one nonzero path behind the ink
page.select(anchor, focus); page.selection(); page.selectionText('rasm_uthmani', true); page.clearSelection();
page.mask('2:255', 'hide'); page.unmaskNext(); page.maskBack(); page.unmask(); page.maskHidden(); page.maskBoxesView();
page.revealStart(lit: 2); page.revealGoto(3); page.revealPosition(); page.revealStepCount(); page.revealStop();
page.cropBounds('2:255'); page.cropSvg('2:255:1', background: '#fffdf7');
page.dispose();                                          // frees the native page

final atlas = engine.loadAtlas(atlasBytes);
atlas.pageOf(2, 255); atlas.pageRange(42); atlas.surah(36); atlas.surahs(); atlas.pageOfSurah(36);
atlas.juz(30); atlas.hizb(1); atlas.rubuAlHizb(1); atlas.juzOf(2, 255); atlas.pagesOfJuz(30); atlas.searchSurahs('cow');

QvpColor.toColor(0x1a73e8ff); QvpColor.fromColor(Colors.blue); rgba('#d6a326', 0.3);   // 0xRRGGBBAA ↔ Color
```

### Widget

```dart
QvpPageView(
  page: page,
  layout: QvpViewLayout(padTop: 24, padBottom: 24, padSide: 16, lineSpacing: 1, fillHeight: false),
  // spacing only opens up: lineSpacing < 1 is clamped to "as printed"
  paper: Color(0xfffffdf7), defaultInk: '#231f20', controller: QvpViewController(),
  onWordTap: (word, hit) {}, onDecorationTap: (decoration) {}, onEmptyTap: () {}, onSelectionChanged: (words) {},
)
```

Draw order per frame: highlight bands → cached base ink (image of every
non-styled path at the current transform; rebuilt only when the styled set,
layout or transform changes) → styled paths → mask boxes. Each frame calls
`page.tick()` first and keeps a `Ticker` running while it returns true. Tap →
`hitTestView` (gap-aware); long-press-drag → whole-word selection via
`page.select` with a band highlight in `QvpLayer.selection`; pinch / pan on
top of the engine layout; double-tap fits.

## Passages

```dart
final passage = QvpPassage.load([page], 2, 255);          // the pages can be disposed after this
final layout = passage.layout(QvpPassageSpec(width: 366, scale: 22 / passage.lineSpacing,
                                             maxRows: 2, keepAyahMark: true, ellipsisWidth: 12));
paintPassage(canvas, layout!, {42: page}, ink);           // every path under its placement
final basmalah = QvpPassage.basmalah([page], 2);          // one row, no words; throws for surahs 1 and 9
passage.close();
```

## Dress

A page can wear another mushaf's ornaments: an ayah medallion, a surah band and a page
border from an ornament set (`ornaments.qvo`). The engine places them. `QvpPageView` paints
them behind the ink and fits the border on screen.

```dart
final set = engine.loadOrnaments(bytes);                  // ornaments.qvo
final info = page.dress(set, const QvpDressSpec(style: 'mushaf-qalon'));   // null when refused
final mark = set.find(QvpOrnamentKind.ayahMark, 'mushaf-warsh');
page.dress(set, QvpDressSpec(style: 'mushaf-qalon', ayahMark: mark, gap: 8,
    lineArt: {QvpOrnamentKind.pageFrame}, colors: [(mark!, 1, '#8a5a1a')]));
for (final d in page.dressGeometry()) { /* path, color, stroke, strokeWidth, placement */ }
page.undress();
set.dispose();                                            // after the pages dressed with it
```

Each ornament carries its `licence`. Read it before you publish a dressed page.

## Tests

```sh
cargo build -p qvp-ffi --release
cd packages/flutter/qvp_flutter && flutter test        # uses $QVP_LIB or target/release/libqvp_ffi.so + dist/pages/042.qvp, 604.qvp, atlas.qva
```

## Example — Mushaf Vector Reader

`qvp_flutter/example` reproduces `web/example/app.js`: page nav + goto (ayah key,
surah name, `juz N`), search, selection panel with per-path chips, copy with
citation, crop → SVG, highlight modes + fade, follow-words, mark colours /
hide marks / gold markers, themes, memorisation (mask, reveal, greyed page),
layout controls, a dress panel (another mushaf's ornaments), page metadata and engine stats. It bundles pages 001–021,
440–445, 582, 604, `atlas.qva` and `ornaments.qvo` (when present) as its own assets (copy from `dist/pages`).

```sh
bash scripts/build-engine-android.sh
cd packages/flutter/qvp_flutter/example
flutter build apk --debug && adb install -r build/app/outputs/flutter-apk/app-debug.apk
flutter run                                        # or: adb shell am start -n ws.quran.qvp_flutter_example/.MainActivity
```
