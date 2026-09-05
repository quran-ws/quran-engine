# Running and extending on macOS

Everything here also builds on macOS. The iOS wrapper is the one piece that needs a Mac.

## Setup (once)

```sh
git clone git@github.com:quranpedia/quran-engine.git && cd quran-engine
curl -sSf https://sh.rustup.rs | sh -s -- -y -t wasm32-unknown-unknown
rustup target add aarch64-apple-ios aarch64-apple-ios-sim x86_64-apple-ios
brew install cmake ninja          # optional, for Flutter desktop
# put the source SVGs in pages/ (they are not in git), then:
cargo test --workspace --release
cargo run -p qvp-convert --release -- batch pages dist/pages
```

## Web demo

```sh
cargo build -p qvp-ffi --release --target wasm32-unknown-unknown
cp target/wasm32-unknown-unknown/release/qvp_ffi.wasm web/ && cp dist/pages/* web/pages/
python3 web/build.py dev && (cd web && python3 -m http.server 8765)     # http://127.0.0.1:8765/?p=42
```

## Android (Kotlin, Flutter, React Native demos) on a Mac

Install Android Studio (SDK 35, NDK 27.2.12479018, an x86_64 or arm64 AVD), JDK 17,
Flutter stable, Node 20+. Then:

```sh
cargo install cargo-ndk
export ANDROID_NDK_HOME=$HOME/Library/Android/sdk/ndk/27.2.12479018
scripts/build-engine-android.sh
(cd packages/android && ./gradlew :demo:installDebug)
(cd packages/flutter/qvp_flutter/example && flutter run)
(cd packages/react-native/example && npm install && npx react-native run-android)
```

## iOS (Swift package + demo)

```sh
scripts/build-engine-ios.sh                          # QvpEngine.xcframework (device, simulator, macOS)
(cd packages/ios/QvpKit && swift test)               # 13 XCTests on the macOS slice
(cd packages/ios/Demo && xcodebuild -scheme Demo -destination 'platform=iOS Simulator,name=iPhone 17' build)
```

See `packages/ios/README.md`. Pick any installed simulator name (`xcrun simctl list devices available`).

## iOS wrapper — the prompt it was built from

This was run in Claude Code from the repo root (kept for reference):

> Build the iOS/Swift wrapper and demo for the QVP engine in this repo. Read README.md,
> docs/API.md, crates/qvp-ffi/include/qvp.h, web/qvp.js (reference wrapper),
> packages/android/qvp/src/main/kotlin/net/quranpedia/qvp/*.kt (the Kotlin wrapper: mirror
> its class and method names in Swift: QvpEngine, QvpPage, QvpAtlas, Selector, Target,
> QvpHighlightStyle, QvpTheme, QvpLayoutSpec, QvpPageView) and packages/android/demo for the
> demo behaviours. Deliverables: (1) scripts/build-engine-ios.sh that builds qvp-ffi for
> aarch64-apple-ios, aarch64-apple-ios-sim and x86_64-apple-ios (`cargo build -p qvp-ffi
> --release --target …`), lipo's the simulator slices, and packages a QvpEngine.xcframework
> from the static libs with include/qvp.h and a module.modulemap; (2) packages/ios/QvpKit — a
> Swift package (Package.swift, binaryTarget on the xcframework) exposing the same API as the
> Kotlin wrapper over the C ABI (all coordinates/colours as in docs/API.md; geometry copied
> once into Swift arrays; CGPath per engine path built once), plus QvpPageView (UIView) that
> draws with CoreGraphics in this order: highlightBoxes (one path per highlight id, nonzero,
> behind the ink) → cached base ink (CGLayer or bitmap of every non-styled path at the current
> transform, rebuilt only when the styled set / layout / transform changes) → styled paths from
> styled() → maskBoxes; per-line transforms from the engine layout (vx = ox + x*scale, vy =
> oy + (y + lineDy[line])*scale) with pinch/pan on top; a CADisplayLink calls page.tick(now)
> and keeps running while it returns true; UITapGestureRecognizer → hitTestViewEx(maxDistance
> 6) → onWordTap / onDecoTap / onEmptyTap; UILongPressGestureRecognizer + drag → whole-word
> selection via page.select and a band highlight in the selection layer; (3)
> packages/ios/Demo — a SwiftUI app reproducing packages/android/demo (page nav + atlas goto,
> search, selection panel with per-path chips using Selector.wordMark, copy with citation, crop
> → SVG share sheet, highlight mode + fade slider, follow words, mark colours / hide marks / gold
> markers, themes, mask/reveal, layout sliders, metadata, engine stats), bundling as APP assets
> only dist/pages 001–021, 440–445, 582, 604 (.qvp + .words.json) and atlas.qva — the package
> ships no data; (4) XCTest target with the same assertions as
> packages/flutter/qvp_flutter/test/qvp_flutter_test.dart (page 042: 147 words, search الله = 7,
> resolve 2:255 = 50 words, layout fill 690×1100 → scale 2, highlight tick, mask/reveal, crop,
> atlas pageOf(2,255) = 42, pagesOfJuz(30) = 582…604). Build the demo for the iOS simulator
> (`xcodebuild -scheme Demo -destination 'platform=iOS Simulator,name=iPhone 16'`), run it, take
> a screenshot with `xcrun simctl io booted screenshot`, look at it and fix what is broken.
> Then write packages/ios/README.md and update the packages table in README.md. Commit on a
> branch `ios-wrapper` and open a PR.

## Notes

- Pages 1–2 have 8 lines and a different viewBox; nothing in the engine hardcodes 15 or
  345×550, the layout centres short pages on the nominal grid.
- The engine's C ABI is the only contract; the Swift wrapper needs no Objective-C++.
- `docs/UPSTREAM-DATA-ISSUES.md` lists what the SVG exporter should fix; the engine does not
  patch data.
