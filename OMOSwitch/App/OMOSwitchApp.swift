import AppKit

enum OMOSwitchApp {
  private static var retainedDelegate: AppDelegate?

  static func bootstrap(application: NSApplication) {
    MainActor.assumeIsolated {
      let delegate = AppDelegate()
      retainedDelegate = delegate
      application.setActivationPolicy(.accessory)
      application.delegate = delegate
    }
  }

}
