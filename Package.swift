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
            url: "https://github.com/quran-ws/quran-engine/releases/download/v0.6.1/QvpEngine.xcframework.zip",
            checksum: "fc7244aac337f2743c42c002f4aebe5a6a5c7481e9ab06830a679b9ea2c8ae61"
        ),
        .target(
            name: "QvpKit",
            dependencies: ["QvpEngine"],
            path: "packages/ios/QvpKit/Sources/QvpKit"
        ),
    ]
)
