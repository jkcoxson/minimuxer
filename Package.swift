// swift-tools-version: 6.0
// The swift-tools-version declares the minimum version of Swift required to build this package.

import PackageDescription

let package = Package(
    name: "minimuxer",
    products: [
        .library(
            name: "minimuxer",
            targets: ["minimuxer"]),
    ],
    targets: [
        .target(
            name: "minimuxer",
            dependencies: ["libminimuxer"]),
        
        .target(
            name: "libminimuxer",
            dependencies: ["minimuxer-binary"]
        ),

        .binaryTarget(
            name: "minimuxer-binary",
            url: "https://github.com/SideStore/minimuxer/releases/download/build/minimuxer.xcframework.zip",
            checksum: "0c3d526007e93e1570451473303f9d01f43d03847a986e12932de69d21cfe21a"),
    
        .testTarget(
            name: "minimuxerTests",
            dependencies: ["minimuxer"]
        ),
    ],
    swiftLanguageModes: [.v5]
)
