import AppKit

@MainActor
final class DependencyContainer {
  static let live = DependencyContainer()

  let modelGroupRepository: ModelGroupRepository
  let appStateRepository: AppStateRepository
  let backupRepository: BackupRepository
  let openCodeConfigRepository: OpenCodeConfigRepository
  let ohMyConfigRepository: OhMyOpenAgentConfigRepository
  let switchUseCase: SwitchGroupUseCase
  let appStore: AppStore
  let loginItemService: any LoginItemService
  let processManager: any OpenCodeServeProcessManaging

  private let statusBarProvider: StatusBarProviding
  private let popoverControllerFactory: () -> QuickSwitchPopoverController
  private let settingsWindowControllerFactory: () -> SettingsWindowController

  init() {
    let modelGroupRepository = ModelGroupRepository()
    let appStateRepository = AppStateRepository()
    let backupRepository = BackupRepository()
    let openCodeConfigRepository = OpenCodeConfigRepository()
    let ohMyConfigRepository = OhMyOpenAgentConfigRepository()
    let switchUseCase = SwitchGroupUseCase(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      backupRepository: backupRepository,
      openCodeConfigRepository: openCodeConfigRepository,
      ohMyConfigRepository: ohMyConfigRepository,
    )
    let loginItemService = SMAppServiceLoginItemService()
    let updateChecker = GitHubUpdateChecker()
    let processManager = OpenCodeServeProcessManager()
    let appStore = AppStore(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      openCodeConfigRepository: openCodeConfigRepository,
      switchUseCase: switchUseCase,
      loginItemService: loginItemService,
      updateChecker: updateChecker,
      processManager: processManager
    )
    let sharedSettingsWindowController = SettingsWindowController(appStore: appStore)
    self.modelGroupRepository = modelGroupRepository
    self.appStateRepository = appStateRepository
    self.backupRepository = backupRepository
    self.openCodeConfigRepository = openCodeConfigRepository
    self.ohMyConfigRepository = ohMyConfigRepository
    self.switchUseCase = switchUseCase
    self.appStore = appStore
    self.loginItemService = loginItemService
    self.processManager = processManager
    self.statusBarProvider = CocoaStatusBarProvider()
    self.popoverControllerFactory = { QuickSwitchPopoverController(appStore: appStore) }
    self.settingsWindowControllerFactory = { sharedSettingsWindowController }
  }

  init(
    statusBarProvider: StatusBarProviding,
    popoverControllerFactory: @escaping () -> QuickSwitchPopoverController,
    settingsWindowController: SettingsWindowController,
    loginItemService: (any LoginItemService)? = nil,
    updateChecker: (any UpdateChecker)? = nil,
    processManager: (any OpenCodeServeProcessManaging)? = nil,
    configRootURL: URL? = nil
  ) {
    let modelGroupRepository = ModelGroupRepository(configRootURL: configRootURL)
    let appStateRepository = AppStateRepository(configRootURL: configRootURL)
    let backupRepository = BackupRepository(configRootURL: configRootURL)
    let openCodeConfigRepository = OpenCodeConfigRepository(configRootURL: configRootURL)
    let ohMyConfigRepository = OhMyOpenAgentConfigRepository(configRootURL: configRootURL)
    let switchUseCase = SwitchGroupUseCase(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      backupRepository: backupRepository,
      openCodeConfigRepository: openCodeConfigRepository,
      ohMyConfigRepository: ohMyConfigRepository,
    )
    let resolvedLoginItemService = loginItemService ?? SMAppServiceLoginItemService()
    let resolvedUpdateChecker = updateChecker ?? GitHubUpdateChecker()
    let resolvedProcessManager = processManager ?? OpenCodeServeProcessManager()
    let appStore = AppStore(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      openCodeConfigRepository: openCodeConfigRepository,
      switchUseCase: switchUseCase,
      loginItemService: resolvedLoginItemService,
      updateChecker: resolvedUpdateChecker,
      processManager: resolvedProcessManager
    )
    let sharedSettingsWindowController = settingsWindowController
    self.modelGroupRepository = modelGroupRepository
    self.appStateRepository = appStateRepository
    self.backupRepository = backupRepository
    self.openCodeConfigRepository = openCodeConfigRepository
    self.ohMyConfigRepository = ohMyConfigRepository
    self.switchUseCase = switchUseCase
    self.appStore = appStore
    self.loginItemService = resolvedLoginItemService
    self.processManager = resolvedProcessManager
    self.statusBarProvider = statusBarProvider
    self.popoverControllerFactory = popoverControllerFactory
    self.settingsWindowControllerFactory = { sharedSettingsWindowController }
  }

  convenience init(
    statusBarProvider: StatusBarProviding,
    popoverControllerFactory: @escaping () -> QuickSwitchPopoverController,
    globalSettingsWindowController: SettingsWindowController,
    groupSettingsWindowController: SettingsWindowController,
    loginItemService: (any LoginItemService)? = nil,
    updateChecker: (any UpdateChecker)? = nil,
    processManager: (any OpenCodeServeProcessManaging)? = nil,
    configRootURL: URL? = nil
  ) {
    self.init(
      statusBarProvider: statusBarProvider,
      popoverControllerFactory: popoverControllerFactory,
      settingsWindowController: groupSettingsWindowController,
      loginItemService: loginItemService,
      updateChecker: updateChecker,
      processManager: processManager,
      configRootURL: configRootURL
    )
  }

  func makeStatusItemController() -> StatusItemController {
    StatusItemController(
      statusBarProvider: statusBarProvider,
      appStore: appStore,
      popoverController: popoverControllerFactory(),
      settingsWindowControllerProvider: settingsWindowControllerFactory,
    )
  }

  func sharedSettingsWindowController() -> SettingsWindowController {
    settingsWindowControllerFactory()
  }
}
