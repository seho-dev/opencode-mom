import AppKit

MainActor.assumeIsolated {
  OMOSwitchApp.bootstrap(application: .shared)
}
_ = NSApplicationMain(CommandLine.argc, CommandLine.unsafeArgv)
