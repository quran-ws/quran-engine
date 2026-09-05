# QvpKit — Swift wrapper for the QVP vector mushaf engine

A Swift package over the engine's C ABI (`crates/qvp-ffi/include/qvp.h`), plus a SwiftUI demo.
The engine decides everything (layout, hit-testing, style resolution, highlights, masks,
search, text); the package only marshals the C structs and draws with CoreGraphics. Class and
method names mirror the Kotlin wrapper (`packages/android/qvp`) and `web/qvp.js`, so
`docs/API.md` applies one to one. The package ships **no page data**.

```
packages/ios/
├── QvpKit/                         Swift package
│   ├── Package.swift               binaryTarget QvpEngine.xcframework + target QvpKit + tests
│   ├── QvpEngine.xcframework/      built by scripts/build-engine-ios.sh (gitignored)
│   ├── Sources/QvpKit/
│   │   ├── Types.swift             constants, Selector, Target, QvpColor, records, QvpHighlightStyle, QvpTheme, QvpLayoutSpec
│   │   ├── QvpEngine.swift         names, Arabic tools, gapToFill / wastedFraction
│   │   ├── QvpPage.swift           QvpPage (geometry copied once, CGPath per path built once) and QvpAtlas
│   │   └── QvpPageView.swift       UIView renderer: bands → cached base ink → styled ink → mask boxes; gestures; CADisplayLink
│   └── Tests/QvpKitTests/          XCTest: the same assertions as the Flutter/Dart test (page 042, atlas)
└── Demo/                           SwiftUI app (xcodegen project.yml → Demo.xcodeproj, committed)
    ├── Sources/                    DemoModel (a port of the Android MainActivity) + ContentView
    ├── UITests/                    XCUITest: tap, long-press drag selection, pinch on the real view
    ├── sync-pages.sh               copies 001–021, 440–445, 582, 604 + atlas.qva from dist/pages (pre-build step)
    └── pages/                      the demo's assets (gitignored)
```

## Build the engine once

```sh
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios     # added on demand by the script
scripts/build-engine-ios.sh
```

