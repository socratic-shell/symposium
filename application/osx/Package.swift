// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "Symposium",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "Symposium", targets: ["Symposium"])
    ],
    targets: [
        .executableTarget(
            name: "Symposium",
            dependencies: [],
            resources: [
                .copy("../Info.plist")
            ]
        )
    ]
)
