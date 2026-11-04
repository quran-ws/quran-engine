// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "QvpKit",
    platforms: [.iOS(.v15), .macOS(.v12)],
    products: [
        .library(name: "QvpKit", targets: ["QvpKit"]),
    ],
    targets: [
        // Built by .github/workflows/prepare-ios-release.yml from commit 8808361743ca53c81b48501847c7f1f4aa618fef.
        .binaryTarget(
            name: "QvpEngine",
            url: "https://github.com/quran-ws/quran-engine/releases/download/v0.8.0/QvpEngine.xcframework.zip",
            checksum: "454b42710fde5a5752ba72866eedef9b58092e8e13a67a6708d958ac7eec0a5a"
        ),
        .target(
            name: "QvpKit",
            dependencies: ["QvpEngine"],
            path: "packages/ios/QvpKit/Sources/QvpKit"
        ),
    ]
)
