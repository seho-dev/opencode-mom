import Foundation
import XCTest
@testable import OMOSwitch

final class OpenCodeDocumentTests: XCTestCase {

    func testParsesStandardJSONFixture() throws {
        let data = try FixtureLoader.fixtureData(
            named: "current-opencode.json",
            subdirectory: "opencode"
        )
        let result = OpenCodeDocument.parse(jsonData: data)

        guard case .success(let document) = result else {
            XCTFail("Expected successful parse of OpenCode fixture")
            return
        }

        XCTAssertEqual(document.rawDictionary["$schema"] as? String, "https://opencode.ai/config.json")
        XCTAssertTrue(document.agents.keys.contains("creative-ui-coder"))
        XCTAssertTrue(document.agents.keys.contains("Jenny"))
        XCTAssertTrue(document.agents.keys.contains("karen"))
        XCTAssertNil(document.rawDictionary["agents"])
        XCTAssertNotNil(document.rawDictionary["plugin"] as? [String])
        XCTAssertNotNil(document.rawDictionary["provider"] as? [String: Any])
    }

    func testParsesJSONCFixtureByStrippingComments() {
        let jsoncString = """
        {
          // Schema comment
          "$schema": "https://opencode.ai/config.json",
          "plugin": [
            "oh-my-openagent"
          ],
          /* Agent definitions */
          "agent": {
            "reviewer": {
              "model": "provider/model/ref",
              "description": "Reviews generated code",
              "mode": "subagent"
            }
          },
          "provider": {
            "cliproxyapi": {
              "name": "CLIProxyAPI"
            }
          }
        }
        """

        let result = OpenCodeDocument.parse(jsoncString: jsoncString)

        guard case .success(let document) = result else {
            XCTFail("Expected successful parse of OpenCode JSONC")
            return
        }

        XCTAssertEqual(Set(document.agents.keys), ["reviewer"])
        XCTAssertEqual(document.rawDictionary["$schema"] as? String, "https://opencode.ai/config.json")
        XCTAssertNotNil(document.rawDictionary["provider"] as? [String: Any])
    }

    func testRejectsMalformedInput() {
        let malformedJSON = "{ this is not valid JSON !!!".data(using: .utf8)!
        XCTAssertEqual(OpenCodeDocument.parse(jsonData: malformedJSON), .failure(.malformedJSON))

        let malformedJSONC = "{ // comment without closing brace"
        XCTAssertEqual(OpenCodeDocument.parse(jsoncString: malformedJSONC), .failure(.malformedJSON))
    }

    func testPreservesUnknownTopLevelFieldsDuringRoundTrip() throws {
        let original: [String: Any] = [
            "$schema": "https://opencode.ai/config.json",
            "plugin": ["oh-my-openagent", "custom-plugin"],
            "agent": [
                "karen": [
                    "model": "cliproxyapi/minimax-m2.7",
                    "description": "Assess completion state",
                    "mode": "subagent",
                ]
            ],
            "provider": [
                "cliproxyapi": [
                    "name": "CLIProxyAPI",
                    "options": ["baseURL": "https://example.invalid/v1"] as [String: Any],
                ]
            ],
            "futureKey": [
                "enabled": true,
                "threshold": 3,
            ] as [String: Any],
            "_customMarker": "preserve-me",
        ]

        let data = try JSONSerialization.data(withJSONObject: original, options: [.sortedKeys])
        let parseResult = OpenCodeDocument.parse(jsonData: data)
        guard case .success(let parsed) = parseResult else {
            XCTFail("Expected successful parse before round trip")
            return
        }

        let serialized = try XCTUnwrap(parsed.serialize())
        let roundTripResult = OpenCodeDocument.parse(jsonData: serialized)
        guard case .success(let roundTripped) = roundTripResult else {
            XCTFail("Expected successful parse after round trip")
            return
        }

        XCTAssertEqual(roundTripped.rawDictionary["$schema"] as? String, "https://opencode.ai/config.json")
        XCTAssertEqual(roundTripped.rawDictionary["plugin"] as? [String], ["oh-my-openagent", "custom-plugin"])
        XCTAssertEqual(roundTripped.rawDictionary["_customMarker"] as? String, "preserve-me")

        let futureKey = roundTripped.rawDictionary["futureKey"] as? [String: Any]
        XCTAssertEqual(futureKey?["enabled"] as? Bool, true)
        XCTAssertEqual(futureKey?["threshold"] as? Int, 3)

        let provider = roundTripped.rawDictionary["provider"] as? [String: Any]
        let cliproxyapi = provider?["cliproxyapi"] as? [String: Any]
        XCTAssertEqual(cliproxyapi?["name"] as? String, "CLIProxyAPI")
        XCTAssertNotNil(roundTripped.rawDictionary["agent"] as? [String: Any])
        XCTAssertNil(roundTripped.rawDictionary["agents"])
        XCTAssertTrue(roundTripped.agents.keys.contains("karen"))
    }

    func testSerializeUsesCanonicalAgentKeyWithoutEscapingForwardSlashes() throws {
        let document = OpenCodeDocument(rawDictionary: [
            "$schema": "https://opencode.ai/config.json",
            "agent": [
                "reviewer": ["model": "provider/model/ref"]
            ]
        ])

        let data = try XCTUnwrap(document.serialize())
        let jsonString = try XCTUnwrap(String(data: data, encoding: .utf8))

        XCTAssertTrue(jsonString.contains("\"agent\""))
        XCTAssertFalse(jsonString.contains("\"agents\""))
        XCTAssertTrue(jsonString.contains("provider/model/ref"))
        XCTAssertFalse(jsonString.contains("provider\\/model\\/ref"))
        FixtureAssertions.assertValidJSON(data)
    }
}
