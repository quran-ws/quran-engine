# iOS example

A SwiftUI reader built on `QvpKit`. It implements `docs/EXAMPLE-APP.md`.

```sh
scripts/build-engine-ios.sh          # QvpEngine.xcframework
cd packages/ios/example
./sync-pages.sh                      # bundles the page data (also a pre-build step)
xcodegen generate && open Demo.xcodeproj
```

The three calls the app is built on, in `Sources/DemoModel.swift`:

```swift
let page = try QvpPage(bytes: data)                 // load
canvas.page = page                                  // render: QvpPageCanvas draws it
let hit = page.hitTestViewEx(x, y)                  // tap: the word under the finger
```
