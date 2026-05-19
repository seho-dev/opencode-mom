import Foundation
import XCTest
@testable import OMOSwitch

final class OpenCodeServeArgumentBuilderTests: XCTestCase {
    func testDefaultConfigBuildsExactServeArguments() throws {
        let arguments = try OpenCodeServeArgumentBuilder().arguments(for: OpenCodeServeConfig())

        XCTAssertEqual(arguments, ["serve", "--port", "4096", "--hostname", "127.0.0.1"])
    }

    func testManualExecutablePathIsStoredOnConfigButNotServeArguments() throws {
        let config = try OpenCodeServeArgumentBuilder.config(
            portText: "4096",
            hostname: "127.0.0.1",
            mdns: false,
            mdnsDomain: "opencode.local",
            corsText: "",
            executablePath: " /opt/homebrew/bin/opencode ",
            autoStart: false
        )

        XCTAssertEqual(config.executablePath, "/opt/homebrew/bin/opencode")
        XCTAssertEqual(try OpenCodeServeArgumentBuilder().arguments(for: config), ["serve", "--port", "4096", "--hostname", "127.0.0.1"])
    }

    func testBlankManualExecutablePathBecomesNil() throws {
        let config = try OpenCodeServeArgumentBuilder.config(
            portText: "4096",
            hostname: "127.0.0.1",
            mdns: false,
            mdnsDomain: "opencode.local",
            corsText: "",
            executablePath: "   ",
            autoStart: false
        )

        XCTAssertNil(config.executablePath)
    }

    func testFullConfigBuildsExactServeArguments() throws {
        let config = OpenCodeServeConfig(
            port: 5000,
            hostname: "0.0.0.0",
            mdns: true,
            mdnsDomain: "dev.local",
            cors: ["https://a.example", "https://b.example"]
        )

        let arguments = try OpenCodeServeArgumentBuilder().arguments(for: config)

        XCTAssertEqual(
            arguments,
            [
                "serve",
                "--port",
                "5000",
                "--hostname",
                "0.0.0.0",
                "--mdns",
                "--mdns-domain",
                "dev.local",
                "--cors",
                "https://a.example",
                "--cors",
                "https://b.example",
            ]
        )
    }

    func testMDNSDomainIsOmittedWhenMDNSIsDisabled() throws {
        let config = OpenCodeServeConfig(
            port: 4096,
            hostname: "127.0.0.1",
            mdns: false,
            mdnsDomain: "dev.local"
        )

        let arguments = try OpenCodeServeArgumentBuilder().arguments(for: config)

        XCTAssertFalse(arguments.contains("--mdns"))
        XCTAssertFalse(arguments.contains("--mdns-domain"))
        XCTAssertFalse(arguments.contains("dev.local"))
    }

    func testParseCORSTextAreaTrimsDropsEmptyLinesPreservesOrderAndDuplicates() {
        let text = "  https://a.example  \n\nhttps://b.example\n https://a.example \n   \n"

        let origins = OpenCodeServeArgumentBuilder.parseCORSTextArea(text)

        XCTAssertEqual(origins, ["https://a.example", "https://b.example", "https://a.example"])
    }

    func testConfigCORSArgumentsPreserveExistingValuesWithoutNormalization() throws {
        let config = OpenCodeServeConfig(
            cors: [" https://a.example ", "", "https://a.example"]
        )

        let arguments = try OpenCodeServeArgumentBuilder().arguments(for: config)

        XCTAssertEqual(
            arguments,
            [
                "serve",
                "--port",
                "4096",
                "--hostname",
                "127.0.0.1",
                "--cors",
                " https://a.example ",
                "--cors",
                "",
                "--cors",
                "https://a.example",
            ]
        )
    }

    func testConfigFromUITextTrimsFieldsAndParsesCORS() throws {
        let config = try OpenCodeServeArgumentBuilder.config(
            portText: " 5000 ",
            hostname: " 0.0.0.0 ",
            mdns: true,
            mdnsDomain: " dev.local ",
            corsText: " https://a.example \n\n https://b.example ",
            executablePath: " /usr/local/bin/opencode ",
            autoStart: true
        )

        XCTAssertEqual(config.port, 5000)
        XCTAssertEqual(config.hostname, "0.0.0.0")
        XCTAssertTrue(config.mdns)
        XCTAssertEqual(config.mdnsDomain, "dev.local")
        XCTAssertEqual(config.cors, ["https://a.example", "https://b.example"])
        XCTAssertEqual(config.executablePath, "/usr/local/bin/opencode")
        XCTAssertTrue(config.autoStart)
    }

    func testRejectsInvalidPortTextInputs() {
        XCTAssertThrowsError(try makeConfig(portText: "")) { error in
            XCTAssertEqual(error as? OpenCodeServeValidationError, .emptyPort)
        }
        XCTAssertThrowsError(try makeConfig(portText: "   ")) { error in
            XCTAssertEqual(error as? OpenCodeServeValidationError, .emptyPort)
        }
        XCTAssertThrowsError(try makeConfig(portText: "abc")) { error in
            XCTAssertEqual(error as? OpenCodeServeValidationError, .nonNumericPort("abc"))
        }
        XCTAssertThrowsError(try makeConfig(portText: "-1")) { error in
            XCTAssertEqual(error as? OpenCodeServeValidationError, .portOutOfRange(-1))
        }
        XCTAssertThrowsError(try makeConfig(portText: "0")) { error in
            XCTAssertEqual(error as? OpenCodeServeValidationError, .portOutOfRange(0))
        }
        XCTAssertThrowsError(try makeConfig(portText: "65536")) { error in
            XCTAssertEqual(error as? OpenCodeServeValidationError, .portOutOfRange(65_536))
        }
    }

    func testRejectsInvalidConfigValues() {
        XCTAssertThrowsError(
            try OpenCodeServeArgumentBuilder().arguments(for: OpenCodeServeConfig(port: 0))
        ) { error in
            XCTAssertEqual(error as? OpenCodeServeValidationError, .portOutOfRange(0))
        }

        XCTAssertThrowsError(
            try OpenCodeServeArgumentBuilder().arguments(for: OpenCodeServeConfig(hostname: "   "))
        ) { error in
            XCTAssertEqual(error as? OpenCodeServeValidationError, .emptyHostname)
        }

        XCTAssertThrowsError(
            try OpenCodeServeArgumentBuilder().arguments(for: OpenCodeServeConfig(mdns: true, mdnsDomain: "   "))
        ) { error in
            XCTAssertEqual(error as? OpenCodeServeValidationError, .emptyMDNSDomain)
        }
    }

    func testAllowsEmptyMDNSDomainWhenMDNSIsDisabled() throws {
        let config = OpenCodeServeConfig(mdns: false, mdnsDomain: "   ")

        let arguments = try OpenCodeServeArgumentBuilder().arguments(for: config)

        XCTAssertEqual(arguments, ["serve", "--port", "4096", "--hostname", "127.0.0.1"])
    }

    private func makeConfig(portText: String) throws -> OpenCodeServeConfig {
        try OpenCodeServeArgumentBuilder.config(
            portText: portText,
            hostname: "127.0.0.1",
            mdns: false,
            mdnsDomain: "opencode.local",
            corsText: "",
            executablePath: ""
        )
    }
}
