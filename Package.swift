// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "QvpKit",
    platforms: [.iOS(.v15), .macOS(.v12)],
    products: [
        .library(name: "QvpKit", targets: ["QvpKit"]),
    ],
    targets: [
        // Built by .github/workflows/prepare-ios-release.yml from commit ee176d937f7405cbc5aac993966f18aa0163c3b4.
        .binaryTarget(
            name: "QvpEngine",
            url: "https://github.com/quran-ws/quran-engine/releases/download/v0.11.0/QvpEngine.xcframework.zip",
            checksum: "58a0201d1a2249ddbb4c752bdf24dd84cdc7bc2d06306eb525e52cc429a58a2e"
        ),
        .target(
            name: "QvpKit",
            dependencies: ["QvpEngine"],
            path: "packages/ios/QvpKit/Sources/QvpKit"
        ),
    ]
)
