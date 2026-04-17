import Foundation

final class TemporaryHomeHarness {

    let homeURL: URL

    var opencodeConfigURL: URL {
        homeURL.appendingPathComponent(".config/opencode")
    }

    var omoSwitchConfigURL: URL {
        homeURL.appendingPathComponent(".config/omo-switch")
    }

    private let originalHome: String?

    init() {
        self.originalHome = Environment.currentHome
        self.homeURL = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        try? FileManager.default.createDirectory(at: homeURL, withIntermediateDirectories: true)
        Environment.setHome(homeURL.path)
    }

    init(path: String) {
        self.originalHome = Environment.currentHome
        self.homeURL = URL(fileURLWithPath: path)
        try? FileManager.default.createDirectory(at: homeURL, withIntermediateDirectories: true)
        Environment.setHome(homeURL.path)
    }

    deinit {
        if let originalHome = originalHome {
            Environment.setHome(originalHome)
        }
        try? FileManager.default.removeItem(at: homeURL)
    }

    func setupOpencodeConfig() throws {
        try FileManager.default.createDirectory(at: opencodeConfigURL, withIntermediateDirectories: true)
    }

    func setupOmoSwitchConfig() throws {
        try FileManager.default.createDirectory(at: omoSwitchConfigURL, withIntermediateDirectories: true)
    }

    func installFixture(named fixtureName: String, subdirectory: String, to destinationName: String? = nil) throws {
        let fixtureURL = FixtureLoader.fixtureURL(named: fixtureName, subdirectory: subdirectory)
        let destName = destinationName ?? fixtureURL.lastPathComponent
        let destURL: URL
        if subdirectory == "opencode" {
            destURL = opencodeConfigURL.appendingPathComponent(destName)
        } else if subdirectory == "ohmy" {
            destURL = opencodeConfigURL.appendingPathComponent(destName)
        } else {
            destURL = homeURL.appendingPathComponent(destName)
        }
        try FileManager.default.createDirectory(at: destURL.deletingLastPathComponent(), withIntermediateDirectories: true)
        try FileManager.default.copyItem(at: fixtureURL, to: destURL)
    }

    func resolvedPath(relativeToHome path: String) -> String {
        homeURL.appendingPathComponent(path).path
    }
}

enum Environment {
    static var currentHome: String? {
        ProcessInfo.processInfo.environment["HOME"]
    }

    static func setHome(_ path: String) {
        setenv("HOME", path, 1)
    }
}
