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
    XCTAssertFalse(store.launchAtLoginEnabled)
    XCTAssertNil(store.launchAtLoginStatusMessage)
    XCTAssertEqual(store.discoveredOpenCodeAgentNames, [])
    XCTAssertEqual(store.openCodeAgentDiscoveryError, "OpenCode config not found.")
    XCTAssertFalse(store.isLoading)
  }

  func testReloadReconcilesLaunchAtLoginWithSystemState() throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let group = makeGroup(id: UUID(uuidString: "17171717-1717-1717-1717-171717171717")!, name: "Primary")
    let loginItemService = StubLoginItemService(currentStatus: .enabled)
    let store = makeStore(configRootURL: rootURL, loginItemService: loginItemService)

    try store.modelGroupRepository.save([group])
    try store.appStateRepository.save(AppSelectionState(
      selectedGroupID: group.id,
      selectedGroupName: group.name,
      launchAtLoginEnabled: false
    ))

    store.reload()

    XCTAssertTrue(store.launchAtLoginEnabled)
    XCTAssertTrue(try store.appStateRepository.load().launchAtLoginEnabled)
  }

  func testReloadShowsPendingApprovalWithoutPersistingEnabledState() throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let loginItemService = StubLoginItemService(currentStatus: .requiresApproval)
    let store = makeStore(configRootURL: rootURL, loginItemService: loginItemService)

    try store.appStateRepository.save(AppSelectionState(launchAtLoginEnabled: true))

    store.reload()

    XCTAssertFalse(store.launchAtLoginEnabled)
    XCTAssertEqual(store.launchAtLoginStatusMessage, "Launch at login is pending approval in System Settings.")
    XCTAssertFalse(try store.appStateRepository.load().launchAtLoginEnabled)
  }

  func testReloadSurfacesLaunchAtLoginStatusReadErrors() throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let loginItemService = StubLoginItemService(currentStatus: .disabled)
    loginItemService.currentStatusError = NSError(domain: "LoginItem", code: 8, userInfo: [NSLocalizedDescriptionKey: "status unavailable"])
    let store = makeStore(configRootURL: rootURL, loginItemService: loginItemService)

    store.reload()

    XCTAssertEqual(store.launchAtLoginStatusMessage, "Unable to read launch-at-login status: status unavailable")
  }

  func testSetLaunchAtLoginEnabledPersistsAndCallsService() throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let loginItemService = StubLoginItemService(currentStatus: .disabled)
    let store = makeStore(configRootURL: rootURL, loginItemService: loginItemService)

    try store.setLaunchAtLoginEnabled(true)

    XCTAssertEqual(loginItemService.setEnabledCalls, [true])
    XCTAssertTrue(store.launchAtLoginEnabled)
    XCTAssertNil(store.launchAtLoginStatusMessage)
    XCTAssertTrue(try store.appStateRepository.load().launchAtLoginEnabled)
  }

  func testSetLaunchAtLoginEnabledShowsPendingApprovalState() throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let loginItemService = StubLoginItemService(currentStatus: .disabled)
    loginItemService.statusAfterSetEnabled[true] = .requiresApproval
    let store = makeStore(configRootURL: rootURL, loginItemService: loginItemService)

    try store.setLaunchAtLoginEnabled(true)

    XCTAssertEqual(loginItemService.setEnabledCalls, [true])
    XCTAssertFalse(store.launchAtLoginEnabled)
    XCTAssertEqual(store.launchAtLoginStatusMessage, "Launch at login is pending approval in System Settings.")
    XCTAssertFalse(try store.appStateRepository.load().launchAtLoginEnabled)
  }

  func testSetLaunchAtLoginEnabledDoesNotPersistWhenServiceFails() throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let loginItemService = StubLoginItemService(currentStatus: .disabled)
    loginItemService.setEnabledError = NSError(domain: "LoginItem", code: 7, userInfo: [NSLocalizedDescriptionKey: "register failed"])
    let store = makeStore(configRootURL: rootURL, loginItemService: loginItemService)

    XCTAssertThrowsError(try store.setLaunchAtLoginEnabled(true))
    XCTAssertFalse(store.launchAtLoginEnabled)
    XCTAssertFalse(try store.appStateRepository.load().launchAtLoginEnabled)
  }

  func testReloadDiscoversOpenCodeAgentNamesFromFixture() throws {
    let harness = TemporaryHomeHarness()
    try harness.setupOmoSwitchConfig()
    try harness.setupOpencodeConfig()
    try harness.installFixture(named: "current-opencode.json", subdirectory: "opencode", to: "opencode.json")

    let group = makeGroup(id: UUID(uuidString: "12121212-1212-1212-1212-121212121212")!, name: "Primary")
    let store = makeStore(configRootURL: harness.omoSwitchConfigURL, openCodeConfigRootURL: harness.homeURL.appendingPathComponent(".config", isDirectory: true))

    try store.modelGroupRepository.save([group])
    try store.appStateRepository.save(AppSelectionState(selectedGroupID: group.id, selectedGroupName: group.name))

    store.reload()

    XCTAssertEqual(store.groups, [group])
    XCTAssertEqual(store.currentGroupName, "Primary")
    XCTAssertTrue(store.discoveredOpenCodeAgentNames.contains("creative-ui-coder"))
    XCTAssertTrue(store.discoveredOpenCodeAgentNames.contains("Jenny"))
    XCTAssertTrue(store.discoveredOpenCodeAgentNames.contains("karen"))
    XCTAssertNil(store.openCodeAgentDiscoveryError)
  }

  func testReloadKeepsGroupsAndStateWhenOpenCodeConfigIsMissing() throws {
    let harness = TemporaryHomeHarness()
    try harness.setupOmoSwitchConfig()

    let group = makeGroup(id: UUID(uuidString: "13131313-1313-1313-1313-131313131313")!, name: "Primary")
    let store = makeStore(configRootURL: harness.omoSwitchConfigURL, openCodeConfigRootURL: harness.homeURL.appendingPathComponent(".config", isDirectory: true))

    try store.modelGroupRepository.save([group])
    try store.appStateRepository.save(AppSelectionState(selectedGroupID: group.id, selectedGroupName: group.name))

    store.reload()

    XCTAssertEqual(store.groups, [group])
    XCTAssertEqual(store.currentGroupID, group.id)
    XCTAssertEqual(store.currentGroupName, "Primary")
    XCTAssertEqual(store.discoveredOpenCodeAgentNames, [])
    XCTAssertEqual(store.openCodeAgentDiscoveryError, "OpenCode config not found.")
  }

  func testReloadKeepsGroupsAndStateWhenOpenCodeConfigIsMalformed() throws {
    let harness = TemporaryHomeHarness()
    try harness.setupOmoSwitchConfig()
    try harness.setupOpencodeConfig()
    let malformedURL = harness.opencodeConfigURL.appendingPathComponent("opencode.json")
    try "{ invalid json".data(using: .utf8)!.write(to: malformedURL, options: [.atomic])

    let group = makeGroup(id: UUID(uuidString: "14141414-1414-1414-1414-141414141414")!, name: "Primary")
    let store = makeStore(configRootURL: harness.omoSwitchConfigURL, openCodeConfigRootURL: harness.homeURL.appendingPathComponent(".config", isDirectory: true))

    try store.modelGroupRepository.save([group])
    try store.appStateRepository.save(AppSelectionState(selectedGroupID: group.id, selectedGroupName: group.name))

    store.reload()

    XCTAssertEqual(store.groups, [group])
    XCTAssertEqual(store.currentGroupID, group.id)
    XCTAssertEqual(store.currentGroupName, "Primary")
    XCTAssertEqual(store.discoveredOpenCodeAgentNames, [])
    XCTAssertEqual(store.openCodeAgentDiscoveryError, "OpenCode config is malformed.")
  }

  func testReloadSurfacesDiscoveryErrorWhenOpenCodeAgentShapeIsMalformed() throws {
    let harness = TemporaryHomeHarness()
    try harness.setupOmoSwitchConfig()
    try harness.setupOpencodeConfig()
    let malformedAgentURL = harness.opencodeConfigURL.appendingPathComponent("opencode.json")
    let payload = #"{"$schema":"https://opencode.ai/config.json","agent":"invalid-agent-shape"}"#
    try payload.write(to: malformedAgentURL, atomically: true, encoding: .utf8)

    let group = makeGroup(id: UUID(uuidString: "15151515-1515-1515-1515-151515151515")!, name: "Primary")
    let store = makeStore(configRootURL: harness.omoSwitchConfigURL, openCodeConfigRootURL: harness.homeURL.appendingPathComponent(".config", isDirectory: true))

    try store.modelGroupRepository.save([group])
    try store.appStateRepository.save(AppSelectionState(selectedGroupID: group.id, selectedGroupName: group.name))

    store.reload()

    XCTAssertEqual(store.groups, [group])
    XCTAssertEqual(store.currentGroupID, group.id)
    XCTAssertEqual(store.currentGroupName, "Primary")
    XCTAssertEqual(store.discoveredOpenCodeAgentNames, [])
    XCTAssertEqual(store.openCodeAgentDiscoveryError, "OpenCode config has no valid top-level agent object.")
  }

  func testReloadSurfacesDiscoveryErrorWhenOpenCodeAgentKeyIsMissing() throws {
    let harness = TemporaryHomeHarness()
    try harness.setupOmoSwitchConfig()
    try harness.setupOpencodeConfig()
    let missingAgentURL = harness.opencodeConfigURL.appendingPathComponent("opencode.json")
    let payload = #"{"$schema":"https://opencode.ai/config.json","provider":{"cliproxyapi":{"name":"CLIProxyAPI"}}}"#
    try payload.write(to: missingAgentURL, atomically: true, encoding: .utf8)

    let group = makeGroup(id: UUID(uuidString: "16161616-1616-1616-1616-161616161616")!, name: "Primary")
    let store = makeStore(configRootURL: harness.omoSwitchConfigURL, openCodeConfigRootURL: harness.homeURL.appendingPathComponent(".config", isDirectory: true))

    try store.modelGroupRepository.save([group])
    try store.appStateRepository.save(AppSelectionState(selectedGroupID: group.id, selectedGroupName: group.name))

    store.reload()

    XCTAssertEqual(store.groups, [group])
    XCTAssertEqual(store.currentGroupID, group.id)
    XCTAssertEqual(store.currentGroupName, "Primary")
    XCTAssertEqual(store.discoveredOpenCodeAgentNames, [])
    XCTAssertEqual(store.openCodeAgentDiscoveryError, "OpenCode config has no valid top-level agent object.")
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

  func testSaveGroupRewritesOhMyOpenAgentWhenSavingActiveGroup() async throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }

    let groupID = UUID(uuidString: "44444444-4444-4444-4444-444444444444")!
    let originalGroup = makeGroup(id: groupID, name: "Primary")
    let activeGroup = ModelGroup(
      id: groupID,
      name: "Primary Updated",
      categoryMappings: [
        ModelGroupCategoryMapping(categoryName: "unspecified-high", modelRef: "openai/gpt-5.4")
      ],
      updatedAt: Date(timeIntervalSince1970: 1_700_000_100)
    )
    let store = makeStore(configRootURL: rootURL)
    let configRepository = OhMyOpenAgentConfigRepository(configRootURL: rootURL)

    try store.modelGroupRepository.save([originalGroup])
    try store.appStateRepository.save(AppSelectionState(selectedGroupID: groupID, selectedGroupName: originalGroup.name))
    try configRepository.save(OhMyOpenAgentDocument.bootstrap())
    store.reload()

    try await store.saveGroup(activeGroup)

    let loadResult = configRepository.load()
    guard case .success(let document) = loadResult else {
      return XCTFail("Expected saved oh-my-openagent config")
    }

    XCTAssertEqual(store.currentGroupID, groupID)
    XCTAssertEqual(store.currentGroupName, "Primary Updated")
    XCTAssertEqual((document.categories["unspecified-high"] as? [String: Any])?["model"] as? String, "openai/gpt-5.4")
    XCTAssertNil(store.lastSwitchError)
  }

  func testSaveGroupRewritesOpenCodeAgentModelsWhenSavingActiveGroup() async throws {
    let harness = TemporaryHomeHarness()
    try harness.setupOmoSwitchConfig()
    try harness.setupOpencodeConfig()
    try harness.installFixture(named: "current-opencode.json", subdirectory: "opencode", to: "opencode.json")
    try harness.installFixture(named: "current-oh-my-openagent.json", subdirectory: "ohmy", to: "oh-my-openagent.json")

    let groupID = UUID(uuidString: "45454545-4545-4545-4545-454545454545")!
    let originalGroup = makeGroup(id: groupID, name: "Primary")
    let activeGroup = ModelGroup(
      id: groupID,
      name: "Primary Updated",
      categoryMappings: [
        ModelGroupCategoryMapping(categoryName: "unspecified-high", modelRef: "openai/gpt-5.4")
      ],
      openCodeAgentOverrides: [
        ModelGroupAgentOverride(agentName: "creative-ui-coder", modelRef: "cliproxyapi/gpt-5.4-xhigh")
      ],
      updatedAt: Date(timeIntervalSince1970: 1_700_000_101)
    )
    let store = makeStore(configRootURL: harness.omoSwitchConfigURL, openCodeConfigRootURL: harness.homeURL.appendingPathComponent(".config", isDirectory: true))
    let openCodeRepository = OpenCodeConfigRepository(configRootURL: harness.homeURL.appendingPathComponent(".config", isDirectory: true))

    try store.modelGroupRepository.save([originalGroup])
    try store.appStateRepository.save(AppSelectionState(selectedGroupID: groupID, selectedGroupName: originalGroup.name))
    store.reload()

    try await store.saveGroup(activeGroup)

    let loadResult = openCodeRepository.load()
    guard case .success(let document) = loadResult else {
      return XCTFail("Expected saved opencode config")
    }

    XCTAssertEqual(store.currentGroupID, groupID)
    XCTAssertEqual(store.currentGroupName, "Primary Updated")
    XCTAssertEqual((document.agents["creative-ui-coder"] as? [String: Any])?["model"] as? String, "cliproxyapi/gpt-5.4-xhigh")
    XCTAssertEqual((document.agents["creative-ui-coder"] as? [String: Any])?["mode"] as? String, "subagent")
    XCTAssertNil(store.lastSwitchError)
  }

  private func makeStore(
    configRootURL: URL,
    openCodeConfigRootURL: URL? = nil,
    loginItemService: any LoginItemService = StubLoginItemService(currentStatus: .disabled)
  ) -> AppStore {
    let modelGroupRepository = ModelGroupRepository(configRootURL: configRootURL)
    let appStateRepository = AppStateRepository(configRootURL: configRootURL)
    let openCodeConfigRepository = OpenCodeConfigRepository(configRootURL: openCodeConfigRootURL ?? configRootURL)
    let switchUseCase = SwitchGroupUseCase(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      backupRepository: BackupRepository(configRootURL: configRootURL),
      openCodeConfigRepository: openCodeConfigRepository,
      ohMyConfigRepository: OhMyOpenAgentConfigRepository(configRootURL: configRootURL),
    )
    return AppStore(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      openCodeConfigRepository: openCodeConfigRepository,
      switchUseCase: switchUseCase,
      loginItemService: loginItemService,
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

@MainActor
private final class StubLoginItemService: LoginItemService {
  var currentStatusValue: LoginItemStatus
  var currentStatusError: Error?
  var setEnabledError: Error?
  var statusAfterSetEnabled: [Bool: LoginItemStatus] = [:]
  private(set) var setEnabledCalls: [Bool] = []

  init(currentStatus: LoginItemStatus) {
    self.currentStatusValue = currentStatus
  }

  func currentStatus() throws -> LoginItemStatus {
    if let currentStatusError {
      throw currentStatusError
    }

    return currentStatusValue
  }

  func setEnabled(_ isEnabled: Bool) throws {
    setEnabledCalls.append(isEnabled)

    if let setEnabledError {
      throw setEnabledError
    }

    currentStatusValue = statusAfterSetEnabled[isEnabled] ?? (isEnabled ? .enabled : .disabled)
  }
}
