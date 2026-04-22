import Foundation

@MainActor
final class AppStore: NSObject, ObservableObject {
  @Published var groups: [ModelGroup] = []
  @Published var currentGroupID: UUID? = nil
  @Published var currentGroupName: String? = nil
  @Published var launchAtLoginEnabled: Bool = false
  @Published var launchAtLoginStatusMessage: String? = nil
  @Published var discoveredOpenCodeAgentNames: [String] = []
  @Published var openCodeAgentDiscoveryError: String? = nil
  @Published var lastSwitchError: String? = nil
  @Published var lastSwitchWarning: String? = nil
  @Published var isLoading: Bool = false

  let modelGroupRepository: ModelGroupRepository
  let appStateRepository: AppStateRepository
  let openCodeConfigRepository: OpenCodeConfigRepository
  let switchUseCase: SwitchGroupUseCase
  let loginItemService: any LoginItemService

  static var livePreview: AppStore {
    let modelGroupRepository = ModelGroupRepository()
    let appStateRepository = AppStateRepository()
    let switchUseCase = SwitchGroupUseCase(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      backupRepository: BackupRepository(),
      openCodeConfigRepository: OpenCodeConfigRepository(),
      ohMyConfigRepository: OhMyOpenAgentConfigRepository(),
    )
    return AppStore(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      openCodeConfigRepository: OpenCodeConfigRepository(),
      switchUseCase: switchUseCase,
      loginItemService: SMAppServiceLoginItemService(),
    )
  }

  init(
    modelGroupRepository: ModelGroupRepository,
    appStateRepository: AppStateRepository,
    openCodeConfigRepository: OpenCodeConfigRepository,
    switchUseCase: SwitchGroupUseCase,
    loginItemService: any LoginItemService
  ) {
    self.modelGroupRepository = modelGroupRepository
    self.appStateRepository = appStateRepository
    self.openCodeConfigRepository = openCodeConfigRepository
    self.switchUseCase = switchUseCase
    self.loginItemService = loginItemService
    super.init()
  }

  func reload() {
    isLoading = true
    defer { isLoading = false }

    do {
      groups = try modelGroupRepository.load()
      var state = try appStateRepository.load()
      currentGroupID = state.selectedGroupID
      currentGroupName = state.selectedGroupName
      launchAtLoginEnabled = state.launchAtLoginEnabled
      launchAtLoginStatusMessage = nil
      lastSwitchError = state.lastErrorSummary?.message
      lastSwitchWarning = state.lastWarningSummary?.message

      do {
        let systemLaunchAtLoginStatus = try loginItemService.currentStatus()
        let resolvedLaunchAtLoginEnabled = applyLaunchAtLoginStatus(systemLaunchAtLoginStatus)
        launchAtLoginEnabled = resolvedLaunchAtLoginEnabled

        if state.launchAtLoginEnabled != resolvedLaunchAtLoginEnabled {
          state.launchAtLoginEnabled = resolvedLaunchAtLoginEnabled
          try appStateRepository.save(state)
        }
      } catch {
        launchAtLoginStatusMessage = "Unable to read launch-at-login status: \(error.localizedDescription)"
      }
    } catch {
      lastSwitchError = error.localizedDescription
    }

    loadDiscoveredOpenCodeAgents()
  }

  func setLaunchAtLoginEnabled(_ isEnabled: Bool) throws {
    var state = try appStateRepository.load()
    launchAtLoginStatusMessage = nil
    try loginItemService.setEnabled(isEnabled)

    let resolvedStatus = try loginItemService.currentStatus()
    let resolvedLaunchAtLoginEnabled = applyLaunchAtLoginStatus(resolvedStatus)
    if isEnabled, resolvedStatus == .disabled {
      throw NSError(
        domain: "AppStore.launchAtLogin",
        code: 1,
        userInfo: [NSLocalizedDescriptionKey: "Launch at login could not be enabled."]
      )
    }

    if isEnabled == false, resolvedStatus != .disabled {
      throw NSError(
        domain: "AppStore.launchAtLogin",
        code: 2,
        userInfo: [NSLocalizedDescriptionKey: "Launch at login could not be disabled."]
      )
    }

    state.launchAtLoginEnabled = resolvedLaunchAtLoginEnabled
    try appStateRepository.save(state)
    launchAtLoginEnabled = resolvedLaunchAtLoginEnabled
  }

