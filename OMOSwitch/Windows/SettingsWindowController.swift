import AppKit
import SwiftUI

@MainActor
final class SettingsWindowController: NSWindowController {
  enum Kind {
    case global
    case group

    var title: String {
      switch self {
      case .global:
        ""
      case .group:
        "Group Settings"
      }
    }

    var defaultSize: NSSize {
      switch self {
      case .global:
        NSSize(width: 420, height: 220)
      case .group:
        NSSize(width: 700, height: 500)
      }
    }
  }

  let appStore: AppStore
  let kind: Kind

  convenience init() {
    self.init(appStore: .livePreview, kind: .group)
  }

  init(appStore: AppStore, kind: Kind = .group) {
    self.appStore = appStore
    self.kind = kind
    let rootView: AnyView
    switch kind {
    case .global:
      rootView = AnyView(GlobalSettingsView(appStore: appStore))
    case .group:
      rootView = AnyView(SettingsView(appStore: appStore))
    }
    let hostingController = NSHostingController(rootView: rootView)
    let window = NSWindow(contentViewController: hostingController)
    window.title = kind.title
    window.setContentSize(kind.defaultSize)
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
