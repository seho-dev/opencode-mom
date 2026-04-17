import XCTest

final class TemporaryHomeHarnessTests: XCTestCase {

    func testHarnessCreatesTempHomeDirectory() {
        let harness = TemporaryHomeHarness()
        XCTAssertTrue(FileManager.default.fileExists(atPath: harness.homeURL.path))
    }

    func testHarnessSetsHomeEnvironment() {
        let harness = TemporaryHomeHarness()
        XCTAssertEqual(Environment.currentHome, harness.homeURL.path)
    }

    func testHarnessRestoresOriginalHomeOnDeinit() {
        let originalHome = Environment.currentHome
        var harness: TemporaryHomeHarness? = TemporaryHomeHarness()
        XCTAssertEqual(Environment.currentHome, harness?.homeURL.path)
        harness = nil
        XCTAssertEqual(Environment.currentHome, originalHome)
    }

    func testHarnessWithPathCreatesDirectory() {
        let tempDir = FileManager.default.temporaryDirectory.appendingPathComponent(UUID().uuidString)
        let harness = TemporaryHomeHarness(path: tempDir.path)
        XCTAssertTrue(FileManager.default.fileExists(atPath: harness.homeURL.path))
        try? FileManager.default.removeItem(at: tempDir)
    }

    func testOpencodeConfigURLResolvesInsideHome() {
        let harness = TemporaryHomeHarness()
        let expectedPath = harness.homeURL.appendingPathComponent(".config/opencode").path
        XCTAssertEqual(harness.opencodeConfigURL.path, expectedPath)
    }

    func testOmoSwitchConfigURLResolvesInsideHome() {
        let harness = TemporaryHomeHarness()
        let expectedPath = harness.homeURL.appendingPathComponent(".config/omo-switch").path
        XCTAssertEqual(harness.omoSwitchConfigURL.path, expectedPath)
    }

    func testSetupOpencodeConfigCreatesDirectory() throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOpencodeConfig()
        XCTAssertTrue(FileManager.default.fileExists(atPath: harness.opencodeConfigURL.path))
    }

    func testSetupOmoSwitchConfigCreatesDirectory() throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()
        XCTAssertTrue(FileManager.default.fileExists(atPath: harness.omoSwitchConfigURL.path))
    }

    func testInstallFixtureCopiesToOpencodeDir() throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOpencodeConfig()
        try harness.installFixture(named: "current-opencode.json", subdirectory: "opencode")
        let destURL = harness.opencodeConfigURL.appendingPathComponent("current-opencode.json")
        XCTAssertTrue(FileManager.default.fileExists(atPath: destURL.path))
    }

    func testResolvedPathJoinsWithHome() {
        let harness = TemporaryHomeHarness()
        let resolved = harness.resolvedPath(relativeToHome: ".config/opencode/opencode.json")
        XCTAssertTrue(resolved.hasPrefix(harness.homeURL.path))
        XCTAssertTrue(resolved.hasSuffix(".config/opencode/opencode.json"))
    }

    func testConfigPathsNeverUseRealHome() {
        let harness = TemporaryHomeHarness()
        let realHome = FileManager.default.homeDirectoryForCurrentUser.path
        XCTAssertNotEqual(harness.homeURL.path, realHome)
        XCTAssertNotEqual(harness.opencodeConfigURL.path, "\(realHome)/.config/opencode")
        XCTAssertNotEqual(harness.omoSwitchConfigURL.path, "\(realHome)/.config/omo-switch")
    }
}
