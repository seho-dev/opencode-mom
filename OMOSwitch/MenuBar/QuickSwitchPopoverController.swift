import AppKit
import SwiftUI

private struct QuickSwitchPlaceholderView: View {
  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      Text("Quick Switch")
        .font(.headline)
      Text("Placeholder")
        .foregroundStyle(.secondary)
    }
    .frame(width: 240, height: 120, alignment: .topLeading)
    .padding(16)
  }
}

@MainActor
final class QuickSwitchPopoverController {
  let popover: NSPopover
  let appStore: AppStore

  convenience init() {
    self.init(appStore: .livePreview)
  }

  init(appStore: AppStore) {
    self.appStore = appStore
    self.popover = NSPopover()
    configurePopover()
  }

  init(popover: NSPopover, appStore: AppStore = AppStore.livePreview) {
    self.appStore = appStore
    self.popover = popover
    configurePopover()
  }

  func toggle(relativeTo positioningRect: NSRect, of positioningView: NSView, preferredEdge: NSRectEdge) {
    if popover.isShown {
      popover.performClose(nil)
      return
    }

    popover.show(relativeTo: positioningRect, of: positioningView, preferredEdge: preferredEdge)
  }

  func reloadPlaceholderContent() {
    popover.contentViewController = NSHostingController(rootView: QuickSwitchPlaceholderView())
  }

  private func configurePopover() {
    popover.behavior = .transient
    popover.contentSize = NSSize(width: 240, height: 120)
    popover.contentViewController = NSHostingController(rootView: QuickSwitchPlaceholderView())
  }
}
