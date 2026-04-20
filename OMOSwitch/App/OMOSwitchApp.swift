import AppKit

enum OMOSwitchApp {
  static func bootstrap(application: NSApplication) {
    let delegate = MainActor.assumeIsolated { AppDelegate() }
    application.setActivationPolicy(.accessory)
    application.delegate = delegate
  }

}
