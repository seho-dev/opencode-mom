import Foundation
import XCTest
@testable import OMOSwitch

@MainActor
final class AppStoreServerTests: XCTestCase {
  func testReloadAutoStartsWhenEnabled() async throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let manager = StubOpenCodeServeProcessManager()
    let store = makeStore(configRootURL: rootURL, processManager: manager)
    let config = OpenCodeServeConfig(port: 4999, autoStart: true)
    try store.appStateRepository.save(AppSelectionState(openCodeServeConfig: config))

    store.reload()

    try await waitUntil { await manager.recordedStartConfigs() == [config] }
    XCTAssertEqual(store.openCodeServeConfig, config)
  }

  func testReloadDoesNotAutoStartWhenDisabled() async throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let manager = StubOpenCodeServeProcessManager()
    let store = makeStore(configRootURL: rootURL, processManager: manager)
    let config = OpenCodeServeConfig(port: 4998, autoStart: false)
    try store.appStateRepository.save(AppSelectionState(openCodeServeConfig: config))

    store.reload()
    try await Task.sleep(nanoseconds: 50_000_000)

    let startConfigs = await manager.recordedStartConfigs()
    XCTAssertEqual(startConfigs, [])
    XCTAssertEqual(store.openCodeServeConfig, config)
  }

  func testManualStartDoesNotPersistAutoStart() async throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let manager = StubOpenCodeServeProcessManager()
    let store = makeStore(configRootURL: rootURL, processManager: manager)
    let config = OpenCodeServeConfig(port: 4997, autoStart: false)
    try store.appStateRepository.save(AppSelectionState(openCodeServeConfig: config))
    store.reload()

    store.startServer()

    try await waitUntil { await manager.recordedStartConfigs() == [config] }
    XCTAssertFalse(try store.appStateRepository.load().openCodeServeConfig.autoStart)
  }

  func testStartStopRestartCallProcessManager() async throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let manager = StubOpenCodeServeProcessManager()
    let store = makeStore(configRootURL: rootURL, processManager: manager)
    let config = OpenCodeServeConfig(port: 4996, hostname: "0.0.0.0")
    try store.appStateRepository.save(AppSelectionState(openCodeServeConfig: config))
    store.reload()

    store.startServer()
    try await waitUntil { await manager.recordedEvents().contains(.start(config)) }
    store.stopServer()
    try await waitUntil { await manager.recordedEvents().contains(.stop) }
    store.restartServer()
    try await waitUntil { await manager.recordedEvents().contains(.restart(config)) }

    let events = await manager.recordedEvents()
    XCTAssertEqual(events, [.currentState, .start(config), .currentState, .stop, .currentState, .restart(config), .currentState])
  }

  func testRetryFromFailedClearsFailureAndEntersStarting() async throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let manager = StubOpenCodeServeProcessManager(initialState: .failed(reason: "boom"), holdStartingState: true)
    let store = makeStore(configRootURL: rootURL, processManager: manager)
    let config = OpenCodeServeConfig(port: 4995)
    try store.appStateRepository.save(AppSelectionState(openCodeServeConfig: config))
    store.reload()
    try await waitUntil { store.openCodeServeStatus == .failed(reason: "boom") }

    store.startServer()

    try await waitUntil {
      let startConfigs = await manager.recordedStartConfigs()
      return store.openCodeServeStatus == .starting && startConfigs == [config]
    }
    let startConfigs = await manager.recordedStartConfigs()
    XCTAssertEqual(startConfigs, [config])
  }

  func testSaveServerConfigValidatesPersistsAndPublishes() throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let store = makeStore(configRootURL: rootURL)
    let config = OpenCodeServeConfig(port: 4994, hostname: "0.0.0.0", mdns: true, autoStart: true)

    try store.saveServerConfig(config)

    XCTAssertEqual(store.openCodeServeConfig, config)
    XCTAssertEqual(try store.appStateRepository.load().openCodeServeConfig, config)
    XCTAssertThrowsError(try store.saveServerConfig(OpenCodeServeConfig(port: 0)))
  }

  func testSavingActiveGroupRestartsServerOnceWhenRunning() async throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let manager = StubOpenCodeServeProcessManager(initialState: .running)
    let store = makeStore(configRootURL: rootURL, processManager: manager)
    let config = OpenCodeServeConfig(port: 4993)
    let group = makeGroup(id: UUID(uuidString: "66666666-6666-6666-6666-666666666666")!, name: "Primary")
    let updatedGroup = makeGroup(id: group.id, name: "Primary Updated")

    try store.modelGroupRepository.save([group])
    try store.appStateRepository.save(AppSelectionState(
      selectedGroupID: group.id,
      selectedGroupName: group.name,
      openCodeServeConfig: config
    ))
    try OhMyOpenAgentConfigRepository(configRootURL: rootURL).save(OhMyOpenAgentDocument.bootstrap())
    store.reload()
    try await waitUntil { store.openCodeServeStatus == .running }

    try await store.saveGroup(updatedGroup)

    let restartConfigs = await manager.recordedRestartConfigs()
    XCTAssertEqual(restartConfigs, [config])
  }

  func testSavingActiveGroupDoesNotRestartServerWhenStopped() async throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let manager = StubOpenCodeServeProcessManager(initialState: .stopped)
    let store = makeStore(configRootURL: rootURL, processManager: manager)
    let group = makeGroup(id: UUID(uuidString: "77777777-7777-7777-7777-777777777777")!, name: "Primary")
    let updatedGroup = makeGroup(id: group.id, name: "Primary Updated")

    try store.modelGroupRepository.save([group])
    try store.appStateRepository.save(AppSelectionState(selectedGroupID: group.id, selectedGroupName: group.name))
    try OhMyOpenAgentConfigRepository(configRootURL: rootURL).save(OhMyOpenAgentDocument.bootstrap())
    store.reload()
    try await waitUntil { store.openCodeServeStatus == .stopped }

    try await store.saveGroup(updatedGroup)
    try await Task.sleep(nanoseconds: 50_000_000)

    let restartConfigs = await manager.recordedRestartConfigs()
    XCTAssertEqual(restartConfigs, [])
  }

  func testFailedActiveGroupSaveDoesNotRestartServer() async throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let manager = StubOpenCodeServeProcessManager(initialState: .running)
    let store = makeStore(configRootURL: rootURL, processManager: manager)
    let group = makeGroup(id: UUID(uuidString: "88888888-8888-8888-8888-888888888888")!, name: "Primary")
    var disabledGroup = makeGroup(id: group.id, name: "Primary Disabled")
    disabledGroup.isEnabled = false

    try store.modelGroupRepository.save([group])
    try store.appStateRepository.save(AppSelectionState(selectedGroupID: group.id, selectedGroupName: group.name))
    store.reload()
    try await waitUntil { store.openCodeServeStatus == .running }

    do {
      try await store.saveGroup(disabledGroup)
      XCTFail("Expected active group save to fail")
    } catch {
      // Expected.
    }
    try await Task.sleep(nanoseconds: 50_000_000)

    let restartConfigs = await manager.recordedRestartConfigs()
    XCTAssertEqual(restartConfigs, [])
  }

  func testSavingServerConfigRestartsOnceWhenServerArgumentsChangeAndRunning() async throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let manager = StubOpenCodeServeProcessManager(initialState: .running)
    let store = makeStore(configRootURL: rootURL, processManager: manager)
    let originalConfig = OpenCodeServeConfig(port: 4992, hostname: "127.0.0.1", mdns: false, mdnsDomain: "opencode.local", cors: [], autoStart: false)
    let changedConfig = OpenCodeServeConfig(port: 4991, hostname: "0.0.0.0", mdns: true, mdnsDomain: "omo.local", cors: ["http://localhost:5173"], autoStart: false)

    try store.appStateRepository.save(AppSelectionState(openCodeServeConfig: originalConfig))
    store.reload()
    try await waitUntil { store.openCodeServeStatus == .running }

    try store.saveServerConfig(changedConfig)

    try await waitUntil { await manager.recordedRestartConfigs() == [changedConfig] }
    XCTAssertEqual(try store.appStateRepository.load().openCodeServeConfig, changedConfig)
  }

  func testSavingServerConfigAutoStartOnlyChangeDoesNotRestartWhenRunning() async throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let manager = StubOpenCodeServeProcessManager(initialState: .running)
    let store = makeStore(configRootURL: rootURL, processManager: manager)
    let originalConfig = OpenCodeServeConfig(port: 4990, autoStart: false)
    let changedConfig = OpenCodeServeConfig(port: 4990, autoStart: true)

    try store.appStateRepository.save(AppSelectionState(openCodeServeConfig: originalConfig))
    store.reload()
    try await waitUntil { store.openCodeServeStatus == .running }

    try store.saveServerConfig(changedConfig)
    try await Task.sleep(nanoseconds: 50_000_000)

    let restartConfigs = await manager.recordedRestartConfigs()
    XCTAssertEqual(restartConfigs, [])
    XCTAssertEqual(try store.appStateRepository.load().openCodeServeConfig, changedConfig)
  }

  func testSavingInvalidServerConfigDoesNotPersistOrRestart() async throws {
    let rootURL = try TestSupport.makeTemporaryDirectory()
    defer { TestSupport.removeIfExists(rootURL) }
    let manager = StubOpenCodeServeProcessManager(initialState: .running)
    let store = makeStore(configRootURL: rootURL, processManager: manager)
    let originalConfig = OpenCodeServeConfig(port: 4989, autoStart: true)

    try store.appStateRepository.save(AppSelectionState(openCodeServeConfig: originalConfig))
    store.reload()
    try await waitUntil { store.openCodeServeStatus == .running }

    XCTAssertThrowsError(try store.saveServerConfig(OpenCodeServeConfig(port: 0, autoStart: false)))
    try await Task.sleep(nanoseconds: 50_000_000)

    let restartConfigs = await manager.recordedRestartConfigs()
    XCTAssertEqual(restartConfigs, [])
    XCTAssertEqual(try store.appStateRepository.load().openCodeServeConfig, originalConfig)
  }

  private func makeStore(
    configRootURL: URL,
    processManager: any OpenCodeServeProcessManaging = StubOpenCodeServeProcessManager()
  ) -> AppStore {
    let modelGroupRepository = ModelGroupRepository(configRootURL: configRootURL)
    let appStateRepository = AppStateRepository(configRootURL: configRootURL)
    let openCodeConfigRepository = OpenCodeConfigRepository(configRootURL: configRootURL)
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
      loginItemService: StubServerLoginItemService(),
      updateChecker: StubServerUpdateChecker(),
      processManager: processManager
    )
  }

  private func makeGroup(id: UUID, name: String) -> ModelGroup {
    ModelGroup(
      id: id,
      name: name,
      categoryMappings: [
        ModelGroupCategoryMapping(categoryName: "unspecified-high", modelRef: "cliproxyapi/gpt-5.5")
      ],
      updatedAt: Date(timeIntervalSince1970: 1_700_001_000)
    )
  }
}

