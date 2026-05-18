import Foundation
import AppKit

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
  @Published var updateInfo: UpdateInfo? = nil
  @Published var isCheckingForUpdates: Bool = false
  @Published var updateError: String? = nil
  @Published var isDownloadingUpdate: Bool = false
  @Published var downloadProgress: Double? = nil
  @Published var openCodeServeConfig: OpenCodeServeConfig = OpenCodeServeConfig()
  @Published var openCodeServeStatus: OpenCodeServeProcessState = .stopped

  let modelGroupRepository: ModelGroupRepository
  let appStateRepository: AppStateRepository
  let openCodeConfigRepository: OpenCodeConfigRepository
  let switchUseCase: SwitchGroupUseCase
  let loginItemService: any LoginItemService
  let updateChecker: any UpdateChecker
  let processManager: any OpenCodeServeProcessManaging

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
      updateChecker: GitHubUpdateChecker(),
      processManager: OpenCodeServeProcessManager()
    )
  }

  init(
    modelGroupRepository: ModelGroupRepository,
    appStateRepository: AppStateRepository,
    openCodeConfigRepository: OpenCodeConfigRepository,
    switchUseCase: SwitchGroupUseCase,
    loginItemService: any LoginItemService,
    updateChecker: any UpdateChecker,
    processManager: any OpenCodeServeProcessManaging = OpenCodeServeProcessManager()
  ) {
    self.modelGroupRepository = modelGroupRepository
    self.appStateRepository = appStateRepository
    self.openCodeConfigRepository = openCodeConfigRepository
    self.switchUseCase = switchUseCase
    self.loginItemService = loginItemService
    self.updateChecker = updateChecker
    self.processManager = processManager
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
      openCodeServeConfig = state.openCodeServeConfig
      launchAtLoginStatusMessage = nil
      lastSwitchError = state.lastErrorSummary?.message
      lastSwitchWarning = state.lastWarningSummary?.message

      refreshServerStatus()
      if state.openCodeServeConfig.autoStart {
        startServer(config: state.openCodeServeConfig)
      }

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

  func loadServerConfig() {
    do {
      let state = try appStateRepository.load()
      openCodeServeConfig = state.openCodeServeConfig
    } catch {
      lastSwitchError = error.localizedDescription
    }
  }

  func saveServerConfig(_ config: OpenCodeServeConfig) throws {
    let validationErrors = OpenCodeServeArgumentBuilder.validationErrors(for: config)
    guard validationErrors.isEmpty else { throw validationErrors[0] }

    var state = try appStateRepository.load()
    let previousConfig = state.openCodeServeConfig
    state.openCodeServeConfig = config
    try appStateRepository.save(state)
    openCodeServeConfig = config

    guard openCodeServeStatus == .running, serverArgumentsChanged(from: previousConfig, to: config) else {
      return
    }

    Task { @MainActor [weak self] in
      guard let self else { return }
      await processManager.restart(config: config)
      await updateServerStatusFromProcessManager()
    }
  }

  private func serverArgumentsChanged(from previous: OpenCodeServeConfig, to next: OpenCodeServeConfig) -> Bool {
    previous.port != next.port
      || previous.hostname != next.hostname
      || previous.mdns != next.mdns
      || previous.mdnsDomain != next.mdnsDomain
      || previous.cors != next.cors
  }

  func startServer() {
    startServer(config: openCodeServeConfig)
  }

  func stopServer() {
    openCodeServeStatus = .stopping
    Task { @MainActor [weak self] in
      guard let self else { return }
      await processManager.stop()
      await updateServerStatusFromProcessManager()
    }
  }

  func stopServerAndWait(timeoutNanoseconds: UInt64) {
    openCodeServeStatus = .stopping
    let processManager = processManager
    let semaphore = DispatchSemaphore(value: 0)

    Task.detached {
      await processManager.stop()
      semaphore.signal()
    }

    let timeout = DispatchTime.now() + .nanosecondsClamped(timeoutNanoseconds)
    if semaphore.wait(timeout: timeout) == .success {
      openCodeServeStatus = .stopped
    }
  }

  func restartServer() {
    openCodeServeStatus = .starting
    let config = openCodeServeConfig
    Task { @MainActor [weak self] in
      guard let self else { return }
      await processManager.restart(config: config)
      await updateServerStatusFromProcessManager()
    }
  }

  private func startServer(config: OpenCodeServeConfig) {
    openCodeServeStatus = .starting
    Task { @MainActor [weak self] in
      guard let self else { return }
      await processManager.start(config: config)
      await updateServerStatusFromProcessManager()
    }
  }

  private func refreshServerStatus() {
    Task { @MainActor [weak self] in
      await self?.updateServerStatusFromProcessManager()
    }
  }

  private func updateServerStatusFromProcessManager() async {
    openCodeServeStatus = await processManager.currentState()
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

  func copyGroup(id: UUID) throws -> ModelGroup {
    guard let sourceGroup = groups.first(where: { $0.id == id }) else {
      throw NSError(domain: "AppStore.copyGroup", code: 1, userInfo: [NSLocalizedDescriptionKey: "Group not found."])
    }

    let copiedGroup = ModelGroup(
      id: UUID(),
      name: "\(sourceGroup.name) Copy",
      description: sourceGroup.description,
      categoryMappings: sourceGroup.categoryMappings,
      agentOverrides: sourceGroup.agentOverrides,
      openCodeAgentOverrides: sourceGroup.openCodeAgentOverrides,
      isEnabled: sourceGroup.isEnabled,
      updatedAt: Date()
    )

    var currentGroups = try modelGroupRepository.load()
    currentGroups.append(copiedGroup)
    try modelGroupRepository.save(currentGroups)
    reload()

    return copiedGroup
  }

  func checkForUpdates() async {
    isCheckingForUpdates = true
    updateError = nil

    do {
      updateInfo = try await updateChecker.checkForUpdates()
    } catch {
      updateError = error.localizedDescription
    }

    isCheckingForUpdates = false
  }

  func downloadUpdate() async {
    guard let updateInfo = updateInfo else { return }

    isDownloadingUpdate = true
    updateError = nil

    do {
      let downloadedURL = try await updateChecker.downloadUpdate(updateInfo)
      try updateChecker.installUpdate(at: downloadedURL)

      DispatchQueue.main.async {
        let alert = NSAlert()
        alert.messageText = "Update Ready"
        alert.informativeText = "omo-switch has been updated to version \(updateInfo.latestVersion). The app will now restart."
        alert.alertStyle = .informational
        alert.addButton(withTitle: "Restart")

        if alert.runModal() == .alertFirstButtonReturn {
          self.restartApp()
        }
      }
    } catch {
      updateError = error.localizedDescription
    }

    isDownloadingUpdate = false
  }

  private func restartApp() {
    let task = Process()
    task.executableURL = URL(fileURLWithPath: "/usr/bin/open")
    task.arguments = ["-n", Bundle.main.bundlePath]

    try? task.run()

    NSApplication.shared.terminate(nil)
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
        if openCodeServeStatus == .running {
          await processManager.restart(config: openCodeServeConfig)
        }
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

private extension DispatchTimeInterval {
  static func nanosecondsClamped(_ value: UInt64) -> DispatchTimeInterval {
    .nanoseconds(Int(min(value, UInt64(Int.max))))
  }
}
