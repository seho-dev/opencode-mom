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
