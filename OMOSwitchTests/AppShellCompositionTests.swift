import AppKit
import XCTest
@testable import OMOSwitch

@MainActor
final class AppShellCompositionTests: XCTestCase {
  nonisolated(unsafe) private var tempRootURLs: [URL] = []

  override func tearDown() {
    tempRootURLs.forEach(TestSupport.removeIfExists)
    tempRootURLs.removeAll()
    super.tearDown()
  }

  func testAppDelegateCreatesStatusItemWithExpectedMenuTitles() {
    let fakeStatusItem = FakeStatusItem()
    let rootURL = try! TestSupport.makeTemporaryDirectory()
    tempRootURLs.append(rootURL)
    let currentGroup = ModelGroup(
      id: UUID(uuidString: "99999999-9999-9999-9999-999999999999")!,
      name: "Primary",
      categoryMappings: [ModelGroupCategoryMapping(categoryName: "quick", modelRef: "model")],
      updatedAt: Date(timeIntervalSince1970: 1_700_000_000),
    )
    let container = DependencyContainer(
      statusBarProvider: FakeStatusBarProvider(statusItem: fakeStatusItem),
      popoverControllerFactory: { QuickSwitchPopoverController(popover: NSPopover()) },
      globalSettingsWindowController: SettingsWindowController(appStore: .livePreview, kind: .global),
      groupSettingsWindowController: SettingsWindowController(appStore: .livePreview, kind: .group),
      configRootURL: rootURL,
    )
    try! container.modelGroupRepository.save([currentGroup])
    try! container.appStateRepository.save(AppSelectionState(selectedGroupID: currentGroup.id, selectedGroupName: currentGroup.name))
    let appDelegate = AppDelegate(container: container)

    appDelegate.applicationDidFinishLaunching(Notification(name: NSApplication.didFinishLaunchingNotification))

    XCTAssertNotNil(appDelegate.statusItemController)
    XCTAssertEqual(fakeStatusItem.button?.title, "OMO")
    XCTAssertEqual(appDelegate.statusItemController?.currentMenuTitles(), ["Current Group: Primary", "OpenCode Server: Stopped", "Primary", "Server Config", "Start Server", "Global Settings", "Group Settings", "Reload", "Quit"])
  }

  func testStatusMenuShowsOnlyEnabledGroupsAndChecksCurrentGroup() {
    let fakeStatusItem = FakeStatusItem()
    let rootURL = try! TestSupport.makeTemporaryDirectory()
    tempRootURLs.append(rootURL)

    let currentGroup = ModelGroup(
      id: UUID(uuidString: "11111111-1111-1111-1111-111111111111")!,
      name: "Current",
      categoryMappings: [],
      isEnabled: true,
      updatedAt: Date(timeIntervalSince1970: 1_700_000_000),
    )
    let enabledGroup = ModelGroup(
      id: UUID(uuidString: "22222222-2222-2222-2222-222222222222")!,
      name: "Enabled",
      categoryMappings: [],
      isEnabled: true,
      updatedAt: Date(timeIntervalSince1970: 1_700_000_000),
    )
    let disabledGroup = ModelGroup(
      id: UUID(uuidString: "33333333-3333-3333-3333-333333333333")!,
      name: "Disabled",
      categoryMappings: [],
      isEnabled: false,
      updatedAt: Date(timeIntervalSince1970: 1_700_000_000),
    )

    let modelGroupRepository = ModelGroupRepository(configRootURL: rootURL)
    let appStateRepository = AppStateRepository(configRootURL: rootURL)
    let switchUseCase = SwitchGroupUseCase(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      backupRepository: BackupRepository(configRootURL: rootURL),
      openCodeConfigRepository: OpenCodeConfigRepository(configRootURL: rootURL),
      ohMyConfigRepository: OhMyOpenAgentConfigRepository(configRootURL: rootURL),
    )
    let appStore = AppStore(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      openCodeConfigRepository: OpenCodeConfigRepository(configRootURL: rootURL),
      switchUseCase: switchUseCase,
      loginItemService: StubLoginItemService(),
      updateChecker: StubShellUpdateChecker()
    )
    let controller = StatusItemController(
      statusBarProvider: FakeStatusBarProvider(statusItem: fakeStatusItem),
      appStore: appStore,
      popoverController: QuickSwitchPopoverController(popover: NSPopover()),
      globalSettingsWindowControllerProvider: { SettingsWindowController(appStore: appStore, kind: .global) },
      groupSettingsWindowControllerProvider: { SettingsWindowController(appStore: appStore, kind: .group) }
    )

    try! modelGroupRepository.save([currentGroup, enabledGroup, disabledGroup])
    try! appStateRepository.save(AppSelectionState(selectedGroupID: currentGroup.id, selectedGroupName: currentGroup.name))
    appStore.reload()
    controller.menuWillOpen(controller.statusMenu)

    XCTAssertEqual(controller.currentMenuTitles(), ["Current Group: Current", "OpenCode Server: Stopped", "Current", "Enabled", "Server Config", "Start Server", "Global Settings", "Group Settings", "Reload", "Quit"])
    XCTAssertFalse(controller.statusMenu.items.contains(where: { $0.title == "Disabled" }))
    XCTAssertEqual(controller.statusMenu.items[2].state, .on)
    XCTAssertEqual(controller.statusMenu.items[2].representedObject as? UUID, currentGroup.id)
    XCTAssertEqual(controller.statusMenu.items[3].state, .off)
    XCTAssertEqual(controller.statusMenu.items[3].representedObject as? UUID, enabledGroup.id)
  }

