import Foundation
import XCTest
@testable import OMOSwitch

@MainActor
final class SettingsViewSelectionSyncTests: XCTestCase {
  func testDoesNotPreserveSelectionWhenNoDraftExists() {
    let group = makeGroup(id: UUID(uuidString: "11111111-1111-1111-1111-111111111111")!, name: "Primary")

    let state = SettingsView.SelectionSyncState(
      selectedGroupID: group.id,
      activeGroupID: UUID(uuidString: "22222222-2222-2222-2222-222222222222")!,
      baselineGroup: nil,
      draftGroup: nil,
      draftCategoryMappings: [],
      draftAgentOverrides: [],
    )

    XCTAssertFalse(SettingsView.shouldPreserveSelectedGroupID(for: state))
  }

  func testDoesNotPreserveSelectionWhenExistingDraftIsClean() {
    let group = makeGroup(id: UUID(uuidString: "11111111-1111-1111-1111-111111111111")!, name: "Primary")

    let state = SettingsView.SelectionSyncState(
      selectedGroupID: group.id,
      activeGroupID: UUID(uuidString: "22222222-2222-2222-2222-222222222222")!,
      baselineGroup: group,
      draftGroup: group,
      draftCategoryMappings: group.categoryMappings,
      draftAgentOverrides: group.agentOverrides,
    )

    XCTAssertFalse(SettingsView.shouldPreserveSelectedGroupID(for: state))
  }

  func testPreservesSelectionWhenExistingDraftMetadataIsDirty() {
    let group = makeGroup(id: UUID(uuidString: "11111111-1111-1111-1111-111111111111")!, name: "Primary")
    var dirtyDraft = group
    dirtyDraft.name = "Updated"

    let state = SettingsView.SelectionSyncState(
      selectedGroupID: group.id,
      activeGroupID: UUID(uuidString: "22222222-2222-2222-2222-222222222222")!,
      baselineGroup: group,
      draftGroup: dirtyDraft,
      draftCategoryMappings: group.categoryMappings,
      draftAgentOverrides: group.agentOverrides,
    )

    XCTAssertTrue(SettingsView.shouldPreserveSelectedGroupID(for: state))
  }

  func testPreservesSelectionWhenCategoryMappingsAreDirty() {
    let group = makeGroup(id: UUID(uuidString: "11111111-1111-1111-1111-111111111111")!, name: "Primary")
    let dirtyMappings = group.categoryMappings + [ModelGroupCategoryMapping(categoryName: "quick", modelRef: "openai/gpt-5.4")]

    let state = SettingsView.SelectionSyncState(
      selectedGroupID: group.id,
      activeGroupID: UUID(uuidString: "22222222-2222-2222-2222-222222222222")!,
      baselineGroup: group,
      draftGroup: group,
      draftCategoryMappings: dirtyMappings,
      draftAgentOverrides: group.agentOverrides,
    )

    XCTAssertTrue(SettingsView.shouldPreserveSelectedGroupID(for: state))
  }

  func testPreservesSelectionWhenAgentOverridesAreDirty() {
    let group = makeGroup(id: UUID(uuidString: "11111111-1111-1111-1111-111111111111")!, name: "Primary")
    let dirtyOverrides = group.agentOverrides + [ModelGroupAgentOverride(agentName: "oracle", modelRef: "openai/o3")]

    let state = SettingsView.SelectionSyncState(
      selectedGroupID: group.id,
      activeGroupID: UUID(uuidString: "22222222-2222-2222-2222-222222222222")!,
      baselineGroup: group,
      draftGroup: group,
      draftCategoryMappings: group.categoryMappings,
      draftAgentOverrides: dirtyOverrides,
    )

    XCTAssertTrue(SettingsView.shouldPreserveSelectedGroupID(for: state))
  }

  func testPreservesSelectionWhenEditingNewUnsavedGroup() {
    let draftGroup = makeGroup(id: UUID(uuidString: "33333333-3333-3333-3333-333333333333")!, name: "Draft")

    let state = SettingsView.SelectionSyncState(
      selectedGroupID: draftGroup.id,
      activeGroupID: UUID(uuidString: "22222222-2222-2222-2222-222222222222")!,
      baselineGroup: nil,
      draftGroup: draftGroup,
      draftCategoryMappings: draftGroup.categoryMappings,
      draftAgentOverrides: draftGroup.agentOverrides,
    )

    XCTAssertTrue(SettingsView.shouldPreserveSelectedGroupID(for: state))
  }

  private func makeGroup(id: UUID, name: String) -> ModelGroup {
    ModelGroup(
      id: id,
      name: name,
      categoryMappings: [
        ModelGroupCategoryMapping(categoryName: "unspecified-high", modelRef: "cliproxyapi/gpt-5.4")
      ],
      agentOverrides: [
        ModelGroupAgentOverride(agentName: "general", modelRef: "cliproxyapi/gpt-5.4")
      ],
      updatedAt: Date(timeIntervalSince1970: 1_700_000_000),
    )
  }
}
