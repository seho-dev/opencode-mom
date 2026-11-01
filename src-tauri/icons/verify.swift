import AppKit
import Foundation

let directory = URL(fileURLWithPath: CommandLine.arguments.dropFirst().first ?? "src-tauri/icons")
let files = try FileManager.default.contentsOfDirectory(at: directory, includingPropertiesForKeys: nil)
let sizes = ["32x32": 32, "64x64": 64, "128x128": 128, "128x128@2x": 256, "icon": 512, "StoreLogo": 50]

for file in files.filter({ $0.pathExtension == "png" }).sorted(by: { $0.lastPathComponent < $1.lastPathComponent }) {
    let name = file.deletingPathExtension().lastPathComponent
    let image = NSBitmapImageRep(data: try Data(contentsOf: file))!
    let size = image.pixelsWide
    let tray = name.hasPrefix("tray-")
    let expected = tray ? (name.hasSuffix("-16") ? 16 : name.hasSuffix("@2x") ? 64 : 32)
        : sizes[name] ?? Int(name.dropFirst(6).prefix(while: { $0.isNumber }))!
    precondition(size == expected && image.pixelsHigh == expected && image.hasAlpha, "Invalid dimensions/alpha: \(name)")
    func pixel(_ x: Int, _ y: Int) -> NSColor {
        image.colorAt(x: x, y: y)!.usingColorSpace(.deviceRGB)!
    }
    for (x, y) in [(0, 0), (size - 1, 0), (0, size - 1), (size - 1, size - 1)] {
        precondition(pixel(x, y).alphaComponent == 0, "Opaque corner: \(name)")
    }
    var opaque = 0
    var transparent = 0
    var dark = 0
    var cyan = 0
    for y in 0..<size {
        for x in 0..<size {
            let color = pixel(x, y)
            if color.alphaComponent == 0 { transparent += 1; continue }
            if name.hasPrefix("tray-macos") {
                precondition(color.redComponent == 0 && color.greenComponent == 0 && color.blueComponent == 0, "Non-black template: \(name)")
            }
            if color.alphaComponent > 0.99 {
                opaque += 1
                if color.redComponent < 0.1 && color.greenComponent < 0.15 && color.blueComponent < 0.15 { dark += 1 }
                if color.redComponent < 0.05 && color.greenComponent > 0.85 && color.blueComponent > 0.95 { cyan += 1 }
            }
        }
    }
    precondition(transparent > 0 && opaque > size * size / 8, "Empty or opaque image: \(name)")
    precondition(dark > 0 && (name.hasPrefix("tray-macos") || cyan > 0), "Missing M/palette: \(name)")
    if tray {
        precondition(pixel(size / 2, size / 8).alphaComponent == 0, "M counter is filled: \(name)")
        precondition(pixel(size / 2, size * 7 / 8).alphaComponent == 0, "Tray has a tile background: \(name)")
    }
    print("PASS \(file.lastPathComponent): \(size)×\(size), transparent=\(transparent), opaque=\(opaque)")
}

for name in ["icon.ico", "icon.icns"] {
    let image = NSImage(contentsOf: directory.appendingPathComponent(name))!
    precondition(image.isValid && !image.representations.isEmpty, "Unreadable bundle icon: \(name)")
    print("PASS \(name): AppKit decoded \(image.representations.count) representations")
}
