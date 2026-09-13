# qvp_flutter — Flutter wrapper for the QVP vector mushaf engine

A dart:ffi plugin (no method channels, no Kotlin) over `libqvp_ffi` — the one
Rust core in `crates/qvp-core`. The engine decides everything (layout,
hit-testing, style resolution, highlights, masks, search, text); the plugin
only marshals the C ABI in `crates/qvp-ffi/include/qvp.h` and draws with a
`CustomPainter`. Method names are identical to `web/qvp.js`, so the web docs
apply one to one. The package ships **no page data**.

## Depend on it

```yaml
dependencies:
  qvp_flutter:
    path: ../packages/flutter/qvp_flutter      # or a git dependency
```

Produce the engine library once (writes
`qvp_flutter/android/src/main/jniLibs/<abi>/libqvp_ffi.so` for arm64-v8a,
armeabi-v7a and x86_64 — a build artefact, gitignored on purpose):

```sh
export ANDROID_NDK_HOME=$ANDROID_HOME/ndk/27.2.12479018
bash scripts/build-engine-android.sh          # needs rustup targets + cargo-ndk (installed on demand)
cargo build -p qvp-ffi --release              # host lib for `flutter test`: target/release/libqvp_ffi.so
```

Page data comes from the converter: `cargo run -p qvp-convert --release -- batch pages dist/pages`
→ `dist/pages/NNN.qvp`, `NNN.words.json` (text forms sidecar) and `atlas.qva`.

## API (mirrors web/qvp.js)

```dart
import 'package:qvp_flutter/qvp_flutter.dart';

final engine = QvpEngine.open();                        // Android: libqvp_ffi.so from the plugin; host: $QVP_LIB or path:
engine.markName(7); engine.kindName(QvpKind.mark); engine.categoryName(1); engine.familyName(1);
engine.strip(s); engine.fold(s); engine.normalize(s); engine.looseKey(s);          // Arabic text tools
engine.gapToFill(pw, ph, lines, vw, vh); engine.wastedFraction(pw, ph, vw, vh);

final page = engine.loadPage(bytes);                    // geometry copied once: page.ops / page.pts / page.table (stride 8)
page.words / ayahs / lines / decos;  page.wordForm(i, 'rasm_imlai');  page.findWord(2, 255, 3);
page.resolve('2:255');                                  // targets: 'page' | '2:255' | '2:255:3' | '2:255-257' | 'line:7' | 'surah:2' | T.word(i) | [w0, w1]
page.surahs(); page.divisions(); page.markers(); page.rosettes(); page.sajdahs(); page.ayahKeys();
page.ayahWordCount(2, 255); page.reciteMap(2, 255, 4); page.wordLabel(i); page.ayahLabel(ai);
page.text('2:255'); page.search('الله', mode: 'includes'); page.citation(words); page.attachWords(json); page.hasForm('qpc');
page.hitTest(x, y); page.hitTestEx(x, y, QvpHitOptions(maxDistance: 6));         // page units, exact / gap-aware
page.hitTestView(vx, vy); page.hitTestViewEx(vx, vy);                             // viewport px through the layout
page.lineBands(); page.hitBoxes();
final l = page.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100, padTop: 50, padBottom: 50, fillHeight: true)); // l.scale, l.lineDy[line]
page.wordBoxView(i);
final h = page.style(Sel.wordMark(w, 1), '#ef6c00', ms: 200, layer: QvpLayer.top);  // Sel.path/word/ayah/line/mark/category/family/kind/deco…
page.styleTarget('2:255', color); page.restyle(h, color); page.unstyle(h); page.hide(Sel.kind(QvpKind.mark));
page.theme(QvpTheme(diacritics: '#1a73e8', marks: {'shaddah': '#0a7d32'})); page.setDefaultInk('#231f20'); page.clearStyles(); page.clearLayer(QvpLayer.theme);
page.tick(nowMs);                                       // true while animating — keep drawing frames
page.paint(); page.styled(); page.colorOf(i);            // display list (per-path colours)
final hl = page.highlight('2:255', QvpHighlightStyle(mode: 'both', ms: 200)); page.rehighlight(hl, T.word(3)); page.unhighlight(hl);
page.highlightBoxes(); page.bandBoxes(words);            // viewport px; draw each id as one nonzero path behind the ink
page.select(anchor, focus); page.selection(); page.selectionText('rasm_uthmani', true); page.clearSelection();
page.mask('2:255', 'hide'); page.revealNext(); page.hideBack(); page.unmask(); page.maskHidden(); page.maskBoxes();
page.revealStart(lit: 2); page.revealGoto(3); page.revealAt(); page.revealSteps(); page.revealStop();
page.cropBox('2:255'); page.cropSvg('2:255:1', background: '#fffdf7');
page.dispose();                                          // frees the native page

final atlas = engine.loadAtlas(atlasBytes);
atlas.pageOf(2, 255); atlas.pageRange(42); atlas.surah(36); atlas.surahs(); atlas.pageOfSurah(36);
atlas.juz(30); atlas.hizb(1); atlas.rubuAlHizb(1); atlas.juzAt(2, 255); atlas.pagesOfJuz(30); atlas.findSurah('cow');

QvpColor.toColor(0x1a73e8ff); QvpColor.fromColor(Colors.blue); rgba('#d6a326', 0.3);   // 0xRRGGBBAA ↔ Color
```

### Widget

```dart
QvpPageView(
  page: page,
  layout: QvpViewLayout(padTop: 24, padBottom: 24, padSide: 16, lineSpacing: 1, lineGap: 0, fillHeight: false),
  // spacing only opens up: lineSpacing < 1 and a negative lineGap are clamped to "as printed"
  paper: Color(0xfffffdf7), defaultInk: '#231f20', controller: QvpViewController(),
  onWordTap: (word, hit) {}, onDecoTap: (deco) {}, onEmptyTap: () {}, onSelectionChanged: (words) {},
)
```

Draw order per frame: highlight bands → cached base ink (image of every
non-styled path at the current transform; rebuilt only when the styled set,
layout or transform changes) → styled paths → mask boxes. Each frame calls
`page.tick()` first and keeps a `Ticker` running while it returns true. Tap →
`hitTestViewEx` (gap-aware); long-press-drag → whole-word selection via
`page.select` with a band highlight in `QvpLayer.selection`; pinch / pan on
top of the engine layout; double-tap fits.

## Tests

```sh
cargo build -p qvp-ffi --release
cd packages/flutter/qvp_flutter && flutter test        # uses $QVP_LIB or target/release/libqvp_ffi.so + dist/pages/042.qvp, atlas.qva
```

## Example — Mushaf Vector Reader

`qvp_flutter/example` reproduces `web/example/app.js`: page nav + goto (ayah key,
surah name, `juz N`), search, selection panel with per-path chips, copy with
citation, crop → SVG, highlight modes + fade, follow-words, mark colours /
hide marks / gold markers, themes, memorisation (mask, reveal, greyed page),
layout controls, page metadata and engine stats. It bundles pages 001–021,
440–445, 582, 604 and `atlas.qva` as its own assets (copy from `dist/pages`).

```sh
bash scripts/build-engine-android.sh
cd packages/flutter/qvp_flutter/example
flutter build apk --debug && adb install -r build/app/outputs/flutter-apk/app-debug.apk
flutter run                                        # or: adb shell am start -n ws.quran.qvp_flutter_example/.MainActivity
```
