import XCTest

final class FixtureCorpusTests: XCTestCase {

    func testCurrentOpencodeFixtureLoads() throws {
        let data = try FixtureLoader.fixtureData(named: "current-opencode.json", subdirectory: "opencode")
        FixtureAssertions.assertValidJSON(data)
        FixtureAssertions.assertNoLeakedSecrets(in: data)
        FixtureAssertions.assertHasKeys(data, keys: ["$schema", "plugin", "agent", "provider"])
    }

    func testCurrentOhMyOpenAgentFixtureLoads() throws {
        let data = try FixtureLoader.fixtureData(named: "current-oh-my-openagent.json", subdirectory: "ohmy")
        FixtureAssertions.assertValidJSON(data)
        FixtureAssertions.assertNoLeakedSecrets(in: data)
        FixtureAssertions.assertHasKeys(data, keys: ["$schema", "agents", "categories"])
    }

    func testLegacyOhMyOpenCodeBakFixtureLoads() throws {
        let data = try FixtureLoader.fixtureData(named: "legacy-oh-my-opencode.json.bak", subdirectory: "ohmy")
        FixtureAssertions.assertValidJSON(data)
        FixtureAssertions.assertNoLeakedSecrets(in: data)
        FixtureAssertions.assertHasKeys(data, keys: ["agents", "categories"])
    }

    func testWithUnknownFieldsFixtureLoads() throws {
        let data = try FixtureLoader.fixtureData(named: "with-unknown-fields.json", subdirectory: "json")
        FixtureAssertions.assertValidJSON(data)
        FixtureAssertions.assertNoLeakedSecrets(in: data)
        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertNotNil(json?["unknownField"])
        XCTAssertNotNil(json?["nestedUnknown"])
    }

    func testWithWeirdKeyOrderFixtureLoads() throws {
        let data = try FixtureLoader.fixtureData(named: "with-weird-key-order.json", subdirectory: "json")
        FixtureAssertions.assertValidJSON(data)
        FixtureAssertions.assertNoLeakedSecrets(in: data)
    }

    func testWithJsoncMarkersFixtureLoads() throws {
        let string = try FixtureLoader.fixtureString(named: "with-jsonc-markers.jsonc", subdirectory: "json")
        XCTAssertTrue(string.contains("//") || string.contains("/*"))
    }

    func testWithUndeclaredModelRefFixtureLoads() throws {
        let data = try FixtureLoader.fixtureData(named: "with-undeclared-model-ref.json", subdirectory: "json")
        FixtureAssertions.assertValidJSON(data)
        FixtureAssertions.assertNoLeakedSecrets(in: data)
        let json = try JSONSerialization.jsonObject(with: data) as? [String: Any]
        XCTAssertNotNil(json?["agents"])
        XCTAssertNotNil(json?["categories"])
    }

    func testWithConflictWriteBaselineFixtureLoads() throws {
        let data = try FixtureLoader.fixtureData(named: "with-conflict-write-baseline.json", subdirectory: "json")
        FixtureAssertions.assertValidJSON(data)
        FixtureAssertions.assertNoLeakedSecrets(in: data)
        FixtureAssertions.assertHasKeys(data, keys: ["schema", "plugin", "agent", "provider"])
    }

    func testAllExpectedFixturesExist() {
        let opencodeFixtures = FixtureLoader.listFixtures(in: "opencode")
        XCTAssertTrue(opencodeFixtures.contains("current-opencode.json"))

        let ohmyFixtures = FixtureLoader.listFixtures(in: "ohmy")
        XCTAssertTrue(ohmyFixtures.contains("current-oh-my-openagent.json"))
        XCTAssertTrue(ohmyFixtures.contains("legacy-oh-my-opencode.json.bak"))

        let jsonFixtures = FixtureLoader.listFixtures(in: "json")
        XCTAssertTrue(jsonFixtures.contains("with-unknown-fields.json"))
        XCTAssertTrue(jsonFixtures.contains("with-weird-key-order.json"))
        XCTAssertTrue(jsonFixtures.contains("with-jsonc-markers.jsonc"))
        XCTAssertTrue(jsonFixtures.contains("with-undeclared-model-ref.json"))
        XCTAssertTrue(jsonFixtures.contains("with-conflict-write-baseline.json"))
    }
}
