import Foundation

enum TestSupport {
    static func makeTemporaryDirectory(named name: String = UUID().uuidString) throws -> URL {
        let directory = FileManager.default.temporaryDirectory.appending(path: "omo-switch-tests", directoryHint: .isDirectory).appending(path: name, directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: directory, withIntermediateDirectories: true)
        return directory
    }

    static func removeIfExists(_ url: URL) {
        try? FileManager.default.removeItem(at: url)
    }
}
