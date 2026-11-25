# QVP Android

`qvp` is the Kotlin/JNI SDK for loading original `.qvp` pages and drawing them with Android
Canvas. The AAR contains the Rust engine for `arm64-v8a`, `armeabi-v7a`, and `x86_64`; applications
supply their own page data. Its 64-bit libraries support 16 KiB memory-page devices. The minimum
Android API is 24.

## Install

Releases are available from Maven Central:

```kotlin
dependencies {
    implementation("ws.quran:qvp-android:0.11.0")
}
```

Each code release also includes `qvp-android-X.Y.Z.aar` and its SHA-256 checksum. To use the
release artifact directly, verify the download, copy the AAR into the application's `libs/`
directory, and add it as a file dependency:

```kotlin
dependencies {
    implementation(files("libs/qvp-android-0.11.0.aar"))
}
```

## Build an AAR

Install JDK 17, Android SDK 35, NDK 27.2.12479018, Rust, and `cargo-ndk`, then run:

```sh
scripts/package-android.sh 0.11.0
```

The versioned AAR and its checksum are written to `dist/android/`. To publish the same component
to Maven Local for development:

```sh
scripts/build-engine-android.sh
cd packages/android
./gradlew :qvp:publishToMavenLocal
```

The build file's default `qvpVersion` is the workspace version, which `scripts/set-version.sh`
stamps. Pass `-PqvpVersion=X.Y.Z` to publish under another number.

Or consume a Maven-local build:

```kotlin
repositories { mavenLocal(); google(); mavenCentral() }
dependencies { implementation("ws.quran:qvp-android:0.11.0") }
```

The AAR's consumer rules preserve the name-based JNI bridge when the application enables R8.

## Load and draw a page

```kotlin
val page = assets.open("pages/001.qvp").use { QvpPage(it.readBytes()) }
val view = QvpPageView(context).apply {
    this.page = page
    onWordTap = { word, _ -> println(word.wordKey) }
}

// The caller owns the native page. Clear the view reference, then close it.
view.page = null
page.close()
```

`QvpPage` and `QvpAtlas` are `AutoCloseable`. Closing either object is idempotent; engine operations
after close throw `IllegalStateException`. Keep application-specific Compose surfaces,
accessibility nodes, navigation, and state outside the SDK and use `QvpPage.buildPaths()`,
`targetWords()`, `cropBounds()`, `hitAreas()`, and the style/mask APIs as their data layer.

## Lay out a passage

A passage is a range of complete ayahs shown away from its page: a card, a tooltip, a list
row. The engine lays it out; the pages can close once it is loaded.

```kotlin
val passage = QvpPassage.load(listOf(page), surah = 2, from = 255)
val layout = passage.layout(QvpPassageSpec(width = 366f, scale = 22f / passage.lineSpacing,
                                           maxRows = 2, keepAyahMark = true, ellipsisWidth = 12f))
layout?.draw(canvas, mapOf(42 to page), paint)       // every path under its placement
// the letters the page colours, in their colours: the last layout's draws as the page styles them
layout?.draw(canvas, mapOf(42 to page), paint, draws = passage.styledDrawList(listOf(page)), pageColors = true)
val basmalah = QvpPassage.basmalah(listOf(page), surah = 2)   // one row, no words; throws for surahs 1 and 9
val mark = QvpPassage.ayahMark(listOf(page), surah = 2, ayah = 255)   // the medallion alone, for omitAyahMarks = true
passage.dress(set, spec)   // the page's QvpDressSpec: its medallion on every ring; lay out again, and draw paints layout.ornaments behind the ink
passage.close()
```

## Dress a page in another mushaf's ornaments

An ornament set (`ornaments.qvo`, from the data release) holds the medallions, bands and
borders of other mushafs and fonts. The engine places each one. `QvpPageView` paints them
behind the ink and fits the border on screen.

```kotlin
val set = QvpOrnaments(assets.open("pages/ornaments.qvo").readBytes())
val info = page.dress(set, QvpDressSpec(style = "mushaf-qalon", gap = 5f))   // null when refused
val marker = set.find(QvpOrnamentKind.AYAH_MARK, "font-003-regular")
page.dress(set, QvpDressSpec(style = "mushaf-qalon", ayahMark = marker))       // a kind overrides the style
for (o in set.list) println("${o.style} ${o.kind}: ${o.licence.id}, ${o.licence.status}")
page.undress()
```

Each ornament carries its licence. Read it before you publish a dressed page.

## Colour and tap single letters

A letter layer (`NNN.qvl`) names the letters of each word and the paths that draw them. Where
two letters share one outline, the layer cuts the outline into pieces. Attach the layer, then
address a letter, its marks or a range of characters.

```kotlin
if (!page.attachLetters(assets.open("pages/003.qvl").readBytes())) error("not this page's layer")
// the pieces come after the page's own paths: nPaths grew, and no index below it changed
val lam = page.letters(4).first { it.text == "ل" }   // text is the character, codePoint the number
page.style(Selector.letter(4, lam.index), QvpColor.parse("#c0392b"))
page.style(Selector.letterMarkNamed(4, lam.index, "shaddah"), QvpColor.parse("#1a73e8"))
page.style(Selector.wordChars(4, 0, 2), QvpColor.parse("#2e7d32"))           // qpc offsets 0 to 2
page.styleLetters(QvpLetterQuery.of(text = "ن", mark = "shaddah"), QvpColor.parse("#8e44ad"))
val hit = page.hitTestLetterView(x, y)       // the letter or mark under a tap, or null: mark and markName
val same = view.letterAt(x, y)               // the same from a point of the view, through pan and zoom
```

`QvpPageView` draws a page with a layer with no extra work. A cut path draws as its pieces
while its letters differ in colour. A host that draws the page itself reads, each frame:
`tick`, then `colors()` or `styledPaths()`, then `layoutDrawRevision()`, then
`layoutDrawList()` when the revision, the layout or the band changed.

After a successful `attachLetters`, the arrays that you got from the page before it are stale:
`ops`, `pts`, `table`, `colors()`, the paths from `buildPaths()` and every array that you sized
from `nPaths`. Read them again.

A query's `text` is one character, or "" for any letter. A text of more than one character
throws an `IllegalArgumentException`. `QvpLetterQuery(codePoint = …)` takes the number. A
letter's `marks` are its members that are not body, in the engine's order. A member and a hit
give the mark as its id (`mark`) and its name (`markName`).

## Verify and benchmark

Unit tests do not need page data:

```sh
cd packages/android
./gradlew :qvp:testDebugUnitTest
```

Instrumentation uses a checksummed page from the data release:

```sh
scripts/sync-android-test-data.sh
cd packages/android
./gradlew :qvp:connectedDebugAndroidTest
```

The script also copies the letter scenarios (`conformance/scenarios/letters.json`) and the
synthetic pages and letter layers that they load. `QvpLettersConformanceTest` replays them.

`QvpAndroidBenchmarkTest` reports `QvpBenchmark` JSON containing median page-load, Android Path
construction, first-draw, and cached-draw times. It deliberately has no duration threshold because
emulators and physical devices have different performance. Compare results on the same device and
build type when evaluating a renderer change.
