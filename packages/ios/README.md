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
│   │   ├── QvpPageView.swift       UIView renderer: bands → cached base ink → styled ink → mask boxes; gestures; CADisplayLink
│   │   └── QvpPageCanvas.swift     the same frame as SwiftUI Canvas (iOS 17+): QvpCanvasController + QvpPageCanvas
│   └── Tests/QvpKitTests/          XCTest: the same assertions as the Flutter/Dart test (page 042, atlas)
└── Demo/                           SwiftUI app (`xcodegen generate` → Demo.xcodeproj, not committed)
    ├── Sources/                    DemoModel (engine state, a port of the Android MainActivity) + ContentView (the reader UI)
    ├── UITests/                    XCUITest: tap, swipe page flip, pinch, search sheet on the real view
    ├── sync-pages.sh               fills pages/ with all 604 pages + atlas.qva, from the data release or dist/pages (pre-build step)
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

Page data is a separate download — the whole mushaf, not built here:
`Demo/sync-pages.sh` fetches the `quran-engine-pages-hafs-kfgqpc.tar.gz` asset of the
[`v0.1.0` data release](https://github.com/quran-ws/quran-engine/releases/tag/v0.1.0), checks its
sha256 and unpacks it into `dist/pages` (`NNN.qvp`, `NNN.words.json` text-forms sidecar, `atlas.qva`).
Set `QVP_DATA_TAG` to pin another release, or `QVP_PAGES=<dir>` to use converter output
(`cargo run -p qvp-convert --release -- batch pages dist/pages`) instead.

## Depend on it

Xcode → File → Add Package Dependencies → local path `packages/ios/QvpKit`, or in a `Package.swift`:

```swift
.package(path: "../quran-engine/packages/ios/QvpKit")     // product "QvpKit"
```

## API (mirrors the Kotlin wrapper — see docs/API.md)

```swift
import QvpKit

QvpEngine.version(); QvpEngine.markName(7); QvpEngine.kindName(QvpKind.MARK); QvpEngine.markFromName("shaddah")
QvpEngine.strip(s); QvpEngine.fold(s); QvpEngine.normalize(s); QvpEngine.looseKey(s)          // Arabic text tools
QvpEngine.gapToFill(pageW:pageH:lines:viewW:viewH:); QvpEngine.wastedFraction(pageW:pageH:viewW:viewH:)

let page = try QvpPage(bytes: data)                     // geometry copied once: page.ops / page.pts / page.table (stride 8)
page.words / ayahs / lines / decos;  page.wordForm(i, .rasmImlai);  page.findWord(2, 255, 3);  page.buildPaths()  // [CGPath]
page.resolve("2:255")                                   // "page" | "2:255" | "2:255:3" | "2:255-257" | "line:7" | "surah:2" | Target.word(i) | [w0, w1]
page.surahs(); page.divisions(); page.ayahMarks(); page.rosettes(); page.sajdahs(); page.ayahKeys()
page.ayahWordCount(2, 255); page.reciteMap(2, 255, nSegments: 4); page.wordLabel(i); page.ayahLabel(ai)
page.text("2:255"); page.search("الله", mode: .includes); page.citation(words); page.attachWords(json); page.hasForm(.qpc)
page.hitTest(x, y); page.hitTestEx(x, y, QvpHitOptions(maxDistance: 6))                        // page units, exact / gap-aware
page.hitTestView(vx, vy); page.hitTestViewEx(vx, vy)                                            // viewport px through the layout
page.lineBands(); page.hitBoxes()
let l = page.layout(QvpLayoutSpec(viewportW: 690, viewportH: 1100, padTop: 50, padBottom: 50, fillHeight: true))  // l.scale, l.lineDy[line]
page.wordBoxView(i)
let h = page.style(Selector.wordMark(w, 1), 0xef6c00ff, transitionMs: 200, layer: QvpLayer.TOP)   // Selector.path/word/ayah/line/mark/category/family/kind/deco…
page.styleTarget("2:255", rgba); page.restyle(h, rgba); page.unstyle(h); page.hide(Selector.kind(QvpKind.MARK))
page.theme(QvpTheme(diacritics: 0x1a73e8ff, marks: ["shaddah": 0x0a7d32ff])); page.setDefaultInk(0x231f20ff); page.clearStyles(); page.clearLayer(QvpLayer.THEME)
page.tick(nowMs)                                        // true while animating — keep drawing frames
page.paint(); page.styled(); page.colorOf(i)             // display list (per-path colours)
let hl = page.highlight("2:255", QvpHighlightStyle(mode: .both, transitionMs: 200)); page.rehighlight(hl, Target.word(3)); page.unhighlight(hl)
page.highlightBoxes(); page.bandBoxes(words)             // viewport px; draw each id as one nonzero path behind the ink
page.select(anchor, focus); page.selection(); page.selectionText(.rasmUthmani, citation: true); page.clearSelection()
page.mask("2:255", .hide); page.revealNext(); page.hideBack(); page.unmask(); page.maskHidden(); page.maskBoxes()
page.revealStart(lit: 2); page.revealGoto(3); page.revealAt(); page.revealSteps(); page.revealStop()
page.cropBox("2:255"); page.cropSvg("2:255:1", background: 0xfffdf7ff)
page.close()                                             // frees the native page (also on deinit)

let atlas = try QvpAtlas(bytes: atlasData)
atlas.pageOf(2, 255); atlas.pageRange(42); atlas.surah(36); atlas.surahs(); atlas.pageOfSurah(36)
atlas.juz(30); atlas.hizb(1); atlas.rubuAlHizb(1); atlas.juzAt(2, 255); atlas.pagesOfJuz(30); atlas.findSurah("cow")

QvpColor.parse("#d6a326", alpha: 0.3)  // 0xd6a3264d
QvpColor.withAlpha(rgba, 0.18); QvpColor.cgColor(rgba); QvpColor.rgba(cgColor)
```

Colours are `0xRRGGBBAA` everywhere (alpha 0 = hidden / leave alone). Page units are the
printed viewBox (345 × 550, y down); anything `…View` is viewport px through the current layout.

### QvpPageView (UIKit)

```swift
let view = QvpPageView()
view.padTop = 12; view.padBottom = 12; view.padSide = 8; view.lineSpacing = 1; view.lineGap = 0; view.fillHeight = false
// lineSpacing < 1 / a negative lineGap are clamped by the engine: spacing only ever opens up
view.paperColor = UIColor(...); view.selectionBand = 0x2d6fd640; view.hitOptions = QvpHitOptions(maxDistance: 6)
view.onWordTap = { word, hit in }; view.onDecoTap = { deco, hit in }; view.onEmptyTap = { }; view.onSelectionChanged = { words in }
view.onSwipe = { dir in }                 // horizontal swipe while not zoomed (+1 finger right, −1 left): flip pages; view.isZoomed
view.onDoubleTap = { hit in }              // nil (default) resets the view; a host that repurposes it calls resetView() itself
view.onLongPress = { hit in }              // only while selectionEnabled is false; longPressDuration (0.35)
view.zoomSpringsBack = true               // zoom lasts only while pinching: on release the page eases back to its fitted size
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
whole-word selection through `page.select` with a band highlight in `QvpLayer.SELECTION`
(`selectionEnabled = false` turns it off); a horizontal pan while the page is at its fitted size is reported
through `onSwipe` instead of panning, so the host can flip pages; double-tap resets the view (or runs
`onDoubleTap` when set). Wrap it for SwiftUI with a `UIViewRepresentable` (see `Demo/Sources/ContentView.swift`).

### QvpPageCanvas (SwiftUI, iOS 17+ / macOS 14+)

The same frame drawn with SwiftUI `Canvas` — for SwiftUI-first apps that would otherwise bridge
`QvpPageView` through a representable. Identical draw order and base-ink bitmap cache; frames run
through `TimelineView(.animation)` only while `page.tick(now)` reports a transition in flight.

```swift
@State private var controller = QvpCanvasController()

QvpPageCanvas(controller: controller)

controller.page = page                    // relayouts, fits and centres
controller.padTop = 12; controller.fillHeight = true; controller.paperColor = 0xfffdf7ff
controller.onWordTap = { word, hit in }; controller.onDecoTap = { deco, hit in }; controller.onEmptyTap = { }
controller.onDoubleTap = { hit in }        // nil (default) resets the view
controller.onLongPress = { hit in }        // only while selectionEnabled is false — a UIKit recognizer on iOS, never takes a pager's swipe
controller.zoomSpringsBack = true         // zoom lasts only while pinching, as on the UIKit view
controller.invalidate()                   // after engine calls the controller cannot see (highlight, style, mask, …)
controller.resetView(); controller.relayout(); controller.clearSelection(); controller.lineTransform(line); controller.isZoomed
controller.cropLeft = box.x0; controller.cropRight = page.width - box.x1   // box = page.cropBox("page"): the ink spans the viewport

// A pager over many pages: one PERMANENT controller per page; only page data loads, LRU-evicts
// (never the current page ±1) and reattaches to the same controller — a view never holds a closed page.
let cache = QvpPageCache(data: { n in /* the page's .qvp bytes */ }, configure: { page, controller in /* ink, crop, layout */ })
QvpPageCanvas(controller: cache.controller(for: n))
cache.setCurrentPage(n)                   // preload neighbours; cache.reconfigure() after a theme change
```

SwiftUI has no `setNeedsDisplay()`, so the mutable surface lives on `QvpCanvasController`
(`@Observable`): the knobs, callbacks, transform state and HUD stats carry the exact names the
UIKit view has, and `invalidate()` is the redraw call. `lineSpacing` only opens up — the
printed pitch is the floor, values below 1 clamp to 1. Gesture differences from UIKit, both
deliberate: when the page is not zoomed and `onSwipe` is nil the drag gesture is detached
entirely, so an enclosing pager (`TabView`, `ScrollView`) keeps its own swipe; and drag-selection
begins on the first finger movement after the long press rather than at the press itself
(`LongPressGesture` reports no location).

## Demo

```sh
scripts/build-engine-ios.sh                                                   # engine
# data: Demo/sync-pages.sh downloads the release bundle on the first build
cd packages/ios/Demo
xcodebuild -scheme Demo -destination 'platform=iOS Simulator,name=iPhone 17' build   # sync-pages.sh runs as a pre-build step
xcrun simctl install booted build/…/Demo.app && xcrun simctl launch booted ws.quran.qvp.demo
```

Or run `xcodegen generate` in `Demo/` and open the `Demo.xcodeproj` it writes (it is not committed).
The app bundles the complete mushaf — all 604 pages (`.qvp` + `.words.json`) and `atlas.qva`, about
91 MB — put there by `sync-pages.sh` and never committed.

A simple Quran reader built from stock iOS components (`NavigationStack`, toolbars, `Form`, `List`,
`.searchable`, `Menu`, sheets), with the engine doing every visual decision:

- **Reader** — the page fills the screen height (`fillHeight`, the engine adds equal leading between the printed lines),
  swipe right/left to flip pages in mushaf order (a snapshot of the old page slides away while the new
  one is already drawn), pinch to zoom then pan, double-tap to reset. The title shows surah · page · juz.
- **Tap** a word or an ayah mark to highlight it (engine highlight in the selection layer; the page's
  accessibility value announces it). Tap empty paper to clear.
- **Go to** (list icon) — ayah key (`2:255`), juz buttons, searchable surah list from the atlas.
- **Search** (magnifier) — engine search with normalisation; hits are highlighted on the page, pick one to jump.
- **Reading** (AA) — theme (light / sepia / dark), coloured marks, hide tashkil, gold ayah marks; fill height,
  line spacing, padding, leading-to-fill; highlight style and fade; page metadata; engine stats; reset.
- **Memorise** (bottom bar) — mask the current ayah (hide or cover), reveal next / hide back / show all,
  greyed page with a slider; **Follow words** walks the page word by word with one animated highlight.

All 604 pages (`.qvp` + `.words.json`) and `atlas.qva` are bundled by `sync-pages.sh` — never
committed; page navigation covers the whole mushaf. Launch arguments script a state for
screenshots and QA: `-qvpPage 582 -qvpGoto 2:255 -qvpSearch الله -qvpAyah 78:1 -qvpWord 12 -qvpTheme dark
-qvpMarks 1 -qvpGold 1 -qvpMask 1 -qvpFill 1 -qvpSheet settings|search|goto`.

## Share with an iOS developer

`scripts/package-ios-demo.sh` → `dist/qvp-ios-demo.zip`: `packages/ios` with the built XCFramework and the
demo's pages, plus `docs/API.md` and `qvp.h`. Open `Demo/Demo.xcodeproj`, set a signing team, run — no Rust,
no data pipeline.

## Tests

```sh
cd packages/ios/QvpKit && swift test                                                          # macOS slice, 13 tests
xcodebuild test -scheme QvpKit -destination 'platform=iOS Simulator,name=iPhone 17'           # same on the simulator
cd ../Demo && xcodebuild test -scheme Demo -destination 'platform=iOS Simulator,name=iPhone 17' -only-testing:DemoUITests   # 5 gesture/UI tests
```

`QvpKitTests` mirrors `packages/flutter/qvp_flutter/test/qvp_flutter_test.dart`: page 042 has 147
words / 1061 paths / 15 lines, `search("الله")` → 7, `resolve("2:255")` → 50 words, layout
690×1100 fill-height → scale 2, highlight tick / 6 band boxes, mask / reveal, selection, crop,
atlas `pageOf(2, 255)` = 42 and `pagesOfJuz(30)` = 582…604. It reads `dist/pages` from the repo
(or `$QVP_REPO`).
