import AppKit
import XCTest
@testable import OMOSwitch

@MainActor
final class AppShellCompositionTests: XCTestCase {
  func testAppDelegateCreatesStatusItemWithExpectedMenuTitles() {
    let fakeStatusItem = FakeStatusItem()
    let container = DependencyContainer(
      statusBarProvider: FakeStatusBarProvider(statusItem: fakeStatusItem),
      popoverControllerFactory: { QuickSwitchPopoverController(popover: NSPopover()) },
      settingsWindowController: SettingsWindowController(),
    )
    let appDelegate = AppDelegate(container: container)

    appDelegate.applicationDidFinishLaunching(Notification(name: NSApplication.didFinishLaunchingNotification))

    XCTAssertNotNil(appDelegate.statusItemController)
    XCTAssertEqual(fakeStatusItem.button?.title, "OMO")
    XCTAssertEqual(appDelegate.statusItemController?.currentMenuTitles(), ["Current Group", "Open Settings", "Reload", "Quit"])
  }

  func testDependencyContainerReusesSingleSettingsWindowController() {
    let fakeStatusItem = FakeStatusItem()
    let sharedSettingsWindowController = SettingsWindowController()
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