private enum StubOpenCodeServeProcessEvent: Equatable {
  case currentState
  case start(OpenCodeServeConfig)
  case stop
  case restart(OpenCodeServeConfig)
}

private actor StubOpenCodeServeProcessManager: OpenCodeServeProcessManaging {
  private(set) var events: [StubOpenCodeServeProcessEvent] = []
  private(set) var startConfigs: [OpenCodeServeConfig] = []
  private let holdStartingState: Bool
  private var state: OpenCodeServeProcessState

  init(initialState: OpenCodeServeProcessState = .stopped, holdStartingState: Bool = false) {
    self.state = initialState
    self.holdStartingState = holdStartingState
  }

  func currentState() async -> OpenCodeServeProcessState {
    events.append(.currentState)
    return state
  }

  func recordedEvents() -> [StubOpenCodeServeProcessEvent] {
    events
  }

  func recordedStartConfigs() -> [OpenCodeServeConfig] {
    startConfigs
  }

  func recordedRestartConfigs() -> [OpenCodeServeConfig] {
    events.compactMap {
      if case .restart(let config) = $0 { return config }
      return nil
    }
  }

  func start(config: OpenCodeServeConfig) async {
    events.append(.start(config))
    startConfigs.append(config)
    state = .starting
    if holdStartingState == false {
      state = .running
    }
  }

  func restart(config: OpenCodeServeConfig) async {
    events.append(.restart(config))
    state = .running
  }

  func start(arguments: [String]) async {
    state = .running
  }

  func stop() async {
    events.append(.stop)
    state = .stopped
  }

  func restart(arguments: [String]) async {
    state = .running
  }
}

@MainActor
private final class StubServerLoginItemService: LoginItemService {
  func currentStatus() throws -> LoginItemStatus { .disabled }
  func setEnabled(_ isEnabled: Bool) throws {}
}

private struct StubServerUpdateChecker: UpdateChecker {
  func checkForUpdates() async throws -> UpdateInfo { throw NSError(domain: "Stub", code: 0) }
  func downloadUpdate(_ updateInfo: UpdateInfo) async throws -> URL { throw NSError(domain: "Stub", code: 0) }
  func installUpdate(at url: URL) throws {}
}

@MainActor
private func waitUntil(
  timeoutNanoseconds: UInt64 = 1_000_000_000,
  condition: @escaping @MainActor () async throws -> Bool
) async throws {
  let deadline = DispatchTime.now().uptimeNanoseconds + timeoutNanoseconds
  while DispatchTime.now().uptimeNanoseconds < deadline {
    if try await condition() { return }
    try await Task.sleep(nanoseconds: 10_000_000)
  }
  XCTFail("Timed out waiting for condition")
}
