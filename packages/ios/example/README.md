# iOS example

A SwiftUI reader built on `QvpKit`. It implements `docs/EXAMPLE-APP.md`.

```sh
scripts/build-engine-ios.sh          # QvpEngine.xcframework
cd packages/ios/example
./sync-pages.sh                      # bundles the page data (also a pre-build step)
xcodegen generate && open Demo.xcodeproj
```

`sync-pages.sh` also copies the letter layers (`NNN.qvl`) from `$QVP_LETTERS`, else from
`dist/letters`, when they are there. A layer attaches only to the page file it was built
with. Without a layer that attaches, the letter controls in Settings are disabled.

The three calls the app is built on, in `Sources/DemoModel.swift`:

```swift
let page = try QvpPage(bytes: data)                 // load
canvas.page = page                                  // render: QvpPageCanvas draws it
let hit = page.hitTestView(x, y)                  // tap: the word under the finger
```
