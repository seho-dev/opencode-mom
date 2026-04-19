import AppKit
import SwiftUI

@MainActor
final class SettingsWindowController: NSWindowController {
  let appStore: AppStore

  convenience init() {
    self.init(appStore: .livePreview)
  }

  init(appStore: AppStore) {
    self.appStore = appStore
    let rootView = SettingsView(appStore: appStore)
    let hostingController = NSHostingController(rootView: rootView)
    let window = NSWindow(contentViewController: hostingController)
    window.title = "Settings"
    window.setContentSize(NSSize(width: 700, height: 500))
    window.styleMask = [.titled, .closable, .miniaturizable, .resizable]
    window.isReleasedWhenClosed = false
    super.init(window: window)
    shouldCascadeWindows = true
  }

  @available(*, unavailable)
  required init?(coder: NSCoder) {
    fatalError("init(coder:) has not been implemented")
  }

  override func showWindow(_ sender: Any?) {
    super.showWindow(sender)
    window?.makeKeyAndOrderFront(sender)
  }
}
