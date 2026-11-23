# Android example

A Kotlin reader built on the `qvp` module. It implements `docs/EXAMPLE-APP.md`.

```sh
scripts/build-engine-android.sh            # native libraries
scripts/sync-example-data.sh android       # the 29-page set into src/main/assets/pages
cd packages/android && ./gradlew :example:installDebug
```

The three calls the app is built on, in `src/main/kotlin/ws/quran/qvp/demo/MainActivity.kt`:

```kotlin
val page = QvpPage(bytes)                          // load
view.page = page                                   // render: QvpPageView draws it
val hit = page.hitTestView(x, y)                 // tap: the word under the finger
```

The Letters section colours one letter or one mark per tap. It needs the page's letter
layer, `NNN.qvl`, which the sync script copies when `dist/letters/` has it. A layer attaches
only to its own page file. Without a layer, the app disables the controls.
