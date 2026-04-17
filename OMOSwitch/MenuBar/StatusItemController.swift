import AppKit

@MainActor
protocol StatusItemType: AnyObject {
  var button: NSStatusBarButton? { get }
  var menu: NSMenu? { get set }
}

@MainActor
protocol StatusBarProviding {
  func makeStatusItem(length: CGFloat) -> StatusItemType
}

final class CocoaStatusItemAdapter: StatusItemType {
  private let statusItem: NSStatusItem

  init(statusItem: NSStatusItem) {
    self.statusItem = statusItem
  }

  var button: NSStatusBarButton? {
    statusItem.button
  }

  var menu: NSMenu? {
    get { statusItem.menu }
    set { statusItem.menu = newValue }
  }
}

struct CocoaStatusBarProvider: StatusBarProviding {
  func makeStatusItem(length: CGFloat) -> StatusItemType {
    CocoaStatusItemAdapter(statusItem: NSStatusBar.system.statusItem(withLength: length))
  }
}

@MainActor
final class StatusItemController: NSObject {
  let statusItem: StatusItemType
  let popoverController: QuickSwitchPopoverController
  private let settingsWindowControllerProvider: () -> SettingsWindowController
  private(set) lazy var statusMenu: NSMenu = makeStatusMenu()

  override init() {
    self.statusItem = CocoaStatusBarProvider().makeStatusItem(length: NSStatusItem.variableLength)
    self.popoverController = QuickSwitchPopoverController()
    self.settingsWindowControllerProvider = { SettingsWindowController() }
    super.init()
    configureStatusItem()
  }

  init(
    statusBarProvider: StatusBarProviding,
    popoverController: QuickSwitchPopoverController,
    settingsWindowControllerProvider: @escaping () -> SettingsWindowController
  ) {
    self.statusItem = statusBarProvider.makeStatusItem(length: NSStatusItem.variableLength)
    self.popoverController = popoverController
    self.settingsWindowControllerProvider = settingsWindowControllerProvider
    super.init()
    configureStatusItem()
  }

  func currentMenuTitles() -> [String] {
    statusMenu.items.map(\.title)
  }

  func resolveSettingsWindowController() -> SettingsWindowController {
    settingsWindowControllerProvider()
  }

  private func configureStatusItem() {
    statusItem.button?.title = "OMO"
    statusItem.menu = statusMenu
  }

  private func makeStatusMenu() -> NSMenu {
    let menu = NSMenu()
    menu.addItem(NSMenuItem(title: "Current Group", action: #selector(toggleQuickSwitchPopover), keyEquivalent: ""))
    menu.addItem(NSMenuItem(title: "Open Settings", action: #selector(openSettings), keyEquivalent: ","))
    menu.addItem(NSMenuItem(title: "Reload", action: #selector(reload), keyEquivalent: "r"))
    menu.addItem(NSMenuItem(title: "Quit", action: #selector(quit), keyEquivalent: "q"))
    menu.items.forEach { $0.target = self }
    return menu
  }

  @objc
  private func toggleQuickSwitchPopover() {
    guard let button = statusItem.button else {
      return
    }

    popoverController.toggle(
      relativeTo: button.bounds,
      of: button,
      preferredEdge: .minY,
    )
  }

  @objc
  func openSettings() {
    let controller = settingsWindowControllerProvider()
    controller.showWindow(nil)
    NSApplication.shared.activate(ignoringOtherApps: true)
  }

  @objc
  private func reload() {
    popoverController.reloadPlaceholderContent()
  }

  @objc
  private func quit() {
    NSApplication.shared.terminate(nil)
  }
}
