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

  private let statusBarProvider: StatusBarProviding
  private let popoverControllerFactory: () -> QuickSwitchPopoverController
  private let globalSettingsWindowControllerFactory: () -> SettingsWindowController
  private let groupSettingsWindowControllerFactory: () -> SettingsWindowController

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
    let appStore = AppStore(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      openCodeConfigRepository: openCodeConfigRepository,
      switchUseCase: switchUseCase,
      loginItemService: loginItemService,
    )
    let sharedGlobalSettingsWindowController = SettingsWindowController(appStore: appStore, kind: .global)
    let sharedGroupSettingsWindowController = SettingsWindowController(appStore: appStore, kind: .group)
    self.modelGroupRepository = modelGroupRepository
    self.appStateRepository = appStateRepository
    self.backupRepository = backupRepository
    self.openCodeConfigRepository = openCodeConfigRepository
    self.ohMyConfigRepository = ohMyConfigRepository
    self.switchUseCase = switchUseCase
    self.appStore = appStore
    self.loginItemService = loginItemService
    self.statusBarProvider = CocoaStatusBarProvider()
    self.popoverControllerFactory = { QuickSwitchPopoverController(appStore: appStore) }
    self.globalSettingsWindowControllerFactory = { sharedGlobalSettingsWindowController }
    self.groupSettingsWindowControllerFactory = { sharedGroupSettingsWindowController }
  }

  init(
    statusBarProvider: StatusBarProviding,
    popoverControllerFactory: @escaping () -> QuickSwitchPopoverController,
    globalSettingsWindowController: SettingsWindowController,
    groupSettingsWindowController: SettingsWindowController,
    loginItemService: (any LoginItemService)? = nil,
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
    let appStore = AppStore(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      openCodeConfigRepository: openCodeConfigRepository,
      switchUseCase: switchUseCase,
      loginItemService: resolvedLoginItemService,
    )
    let sharedGlobalSettingsWindowController = globalSettingsWindowController
    let sharedGroupSettingsWindowController = groupSettingsWindowController
    self.modelGroupRepository = modelGroupRepository
    self.appStateRepository = appStateRepository
    self.backupRepository = backupRepository
    self.openCodeConfigRepository = openCodeConfigRepository
    self.ohMyConfigRepository = ohMyConfigRepository
    self.switchUseCase = switchUseCase
    self.appStore = appStore
    self.loginItemService = resolvedLoginItemService
    self.statusBarProvider = statusBarProvider
    self.popoverControllerFactory = popoverControllerFactory
    self.globalSettingsWindowControllerFactory = { sharedGlobalSettingsWindowController }
    self.groupSettingsWindowControllerFactory = { sharedGroupSettingsWindowController }
  }

  func makeStatusItemController() -> StatusItemController {
    StatusItemController(
      statusBarProvider: statusBarProvider,
      appStore: appStore,
      popoverController: popoverControllerFactory(),
      globalSettingsWindowControllerProvider: globalSettingsWindowControllerFactory,
      groupSettingsWindowControllerProvider: groupSettingsWindowControllerFactory,
    )
  }

  func sharedGlobalSettingsWindowController() -> SettingsWindowController {
    globalSettingsWindowControllerFactory()
  }

  func sharedGroupSettingsWindowController() -> SettingsWindowController {
    groupSettingsWindowControllerFactory()
  }
}
