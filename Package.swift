// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "QvpKit",
    platforms: [.iOS(.v15), .macOS(.v12)],
    products: [
        .library(name: "QvpKit", targets: ["QvpKit"]),
    ],
    targets: [
        // Built by .github/workflows/prepare-ios-release.yml from commit 91f643cc88e253a104e114c5236e24aa5ae60b11.
        .binaryTarget(
            name: "QvpEngine",
            url: "https://github.com/quran-ws/quran-engine/releases/download/v0.10.0/QvpEngine.xcframework.zip",
            checksum: "9a19c0c68b40c0e0586e3d5a285344e782ac9e5fae2bc76b0a17b0cb5edfbd95"
        ),
        .target(
            name: "QvpKit",
            dependencies: ["QvpEngine"],
            path: "packages/ios/QvpKit/Sources/QvpKit"
        ),
    ]
)
