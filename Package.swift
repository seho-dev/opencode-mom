// swift-tools-version: 6.3
import PackageDescription

let package = Package(
    name: "OMOSwitch",
    platforms: [.macOS(.v13)],
    products: [
        .executable(name: "omo-switch", targets: ["OMOSwitch"]),
    ],
    targets: [
        .executableTarget(
            name: "OMOSwitch",
            path: "OMOSwitch"
        ),
        .testTarget(
            name: "OMOSwitchTests",
            dependencies: ["OMOSwitch"],
            path: "OMOSwitchTests",
            resources: [
                .copy("Fixtures")
            ]
        ),
    ],
    swiftLanguageModes: [.v6]
)
