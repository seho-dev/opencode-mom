import Foundation
import XCTest
@testable import OMOSwitch

final class OpenCodeProjectionServiceTests: XCTestCase {

    func testSelectivelyUpdatesExistingAgentModelAndPreservesOtherFields() throws {
        let fixtureDocument = try loadFixtureDocument()
        let originalAgents = fixtureDocument.agents
        let originalKaren = try XCTUnwrap(originalAgents["karen"] as? [String: Any])
        let originalJenny = try XCTUnwrap(originalAgents["Jenny"] as? [String: Any])

        var rawDictionary = fixtureDocument.rawDictionary
        rawDictionary["futureOpenCodeKey"] = [
            "enabled": true,
            "label": "preserve-me",
        ] as [String: Any]
        let document = OpenCodeDocument(rawDictionary: rawDictionary)
        let group = ModelGroup(
            name: "OpenCodeGroup",
            categoryMappings: [],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(
                    agentName: "karen",
                    modelRef: "  cliproxyapi/gpt-5.4-xhigh  "
                ),
            ]
        )

        let result = OpenCodeProjectionService.project(group: group, onto: document)

        let projectedAgents = result.document.agents
        let projectedKaren = try XCTUnwrap(projectedAgents["karen"] as? [String: Any])
        let projectedJenny = try XCTUnwrap(projectedAgents["Jenny"] as? [String: Any])

        XCTAssertTrue(result.warnings.isEmpty)
        XCTAssertEqual(projectedAgents.count, originalAgents.count)
        XCTAssertEqual(projectedKaren["model"] as? String, "cliproxyapi/gpt-5.4-xhigh")
        assertNonModelFieldsEqual(projectedKaren, originalKaren)
        XCTAssertEqual(projectedJenny["model"] as? String, originalJenny["model"] as? String)
        XCTAssertNil(result.document.rawDictionary["agents"])
        XCTAssertNotNil(result.document.rawDictionary["agent"] as? [String: Any])

