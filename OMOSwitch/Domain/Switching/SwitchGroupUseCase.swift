import Foundation

public struct SwitchGroupUseCase: @unchecked Sendable {
    public let modelGroupRepository: ModelGroupRepository
    public let appStateRepository: AppStateRepository
    public let backupRepository: BackupRepository
    public let openCodeConfigRepository: OpenCodeConfigRepository
    public let ohMyConfigRepository: OhMyOpenAgentConfigRepository

    public init(
        modelGroupRepository: ModelGroupRepository,
        appStateRepository: AppStateRepository,
        backupRepository: BackupRepository,
        openCodeConfigRepository: OpenCodeConfigRepository,
        ohMyConfigRepository: OhMyOpenAgentConfigRepository
    ) {
        self.modelGroupRepository = modelGroupRepository
        self.appStateRepository = appStateRepository
        self.backupRepository = backupRepository
        self.openCodeConfigRepository = openCodeConfigRepository
        self.ohMyConfigRepository = ohMyConfigRepository
    }

    public func switchTo(groupID: UUID) async -> SwitchResult {
        let loadContextResult = loadContext(for: groupID)
        if let failure = loadContextResult.failure {
            return failure
        }
        guard let group = loadContextResult.group, let currentState = loadContextResult.currentState else {
            return .failure("Failed to load switch context")
        }
        if currentState.selectedGroupID == groupID {
            return .noOp
        }
        return persistProjection(group: group, currentState: currentState)
    }

    public func saveActiveGroupProjection(groupID: UUID) async -> SwitchResult {
        let loadContextResult = loadContext(for: groupID)
        if let failure = loadContextResult.failure {
            return failure
        }
        guard let group = loadContextResult.group, let currentState = loadContextResult.currentState else {
            return .failure("Failed to load switch context")
        }
        guard currentState.selectedGroupID == groupID else {
            return .noOp
        }
        return persistProjection(group: group, currentState: currentState)
    }

    private func loadContext(for groupID: UUID) -> (group: ModelGroup?, currentState: AppSelectionState?, failure: SwitchResult?) {
        let groups: [ModelGroup]
        do {
            groups = try modelGroupRepository.load()
        } catch {
            return (nil, nil, .failure("Failed to load groups: \(error.localizedDescription)"))
        }

        guard let group = groups.first(where: { $0.id == groupID }) else {
            return (nil, nil, .failure("Group not found"))
        }

        guard group.isEnabled else {
            return (nil, nil, .failure("Group is disabled"))
        }

        let currentState: AppSelectionState
        do {
            currentState = try appStateRepository.load()
        } catch {
            return (nil, nil, .failure("Failed to load app state: \(error.localizedDescription)"))
        }

        return (group, currentState, nil)
    }

