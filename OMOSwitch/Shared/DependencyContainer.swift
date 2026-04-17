import AppKit

@MainActor
final class DependencyContainer {
  static let live = DependencyContainer()

  private let statusBarProvider: StatusBarProviding
  private let popoverControllerFactory: () -> QuickSwitchPopoverController
  private let settingsWindowControllerFactory: () -> SettingsWindowController

  init() {
    let sharedSettingsWindowController = SettingsWindowController()
    self.statusBarProvider = CocoaStatusBarProvider()
    self.popoverControllerFactory = { QuickSwitchPopoverController() }
    self.settingsWindowControllerFactory = { sharedSettingsWindowController }
  }

  init(
    statusBarProvider: StatusBarProviding,
    popoverControllerFactory: @escaping () -> QuickSwitchPopoverController,
    settingsWindowController: SettingsWindowController
  ) {
    let sharedSettingsWindowController = settingsWindowController
    self.statusBarProvider = statusBarProvider
    self.popoverControllerFactory = popoverControllerFactory
    self.settingsWindowControllerFactory = { sharedSettingsWindowController }
  }

  func makeStatusItemController() -> StatusItemController {
    StatusItemController(
      statusBarProvider: statusBarProvider,
      popoverController: popoverControllerFactory(),
      settingsWindowControllerProvider: settingsWindowControllerFactory,
    )
  }

  func sharedSettingsWindowController() -> SettingsWindowController {
    settingsWindowControllerFactory()
  }
}
