import AppKit

@MainActor
final class AppDelegate: NSObject, NSApplicationDelegate {
  let container: DependencyContainer
  private(set) var statusItemController: StatusItemController?

  override init() {
    self.container = .live
    super.init()
  }

  init(container: DependencyContainer) {
    self.container = container
    super.init()
  }

  func applicationDidFinishLaunching(_ notification: Notification) {
    buildMainMenu()
    statusItemController = container.makeStatusItemController()
  }

  private func buildMainMenu() {
    let mainMenu = NSMenu()
    let appItem = NSMenuItem()
    mainMenu.addItem(appItem)
    let appMenu = NSMenu()
    appMenu.addItem(NSMenuItem(title: "Hide OMO Switch", action: #selector(NSApplication.hide(_:)), keyEquivalent: "h"))
    appMenu.addItem(NSMenuItem.separator())
    appMenu.addItem(NSMenuItem(title: "Quit OMO Switch", action: #selector(NSApplication.terminate(_:)), keyEquivalent: "q"))
    appItem.submenu = appMenu

    let editItem = NSMenuItem()
    mainMenu.addItem(editItem)
    let editMenu = NSMenu(title: "Edit")
    editMenu.addItem(NSMenuItem(title: "Cut", action: #selector(NSText.cut(_:)), keyEquivalent: "x"))
    editMenu.addItem(NSMenuItem(title: "Copy", action: #selector(NSText.copy(_:)), keyEquivalent: "c"))
    editMenu.addItem(NSMenuItem(title: "Paste", action: #selector(NSText.paste(_:)), keyEquivalent: "v"))
    editMenu.addItem(NSMenuItem(title: "Select All", action: #selector(NSText.selectAll(_:)), keyEquivalent: "a"))
    editItem.submenu = editMenu

    NSApp.mainMenu = mainMenu
  }
}
