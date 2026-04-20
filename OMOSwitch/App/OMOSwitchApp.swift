import AppKit

enum OMOSwitchApp {
  @MainActor
  private static var retainedDelegate: AppDelegate?

  nonisolated
  static func bootstrap(application: NSApplication) {
    let delegate = AppDelegate()
    retainedDelegate = delegate
    application.setActivationPolicy(.accessory)
    application.delegate = delegate
  }

}