  private func applyLaunchAtLoginStatus(_ status: LoginItemStatus) -> Bool {
    switch status {
    case .enabled:
      launchAtLoginStatusMessage = nil
      return true
    case .requiresApproval:
      launchAtLoginStatusMessage = "Launch at login is pending approval in System Settings."
      return false
    case .disabled:
      launchAtLoginStatusMessage = nil
      return false
    }
  }

  private func loadDiscoveredOpenCodeAgents() {
    switch openCodeConfigRepository.load() {
    case .success(let document):
      guard let agents = document.rawDictionary["agent"] as? [String: Any] else {
        discoveredOpenCodeAgentNames = []
        openCodeAgentDiscoveryError = "OpenCode config has no valid top-level agent object."
        return
      }

      discoveredOpenCodeAgentNames = agents.keys.sorted {
        $0.localizedCaseInsensitiveCompare($1) == .orderedAscending
      }
      openCodeAgentDiscoveryError = nil
    case .failure(.fileNotFound):
      discoveredOpenCodeAgentNames = []
      openCodeAgentDiscoveryError = "OpenCode config not found."
    case .failure(.malformedConfig):
      discoveredOpenCodeAgentNames = []
      openCodeAgentDiscoveryError = "OpenCode config is malformed."
    case .failure(.writeFailed):
      discoveredOpenCodeAgentNames = []
      openCodeAgentDiscoveryError = "OpenCode config is malformed."
    }
  }

  @discardableResult
  func switchTo(groupID: UUID) async -> SwitchResult {
    isLoading = true
    let result = await switchUseCase.switchTo(groupID: groupID)
    isLoading = false

    reload()

    switch result {
    case .success(let projectionResult):
      lastSwitchError = nil
      lastSwitchWarning = projectionResult.warnings.isEmpty ? nil : projectionResult.warnings.joined(separator: "; ")
    case .noOp:
      lastSwitchError = nil
      lastSwitchWarning = "Already using this group."
    case .failure(let message):
      lastSwitchError = message
      lastSwitchWarning = nil
    }

    return result
  }

  func deleteGroup(id: UUID) throws {
    let remainingGroups = try modelGroupRepository.load().filter { $0.id != id }
    try modelGroupRepository.save(remainingGroups)

    var state = try appStateRepository.load()
    if state.selectedGroupID == id {
      state.selectedGroupID = nil
      state.selectedGroupName = nil
      try appStateRepository.save(state)
    }

    reload()
  }

  func saveGroup(_ group: ModelGroup) async throws {
    var currentGroups = try modelGroupRepository.load()
    if let index = currentGroups.firstIndex(where: { $0.id == group.id }) {
      currentGroups[index] = group
    } else {
      currentGroups.append(group)
    }

    try modelGroupRepository.save(currentGroups)

    if currentGroupID == group.id {
      let result = await switchUseCase.saveActiveGroupProjection(groupID: group.id)
      switch result {
      case .success(let projectionResult):
        lastSwitchError = nil
        lastSwitchWarning = projectionResult.warnings.isEmpty ? nil : projectionResult.warnings.joined(separator: "; ")
      case .noOp:
        lastSwitchError = nil
        lastSwitchWarning = nil
      case .failure(let message):
        lastSwitchError = message
        lastSwitchWarning = nil
        throw NSError(domain: "AppStore.saveGroup", code: 1, userInfo: [NSLocalizedDescriptionKey: message])
      }
    }

    reload()
  }
}
