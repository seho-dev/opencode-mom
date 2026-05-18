import AppKit
import XCTest
@testable import OMOSwitch

@MainActor
final class IntegrationHardeningTests: XCTestCase {
  func testAutoStartTrueStartsFakeServerAfterAppLoad() async throws {
    let harness = TemporaryHomeHarness()
    try harness.setupOmoSwitchConfig()
    let manager = IntegrationOpenCodeServeProcessManager(initialState: .stopped)
    let container = makeContainer(harness: harness, processManager: manager)
    let config = OpenCodeServeConfig(port: 4979, hostname: "127.0.0.1", autoStart: true)
    try container.appStateRepository.save(AppSelectionState(openCodeServeConfig: config))
    let appDelegate = AppDelegate(container: container)

    appDelegate.applicationDidFinishLaunching(Notification(name: NSApplication.didFinishLaunchingNotification))

    try await waitUntil { await manager.recordedStartConfigs() == [config] }
    let state = await manager.currentState()
    XCTAssertEqual(state, .running)
    XCTAssertEqual(container.appStore.openCodeServeConfig, config)
  }

  func testMenuStartStopActionsChangeFakeProcessManagerState() async throws {
    let harness = TemporaryHomeHarness()
    try harness.setupOmoSwitchConfig()
    let manager = IntegrationOpenCodeServeProcessManager(initialState: .stopped)
    let store = makeAppStore(harness, processManager: manager)
    let config = OpenCodeServeConfig(port: 4978, hostname: "127.0.0.1")
    try store.appStateRepository.save(AppSelectionState(openCodeServeConfig: config))
    let controller = makeStatusItemController(appStore: store)

    controller.menuWillOpen(controller.statusMenu)
    try performMenuItem(titled: "Start Server", in: controller)
    try await waitUntil { await manager.currentState() == .running }
    let startConfigs = await manager.recordedStartConfigs()
    XCTAssertEqual(startConfigs, [config])

    controller.menuWillOpen(controller.statusMenu)
    try performMenuItem(titled: "Stop Server", in: controller)
    try await waitUntil { await manager.currentState() == .stopped }
    let stopCount = await manager.recordedStopCount()
    XCTAssertEqual(stopCount, 1)
  }

  func testSavingActiveGroupRestartsRunningFakeServerAndRewritesProjection() async throws {
    let harness = TemporaryHomeHarness()
    try harness.setupOmoSwitchConfig()
    try harness.setupOpencodeConfig()
    let manager = IntegrationOpenCodeServeProcessManager(initialState: .running)
    let store = makeAppStore(harness, processManager: manager)
    let config = OpenCodeServeConfig(port: 4977, hostname: "127.0.0.1")
    let group = makeGroup(
      id: UUID(uuidString: "10101010-1010-1010-1010-101010101010")!,
      name: "Primary",
      categoryMappings: [ModelGroupCategoryMapping(categoryName: "quick", modelRef: "cliproxyapi/old")]
    )
    let updatedGroup = makeGroup(
      id: group.id,
      name: "Primary Updated",
      categoryMappings: [ModelGroupCategoryMapping(categoryName: "quick", modelRef: "cliproxyapi/new")]
    )
    try store.modelGroupRepository.save([group])
    try store.appStateRepository.save(AppSelectionState(
      selectedGroupID: group.id,
      selectedGroupName: group.name,
      openCodeServeConfig: config
    ))
    try makeOhMyConfigRepository(harness).save(OhMyOpenAgentDocument.bootstrap())
    store.reload()
    try await waitUntil { store.openCodeServeStatus == .running }

    try await store.saveGroup(updatedGroup)

    try await waitUntil { await manager.recordedRestartConfigs() == [config] }
    guard case .success(let document) = makeOhMyConfigRepository(harness).load() else {
      XCTFail("Expected rewritten oh-my-openagent config")
      return
    }
    XCTAssertEqual((document.categories["quick"] as? [String: Any])?["model"] as? String, "cliproxyapi/new")
    XCTAssertEqual(try store.appStateRepository.load().selectedGroupName, "Primary Updated")
  }

  func testAppTerminationStopsFakeServer() async throws {
    let harness = TemporaryHomeHarness()
    try harness.setupOmoSwitchConfig()
    let manager = IntegrationOpenCodeServeProcessManager(initialState: .running)
    let appDelegate = AppDelegate(container: makeContainer(harness: harness, processManager: manager))

    (appDelegate as NSApplicationDelegate).applicationWillTerminate?(Notification(name: NSApplication.willTerminateNotification))

    try await waitUntil { await manager.recordedStopCount() == 1 }
    let state = await manager.currentState()
    XCTAssertEqual(state, .stopped)
  }

  private func makeContainer(
    harness: TemporaryHomeHarness,
    processManager: any OpenCodeServeProcessManaging
  ) -> DependencyContainer {
    let store = makeAppStore(harness, processManager: processManager)
    return DependencyContainer(
      statusBarProvider: IntegrationStatusBarProvider(),
      popoverControllerFactory: { QuickSwitchPopoverController(popover: NSPopover()) },
      globalSettingsWindowController: SettingsWindowController(appStore: store, kind: .global),
      groupSettingsWindowController: SettingsWindowController(appStore: store, kind: .group),
      loginItemService: IntegrationLoginItemService(),
      updateChecker: IntegrationUpdateChecker(),
      processManager: processManager,
      configRootURL: harness.omoSwitchConfigURL
    )
  }

