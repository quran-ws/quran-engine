# QvpExample — React Native demo of the QVP mushaf engine, on Android and iOS

One screen (`App.tsx`) reproducing the web / Android demos on top of
`@quran.ws/qvp-react-native` (`../qvp-react-native`, linked as a `file:` dependency):
page prev/next + goto (atlas: `2:255`, `Yasin`, `juz 30`), search with highlighted results, tap word →
selection panel (rasm_uthmani, wordKey, forms, per-path chips colouring one mark via `Sel.wordMark`), tap
marker → ayah, long-press-drag selection with copy + citation, highlight mode + fade slider, follow-words
timer (moveHighlight through the `highlights` prop), mark colours / hide marks / gold markers toggles
(`theme` / `styles` props), light / sepia / dark, memorisation (mask ayah, reveal next, hide back, unmask,
greyed page with slider), layout sliders (line spacing, pad top/bottom, fill height, leading-to-fill),
page metadata and engine stats, and a dress panel: a mushaf preset, a picker for each kind, line
art per kind, gap, band gap, medallion and border size, a colour well for each part, "As printed",
and the engine's readout with each ornament's licence.

## Data

The package ships no page data. The example copies a subset from `dist/pages` (built by
`cargo run -p qvp-convert --release -- batch pages dist/pages` at the repo root) into
`android/app/src/main/assets/pages/` and `ios/QvpExample/pages/` (both gitignored): pages 001–021, 440–445, 582, 604 (`.qvp` +
`.words.json`), `atlas.qva` and `ornaments.qvo` when the release has one.
`scripts/sync-example-data.sh react-native` copies them to both. The iOS project bundles
`ios/QvpExample/pages` as a folder, so `asset://pages/001.qvp` names the same file on both
platforms. The dress panel works without the ornament set.

```sh
mkdir -p android/app/src/main/assets/pages
for i in $(seq 1 21) 440 441 442 443 444 445 582 604; do n=$(printf %03d $i); cp ../../../dist/pages/$n.qvp ../../../dist/pages/$n.words.json android/app/src/main/assets/pages/; done
cp ../../../dist/pages/atlas.qva android/app/src/main/assets/pages/
cp ../../../dist/pages/ornaments.qvo android/app/src/main/assets/pages/   # if present
```

## Build (Android)

`android/settings.gradle` includes the Kotlin library as `:qvp` (`../../../android/qvp`); its
prebuilt engine comes from `packages/android/qvp/prebuilt/<abi>/libqvp_ffi.a`
(`scripts/build-engine-android.sh`). JS is bundled into every variant (`debuggableVariants = []`),
so the debug APK runs standalone without Metro.

```sh
npm install
cd android && ./gradlew :app:assembleDebug -PreactNativeArchitectures=x86_64,arm64-v8a
adb install -r app/build/outputs/apk/debug/app-debug.apk
adb shell am start -n ws.quran.qvp_example/.MainActivity
```

For live-reload development run `npm start` and build with `debuggableVariants = ["debug"]` again.
`metro.config.js` adds `../qvp-react-native` to `watchFolders` and resolves `react` / `react-native`
from this app's `node_modules`.

## Build (iOS)

The pod `qvp-react-native` compiles QvpKit's sources from `packages/ios/QvpKit` and links the
engine from `packages/ios/QvpKit/QvpEngine.xcframework`. Build the engine once with
`scripts/build-engine-ios.sh` from the repository root, and again after a change under
`crates/`. Run `pod install` again after a change to QvpKit's sources. The example needs
macOS, Xcode 16 and CocoaPods.

```sh
npm install
cd ios && pod install
xcodebuild -workspace QvpExample.xcworkspace -scheme QvpExample -configuration Release \
  -sdk iphonesimulator -destination 'generic/platform=iOS Simulator' \
  -derivedDataPath build CODE_SIGNING_ALLOWED=NO build
xcrun simctl install booted build/Build/Products/Release-iphonesimulator/QvpExample.app
xcrun simctl launch booted org.reactjs.native.example.QvpExample
```

The Release build bundles the JavaScript, so the app runs without Metro. A Debug build loads
the JavaScript from Metro (`npm start`). `metro.config.js` leaves
`../qvp-react-native/node_modules` out of the bundle. That folder has its own copy of React
after an `npm install` there, and two copies of React break every hook.

## Tests

`npm test` runs Jest. `jest/qvpModule.js` stands in for the native module, so the tests check
what the JavaScript side sends and reads back. `__tests__/dress.test.tsx` covers the dress calls.