`cargo build -p qvp-ffi --release --target …` for device (arm64), simulator (arm64 + x86_64,
lipo'd into one slice) and macOS (arm64, so `swift test` runs on the Mac), then
`xcodebuild -create-xcframework` with `qvp.h` and a `module.modulemap` (clang module `QvpFFI`)
→ `packages/ios/QvpKit/QvpEngine.xcframework`. Rerun after any engine change.

Page data comes from the converter: `cargo run -p qvp-convert --release -- batch pages dist/pages`
→ `dist/pages/NNN.qvp`, `NNN.words.json` (text forms sidecar), `atlas.qva`.

## Depend on it

Xcode → File → Add Package Dependencies → local path `packages/ios/QvpKit`, or in a `Package.swift`:

```swift
.package(path: "../quran-engine/packages/ios/QvpKit")     // product "QvpKit"
```

## API (mirrors the Kotlin wrapper — see docs/API.md)

```swift
import QvpKit

QvpEngine.version(); QvpEngine.markName(7); QvpEngine.kindName(QvpKind.MARK); QvpEngine.markFromName("shadda")
QvpEngine.strip(s); QvpEngine.fold(s); QvpEngine.normalize(s); QvpEngine.looseKey(s)          // Arabic text tools
QvpEngine.gapToFill(pageW:pageH:lines:viewW:viewH:); QvpEngine.wastedFraction(pageW:pageH:viewW:viewH:)

let page = try QvpPage(bytes: data)                     // geometry copied once: page.ops / page.pts / page.table (stride 8)
page.words / ayahs / lines / decos;  page.wordForm(i, .imlaei);  page.findWord(2, 255, 3);  page.buildPaths()  // [CGPath]
page.resolve("2:255")                                   // "page" | "2:255" | "2:255:3" | "2:255-257" | "line:7" | "surah:2" | Target.word(i) | [w0, w1]
page.surahs(); page.divisions(); page.markers(); page.rosettes(); page.sajdahs(); page.ayahKeys()
page.ayahWordCount(2, 255); page.reciteMap(2, 255, nSegments: 4); page.wordLabel(i); page.ayahLabel(ai)
page.text("2:255"); page.search("الله", mode: .includes); page.citation(words); page.attachWords(json); page.hasForm(.qpc)
page.hitTest(x, y); page.hitTestEx(x, y, QvpHitOptions(maxDistance: 6))                        // page units, exact / gap-aware
page.hitTestView(vx, vy); page.hitTestViewEx(vx, vy)                                            // viewport px through the layout
page.lineBands(); page.hitBoxes()
let l = page.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100, padTop: 50, padBottom: 50, fillHeight: true))  // l.scale, l.lineDy[line]
page.wordBoxView(i)
let h = page.style(Selector.wordMark(w, 1), 0xef6c00ff, transitionMs: 200, layer: QvpLayer.TOP)   // Selector.path/word/ayah/line/mark/category/family/kind/deco…
page.styleTarget("2:255", rgba); page.restyle(h, rgba); page.unstyle(h); page.hide(Selector.kind(QvpKind.MARK))
page.theme(QvpTheme(diacritics: 0x1a73e8ff, marks: ["shadda": 0x0a7d32ff])); page.setDefaultInk(0x231f20ff); page.clearStyles(); page.clearLayer(QvpLayer.THEME)
page.tick(nowMs)                                        // true while animating — keep drawing frames
page.paint(); page.styled(); page.colorOf(i)             // display list (per-path colours)
let hl = page.highlight("2:255", QvpHighlightStyle(mode: .both, transitionMs: 200)); page.rehighlight(hl, Target.word(3)); page.unhighlight(hl)
page.highlightBoxes(); page.bandBoxes(words)             // viewport px; draw each id as one nonzero path behind the ink
page.select(anchor, focus); page.selection(); page.selectionText(.uthmani, citation: true); page.clearSelection()
page.mask("2:255", .hide); page.revealNext(); page.hideBack(); page.unmask(); page.maskHidden(); page.maskBoxes()
page.revealStart(lit: 2); page.revealGoto(3); page.revealAt(); page.revealSteps(); page.revealStop()
page.cropBox("2:255"); page.cropSvg("2:255:1", background: 0xfffdf7ff)
page.close()                                             // frees the native page (also on deinit)

let atlas = try QvpAtlas(bytes: atlasData)
atlas.pageOf(2, 255); atlas.pageRange(42); atlas.surah(36); atlas.surahs(); atlas.pageOfSurah(36)
atlas.juz(30); atlas.hizb(1); atlas.rub(1); atlas.juzAt(2, 255); atlas.pagesOfJuz(30); atlas.findSurah("cow")

QvpColor.parse("#d6a326", alpha: 0.3)  // 0xd6a3264d
QvpColor.withAlpha(rgba, 0.18); QvpColor.cgColor(rgba); QvpColor.rgba(cgColor)
```

Colours are `0xRRGGBBAA` everywhere (alpha 0 = hidden / leave alone). Page units are the
printed viewBox (345 × 550, y down); anything `…View` is viewport px through the current layout.

### QvpPageView (UIKit)

```swift
let view = QvpPageView()
view.padTop = 12; view.padBottom = 12; view.padSide = 8; view.lineSpacing = 1; view.lineGap = 0; view.fillHeight = false
view.paperColor = UIColor(...); view.selectionBand = 0x2d6fd640; view.hitOptions = QvpHitOptions(maxDistance: 6)
view.onWordTap = { word, hit in }; view.onDecoTap = { deco, hit in }; view.onEmptyTap = { }; view.onSelectionChanged = { words in }
view.page = page                          // lays out, fits and centres; setNeedsDisplay() after engine calls
view.relayout(); view.resetView(); view.clearSelection(); view.lineTransform(line)
view.lastBaseMs / lastOverlayMs / lastHitUs / lastBasePaths / lastOverlayPaths / lastBands / animating   // HUD stats
```

Draw order per frame: `highlightBoxes()` (one nonzero path per highlight id, behind the ink) →
cached base ink (a bitmap of every non-styled path at the current per-line transform, rebuilt only
when the styled set, layout or pan/zoom changes) → `styled()` paths → `maskBoxes()`. Per-line
transform: `vx = ox + x·scale`, `vy = oy + (y + lineDy[line])·scale`, with pinch/pan on top. Each
frame calls `page.tick(now)`; a `CADisplayLink` keeps running while it returns true. Tap → gap-aware
`hitTestViewEx` (max distance 6) → `onWordTap` / `onDecoTap` / `onEmptyTap`; long-press + drag →
whole-word selection through `page.select` with a band highlight in `QvpLayer.SELECTION`; double-tap
resets the view. Wrap it for SwiftUI with a `UIViewRepresentable` (see `Demo/Sources/ContentView.swift`).

## Demo

```sh
scripts/build-engine-ios.sh                                                   # engine
cargo run -p qvp-convert --release -- batch pages dist/pages                  # data
cd packages/ios/Demo
xcodebuild -scheme Demo -destination 'platform=iOS Simulator,name=iPhone 17' build   # sync-pages.sh runs as a pre-build step
xcrun simctl install booted build/…/Demo.app && xcrun simctl launch booted net.quranpedia.qvp.demo
```

Or open `Demo/Demo.xcodeproj` (regenerate with `xcodegen generate` after editing `project.yml`).
The app bundles only pages 001–021, 440–445, 582 and 604 (`.qvp` + `.words.json`) and `atlas.qva`,
copied from `dist/pages` by `sync-pages.sh` — never committed.

It reproduces the Android demo: page navigation and atlas goto (`2:255`, `Yasin`, `juz 30`),
search with engine highlights, a selection panel with per-path chips (`Selector.wordMark`),
copy with citation, crop → SVG share sheet, highlight mode + fade slider, follow words, mark
colours / hide marks / gold markers, light/sepia/dark themes, mask (hide/block) and greyed-page
reveal with a slider, line spacing / padding / fill-height / leading-to-fill, page metadata and
engine stats. Launch arguments script a state for screenshots and QA:
`-qvpPage 582 -qvpGoto 2:255 -qvpSearch الله -qvpAyah 78:1 -qvpWord 12 -qvpChip 2 -qvpTheme dark -qvpMarks 1 -qvpGold 1 -qvpMask 1 -qvpFill 1`.

## Tests

```sh
cd packages/ios/QvpKit && swift test                                                          # macOS slice, 13 tests
xcodebuild test -scheme QvpKit -destination 'platform=iOS Simulator,name=iPhone 17'           # same on the simulator
cd ../Demo && xcodebuild test -scheme Demo -destination 'platform=iOS Simulator,name=iPhone 17' -only-testing:DemoUITests   # gestures
```

`QvpKitTests` mirrors `packages/flutter/qvp_flutter/test/qvp_flutter_test.dart`: page 042 has 147
words / 1061 paths / 15 lines, `search("الله")` → 7, `resolve("2:255")` → 50 words, layout
690×1100 fill-height → scale 2, highlight tick / 6 band boxes, mask / reveal, selection, crop,
atlas `pageOf(2, 255)` = 42 and `pagesOfJuz(30)` = 582…604. It reads `dist/pages` from the repo
(or `$QVP_REPO`).
