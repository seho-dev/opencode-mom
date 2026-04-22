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
    XCTAssertEqual(appDelegate.statusItemController?.currentMenuTitles(), ["Current Group: Primary", "Primary", "Global Settings", "Group Settings", "Reload", "Quit"])
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

    XCTAssertEqual(controller.currentMenuTitles(), ["Current Group: Current", "Current", "Enabled", "Global Settings", "Group Settings", "Reload", "Quit"])
    XCTAssertFalse(controller.statusMenu.items.contains(where: { $0.title == "Disabled" }))
    XCTAssertEqual(controller.statusMenu.items[1].state, .on)
    XCTAssertEqual(controller.statusMenu.items[1].representedObject as? UUID, currentGroup.id)
    XCTAssertEqual(controller.statusMenu.items[2].state, .off)
    XCTAssertEqual(controller.statusMenu.items[2].representedObject as? UUID, enabledGroup.id)
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

  private func makeStore() -> AppStore {
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
    )
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