  func testDependencyContainerReusesSingleSettingsWindowControllers() {
    let fakeStatusItem = FakeStatusItem()
    let store = makeStore()
    let sharedGlobalSettingsWindowController = SettingsWindowController(appStore: store, kind: .global)
    let sharedGroupSettingsWindowController = SettingsWindowController(appStore: store, kind: .group)
    let container = DependencyContainer(
      statusBarProvider: FakeStatusBarProvider(statusItem: fakeStatusItem),
      popoverControllerFactory: { QuickSwitchPopoverController(popover: NSPopover()) },
      globalSettingsWindowController: sharedGlobalSettingsWindowController,
      groupSettingsWindowController: sharedGroupSettingsWindowController,
    )
    let appDelegate = AppDelegate(container: container)

    appDelegate.applicationDidFinishLaunching(Notification(name: NSApplication.didFinishLaunchingNotification))

    let firstGlobal = container.sharedGlobalSettingsWindowController()
    let secondGlobal = appDelegate.statusItemController?.resolveGlobalSettingsWindowController()
    let firstGroup = container.sharedGroupSettingsWindowController()
    let secondGroup = appDelegate.statusItemController?.resolveGroupSettingsWindowController()

    XCTAssertTrue(firstGlobal === sharedGlobalSettingsWindowController)
    XCTAssertTrue(firstGlobal === secondGlobal)
    XCTAssertTrue(firstGroup === sharedGroupSettingsWindowController)
    XCTAssertTrue(firstGroup === secondGroup)
    XCTAssertEqual(firstGlobal.window?.title, "")
    XCTAssertEqual(firstGroup.window?.title, "Group Settings")
    XCTAssertEqual(firstGlobal.window?.isReleasedWhenClosed, false)
    XCTAssertEqual(firstGroup.window?.isReleasedWhenClosed, false)
  }

  func testStatusMenuShowsServerStateAndStartStopActionTitles() {
    let controller = makeStatusItemController()

    controller.appStore.openCodeServeStatus = .running
    controller.menuWillOpen(controller.statusMenu)
    XCTAssertTrue(controller.currentMenuTitles().contains("OpenCode Server: Running"))
    XCTAssertTrue(controller.currentMenuTitles().contains("Stop Server"))

    controller.appStore.openCodeServeStatus = .starting
    controller.menuWillOpen(controller.statusMenu)
    XCTAssertTrue(controller.currentMenuTitles().contains("OpenCode Server: Starting..."))
    XCTAssertTrue(controller.currentMenuTitles().contains("Stop Server"))

    controller.appStore.openCodeServeStatus = .failed(reason: "boom")
    controller.menuWillOpen(controller.statusMenu)
    XCTAssertTrue(controller.currentMenuTitles().contains("OpenCode Server: Server failed: boom"))
    XCTAssertTrue(controller.currentMenuTitles().contains("Start Server"))

    controller.appStore.openCodeServeStatus = .stopped
    controller.menuWillOpen(controller.statusMenu)
    XCTAssertTrue(controller.currentMenuTitles().contains("OpenCode Server: Stopped"))
    XCTAssertTrue(controller.currentMenuTitles().contains("Start Server"))
  }

  func testStartStopMenuItemsDispatchAppStoreActionsOnce() async throws {
    let manager = RecordingOpenCodeServeProcessManager(initialState: .stopped)
    let controller = makeStatusItemController(processManager: manager)
    controller.appStore.openCodeServeStatus = .stopped
    controller.menuWillOpen(controller.statusMenu)

    try performMenuItem(titled: "Start Server", in: controller)

    try await waitUntil { await manager.recordedStartCount() == 1 }
    let startCount = await manager.recordedStartCount()
    XCTAssertEqual(startCount, 1)

    controller.appStore.openCodeServeStatus = .running
    controller.menuWillOpen(controller.statusMenu)
    try performMenuItem(titled: "Stop Server", in: controller)

    try await waitUntil { await manager.recordedStopCount() == 1 }
    let stopCount = await manager.recordedStopCount()
    XCTAssertEqual(stopCount, 1)
  }