        XCTAssertEqual(result.document.rawDictionary["$schema"] as? String, document.rawDictionary["$schema"] as? String)
        XCTAssertEqual(result.document.rawDictionary["plugin"] as? [String], document.rawDictionary["plugin"] as? [String])
        assertDictionariesEqual(
            try XCTUnwrap(result.document.rawDictionary["provider"] as? [String: Any]),
            try XCTUnwrap(document.rawDictionary["provider"] as? [String: Any])
        )
        assertDictionariesEqual(
            try XCTUnwrap(result.document.rawDictionary["futureOpenCodeKey"] as? [String: Any]),
            try XCTUnwrap(document.rawDictionary["futureOpenCodeKey"] as? [String: Any])
        )
    }

    func testSkipsEmptyModelRefsWithoutChangingExistingAgents() throws {
        let document = try loadFixtureDocument()
        let originalCreativeUICoder = try XCTUnwrap(document.agents["creative-ui-coder"] as? [String: Any])
        let originalComplianceChecker = try XCTUnwrap(document.agents["claude-md-compliance-checker"] as? [String: Any])
        let group = ModelGroup(
            name: "OpenCodeGroup",
            categoryMappings: [],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "creative-ui-coder", modelRef: ""),
                ModelGroupAgentOverride(agentName: "claude-md-compliance-checker", modelRef: " \n\t "),
            ]
        )

        let result = OpenCodeProjectionService.project(group: group, onto: document)

        let projectedCreativeUICoder = try XCTUnwrap(result.document.agents["creative-ui-coder"] as? [String: Any])
        let projectedComplianceChecker = try XCTUnwrap(result.document.agents["claude-md-compliance-checker"] as? [String: Any])
        XCTAssertTrue(result.warnings.isEmpty)
        XCTAssertEqual(projectedCreativeUICoder["model"] as? String, originalCreativeUICoder["model"] as? String)
        XCTAssertEqual(projectedComplianceChecker["model"] as? String, originalComplianceChecker["model"] as? String)
    }

    func testUnknownAgentOverrideWarnsAndDoesNotCreateEntry() throws {
        let document = try loadFixtureDocument()
        let originalAgents = document.agents
        let group = ModelGroup(
            name: "OpenCodeGroup",
            categoryMappings: [],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "stale-agent", modelRef: "cliproxyapi/gpt-5.4"),
            ]
        )

        let result = OpenCodeProjectionService.project(group: group, onto: document)

        XCTAssertEqual(result.warnings.count, 1)
        XCTAssertTrue(result.warnings[0].contains("stale-agent"))
        XCTAssertNil(result.document.agents["stale-agent"])
        XCTAssertEqual(result.document.agents.count, originalAgents.count)
    }

    func testMissingTopLevelAgentLeavesDocumentUnchangedAndWarns() {
        let document = OpenCodeDocument(rawDictionary: [
            "$schema": "https://opencode.ai/config.json",
            "provider": ["cliproxyapi": ["name": "CLIProxyAPI"]],
        ] as [String: Any])
        let group = ModelGroup(
            name: "MissingAgent",
            categoryMappings: [],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "karen", modelRef: "cliproxyapi/gpt-5.4"),
                ModelGroupAgentOverride(agentName: "empty", modelRef: "   "),
            ]
        )

        let result = OpenCodeProjectionService.project(group: group, onto: document)

        XCTAssertEqual(result.document, document)
        XCTAssertNil(result.document.rawDictionary["agent"])
        XCTAssertEqual(result.warnings.count, 1)
        XCTAssertTrue(result.warnings[0].contains("no valid top-level 'agent' object"))
        XCTAssertTrue(result.warnings[0].contains("karen"))
    }

    func testNonDictionaryTopLevelAgentLeavesDocumentUnchangedAndWarns() {
        let document = OpenCodeDocument(rawDictionary: [
            "$schema": "https://opencode.ai/config.json",
            "agent": "invalid-agent-shape",
            "plugin": ["oh-my-openagent"],
        ] as [String: Any])
        let group = ModelGroup(
            name: "InvalidAgent",
            categoryMappings: [],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "karen", modelRef: "cliproxyapi/gpt-5.4"),
            ]
        )

        let result = OpenCodeProjectionService.project(group: group, onto: document)

        XCTAssertEqual(result.document, document)
        XCTAssertEqual(result.document.rawDictionary["agent"] as? String, "invalid-agent-shape")
        XCTAssertEqual(result.warnings.count, 1)
        XCTAssertTrue(result.warnings[0].contains("no valid top-level 'agent' object"))
        XCTAssertTrue(result.warnings[0].contains("karen"))
    }

    private func loadFixtureDocument(
        file: StaticString = #filePath,
        line: UInt = #line
    ) throws -> OpenCodeDocument {
        let data = try FixtureLoader.fixtureData(
            named: "current-opencode.json",
            subdirectory: "opencode"
        )
        switch OpenCodeDocument.parse(jsonData: data) {
        case .success(let document):
            return document
        case .failure:
            XCTFail("Expected successful parse of OpenCode fixture", file: file, line: line)
            throw FixtureLoadError.parseFailed
        }
    }

    private func assertNonModelFieldsEqual(
        _ actual: [String: Any],
        _ expected: [String: Any],
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        var actualWithoutModel = actual
        var expectedWithoutModel = expected
        actualWithoutModel.removeValue(forKey: "model")
        expectedWithoutModel.removeValue(forKey: "model")
        assertDictionariesEqual(actualWithoutModel, expectedWithoutModel, file: file, line: line)
    }

    private func assertDictionariesEqual(
        _ actual: [String: Any],
        _ expected: [String: Any],
        file: StaticString = #filePath,
        line: UInt = #line
    ) {
        XCTAssertTrue(
            NSDictionary(dictionary: actual).isEqual(NSDictionary(dictionary: expected)),
            file: file,
            line: line
        )
    }
}

private enum FixtureLoadError: Error {
    case parseFailed
}
