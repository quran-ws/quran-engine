// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "QvpKit",
    platforms: [.iOS(.v15), .macOS(.v12)],
    products: [
        .library(name: "QvpKit", targets: ["QvpKit"]),
    ],
    targets: [
        .binaryTarget(
            name: "QvpEngine",
            url: "https://github.com/quran-ws/quran-engine/releases/download/v0.3.1/QvpEngine.xcframework.zip",
            checksum: "898172f2f3bd1c169bcdb022b2380f9e741b1317ce82d80f0538705ecede5ae7"
        ),
        .target(
            name: "QvpKit",
            dependencies: ["QvpEngine"],
            path: "packages/ios/QvpKit/Sources/QvpKit"
        ),
    ]
)