  private func makeAppStore(
    _ harness: TemporaryHomeHarness,
    processManager: any OpenCodeServeProcessManaging
  ) -> AppStore {
    let modelGroupRepository = makeModelGroupRepository(harness)
    let appStateRepository = makeAppStateRepository(harness)
    let openCodeConfigRepository = makeOpenCodeConfigRepository(harness)
    let switchUseCase = SwitchGroupUseCase(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      backupRepository: makeBackupRepository(harness),
      openCodeConfigRepository: openCodeConfigRepository,
      ohMyConfigRepository: makeOhMyConfigRepository(harness)
    )
    return AppStore(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      openCodeConfigRepository: openCodeConfigRepository,
      switchUseCase: switchUseCase,
      loginItemService: IntegrationLoginItemService(),
      updateChecker: IntegrationUpdateChecker(),
      processManager: processManager
    )
  }

  private func makeStatusItemController(appStore: AppStore) -> StatusItemController {
    StatusItemController(
      statusBarProvider: IntegrationStatusBarProvider(),
      appStore: appStore,
      popoverController: QuickSwitchPopoverController(popover: NSPopover()),
      globalSettingsWindowControllerProvider: { SettingsWindowController(appStore: appStore, kind: .global) },
      groupSettingsWindowControllerProvider: { SettingsWindowController(appStore: appStore, kind: .group) }
    )
  }

  private func makeModelGroupRepository(_ harness: TemporaryHomeHarness) -> ModelGroupRepository {
    ModelGroupRepository(fileManager: .default, configRootURL: harness.omoSwitchConfigURL)
  }

  private func makeAppStateRepository(_ harness: TemporaryHomeHarness) -> AppStateRepository {
    AppStateRepository(fileManager: .default, configRootURL: harness.omoSwitchConfigURL)
  }

  private func makeBackupRepository(_ harness: TemporaryHomeHarness) -> BackupRepository {
    BackupRepository(fileManager: .default, configRootURL: harness.omoSwitchConfigURL)
  }

  private func makeOhMyConfigRepository(_ harness: TemporaryHomeHarness) -> OhMyOpenAgentConfigRepository {
    OhMyOpenAgentConfigRepository(fileManager: .default, configRootURL: harness.homeURL.appendingPathComponent(".config", isDirectory: true))
  }

  private func makeOpenCodeConfigRepository(_ harness: TemporaryHomeHarness) -> OpenCodeConfigRepository {
    OpenCodeConfigRepository(fileManager: .default, configRootURL: harness.homeURL.appendingPathComponent(".config", isDirectory: true))
  }

  private func makeGroup(
    id: UUID,
    name: String,
    categoryMappings: [ModelGroupCategoryMapping]
  ) -> ModelGroup {
    ModelGroup(
      id: id,
      name: name,
      categoryMappings: categoryMappings,
      updatedAt: Date(timeIntervalSince1970: 1_700_002_000)
    )
  }

  private func performMenuItem(titled title: String, in controller: StatusItemController) throws {
    let item = try XCTUnwrap(controller.statusMenu.items.first { $0.title == title })
    let action = try XCTUnwrap(item.action)
    let target = try XCTUnwrap(item.target)
    NSApplication.shared.sendAction(action, to: target, from: item)
  }
}

private enum IntegrationOpenCodeServeEvent: Equatable {
  case start(OpenCodeServeConfig)
  case stop
  case restart(OpenCodeServeConfig)
}

private actor IntegrationOpenCodeServeProcessManager: OpenCodeServeProcessManaging {
  private var events: [IntegrationOpenCodeServeEvent] = []
  private var state: OpenCodeServeProcessState

  init(initialState: OpenCodeServeProcessState) {
    state = initialState
  }

  func currentState() async -> OpenCodeServeProcessState {
    state
  }

  func recordedStartConfigs() -> [OpenCodeServeConfig] {
    events.compactMap {
      if case .start(let config) = $0 { return config }
      return nil
    }
  }

  func recordedRestartConfigs() -> [OpenCodeServeConfig] {
    events.compactMap {
      if case .restart(let config) = $0 { return config }
      return nil
    }
  }

  func recordedStopCount() -> Int {
    events.filter { $0 == .stop }.count
  }

  func start(config: OpenCodeServeConfig) async {
    events.append(.start(config))
    state = .running
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
private final class IntegrationStatusBarProvider: StatusBarProviding {
  func makeStatusItem(length: CGFloat) -> StatusItemType {
    IntegrationStatusItem()
  }
}

@MainActor
private final class IntegrationStatusItem: StatusItemType {
  let button: NSStatusBarButton? = NSStatusBarButton(frame: NSRect(x: 0, y: 0, width: 24, height: 24))
  var menu: NSMenu?
}

@MainActor
private final class IntegrationLoginItemService: LoginItemService {
  func currentStatus() throws -> LoginItemStatus { .disabled }
  func setEnabled(_ isEnabled: Bool) throws {}
}

private struct IntegrationUpdateChecker: UpdateChecker {
  func checkForUpdates() async throws -> UpdateInfo { throw NSError(domain: "IntegrationUpdateChecker", code: 0) }
  func downloadUpdate(_ updateInfo: UpdateInfo) async throws -> URL { throw NSError(domain: "IntegrationUpdateChecker", code: 0) }
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
