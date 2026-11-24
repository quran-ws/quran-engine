// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "QvpKit",
    platforms: [.iOS(.v15), .macOS(.v12)],
    products: [
        .library(name: "QvpKit", targets: ["QvpKit"]),
    ],
    targets: [
        // Built by .github/workflows/prepare-ios-release.yml from commit 4c3f49877e78d5b94d4fffe428d3e20d816ca481.
        .binaryTarget(
            name: "QvpEngine",
            url: "https://github.com/quran-ws/quran-engine/releases/download/v0.11.0/QvpEngine.xcframework.zip",
            checksum: "b4cfaf767b6359f51597c376c41ee0844e45ae5536a6cdeb6390efafdb555ac8"
        ),
        .target(
            name: "QvpKit",
            dependencies: ["QvpEngine"],
            path: "packages/ios/QvpKit/Sources/QvpKit"
        ),
    ]
)
