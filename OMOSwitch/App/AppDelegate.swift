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
    statusItemController = container.makeStatusItemController()
  }
}
