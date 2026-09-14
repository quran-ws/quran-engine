# QVP Android

`qvp` is the Kotlin/JNI SDK for loading original `.qvp` pages and drawing them with Android
Canvas. The AAR contains the Rust engine for `arm64-v8a`, `armeabi-v7a`, and `x86_64`; applications
supply their own page data. Its 64-bit libraries support 16 KiB memory-page devices. The minimum
Android API is 24.

## Install

Releases are available from Maven Central:

```kotlin
dependencies {
    implementation("ws.quran:qvp-android:0.2.1")
}
```

Each code release also includes `qvp-android-X.Y.Z.aar` and its SHA-256 checksum. To use the
release artifact directly, verify the download, copy the AAR into the application's `libs/`
directory, and add it as a file dependency:

```kotlin
dependencies {
    implementation(files("libs/qvp-android-0.2.1.aar"))
}
```

## Build an AAR

Install JDK 17, Android SDK 35, NDK 27.2.12479018, Rust, and `cargo-ndk`, then run:

```sh
scripts/package-android.sh 0.2.1
```

The versioned AAR and its checksum are written to `dist/android/`. To publish the same component
to Maven Local for development:

```sh
scripts/build-engine-android.sh
cd packages/android
./gradlew :qvp:publishToMavenLocal -PqvpVersion=0.2.1
```

Or consume a Maven-local build:

```kotlin
repositories { mavenLocal(); google(); mavenCentral() }
dependencies { implementation("ws.quran:qvp-android:0.2.1") }
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

`QvpAndroidBenchmarkTest` reports `QvpBenchmark` JSON containing median page-load, Android Path
construction, first-draw, and cached-draw times. It deliberately has no duration threshold because
emulators and physical devices have different performance. Compare results on the same device and
build type when evaluating a renderer change.