  func testServerConfigMenuItemReusesOneWindowController() throws {
    let fakeStatusItem = FakeStatusItem()
    let store = makeStore()
    let controller = StatusItemController(
      statusBarProvider: FakeStatusBarProvider(statusItem: fakeStatusItem),
      appStore: store,
      popoverController: QuickSwitchPopoverController(popover: NSPopover()),
      globalSettingsWindowControllerProvider: { SettingsWindowController(appStore: store, kind: .global) },
      groupSettingsWindowControllerProvider: { SettingsWindowController(appStore: store, kind: .group) }
    )

    try performMenuItem(titled: "Server Config", in: controller)
    let first = controller.resolveServerConfigWindowController()
    try performMenuItem(titled: "Server Config", in: controller)
    let second = controller.resolveServerConfigWindowController()

    XCTAssertTrue(first === second)
    XCTAssertEqual(first.kind, .serverConfig)
    XCTAssertEqual(first.window?.title, "Server Config")
    XCTAssertEqual(first.window?.isReleasedWhenClosed, false)
  }

  private func makeStore(
    processManager: any OpenCodeServeProcessManaging = RecordingOpenCodeServeProcessManager()
  ) -> AppStore {
    let rootURL = try! TestSupport.makeTemporaryDirectory()
    tempRootURLs.append(rootURL)
    let modelGroupRepository = ModelGroupRepository(configRootURL: rootURL)
    let appStateRepository = AppStateRepository(configRootURL: rootURL)
    let switchUseCase = SwitchGroupUseCase(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      backupRepository: BackupRepository(configRootURL: rootURL),
      openCodeConfigRepository: OpenCodeConfigRepository(configRootURL: rootURL),
      ohMyConfigRepository: OhMyOpenAgentConfigRepository(configRootURL: rootURL),
    )
    return AppStore(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      openCodeConfigRepository: OpenCodeConfigRepository(configRootURL: rootURL),
      switchUseCase: switchUseCase,
      loginItemService: StubLoginItemService(),
      updateChecker: StubShellUpdateChecker(),
      processManager: processManager
    )
  }

  private func makeStatusItemController(
    processManager: any OpenCodeServeProcessManaging = RecordingOpenCodeServeProcessManager()
  ) -> StatusItemController {
    let fakeStatusItem = FakeStatusItem()
    let store = makeStore(processManager: processManager)
    return StatusItemController(
      statusBarProvider: FakeStatusBarProvider(statusItem: fakeStatusItem),
      appStore: store,
      popoverController: QuickSwitchPopoverController(popover: NSPopover()),
      globalSettingsWindowControllerProvider: { SettingsWindowController(appStore: store, kind: .global) },
      groupSettingsWindowControllerProvider: { SettingsWindowController(appStore: store, kind: .group) }
    )
  }

  private func performMenuItem(titled title: String, in controller: StatusItemController) throws {
    let item = try XCTUnwrap(controller.statusMenu.items.first { $0.title == title })
    let action = try XCTUnwrap(item.action)
    let target = try XCTUnwrap(item.target)
    NSApplication.shared.sendAction(action, to: target, from: item)
  }
}

@MainActor
private struct StubLoginItemService: LoginItemService {
  func currentStatus() throws -> LoginItemStatus { .disabled }
  func setEnabled(_ isEnabled: Bool) throws {}
}

@MainActor
private final class FakeStatusBarProvider: StatusBarProviding {
  private let statusItem: FakeStatusItem

  init(statusItem: FakeStatusItem) {
    self.statusItem = statusItem
  }

  func makeStatusItem(length: CGFloat) -> StatusItemType {
    statusItem
  }
}

@MainActor
private final class FakeStatusItem: StatusItemType {
  let button: NSStatusBarButton?
  var menu: NSMenu?

  init() {
    button = NSStatusBarButton(frame: NSRect(x: 0, y: 0, width: 24, height: 24))
  }
}

private actor RecordingOpenCodeServeProcessManager: OpenCodeServeProcessManaging {
  private var state: OpenCodeServeProcessState
  private var startCount = 0
  private var stopCount = 0

  init(initialState: OpenCodeServeProcessState = .stopped) {
    self.state = initialState
  }

  func currentState() async -> OpenCodeServeProcessState {
    state
  }

  func start(config: OpenCodeServeConfig) async {
    startCount += 1
    state = .running
  }

  func restart(config: OpenCodeServeConfig) async {
    state = .running
  }

  func start(arguments: [String]) async {
    state = .running
  }

  func stop() async {
    stopCount += 1
    state = .stopped
  }

  func restart(arguments: [String]) async {
    state = .running
  }

  func recordedStartCount() -> Int {
    startCount
  }

  func recordedStopCount() -> Int {
    stopCount
  }
}

private struct StubShellUpdateChecker: UpdateChecker {
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
