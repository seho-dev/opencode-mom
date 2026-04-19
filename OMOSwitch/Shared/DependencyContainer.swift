import AppKit

@MainActor
final class DependencyContainer {
  static let live = DependencyContainer()

  let modelGroupRepository: ModelGroupRepository
  let appStateRepository: AppStateRepository
  let backupRepository: BackupRepository
  let ohMyConfigRepository: OhMyOpenAgentConfigRepository
  let switchUseCase: SwitchGroupUseCase
  let appStore: AppStore

  private let statusBarProvider: StatusBarProviding
  private let popoverControllerFactory: () -> QuickSwitchPopoverController
  private let settingsWindowControllerFactory: () -> SettingsWindowController

  init() {
    let modelGroupRepository = ModelGroupRepository()
    let appStateRepository = AppStateRepository()
    let backupRepository = BackupRepository()
    let ohMyConfigRepository = OhMyOpenAgentConfigRepository()
    let switchUseCase = SwitchGroupUseCase(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      backupRepository: backupRepository,
      ohMyConfigRepository: ohMyConfigRepository,
    )
    let appStore = AppStore(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      switchUseCase: switchUseCase,
    )
    let sharedSettingsWindowController = SettingsWindowController(appStore: appStore)
    self.modelGroupRepository = modelGroupRepository
    self.appStateRepository = appStateRepository
    self.backupRepository = backupRepository
    self.ohMyConfigRepository = ohMyConfigRepository
    self.switchUseCase = switchUseCase
    self.appStore = appStore
    self.statusBarProvider = CocoaStatusBarProvider()
    self.popoverControllerFactory = { QuickSwitchPopoverController(appStore: appStore) }
    self.settingsWindowControllerFactory = { sharedSettingsWindowController }
  }

  init(
    statusBarProvider: StatusBarProviding,
    popoverControllerFactory: @escaping () -> QuickSwitchPopoverController,
    settingsWindowController: SettingsWindowController,
    configRootURL: URL? = nil
  ) {
    let modelGroupRepository = ModelGroupRepository(configRootURL: configRootURL)
    let appStateRepository = AppStateRepository(configRootURL: configRootURL)
    let backupRepository = BackupRepository(configRootURL: configRootURL)
    let ohMyConfigRepository = OhMyOpenAgentConfigRepository(configRootURL: configRootURL)
    let switchUseCase = SwitchGroupUseCase(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      backupRepository: backupRepository,
      ohMyConfigRepository: ohMyConfigRepository,
    )
    let appStore = AppStore(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      switchUseCase: switchUseCase,
    )
    let sharedSettingsWindowController = settingsWindowController
    self.modelGroupRepository = modelGroupRepository
    self.appStateRepository = appStateRepository
    self.backupRepository = backupRepository
    self.ohMyConfigRepository = ohMyConfigRepository
    self.switchUseCase = switchUseCase
    self.appStore = appStore
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
