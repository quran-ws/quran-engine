# iOS agent notes

- Build the engine first: `scripts/build-engine-ios.sh` writes
  `packages/ios/QvpKit/QvpEngine.xcframework` (device, simulator, macOS). Rebuild it after
  any change under `crates/`. A stale slice makes the Swift tests fail on core behaviour.
- Test on the macOS slice: `cd packages/ios/QvpKit && swift test`.
- xcodegen generates the demo project: `cd packages/ios/example && xcodegen generate` (Homebrew
  `xcodegen`). Git does not track `Demo.xcodeproj`. Page data: `example/sync-pages.sh`.
- Simulator: `xcodebuild -scheme Demo -destination 'platform=iOS Simulator,name=iPhone 17'`.
  Pick any name from `xcrun simctl list devices available`.
- UI checks: the `DemoUITests` target, or the demo's `-qvpPage`, `-qvpGoto`, `-qvpSearch`,
  `-qvpAyah`, `-qvpWord`, `-qvpTheme` launch arguments. Simulator taps cannot be scripted
  from AppleScript.
- Every controller operation checks `page.isOpen` before calling the engine. A closed page
  traps in the engine. The wrapper must not let that happen.
- Rust toolchain: `rustup` with `aarch64-apple-ios`, `aarch64-apple-ios-sim`,
  `x86_64-apple-ios`. `cargo` is under `~/.cargo/bin`.
