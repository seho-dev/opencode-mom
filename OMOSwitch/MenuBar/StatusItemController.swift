import AppKit
import Combine

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
  private let globalSettingsWindowControllerProvider: () -> SettingsWindowController
  private let groupSettingsWindowControllerProvider: () -> SettingsWindowController
  private var serverConfigWindowController: SettingsWindowController?
  private var openCodeServeStatusCancellable: AnyCancellable?
  private(set) lazy var statusMenu: NSMenu = makeStatusMenu()

  private static let serverStatusRole = "serverStatus"
  private static let serverConfigRole = "serverConfig"
  private static let serverActionRole = "serverAction"

  override init() {
    let appStore = AppStore.livePreview
    self.statusItem = CocoaStatusBarProvider().makeStatusItem(length: NSStatusItem.variableLength)
    self.appStore = appStore
    self.popoverController = QuickSwitchPopoverController(appStore: appStore)
    self.globalSettingsWindowControllerProvider = { SettingsWindowController(appStore: appStore, kind: .global) }
    self.groupSettingsWindowControllerProvider = { SettingsWindowController(appStore: appStore, kind: .group) }
    super.init()
    observeOpenCodeServeStatus()
    configureStatusItem()
  }

  init(
    statusBarProvider: StatusBarProviding,
    appStore: AppStore,
    popoverController: QuickSwitchPopoverController,
    globalSettingsWindowControllerProvider: @escaping () -> SettingsWindowController,
    groupSettingsWindowControllerProvider: @escaping () -> SettingsWindowController
  ) {
    self.appStore = appStore
    self.statusItem = statusBarProvider.makeStatusItem(length: NSStatusItem.variableLength)
    self.popoverController = popoverController
    self.globalSettingsWindowControllerProvider = globalSettingsWindowControllerProvider
    self.groupSettingsWindowControllerProvider = groupSettingsWindowControllerProvider
    super.init()
    popoverController.onOpenGlobalSettings = { [weak self] in self?.openGlobalSettings() }
    popoverController.onOpenGroupSettings = { [weak self] in self?.openGroupSettings() }
    observeOpenCodeServeStatus()
    configureStatusItem()
  }

  func currentMenuTitles() -> [String] {
    statusMenu.items.map(\.title)
  }

  func resolveGlobalSettingsWindowController() -> SettingsWindowController {
    globalSettingsWindowControllerProvider()
  }

  func resolveGroupSettingsWindowController() -> SettingsWindowController {
    groupSettingsWindowControllerProvider()
  }

  func resolveServerConfigWindowController() -> SettingsWindowController {
    let controller = serverConfigWindowController ?? SettingsWindowController(appStore: appStore, kind: .serverConfig)
    serverConfigWindowController = controller
    return controller
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
    let serverStatusItem = NSMenuItem(title: openCodeServeStatusMenuTitle(), action: nil, keyEquivalent: "")
    serverStatusItem.isEnabled = false
    serverStatusItem.representedObject = Self.serverStatusRole
    menu.addItem(serverStatusItem)
    let serverConfigItem = NSMenuItem(title: "Server Config", action: #selector(openServerConfig), keyEquivalent: "")
    serverConfigItem.representedObject = Self.serverConfigRole
    menu.addItem(serverConfigItem)
    let serverActionItem = NSMenuItem(title: openCodeServeActionMenuTitle(), action: openCodeServeActionSelector(), keyEquivalent: "")
    serverActionItem.representedObject = Self.serverActionRole
    menu.addItem(serverActionItem)
    menu.addItem(NSMenuItem(title: "Global Settings", action: #selector(openGlobalSettings), keyEquivalent: ","))
    menu.addItem(NSMenuItem(title: "Group Settings", action: #selector(openGroupSettings), keyEquivalent: ""))
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
    refreshServerMenuItems()
    rebuildGroupMenuItems()
  }

  private func observeOpenCodeServeStatus() {
    openCodeServeStatusCancellable = appStore.$openCodeServeStatus.sink { [weak self] _ in
      Task { @MainActor [weak self] in
        self?.refreshMenuState()
      }
    }
  }

  private func refreshServerMenuItems() {
    item(representedObject: Self.serverStatusRole)?.title = openCodeServeStatusMenuTitle()
    if let serverActionItem = item(representedObject: Self.serverActionRole) {
      serverActionItem.title = openCodeServeActionMenuTitle()
      serverActionItem.action = openCodeServeActionSelector()
      serverActionItem.isEnabled = appStore.openCodeServeStatus != .stopping
      serverActionItem.target = self
    }
  }

  private func rebuildGroupMenuItems() {
    statusMenu.items
      .filter { $0.representedObject is UUID }
      .forEach { statusMenu.removeItem($0) }

    let enabledGroups = appStore.groups.filter(\.isEnabled)
    let groupInsertionIndex = min(2, statusMenu.items.count)
    for group in enabledGroups.reversed() {
      let menuItem = NSMenuItem(title: group.name, action: #selector(switchToGroup(_:)), keyEquivalent: "")
      menuItem.target = self
      menuItem.representedObject = group.id
      menuItem.state = group.id == appStore.currentGroupID ? .on : .off
      statusMenu.insertItem(menuItem, at: groupInsertionIndex)
    }
  }

  private func currentGroupMenuTitle() -> String {
    "Current Group: \(appStore.currentGroupName ?? "None")"
  }

  private func openCodeServeStatusMenuTitle() -> String {
    switch appStore.openCodeServeStatus {
    case .stopped:
      "OpenCode Server: Stopped"
    case .starting:
      "OpenCode Server: Starting..."
    case .running:
      "OpenCode Server: Running"
    case .stopping:
      "OpenCode Server: Stopping..."
    case .failed(let reason):
      "OpenCode Server: Server failed: \(reason)"
    }
  }

  private func openCodeServeActionMenuTitle() -> String {
    switch appStore.openCodeServeStatus {
    case .stopped, .failed:
      "Start Server"
    case .starting, .running, .stopping:
      "Stop Server"
    }
  }

  private func openCodeServeActionSelector() -> Selector {
    switch appStore.openCodeServeStatus {
    case .stopped, .failed:
      #selector(startServer)
    case .starting, .running, .stopping:
      #selector(stopServer)
    }
  }

  private func item(representedObject: String) -> NSMenuItem? {
    statusMenu.items.first { ($0.representedObject as? String) == representedObject }
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
  func openGlobalSettings() {
    NSApplication.shared.activate(ignoringOtherApps: true)
    let controller = globalSettingsWindowControllerProvider()
    controller.showWindow(nil)
  }

  @objc
  func openGroupSettings() {
    NSApplication.shared.activate(ignoringOtherApps: true)
    let controller = groupSettingsWindowControllerProvider()
    controller.showWindow(nil)
  }

  @objc
  func openServerConfig() {
    NSApplication.shared.activate(ignoringOtherApps: true)
    let controller = resolveServerConfigWindowController()
    controller.showWindow(nil)
  }

  @objc
  private func startServer() {
    Task { @MainActor [weak self] in
      guard let self else { return }
      appStore.startServer()
      refreshMenuState()
      popoverController.reloadContent()
    }
  }

  @objc
  private func stopServer() {
    Task { @MainActor [weak self] in
      guard let self else { return }
      appStore.stopServer()
      refreshMenuState()
      popoverController.reloadContent()
    }
  }

  @objc
  private func reload() {
    appStore.reload()
    refreshMenuState()
    popoverController.reloadContent()
  }

  @objc
  private func switchToGroup(_ sender: NSMenuItem) {
    guard let groupID = sender.representedObject as? UUID else {
      return
    }

    Task { @MainActor [weak self] in
      guard let self else { return }
      await appStore.switchTo(groupID: groupID)
      refreshMenuState()
      popoverController.reloadContent()
    }
  }

  @objc
  private func quit() {
    NSApplication.shared.terminate(nil)
  }
}
