# QVP for iOS — everything you need to try it

QVP renders the Madani mushaf as **vector outlines**, not a font and not images: the engine keeps
the real curves of the KFGQPC print and answers questions about them — which word is under this
finger, where to draw a highlight band, how to lay the lines out for this screen. This zip has the
SDK, the engine binary, the complete mushaf and a demo app. **No Rust, no build pipeline, no
repository access.**

## What's in here

```
QvpKit/       the SDK — a Swift package (Sources/) + QvpEngine.xcframework (the prebuilt engine)
example/      a SwiftUI reader app built on QvpKit, with all 604 pages in example/pages/
docs/API.md   the full engine API, one page, every call
docs/qvp.h    the C ABI the SDK wraps — the contract, if you want to see underneath
docs/ios.md   the iOS integration guide: architecture, the renderer, gestures, tests
```

## Run the demo (2 minutes)

Requires **Xcode 15 or later**. Nothing else.

```sh
open example/Demo.xcodeproj
```

Pick an iPhone simulator and hit Run. For a physical device, set your signing team first:
select the **Demo** target → **Signing & Capabilities** → choose your team.

What to try: swipe to flip pages, tap a word or an ayah mark to highlight it, pinch to zoom,
the list icon to jump to an ayah / juz / surah, the magnifier to search the page, **AA** for
themes and layout, and the brain icon for the memorisation tools (mask, reveal, greyed page).

## Use it in your own app

Add the package: Xcode → **File → Add Package Dependencies → Add Local…** → pick `QvpKit/`.

```swift
import QvpKit

// 1. load a page and (optionally) its text-forms sidecar
let page = try QvpPage(bytes: Data(contentsOf: pageURL))       // NNN.qvp
_ = page.attachWords(try Data(contentsOf: sidecarURL))          // NNN.words.json

// 2. drop the view in — it draws, hit-tests and animates
let view = QvpPageView()
view.page = page
view.onWordTap = { word, _ in print(word.wordKey, word.text) }  // "2:255:1", "ٱللَّهُ"

// 3. ask the engine for anything visual — it decides, you paint
page.highlight(Target.ayah(2, 255))                             // animated band
page.theme(QvpTheme(diacritics: 0x1a73e8ff, dots: 0xc62828ff))  // colour by mark family
page.mask(Target.ayah(2, 255), .hide)                           // memorisation
page.search("الله")                                             // normalised search
```

`example/Sources/DemoModel.swift` is the honest reference: every visual state in the demo is engine
state, and the view only paints. `docs/API.md` lists the rest — layout, selection, crop-to-SVG,
the cross-page atlas.

## The page data

`example/pages/` holds the whole mushaf, and your app needs the same files:

| file | what |
|---|---|
| `NNN.qvp` | one page: outlines, words, lines, ayahs, decorations |
| `NNN.words.json` | that page's text forms (rasm imlai, QPC, search) — optional |
| `atlas.qva` | cross-page lookup: surah → page, ayah → page, juz, hizb |

604 pages come to about 91 MB. Bundle them all, bundle a subset, or download them on first launch
and cache them — the engine only ever needs the bytes of the page you're showing.

## Good to know

- **Lossless.** The curves are the print's curves; nothing is simplified or rasterised.
- **The engine decides, the wrapper paints.** Hit-testing, layout, highlights, masks, search and
  styles all live in the engine, so iOS, Android, Flutter, React Native and web behave identically.
  If you find yourself re-implementing engine logic in Swift, ask us — there's probably a call for it.
- **Simulator and device** are both covered by the included XCFramework (arm64 + x86_64 simulator,
  arm64 device, arm64 macOS).

Questions, bugs, or an API you wish existed — tell us; the ABI is still moving and shaped by use.
