import Foundation
import XCTest
@testable import OMOSwitch

@MainActor
final class AppStoreTests: XCTestCase {
  func testReloadLoadsGroupsAndCurrentSelection() throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let group = makeGroup(id: UUID(uuidString: "11111111-1111-1111-1111-111111111111")!, name: "Primary")
    let store = makeStore(configRootURL: rootURL)

    try store.modelGroupRepository.save([group])
    try store.appStateRepository.save(AppSelectionState(selectedGroupID: group.id, selectedGroupName: group.name))

    store.reload()

    XCTAssertEqual(store.groups, [group])
    XCTAssertEqual(store.currentGroupID, group.id)
    XCTAssertEqual(store.currentGroupName, "Primary")
    XCTAssertFalse(store.isLoading)
  }

  func testDeleteCurrentGroupClearsSelectionState() throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let selectedGroup = makeGroup(id: UUID(uuidString: "22222222-2222-2222-2222-222222222222")!, name: "Selected")
    let otherGroup = makeGroup(id: UUID(uuidString: "33333333-3333-3333-3333-333333333333")!, name: "Other")
    let store = makeStore(configRootURL: rootURL)

    try store.modelGroupRepository.save([selectedGroup, otherGroup])
    try store.appStateRepository.save(AppSelectionState(selectedGroupID: selectedGroup.id, selectedGroupName: selectedGroup.name))

    try store.deleteGroup(id: selectedGroup.id)

    XCTAssertEqual(store.groups, [otherGroup])
    XCTAssertNil(store.currentGroupID)
    XCTAssertNil(store.currentGroupName)
    XCTAssertEqual(try store.appStateRepository.load().selectedGroupID, nil)
    XCTAssertEqual(try store.appStateRepository.load().selectedGroupName, nil)
  }

  private func makeStore(configRootURL: URL) -> AppStore {
    let modelGroupRepository = ModelGroupRepository(configRootURL: configRootURL)
    let appStateRepository = AppStateRepository(configRootURL: configRootURL)
    let switchUseCase = SwitchGroupUseCase(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      backupRepository: BackupRepository(configRootURL: configRootURL),
      ohMyConfigRepository: OhMyOpenAgentConfigRepository(configRootURL: configRootURL),
    )
    return AppStore(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      switchUseCase: switchUseCase,
    )
  }

  private func makeGroup(id: UUID, name: String) -> ModelGroup {
    ModelGroup(
      id: id,
      name: name,
      categoryMappings: [
        ModelGroupCategoryMapping(categoryName: "unspecified-high", modelRef: "cliproxyapi/gpt-5.4")
      ],
      updatedAt: Date(timeIntervalSince1970: 1_700_000_000),
    )
  }
}
