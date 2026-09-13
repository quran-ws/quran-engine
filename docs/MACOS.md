# Running and extending on macOS

Everything here also builds on macOS. The iOS wrapper is the one piece that needs a Mac.

## Setup (once)

```sh
git clone git@github.com:quran-ws/quran-engine.git && cd quran-engine
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
cp target/wasm32-unknown-unknown/release/qvp_ffi.wasm web/
python3 web/example/build.py dev && (cd dist/web && python3 -m http.server 8765)   # http://127.0.0.1:8765/?p=42
```

## Android (Kotlin, Flutter, React Native demos) on a Mac

Install Android Studio (SDK 35, NDK 27.2.12479018, an x86_64 or arm64 AVD), JDK 17,
Flutter stable, Node 20+. Then:

```sh
cargo install cargo-ndk
export ANDROID_NDK_HOME=$HOME/Library/Android/sdk/ndk/27.2.12479018
scripts/build-engine-android.sh
(cd packages/android && ./gradlew :example:installDebug)
(cd packages/flutter/qvp_flutter/example && flutter run)
(cd packages/react-native/example && npm install && npx react-native run-android)
```

## iOS (Swift package + demo)

```sh
scripts/build-engine-ios.sh                          # QvpEngine.xcframework (device, simulator, macOS)
(cd packages/ios/QvpKit && swift test)               # XCTests on the macOS slice
(cd packages/ios/example && xcodebuild -scheme Demo -destination 'platform=iOS Simulator,name=iPhone 17' build)
```

See `packages/ios/QvpKit/README.md`. Pick any installed simulator name (`xcrun simctl list devices available`).

## Notes

- Pages 1–2 have 8 lines and a different viewBox; nothing in the engine hardcodes 15 or
  345×550, the layout centres short pages on the nominal grid.
- The engine's C ABI is the only contract; the Swift wrapper needs no Objective-C++.
- `docs/UPSTREAM-DATA-ISSUES.md` lists what the SVG exporter should fix; the engine does not
  patch data.
