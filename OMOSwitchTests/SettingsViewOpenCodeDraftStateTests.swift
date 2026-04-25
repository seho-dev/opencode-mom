import Foundation
import XCTest
@testable import OMOSwitch

@MainActor
final class SettingsViewOpenCodeDraftStateTests: XCTestCase {
  func testOpenCodeSectionCountsUseDiscoveredNamesAndFilledDiscoveredOverridesOnly() {
    let counts = SettingsView.openCodeAgentOverrideSectionCounts(
      overrides: [
        ModelGroupAgentOverride(agentName: "alpha", modelRef: " openai/gpt-5.4 "),
        ModelGroupAgentOverride(agentName: "beta", modelRef: "   "),
        ModelGroupAgentOverride(agentName: "stale", modelRef: "openai/o3"),
      ],
      discoveredAgentNames: ["alpha", "beta"]
    )

    XCTAssertEqual(counts.filled, 1)
    XCTAssertEqual(counts.total, 2)
  }

  func testPersistedDraftGroupWritesDraftOpenCodeAgentOverrides() {
    let draftGroup = makeGroup(openCodeAgentOverrides: [
      ModelGroupAgentOverride(agentName: "old", modelRef: "old/model")
    ])
    let expectedOpenCodeOverrides = [
      ModelGroupAgentOverride(agentName: "alpha", modelRef: "openai/gpt-5.4"),
      ModelGroupAgentOverride(agentName: "stale", modelRef: "openai/o3"),
    ]
    let persistedAt = Date(timeIntervalSince1970: 1_800_000_000)

    let persisted = SettingsView.persistedDraftGroup(
      draftGroup: draftGroup,
      draftCategoryMappings: [ModelGroupCategoryMapping(categoryName: "quick", modelRef: "openai/gpt-5.4-mini")],
      draftAgentOverrides: [ModelGroupAgentOverride(agentName: "general", modelRef: "openai/gpt-5.4")],
      draftOpenCodeAgentOverrides: expectedOpenCodeOverrides,
      updatedAt: persistedAt
    )

    XCTAssertEqual(persisted.name, "Primary")
    XCTAssertNil(persisted.description)
    XCTAssertEqual(persisted.openCodeAgentOverrides, expectedOpenCodeOverrides)
    XCTAssertEqual(persisted.updatedAt, persistedAt)
  }

  func testDiscoveryErrorDoesNotClearOpenCodeOverridesForLoadCancelOrSave() {
    let expectedOpenCodeOverrides = [
      ModelGroupAgentOverride(agentName: "stale", modelRef: "openai/o3"),
      ModelGroupAgentOverride(agentName: "alpha", modelRef: "openai/gpt-5.4"),
    ]
    let group = makeGroup(openCodeAgentOverrides: expectedOpenCodeOverrides)

    let retainedForLoad = SettingsView.retainedOpenCodeAgentOverrides(
      from: group,
      discoveredAgentNames: [],
      discoveryError: "OpenCode config is malformed."
    )
    let retainedForCancel = SettingsView.retainedOpenCodeAgentOverrides(
      from: group,
      discoveredAgentNames: [],
      discoveryError: "OpenCode config is malformed."
    )
    let persisted = SettingsView.persistedDraftGroup(
      draftGroup: group,
      draftCategoryMappings: group.categoryMappings,
      draftAgentOverrides: group.agentOverrides,
      draftOpenCodeAgentOverrides: retainedForLoad,
      updatedAt: Date(timeIntervalSince1970: 1_800_000_000)
    )

    XCTAssertEqual(retainedForLoad, expectedOpenCodeOverrides)
    XCTAssertEqual(retainedForCancel, expectedOpenCodeOverrides)
    XCTAssertEqual(persisted.openCodeAgentOverrides, expectedOpenCodeOverrides)
  }

  func testCurrentGroupModelMatchCountsUseTrimmedExactMatchesAcrossCurrentDraftOnly() {
    let counts = SettingsView.currentGroupModelMatchCounts(
      searchValue: "  openai/gpt-5.4  ",
      draftCategoryMappings: [
        ModelGroupCategoryMapping(categoryName: "quick", modelRef: "openai/gpt-5.4"),
        ModelGroupCategoryMapping(categoryName: "slow", modelRef: "openai/gpt-5.4-mini"),
      ],
      draftAgentOverrides: [
        ModelGroupAgentOverride(agentName: "general", modelRef: " openai/gpt-5.4 "),
        ModelGroupAgentOverride(agentName: "oracle", modelRef: "openai/o3"),
      ],
      draftOpenCodeAgentOverrides: [
        ModelGroupAgentOverride(agentName: "alpha", modelRef: "openai/gpt-5.4"),
        ModelGroupAgentOverride(agentName: "beta", modelRef: "openai/gpt-5.4-mini"),
      ]
    )

    XCTAssertEqual(counts.categoryMappings, 1)
    XCTAssertEqual(counts.agentOverrides, 1)
    XCTAssertEqual(counts.openCodeAgentOverrides, 1)
    XCTAssertEqual(counts.total, 3)
  }

