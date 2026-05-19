import XCTest
@testable import OMOSwitch

final class ServerConfigViewTests: XCTestCase {
  func testDraftDefaultsMatchMissingServerConfigDefaults() {
    let draft = ServerConfigDraft()

    XCTAssertEqual(draft.portText, "4096")
    XCTAssertEqual(draft.hostname, "127.0.0.1")
    XCTAssertFalse(draft.mdns)
    XCTAssertEqual(draft.mdnsDomain, "opencode.local")
    XCTAssertEqual(draft.corsText, "")
    XCTAssertEqual(draft.executablePath, "")
    XCTAssertFalse(draft.autoStart)
  }

  func testDraftDisplaysExistingConfigFields() {
    let config = OpenCodeServeConfig(
      port: 5173,
      hostname: "0.0.0.0",
      mdns: true,
      mdnsDomain: "dev.local",
      cors: ["http://localhost:5173", "https://app.example.com"],
      executablePath: "/opt/homebrew/bin/opencode",
      autoStart: true
    )

    let draft = ServerConfigDraft(config: config)

    XCTAssertEqual(draft.portText, "5173")
    XCTAssertEqual(draft.hostname, "0.0.0.0")
    XCTAssertTrue(draft.mdns)
    XCTAssertEqual(draft.mdnsDomain, "dev.local")
    XCTAssertEqual(draft.corsText, "http://localhost:5173\nhttps://app.example.com")
    XCTAssertEqual(draft.executablePath, "/opt/homebrew/bin/opencode")
    XCTAssertTrue(draft.autoStart)
  }

  func testValidatedConfigTrimsFieldsAndDropsEmptyCORSOriginLines() throws {
    let draft = ServerConfigDraft(
      portText: " 5173 ",
      hostname: " 0.0.0.0 ",
      mdns: true,
      mdnsDomain: " dev.local ",
      corsText: " http://localhost:5173 \n\n https://app.example.com \n   ",
      executablePath: " /opt/homebrew/bin/opencode ",
      autoStart: true
    )

    let config = try draft.validatedConfig()

    XCTAssertEqual(config.port, 5173)
    XCTAssertEqual(config.hostname, "0.0.0.0")
    XCTAssertTrue(config.mdns)
    XCTAssertEqual(config.mdnsDomain, "dev.local")
    XCTAssertEqual(config.cors, ["http://localhost:5173", "https://app.example.com"])
    XCTAssertEqual(config.executablePath, "/opt/homebrew/bin/opencode")
    XCTAssertTrue(config.autoStart)
  }

  func testValidatedConfigTreatsBlankExecutablePathAsAutomatic() throws {
    let draft = ServerConfigDraft(portText: "4096", hostname: "127.0.0.1", mdns: false, mdnsDomain: "opencode.local", corsText: "", executablePath: "   ", autoStart: false)

    let config = try draft.validatedConfig()

    XCTAssertNil(config.executablePath)
  }

  func testInvalidPortDoesNotBuildConfig() {
    let draft = ServerConfigDraft(portText: "0", hostname: "127.0.0.1", mdns: false, mdnsDomain: "opencode.local", corsText: "", executablePath: "", autoStart: false)

    XCTAssertThrowsError(try draft.validatedConfig()) { error in
      XCTAssertEqual(error as? OpenCodeServeValidationError, .portOutOfRange(0))
    }
  }

  func testEmptyHostnameDoesNotBuildConfig() {
    let draft = ServerConfigDraft(portText: "4096", hostname: "   ", mdns: false, mdnsDomain: "opencode.local", corsText: "", executablePath: "", autoStart: false)

    XCTAssertThrowsError(try draft.validatedConfig()) { error in
      XCTAssertEqual(error as? OpenCodeServeValidationError, .emptyHostname)
    }
  }

  func testEmptyMDNSDomainDoesNotBuildConfigWhenMDNSEnabled() {
    let draft = ServerConfigDraft(portText: "4096", hostname: "127.0.0.1", mdns: true, mdnsDomain: "   ", corsText: "", executablePath: "", autoStart: false)

    XCTAssertThrowsError(try draft.validatedConfig()) { error in
      XCTAssertEqual(error as? OpenCodeServeValidationError, .emptyMDNSDomain)
    }
  }
}
