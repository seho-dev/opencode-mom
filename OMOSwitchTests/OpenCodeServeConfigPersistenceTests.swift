import Foundation
import XCTest
@testable import OMOSwitch

final class OpenCodeServeConfigPersistenceTests: XCTestCase {
    func testLegacyStateWithoutServeConfigDecodesDefaults() throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()

        let legacyPayload = #"{"migrationVersion":2,"selectedGroupID":"4F648C2A-1D48-44D6-B0C0-4D340816E1F3","selectedGroupName":"Primary","launchAtLoginEnabled":true}"#
        try legacyPayload.write(to: stateFileURL(in: harness), atomically: true, encoding: .utf8)

        let repository = makeRepository(in: harness)
        let state = try repository.load()

        XCTAssertEqual(state.openCodeServeConfig, OpenCodeServeConfig())
        XCTAssertEqual(state.openCodeServeConfig.port, 4096)
        XCTAssertEqual(state.openCodeServeConfig.hostname, "127.0.0.1")
        XCTAssertFalse(state.openCodeServeConfig.mdns)
        XCTAssertEqual(state.openCodeServeConfig.mdnsDomain, "opencode.local")
        XCTAssertEqual(state.openCodeServeConfig.cors, [])
        XCTAssertFalse(state.openCodeServeConfig.autoStart)
        XCTAssertEqual(state.selectedGroupName, "Primary")
        XCTAssertTrue(state.launchAtLoginEnabled)
    }

    func testSaveReloadRoundTripPreservesServeConfigFields() throws {
        let harness = TemporaryHomeHarness()
        let repository = makeRepository(in: harness)
        let serveConfig = OpenCodeServeConfig(
            port: 5173,
            hostname: "0.0.0.0",
            mdns: true,
            mdnsDomain: "myproject.local",
            cors: ["http://localhost:5173", "https://app.example.com"],
            executablePath: "/opt/homebrew/bin/opencode",
            autoStart: true
        )
        let state = AppSelectionState(
            selectedGroupName: "Primary",
            launchAtLoginEnabled: true,
            openCodeServeConfig: serveConfig
        )

        try repository.save(state)
        let reloaded = try repository.load()

        XCTAssertEqual(reloaded, state)
        XCTAssertEqual(reloaded.openCodeServeConfig.port, 5173)
        XCTAssertEqual(reloaded.openCodeServeConfig.hostname, "0.0.0.0")
        XCTAssertTrue(reloaded.openCodeServeConfig.mdns)
        XCTAssertEqual(reloaded.openCodeServeConfig.mdnsDomain, "myproject.local")
        XCTAssertEqual(reloaded.openCodeServeConfig.cors, ["http://localhost:5173", "https://app.example.com"])
        XCTAssertEqual(reloaded.openCodeServeConfig.executablePath, "/opt/homebrew/bin/opencode")
        XCTAssertTrue(reloaded.openCodeServeConfig.autoStart)
    }

    func testSerializedServeConfigExcludesRuntimeOnlyFields() throws {
        let harness = TemporaryHomeHarness()
        let repository = makeRepository(in: harness)
        try repository.save(AppSelectionState(openCodeServeConfig: OpenCodeServeConfig(autoStart: true)))

        let payload = try String(contentsOf: stateFileURL(in: harness), encoding: .utf8)
        XCTAssertTrue(payload.contains("openCodeServeConfig"))
        XCTAssertTrue(payload.contains("autoStart"))
        XCTAssertFalse(payload.contains("status"))
        XCTAssertFalse(payload.contains("pid"))
        XCTAssertFalse(payload.contains("processID"))
        XCTAssertFalse(payload.contains("launchResult"))
        XCTAssertFalse(payload.contains("stderr"))
        XCTAssertFalse(payload.contains("window"))
    }

    private func makeRepository(in harness: TemporaryHomeHarness) -> AppStateRepository {
        AppStateRepository(configRootURL: harness.omoSwitchConfigURL)
    }

    private func stateFileURL(in harness: TemporaryHomeHarness) -> URL {
        harness.omoSwitchConfigURL.appending(path: "state.json")
    }
}
