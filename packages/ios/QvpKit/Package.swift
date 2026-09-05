// swift-tools-version:5.9
// QvpKit — Swift wrapper over the QVP engine C ABI (crates/qvp-ffi/include/qvp.h).
// The engine itself is `QvpEngine.xcframework`, produced by scripts/build-engine-ios.sh
// (device arm64, simulator arm64+x86_64, macOS arm64 so `swift test` runs on the Mac).
// The package ships no page data: apps load NNN.qvp / atlas.qva / NNN.words.json themselves.
import PackageDescription

let package = Package(
    name: "QvpKit",
    platforms: [.iOS(.v15), .macOS(.v12)],
    products: [
        .library(name: "QvpKit", targets: ["QvpKit"]),
    ],
    targets: [
        .binaryTarget(name: "QvpEngine", path: "QvpEngine.xcframework"),
        .target(
            name: "QvpKit",
            dependencies: ["QvpEngine"],
            path: "Sources/QvpKit"
        ),
        .testTarget(
            name: "QvpKitTests",
            dependencies: ["QvpKit"],
            path: "Tests/QvpKitTests"
        ),
    ]
)
