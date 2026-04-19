import Foundation

public struct SwitchGroupUseCase: @unchecked Sendable {
    public let modelGroupRepository: ModelGroupRepository
    public let appStateRepository: AppStateRepository
    public let backupRepository: BackupRepository
    public let ohMyConfigRepository: OhMyOpenAgentConfigRepository

    public init(
        modelGroupRepository: ModelGroupRepository,
        appStateRepository: AppStateRepository,
        backupRepository: BackupRepository,
        ohMyConfigRepository: OhMyOpenAgentConfigRepository
    ) {
        self.modelGroupRepository = modelGroupRepository
        self.appStateRepository = appStateRepository
        self.backupRepository = backupRepository
        self.ohMyConfigRepository = ohMyConfigRepository
    }

    public func switchTo(groupID: UUID) async -> SwitchResult {
        let groups: [ModelGroup]
        do {
            groups = try modelGroupRepository.load()
        } catch {
            return .failure("Failed to load groups: \(error.localizedDescription)")
        }

        guard let group = groups.first(where: { $0.id == groupID }) else {
            return .failure("Group not found")
        }

        guard group.isEnabled else {
            return .failure("Group is disabled")
        }

        let currentState: AppSelectionState
        do {
            currentState = try appStateRepository.load()
        } catch {
            return .failure("Failed to load app state: \(error.localizedDescription)")
        }

        if currentState.selectedGroupID == groupID {
            return .noOp
        }

        let loadResult = ohMyConfigRepository.load()
        let existingDoc: OhMyOpenAgentDocument
        switch loadResult {
        case .success(let doc):
            existingDoc = doc
        case .failure:
            return .failure("Failed to load oh-my-openagent config")
        }

        let configURL = ohMyConfigRepository.ohMyOpenAgentConfigURL
        let existingData = try? Data(contentsOf: configURL)
        let backupURL: URL?
        if let existingData {
            do {
                backupURL = try backupRepository.createBackup(
                    for: "oh-my-openagent",
                    sourceFileURL: configURL,
                    contents: existingData
                )
            } catch {
                return .failure("Failed to create backup: \(error.localizedDescription)")
            }
        } else {
            backupURL = nil
        }

        let projectionResult = OhMyOpenAgentProjectionService.project(
            group: group,
            onto: existingDoc
        )

        do {
            try ohMyConfigRepository.save(projectionResult.document)
        } catch {
            if let backupURL, let backupData = try? Data(contentsOf: backupURL) {
                do {
                    try backupData.write(to: configURL, options: [.atomic])
                } catch let rollbackError {
                    return .failure(
                        "Write failed: \(error.localizedDescription). Rollback also failed: \(rollbackError.localizedDescription)"
                    )
                }
            }
            return .failure("Write failed: \(error.localizedDescription)")
        }

        var newState = currentState
        newState.selectedGroupID = groupID
        newState.selectedGroupName = group.name
        newState.lastSuccessfulWrite = LastSuccessfulWriteMetadata(
            target: "oh-my-openagent",
            wroteAt: Date(),
            backupPath: backupURL?.path
        )
        newState.lastErrorSummary = nil
        if projectionResult.warnings.isEmpty {
            newState.lastWarningSummary = nil
        } else {
            newState.lastWarningSummary = ProjectionIssueSummary(
                message: projectionResult.warnings.joined(separator: "; "),
                count: projectionResult.warnings.count
            )
        }

        do {
            try appStateRepository.save(newState)
        } catch {
            return .failure("Config written but state save failed: \(error.localizedDescription)")
        }

        try? backupRepository.cleanup(target: "oh-my-openagent", keepingLatest: 5)

        return .success(projectionResult)
    }
}
