# Android agent notes

Cover the Kotlin SDK, and the Flutter and React Native packages, which build on it.

- Toolchain: JDK 17, Android SDK 35, NDK 27.2.12479018, `cargo-ndk`
  (`cargo install cargo-ndk`), `ANDROID_NDK_HOME` set. Flutter stable and Node 20+ for the
  other two packages.
- Build the engine first: `scripts/build-engine-android.sh` writes the static libs to
  `packages/android/qvp/prebuilt/<abi>/` and the shared libs to the Flutter plugin's
  `jniLibs`. Rebuild after any change under `crates/`. It also refreshes the header copy at
  `qvp/src/main/cpp/qvp.h`. Never edit that copy.
- AAR: `scripts/package-android.sh <version>` writes `dist/android/qvp-android-<version>.aar`.
- Demo: `cd packages/android && ./gradlew :example:installDebug`. Page data for the
  example: `scripts/sync-example-data.sh android`.
- Tests: `./gradlew :qvp:test` (unit) and `./gradlew :qvp:connectedAndroidTest` (device or
  emulator. Needs page data from `scripts/sync-android-test-data.sh`).
- Flutter: `cd packages/flutter/qvp_flutter && flutter analyze && flutter test`. Example
  with `cd example && flutter run`.
- React Native: `cd packages/react-native/example && npm install && npx react-native
  run-android`.
- The JNI bridge (`qvp_jni.c`) and the Dart bindings (`bindings.dart`) mirror the header's
  `#[repr(C)]` structs by hand. A struct change in the header means both files change in
  the same PR.
