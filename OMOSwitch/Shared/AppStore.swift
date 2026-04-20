import Foundation

@MainActor
final class AppStore: NSObject, ObservableObject {
  @Published var groups: [ModelGroup] = []
  @Published var currentGroupID: UUID? = nil
  @Published var currentGroupName: String? = nil
  @Published var lastSwitchError: String? = nil
  @Published var lastSwitchWarning: String? = nil
  @Published var isLoading: Bool = false

  let modelGroupRepository: ModelGroupRepository
  let appStateRepository: AppStateRepository
  let switchUseCase: SwitchGroupUseCase

  static var livePreview: AppStore {
    let modelGroupRepository = ModelGroupRepository()
    let appStateRepository = AppStateRepository()
    let switchUseCase = SwitchGroupUseCase(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      backupRepository: BackupRepository(),
      ohMyConfigRepository: OhMyOpenAgentConfigRepository(),
    )
    return AppStore(
      modelGroupRepository: modelGroupRepository,
      appStateRepository: appStateRepository,
      switchUseCase: switchUseCase,
    )
  }

  init(
    modelGroupRepository: ModelGroupRepository,
    appStateRepository: AppStateRepository,
    switchUseCase: SwitchGroupUseCase
  ) {
    self.modelGroupRepository = modelGroupRepository
    self.appStateRepository = appStateRepository
    self.switchUseCase = switchUseCase
    super.init()
  }

  func reload() {
    isLoading = true
    defer { isLoading = false }

    do {
      groups = try modelGroupRepository.load()
      let state = try appStateRepository.load()
      currentGroupID = state.selectedGroupID
      currentGroupName = state.selectedGroupName
      lastSwitchError = state.lastErrorSummary?.message
      lastSwitchWarning = state.lastWarningSummary?.message
    } catch {
      lastSwitchError = error.localizedDescription
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
