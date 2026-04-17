import AppKit
import SwiftUI

private struct SettingsPlaceholderView: View {
  var body: some View {
    VStack(alignment: .leading, spacing: 12) {
      Text("Settings")
        .font(.title2)
      Text("Placeholder")
        .foregroundStyle(.secondary)
    }
    .frame(minWidth: 420, minHeight: 260, alignment: .topLeading)
    .padding(24)
  }
}

@MainActor
final class SettingsWindowController: NSWindowController {
  init() {
    let hostingController = NSHostingController(rootView: SettingsPlaceholderView())
    let window = NSWindow(contentViewController: hostingController)
    window.title = "Settings"
    window.setContentSize(NSSize(width: 420, height: 260))
    window.styleMask = [.titled, .closable, .miniaturizable, .resizable]
    window.isReleasedWhenClosed = false
    super.init(window: window)
    shouldCascadeWindows = true
  }

  init<Content: View>(rootView: Content) {
    let hostingController = NSHostingController(rootView: rootView)
    let window = NSWindow(contentViewController: hostingController)
    window.title = "Settings"
    window.setContentSize(NSSize(width: 420, height: 260))
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