  func testCurrentGroupModelBatchReplaceUpdatesOnlyTrimmedExactMatches() {
    let result = SettingsView.replacingCurrentGroupModelMatches(
      searchValue: " openai/gpt-5.4 ",
      replaceValue: " cliproxyapi/gpt-5.5 ",
      draftCategoryMappings: [
        ModelGroupCategoryMapping(categoryName: "quick", modelRef: "openai/gpt-5.4"),
        ModelGroupCategoryMapping(categoryName: "slow", modelRef: "openai/gpt-5.4-mini"),
      ],
      draftAgentOverrides: [
        ModelGroupAgentOverride(agentName: "general", modelRef: " openai/gpt-5.4 "),
        ModelGroupAgentOverride(agentName: "oracle", modelRef: "openai/o3"),
      ],
      draftOpenCodeAgentOverrides: [
        ModelGroupAgentOverride(agentName: "alpha", modelRef: "openai/gpt-5.4"),
        ModelGroupAgentOverride(agentName: "beta", modelRef: "openai/gpt-5.4-mini"),
      ]
    )

    XCTAssertEqual(result.matchCounts.total, 3)
    XCTAssertEqual(
      result.categoryMappings,
      [
        ModelGroupCategoryMapping(categoryName: "quick", modelRef: "cliproxyapi/gpt-5.5"),
        ModelGroupCategoryMapping(categoryName: "slow", modelRef: "openai/gpt-5.4-mini"),
      ]
    )
    XCTAssertEqual(
      result.agentOverrides,
      [
        ModelGroupAgentOverride(agentName: "general", modelRef: "cliproxyapi/gpt-5.5"),
        ModelGroupAgentOverride(agentName: "oracle", modelRef: "openai/o3"),
      ]
    )
    XCTAssertEqual(
      result.openCodeAgentOverrides,
      [
        ModelGroupAgentOverride(agentName: "alpha", modelRef: "cliproxyapi/gpt-5.5"),
        ModelGroupAgentOverride(agentName: "beta", modelRef: "openai/gpt-5.4-mini"),
      ]
    )
  }

  func testCurrentGroupModelBatchReplaceDoesNothingForBlankSearchValue() {
    let categoryMappings = [
      ModelGroupCategoryMapping(categoryName: "quick", modelRef: "openai/gpt-5.4")
    ]
    let agentOverrides = [
      ModelGroupAgentOverride(agentName: "general", modelRef: "openai/gpt-5.4")
    ]
    let openCodeAgentOverrides = [
      ModelGroupAgentOverride(agentName: "alpha", modelRef: "openai/gpt-5.4")
    ]

    let result = SettingsView.replacingCurrentGroupModelMatches(
      searchValue: "   ",
      replaceValue: "cliproxyapi/gpt-5.5",
      draftCategoryMappings: categoryMappings,
      draftAgentOverrides: agentOverrides,
      draftOpenCodeAgentOverrides: openCodeAgentOverrides
    )

    XCTAssertEqual(result.matchCounts.total, 0)
    XCTAssertEqual(result.categoryMappings, categoryMappings)
    XCTAssertEqual(result.agentOverrides, agentOverrides)
    XCTAssertEqual(result.openCodeAgentOverrides, openCodeAgentOverrides)
  }

  private func makeGroup(openCodeAgentOverrides: [ModelGroupAgentOverride]) -> ModelGroup {
    ModelGroup(
      id: UUID(uuidString: "11111111-1111-1111-1111-111111111111")!,
      name: " Primary ",
      description: "   ",
      categoryMappings: [
        ModelGroupCategoryMapping(categoryName: "unspecified-high", modelRef: "cliproxyapi/gpt-5.4")
      ],
      agentOverrides: [
        ModelGroupAgentOverride(agentName: "general", modelRef: "cliproxyapi/gpt-5.4")
      ],
      openCodeAgentOverrides: openCodeAgentOverrides,
      updatedAt: Date(timeIntervalSince1970: 1_700_000_000),
    )
  }
}
