import AppKit
import SwiftUI

@MainActor
final class QuickSwitchPopoverController {
  let popover: NSPopover
  let appStore: AppStore
  var onOpenSettings: (() -> Void)?

  convenience init() {
    self.init(appStore: .livePreview)
  }

  init(appStore: AppStore) {
    self.appStore = appStore
    self.popover = NSPopover()
    configurePopover()
  }

  init(popover: NSPopover, appStore: AppStore? = nil) {
    self.appStore = appStore ?? .livePreview
    self.popover = popover
    configurePopover()
  }

  func toggle(relativeTo positioningRect: NSRect, of positioningView: NSView, preferredEdge: NSRectEdge) {
    if popover.isShown {
      popover.performClose(nil)
      return
    }

    appStore.reload()
    popover.show(relativeTo: positioningRect, of: positioningView, preferredEdge: preferredEdge)
  }

  func reloadContent() {
    let view = QuickSwitchView(appStore: appStore, onOpenSettings: openSettingsAction)
    popover.contentViewController = NSHostingController(rootView: view)
  }

  private var openSettingsAction: (() -> Void) {
    { [weak self] in
      self?.onOpenSettings?()
    }
  }

  private func configurePopover() {
    popover.behavior = .transient
    popover.contentSize = NSSize(width: 320, height: 480)
    let view = QuickSwitchView(appStore: appStore, onOpenSettings: openSettingsAction)
    popover.contentViewController = NSHostingController(rootView: view)
  }
}
