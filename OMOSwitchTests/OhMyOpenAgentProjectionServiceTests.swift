import Foundation
import XCTest
@testable import OMOSwitch

final class OhMyOpenAgentProjectionServiceTests: XCTestCase {

    // MARK: - Replaces agents and categories from selected group

    func testReplacesAgentsAndCategoriesFromSelectedGroup() {
        let existingDoc = OhMyOpenAgentDocument(rawDictionary: [
            "$schema": "https://example.com/schema.json",
            "agents": [
                "old-agent": ["model": "old-model"] as [String: Any]
            ],
            "categories": [
                "old-category": ["model": "old-model"] as [String: Any]
            ],
        ])

        let group = ModelGroup(
            name: "TestGroup",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "ultrabrain", modelRef: "gpt-5.4"),
            ],
            agentOverrides: [
                ModelGroupAgentOverride(agentName: "oracle", modelRef: "gpt-5"),
            ]
        )

        let result = OhMyOpenAgentProjectionService.project(group: group, onto: existingDoc)

        let oracleEntry = result.document.agents["oracle"] as? [String: Any]
        XCTAssertNotNil(oracleEntry)
        XCTAssertEqual(oracleEntry?["model"] as? String, "gpt-5")

        let ultrabrainEntry = result.document.categories["ultrabrain"] as? [String: Any]
        XCTAssertNotNil(ultrabrainEntry)
        XCTAssertEqual(ultrabrainEntry?["model"] as? String, "gpt-5.4")

        XCTAssertNil(result.document.agents["old-agent"])
        XCTAssertNil(result.document.categories["old-category"])
    }

    // MARK: - Preserves schema and unknown top-level fields

    func testPreservesSchemaAndUnknownTopLevelFields() {
        let existingDoc = OhMyOpenAgentDocument(rawDictionary: [
            "$schema": "https://example.com/schema.json",
            "provider": ["name": "openai"] as [String: Any],
            "customField": "preserved",
            "agents": [:] as [String: Any],
            "categories": [:] as [String: Any],
        ])

        let group = ModelGroup(
            name: "TestGroup",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "brain", modelRef: "claude-4"),
            ],
            agentOverrides: [
                ModelGroupAgentOverride(agentName: "coder", modelRef: "claude-4"),
            ]
        )

        let result = OhMyOpenAgentProjectionService.project(group: group, onto: existingDoc)

        XCTAssertEqual(result.document.rawDictionary["$schema"] as? String, "https://example.com/schema.json")
        XCTAssertEqual(result.document.rawDictionary["customField"] as? String, "preserved")
        let provider = result.document.rawDictionary["provider"] as? [String: Any]
        XCTAssertNotNil(provider)
        XCTAssertEqual(provider?["name"] as? String, "openai")
    }

    // MARK: - Empty mappings produce empty sections

    func testEmptyMappingsProduceEmptySections() {
        let existingDoc = OhMyOpenAgentDocument(rawDictionary: [
            "$schema": "https://example.com/schema.json",
            "agents": [
                "old-agent": ["model": "old-model"] as [String: Any]
            ],
            "categories": [
                "old-category": ["model": "old-model"] as [String: Any]
            ],
        ])

        let group = ModelGroup(
            name: "EmptyGroup",
            categoryMappings: [],
            agentOverrides: []
        )

        let result = OhMyOpenAgentProjectionService.project(group: group, onto: existingDoc)

        XCTAssertTrue(result.document.agents.isEmpty)
        XCTAssertTrue(result.document.categories.isEmpty)
    }

    // MARK: - Projection reports no warnings for valid group

    func testProjectionReportsNoWarningsForValidGroup() {
        let existingDoc = OhMyOpenAgentDocument(rawDictionary: [
            "$schema": "https://example.com/schema.json",
            "agents": [:] as [String: Any],
            "categories": [:] as [String: Any],
        ])

        let group = ModelGroup(
            name: "ValidGroup",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "smart", modelRef: "gpt-5"),
            ],
            agentOverrides: [
                ModelGroupAgentOverride(agentName: "helper", modelRef: "gpt-5"),
            ]
        )

        let result = OhMyOpenAgentProjectionService.project(group: group, onto: existingDoc)

        XCTAssertTrue(result.warnings.isEmpty)
    }

    // MARK: - Skips entries with empty modelRef

    func testSkipsAgentOverridesWithEmptyModelRef() {
        let existingDoc = OhMyOpenAgentDocument(rawDictionary: [
            "$schema": "https://example.com/schema.json",
            "agents": [:] as [String: Any],
            "categories": [:] as [String: Any],
        ])

        let group = ModelGroup(
            name: "PartialGroup",
            categoryMappings: [],
            agentOverrides: [
                ModelGroupAgentOverride(agentName: "oracle", modelRef: "gpt-5"),
                ModelGroupAgentOverride(agentName: "hephaestus", modelRef: ""),
                ModelGroupAgentOverride(agentName: "librarian", modelRef: "  "),
            ]
        )

        let result = OhMyOpenAgentProjectionService.project(group: group, onto: existingDoc)

        let oracleEntry = result.document.agents["oracle"] as? [String: Any]
        XCTAssertNotNil(oracleEntry)
        XCTAssertEqual(oracleEntry?["model"] as? String, "gpt-5")
        XCTAssertNil(result.document.agents["hephaestus"])
        XCTAssertNil(result.document.agents["librarian"])
    }

    func testSkipsCategoryMappingsWithEmptyModelRef() {
        let existingDoc = OhMyOpenAgentDocument(rawDictionary: [
            "$schema": "https://example.com/schema.json",
            "agents": [:] as [String: Any],
            "categories": [:] as [String: Any],
        ])

        let group = ModelGroup(
            name: "PartialGroup",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "ultrabrain", modelRef: "gpt-5"),
                ModelGroupCategoryMapping(categoryName: "quick", modelRef: ""),
                ModelGroupCategoryMapping(categoryName: "deep", modelRef: "  "),
            ],
            agentOverrides: []
        )

        let result = OhMyOpenAgentProjectionService.project(group: group, onto: existingDoc)

        let ultrabrainEntry = result.document.categories["ultrabrain"] as? [String: Any]
        XCTAssertNotNil(ultrabrainEntry)
        XCTAssertEqual(ultrabrainEntry?["model"] as? String, "gpt-5")
        XCTAssertNil(result.document.categories["quick"])
        XCTAssertNil(result.document.categories["deep"])
    }

    func testEmptyModelRefDoesNotBreakNonEmptyEntries() {
        let existingDoc = OhMyOpenAgentDocument(rawDictionary: [
            "$schema": "https://example.com/schema.json",
            "agents": [:] as [String: Any],
            "categories": [:] as [String: Any],
        ])

        let group = ModelGroup(
            name: "MixedGroup",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "ultrabrain", modelRef: "claude-4"),
                ModelGroupCategoryMapping(categoryName: "quick", modelRef: ""),
            ],
            agentOverrides: [
                ModelGroupAgentOverride(agentName: "oracle", modelRef: "gpt-5"),
                ModelGroupAgentOverride(agentName: "hephaestus", modelRef: ""),
            ]
        )

        let result = OhMyOpenAgentProjectionService.project(group: group, onto: existingDoc)

        XCTAssertEqual(result.document.agents.count, 1)
        XCTAssertEqual(result.document.categories.count, 1)
    }
}
