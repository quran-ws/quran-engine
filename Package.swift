// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "QvpKit",
    platforms: [.iOS(.v15), .macOS(.v12)],
    products: [
        .library(name: "QvpKit", targets: ["QvpKit"]),
    ],
    targets: [
        // Built by .github/workflows/prepare-ios-release.yml from commit 19e7fe90b043a763fd5ac99bf286e0e4f64019f9.
        .binaryTarget(
            name: "QvpEngine",
            url: "https://github.com/quran-ws/quran-engine/releases/download/v0.9.1/QvpEngine.xcframework.zip",
            checksum: "77b7d5ad6eba4fd1a92f30c747e0325b877ca016fed67726ca7450b86d68f79b"
        ),
        .target(
            name: "QvpKit",
            dependencies: ["QvpEngine"],
            path: "packages/ios/QvpKit/Sources/QvpKit"
        ),
    ]
)
