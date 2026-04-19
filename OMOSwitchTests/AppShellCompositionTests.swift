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
      settingsWindowController: SettingsWindowController(),
      configRootURL: rootURL,
    )
    try! container.modelGroupRepository.save([currentGroup])
    try! container.appStateRepository.save(AppSelectionState(selectedGroupID: currentGroup.id, selectedGroupName: currentGroup.name))
    let appDelegate = AppDelegate(container: container)

    appDelegate.applicationDidFinishLaunching(Notification(name: NSApplication.didFinishLaunchingNotification))

    XCTAssertNotNil(appDelegate.statusItemController)
    XCTAssertEqual(fakeStatusItem.button?.title, "OMO")
    XCTAssertEqual(appDelegate.statusItemController?.currentMenuTitles(), ["Current Group: Primary", "Open Settings", "Reload", "Quit"])
  }

  func testDependencyContainerReusesSingleSettingsWindowController() {
    let fakeStatusItem = FakeStatusItem()
    let sharedSettingsWindowController = SettingsWindowController(appStore: makeStore())
    let container = DependencyContainer(
      statusBarProvider: FakeStatusBarProvider(statusItem: fakeStatusItem),
      popoverControllerFactory: { QuickSwitchPopoverController(popover: NSPopover()) },
      settingsWindowController: sharedSettingsWindowController,
    )
    let appDelegate = AppDelegate(container: container)

    appDelegate.applicationDidFinishLaunching(Notification(name: NSApplication.didFinishLaunchingNotification))

    let first = container.sharedSettingsWindowController()
    let second = appDelegate.statusItemController?.resolveSettingsWindowController()

    XCTAssertTrue(first === sharedSettingsWindowController)
    XCTAssertTrue(first === second)
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
      ohMyConfigRepository: OhMyOpenAgentConfigRepository(configRootURL: rootURL),
    )
    return AppStore(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      switchUseCase: switchUseCase,
    )
  }
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
