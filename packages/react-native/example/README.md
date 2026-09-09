# QvpExample — React Native demo of the QVP mushaf engine

One screen (`App.tsx`) reproducing the web / Android demos on top of
`@quranpedia/qvp-react-native` (`../qvp-react-native`, linked as a `file:` dependency):
page prev/next + goto (atlas: `2:255`, `Yasin`, `juz 30`), search with highlighted results, tap word →
selection panel (rasm_uthmani, wordKey, forms, per-path chips colouring one mark via `Sel.wordMark`), tap
marker → ayah, long-press-drag selection with copy + citation, highlight mode + fade slider, follow-words
timer (rehighlight through the `highlights` prop), mark colours / hide marks / gold markers toggles
(`theme` / `styles` props), light / sepia / dark, memorisation (mask ayah, reveal next, hide back, unmask,
greyed page with slider), layout sliders (line spacing, pad top/bottom, fill height, leading-to-fill),
page metadata and engine stats.

## Data

The package ships no page data. The example copies a subset from `dist/pages` (built by
`cargo run -p qvp-convert --release -- batch pages dist/pages` at the repo root) into
`android/app/src/main/assets/pages/` (gitignored): pages 001–021, 440–445, 582, 604 (`.qvp` +
`.words.json`) and `atlas.qva`.

```sh
mkdir -p android/app/src/main/assets/pages
for i in $(seq 1 21) 440 441 442 443 444 445 582 604; do n=$(printf %03d $i); cp ../../../dist/pages/$n.qvp ../../../dist/pages/$n.words.json android/app/src/main/assets/pages/; done
cp ../../../dist/pages/atlas.qva android/app/src/main/assets/pages/
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
adb shell am start -n com.qvpexample/.MainActivity
```

For live-reload development run `npm start` and build with `debuggableVariants = ["debug"]` again.
`metro.config.js` adds `../qvp-react-native` to `watchFolders` and resolves `react` / `react-native`
from this app's `node_modules`.

iOS is not set up (see the library README).
