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

@MainActor
struct CocoaStatusBarProvider: StatusBarProviding {
  func makeStatusItem(length: CGFloat) -> StatusItemType {
    CocoaStatusItemAdapter(statusItem: NSStatusBar.system.statusItem(withLength: length))
  }
}

@MainActor
final class StatusItemController: NSObject, NSMenuDelegate {
  let statusItem: StatusItemType
  let popoverController: QuickSwitchPopoverController
  let appStore: AppStore
  private let settingsWindowControllerProvider: () -> SettingsWindowController
  private(set) lazy var statusMenu: NSMenu = makeStatusMenu()

  override init() {
    self.statusItem = CocoaStatusBarProvider().makeStatusItem(length: NSStatusItem.variableLength)
    self.appStore = .livePreview
    self.popoverController = QuickSwitchPopoverController(appStore: appStore)
    self.settingsWindowControllerProvider = { SettingsWindowController() }
    super.init()
    configureStatusItem()
  }

  init(
    statusBarProvider: StatusBarProviding,
    appStore: AppStore,
    popoverController: QuickSwitchPopoverController,
    settingsWindowControllerProvider: @escaping () -> SettingsWindowController
  ) {
    self.appStore = appStore
    self.statusItem = statusBarProvider.makeStatusItem(length: NSStatusItem.variableLength)
    self.popoverController = popoverController
    self.settingsWindowControllerProvider = settingsWindowControllerProvider
    super.init()
    popoverController.onOpenSettings = { [weak self] in self?.openSettings() }
    configureStatusItem()
  }

  func currentMenuTitles() -> [String] {
    statusMenu.items.map(\.title)
  }

  func resolveSettingsWindowController() -> SettingsWindowController {
    settingsWindowControllerProvider()
  }

  private func configureStatusItem() {
    appStore.reload()
    statusItem.button?.title = "OMO"
    statusItem.menu = statusMenu
    refreshMenuState()
  }

  private func makeStatusMenu() -> NSMenu {
    let menu = NSMenu()
    menu.delegate = self

    let currentGroupItem = NSMenuItem(title: currentGroupMenuTitle(), action: nil, keyEquivalent: "")
    currentGroupItem.isEnabled = false
    menu.addItem(currentGroupItem)
    menu.addItem(NSMenuItem(title: "Open Settings", action: #selector(openSettings), keyEquivalent: ","))
    menu.addItem(NSMenuItem(title: "Reload", action: #selector(reload), keyEquivalent: "r"))
    menu.addItem(NSMenuItem(title: "Quit", action: #selector(quit), keyEquivalent: "q"))
    menu.items.forEach {
      if $0.action != nil {
        $0.target = self
      }
    }
    return menu
  }

  func menuWillOpen(_ menu: NSMenu) {
    refreshMenuState()
  }

  private func refreshMenuState() {
    statusMenu.items.first?.title = currentGroupMenuTitle()
  }

  private func currentGroupMenuTitle() -> String {
    "Current Group: \(appStore.currentGroupName ?? "None")"
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
    appStore.reload()
    refreshMenuState()
    popoverController.reloadContent()
  }

  @objc
  private func quit() {
    NSApplication.shared.terminate(nil)
  }
}