    private func persistProjection(group: ModelGroup, currentState: AppSelectionState) -> SwitchResult {
        let hasEffectiveOpenCodeOverrides = group.openCodeAgentOverrides.contains {
            !$0.modelRef.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        }

        let openCodeContextResult = loadOpenCodeContextIfNeeded(hasEffectiveOverrides: hasEffectiveOpenCodeOverrides)
        if let failure = openCodeContextResult.failure {
            return failure
        }

        let ohMyLoadResult = ohMyConfigRepository.load()
        let existingOhMyDoc: OhMyOpenAgentDocument
        switch ohMyLoadResult {
        case .success(let doc):
            existingOhMyDoc = doc
        case .failure:
            return .failure("Failed to load oh-my-openagent config")
        }

        let backupResult = createBackups(
            needsOpenCodeBackup: hasEffectiveOpenCodeOverrides,
            openCodeURL: openCodeConfigRepository.openCodeConfigURL,
            ohMyURL: ohMyConfigRepository.ohMyOpenAgentConfigURL
        )
        if let failure = backupResult.failure {
            return failure
        }

        let openCodeProjectionResult = openCodeContextResult.document.map {
            OpenCodeProjectionService.project(group: group, onto: $0)
        }
        let ohMyProjectionResult = OhMyOpenAgentProjectionService.project(
            group: group,
            onto: existingOhMyDoc
        )

        if let openCodeProjectionResult {
            do {
                try openCodeConfigRepository.save(openCodeProjectionResult.document)
            } catch {
                if let rollbackFailure = rollbackIfPossible(
                    backupURL: backupResult.openCodeBackupURL,
                    targetURL: openCodeConfigRepository.openCodeConfigURL,
                    writeError: error
                ) {
                    return rollbackFailure
                }
                return .failure("Write failed: \(error.localizedDescription)")
            }
        }

        do {
            try ohMyConfigRepository.save(ohMyProjectionResult.document)
        } catch {
            if let rollbackFailure = rollbackIfPossible(
                backupURL: backupResult.openCodeBackupURL,
                targetURL: openCodeConfigRepository.openCodeConfigURL,
                writeError: error
            ) {
                return rollbackFailure
            }
            return .failure("Write failed: \(error.localizedDescription)")
        }

        let warnings = (openCodeProjectionResult?.warnings ?? []) + ohMyProjectionResult.warnings

        var newState = currentState
        newState.selectedGroupID = group.id
        newState.selectedGroupName = group.name
        newState.lastSuccessfulWrite = LastSuccessfulWriteMetadata(
            target: "switch-group",
            wroteAt: Date(),
            backupPath: backupSummary(
                openCodeBackupURL: backupResult.openCodeBackupURL,
                ohMyBackupURL: backupResult.ohMyBackupURL
            )
        )
        newState.lastErrorSummary = nil
        if warnings.isEmpty {
            newState.lastWarningSummary = nil
        } else {
            newState.lastWarningSummary = ProjectionIssueSummary(
                message: warnings.joined(separator: "; "),
                count: warnings.count
            )
        }

        do {
            try appStateRepository.save(newState)
        } catch {
            return .failure("Config written but state save failed: \(error.localizedDescription)")
        }

        if hasEffectiveOpenCodeOverrides {
            try? backupRepository.cleanup(target: "opencode", keepingLatest: 5)
        }
        try? backupRepository.cleanup(target: "oh-my-openagent", keepingLatest: 5)

        return .success(ProjectionResult(document: ohMyProjectionResult.document, warnings: warnings))
    }

    private func loadOpenCodeContextIfNeeded(hasEffectiveOverrides: Bool) -> (document: OpenCodeDocument?, failure: SwitchResult?) {
        guard hasEffectiveOverrides else {
            return (nil, nil)
        }

        let loadResult = openCodeConfigRepository.load()
        switch loadResult {
        case .success(let document):
            return (document, nil)
        case .failure(.fileNotFound):
            return (nil, .failure("Failed to load opencode config"))
        case .failure:
            return (nil, .failure("Failed to load opencode config"))
        }
    }

    private func createBackups(
        needsOpenCodeBackup: Bool,
        openCodeURL: URL,
        ohMyURL: URL
    ) -> (openCodeBackupURL: URL?, ohMyBackupURL: URL?, failure: SwitchResult?) {
        do {
            let openCodeBackupURL = try createBackupIfNeeded(for: "opencode", sourceFileURL: openCodeURL, shouldBackup: needsOpenCodeBackup)
            let ohMyBackupURL = try createBackupIfNeeded(for: "oh-my-openagent", sourceFileURL: ohMyURL, shouldBackup: true)
            return (openCodeBackupURL, ohMyBackupURL, nil)
        } catch {
            return (nil, nil, .failure("Failed to create backup: \(error.localizedDescription)"))
        }
    }

    private func createBackupIfNeeded(for target: String, sourceFileURL: URL, shouldBackup: Bool) throws -> URL? {
        guard shouldBackup else {
            return nil
        }

        guard FileManager.default.fileExists(atPath: sourceFileURL.path()) else {
            return nil
        }

        let existingData = try Data(contentsOf: sourceFileURL)

        return try backupRepository.createBackup(
            for: target,
            sourceFileURL: sourceFileURL,
            contents: existingData
        )
    }

    private func rollbackIfPossible(backupURL: URL?, targetURL: URL, writeError: Error) -> SwitchResult? {
        guard let backupURL else {
            return nil
        }

        do {
            let backupData = try Data(contentsOf: backupURL)
            try backupData.write(to: targetURL, options: [.atomic])
            return nil
        } catch {
            return .failure(
                "Write failed: \(writeError.localizedDescription). Rollback also failed: \(error.localizedDescription)"
            )
        }
    }

    private func backupSummary(openCodeBackupURL: URL?, ohMyBackupURL: URL?) -> String {
        "opencode:\(openCodeBackupURL?.path ?? "none");oh-my-openagent:\(ohMyBackupURL?.path ?? "none")"
    }
}
