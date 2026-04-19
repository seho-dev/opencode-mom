import Foundation
import XCTest
@testable import OMOSwitch

final class OhMyOpenAgentDocumentTests: XCTestCase {

    // MARK: - Parse Standard JSON

    func testParsesStandardJSONFixture() throws {
        let data = try FixtureLoader.fixtureData(
            named: "current-oh-my-openagent.json",
            subdirectory: "ohmy"
        )
        let result = OhMyOpenAgentDocument.parse(jsonData: data)

        switch result {
        case .success(let doc):
            XCTAssertFalse(doc.agents.isEmpty)
            XCTAssertFalse(doc.categories.isEmpty)
            XCTAssertEqual(
                doc.rawDictionary["$schema"] as? String,
                "https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/dev/assets/oh-my-opencode.schema.json"
            )
        case .failure:
            XCTFail("Expected successful parse of standard JSON fixture")
        }
    }

    // MARK: - Parse JSONC

    func testParsesJSONCFixtureByStrippingComments() throws {
        let jsoncString = try FixtureLoader.fixtureString(
            named: "with-jsonc-markers.jsonc",
            subdirectory: "json"
        )
        let result = OhMyOpenAgentDocument.parse(jsoncString: jsoncString)

        switch result {
        case .success(let doc):
            XCTAssertFalse(doc.agents.isEmpty)
            XCTAssertFalse(doc.categories.isEmpty)
            let provider = doc.rawDictionary["provider"] as? [String: Any]
            XCTAssertNotNil(provider)
        case .failure:
            XCTFail("Expected successful parse of JSONC fixture")
        }
    }

    // MARK: - Round-trip preserves unknown fields

    func testPreservesUnknownTopLevelFieldsDuringRoundTrip() throws {
        let data = try FixtureLoader.fixtureData(
            named: "with-unknown-fields.json",
            subdirectory: "json"
        )
        let parseResult = OhMyOpenAgentDocument.parse(jsonData: data)
        guard case .success(let parsed) = parseResult else {
            XCTFail("Expected successful parse of unknown fields fixture")
            return
        }
        guard let serialized = parsed.serialize() else {
            XCTFail("Expected successful serialization")
            return
        }
        let roundTripResult = OhMyOpenAgentDocument.parse(jsonData: serialized)
        guard case .success(let roundTripped) = roundTripResult else {
            XCTFail("Expected successful round-trip parse")
            return
        }

        XCTAssertEqual(roundTripped.rawDictionary["knownField"] as? String, "value")
        XCTAssertEqual(roundTripped.rawDictionary["unknownField"] as? String, "should be preserved")
        XCTAssertEqual(roundTripped.rawDictionary["_customMarker"] as? String, "保留的未知字段")

        let someExtraData = roundTripped.rawDictionary["someExtraData"] as? [String: Any]
        XCTAssertNotNil(someExtraData)
        XCTAssertEqual(someExtraData?["key"] as? String, "value")
    }

    // MARK: - Reject malformed input

    func testRejectsMalformedJSONInput() {
        let badData = "{ this is not valid JSON !!!".data(using: .utf8)!
        let result = OhMyOpenAgentDocument.parse(jsonData: badData)
        XCTAssertEqual(result, .failure(.malformedJSON))

        let badJSONC = "{ // comment without closing brace"
        let jsoncResult = OhMyOpenAgentDocument.parse(jsoncString: badJSONC)
        XCTAssertEqual(jsoncResult, .failure(.malformedJSON))
    }

    // MARK: - Bootstrap

    func testBootstrapDocumentContainsSchemaAndEmptySections() {
        let doc = OhMyOpenAgentDocument.bootstrap()
        XCTAssertEqual(
            doc.rawDictionary["$schema"] as? String,
            "https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/dev/assets/oh-my-opencode.schema.json"
        )
        XCTAssertTrue(doc.agents.isEmpty)
        XCTAssertTrue(doc.categories.isEmpty)
    }

    // MARK: - Serialize canonical form

    func testSerializeProducesCanonicalJSON() throws {
        let doc = OhMyOpenAgentDocument.bootstrap()
        let data = try XCTUnwrap(doc.serialize())
        let jsonString = try XCTUnwrap(String(data: data, encoding: .utf8))

        XCTAssertTrue(jsonString.contains("\"$schema\""))
        XCTAssertTrue(jsonString.contains("\"agents\""))
        XCTAssertTrue(jsonString.contains("\"categories\""))
        FixtureAssertions.assertValidJSON(data)
    }

    // MARK: - Round-trip equivalence

    func testRoundTripParseSerializeParseProducesEquivalentDocument() throws {
        let data = try FixtureLoader.fixtureData(
            named: "current-oh-my-openagent.json",
            subdirectory: "ohmy"
        )
        let firstResult = OhMyOpenAgentDocument.parse(jsonData: data)
        guard case .success(let first) = firstResult else {
            XCTFail("Expected successful first parse")
            return
        }
        guard let serialized = first.serialize() else {
            XCTFail("Expected successful serialization")
            return
        }
        let secondResult = OhMyOpenAgentDocument.parse(jsonData: serialized)
        guard case .success(let second) = secondResult else {
            XCTFail("Expected successful second parse")
            return
        }

        XCTAssertEqual(
            first.rawDictionary["$schema"] as? String,
            second.rawDictionary["$schema"] as? String
        )
        XCTAssertEqual(
            Set(first.agents.keys),
            Set(second.agents.keys)
        )
        XCTAssertEqual(
            Set(first.categories.keys),
            Set(second.categories.keys)
        )
    }

    // MARK: - JSONCStripper unit tests

    func testStripCommentsRemovesSingleLineComments() {
        let input = """
        {
          "key": "value" // trailing
        }
        """
        let stripped = JSONCStripper.stripComments(input)
        XCTAssertFalse(stripped.contains("//"))
        XCTAssertTrue(stripped.contains("\"key\": \"value\""))
    }

    func testStripCommentsRemovesMultiLineComments() {
        let input = """
        {
          /* removed */
          "key": "value"
        }
        """
        let stripped = JSONCStripper.stripComments(input)
        XCTAssertFalse(stripped.contains("/*"))
        XCTAssertFalse(stripped.contains("removed"))
        XCTAssertTrue(stripped.contains("\"key\": \"value\""))
    }

    func testStripCommentsPreservesCommentLikeContentInStrings() {
        let input = """
        {
          "url": "https://example.com/path",
          "code": "a // b",
          "block": "/* not a comment */"
        }
        """
        let stripped = JSONCStripper.stripComments(input)
        XCTAssertTrue(stripped.contains("https://example.com/path"))
        XCTAssertTrue(stripped.contains("a // b"))
        XCTAssertTrue(stripped.contains("/* not a comment */"))
    }
}
