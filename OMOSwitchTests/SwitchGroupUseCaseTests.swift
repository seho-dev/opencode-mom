import Foundation
import XCTest
@testable import OMOSwitch

final class SwitchGroupUseCaseTests: XCTestCase {

    private func makeHarness() throws -> (harness: TemporaryHomeHarness, useCase: SwitchGroupUseCase) {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()
        try harness.setupOpencodeConfig()

        let configRoot = harness.omoSwitchConfigURL
        let sharedConfigRoot = harness.homeURL.appendingPathComponent(".config", isDirectory: true)

        let modelGroupRepo = ModelGroupRepository(
            fileManager: .default,
            configRootURL: configRoot
        )
        let appStateRepo = AppStateRepository(
            fileManager: .default,
            configRootURL: configRoot
        )
        let backupRepo = BackupRepository(
            fileManager: .default,
            configRootURL: configRoot
        )
        let openCodeConfigRepo = OpenCodeConfigRepository(
            fileManager: .default,
            configRootURL: sharedConfigRoot
        )
        let ohMyConfigRepo = OhMyOpenAgentConfigRepository(
            fileManager: .default,
            configRootURL: sharedConfigRoot
        )

        let useCase = SwitchGroupUseCase(
            modelGroupRepository: modelGroupRepo,
            appStateRepository: appStateRepo,
            backupRepository: backupRepo,
            openCodeConfigRepository: openCodeConfigRepo,
            ohMyConfigRepository: ohMyConfigRepo
        )

        return (harness, useCase)
    }

    private func seedGroup(
        id: UUID = UUID(),
        name: String = "TestGroup",
        isEnabled: Bool = true,
        categoryMappings: [ModelGroupCategoryMapping] = [],
        agentOverrides: [ModelGroupAgentOverride] = [],
        openCodeAgentOverrides: [ModelGroupAgentOverride] = [],
        configRootURL: URL
    ) throws -> ModelGroup {
        let group = ModelGroup(
            id: id,
            name: name,
            categoryMappings: categoryMappings,
            agentOverrides: agentOverrides,
            openCodeAgentOverrides: openCodeAgentOverrides,
            isEnabled: isEnabled
        )
        let repo = ModelGroupRepository(fileManager: .default, configRootURL: configRootURL)
        try repo.save([group])
        return group
    }

    private func seedOhMyOpenAgentConfig(configRootURL: URL) throws {
        let repo = OhMyOpenAgentConfigRepository(fileManager: .default, configRootURL: configRootURL)
        let doc = OhMyOpenAgentDocument(rawDictionary: [
            "$schema": "https://example.com/schema.json",
            "agents": ["oracle": ["model": "gpt-4"]] as [String: Any],
            "categories": ["deep": ["model": "gpt-4"]] as [String: Any],
        ])
        try repo.save(doc)
    }

    private func seedOpenCodeConfig(configRootURL: URL) throws {
        let repo = OpenCodeConfigRepository(fileManager: .default, configRootURL: configRootURL)
        let doc = OpenCodeDocument(rawDictionary: [
            "$schema": "https://opencode.ai/config.json",
            "agent": [
                "karen": ["model": "openai/gpt-4.1", "mode": "subagent"],
                "oracle": ["model": "openai/gpt-4.1", "mode": "subagent"],
            ] as [String: Any],
        ])
        try repo.save(doc)
    }

    private func makeSharedConfigRoot(_ harness: TemporaryHomeHarness) -> URL {
        harness.homeURL.appendingPathComponent(".config", isDirectory: true)
    }

    func testSwitchWritesBothTargetsWhenOpenCodeOverridesExist() async throws {
        let (harness, useCase) = try makeHarness()
        let sharedConfigRoot = makeSharedConfigRoot(harness)
        try seedOhMyOpenAgentConfig(configRootURL: sharedConfigRoot)
        try seedOpenCodeConfig(configRootURL: sharedConfigRoot)

        let groupID = UUID()
        _ = try seedGroup(
            id: groupID,
            name: "DualTargetGroup",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "quick", modelRef: "minimax-m2.7"),
            ],
            agentOverrides: [
                ModelGroupAgentOverride(agentName: "oracle", modelRef: "gpt-5.4"),
            ],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "karen", modelRef: "openai/o3"),
            ],
            configRootURL: harness.omoSwitchConfigURL
        )

        let result = await useCase.switchTo(groupID: groupID)

        guard case .success(let projectionResult) = result else {
            XCTFail("Expected success, got \(result)")
            return
        }

        let ohMyRepo = OhMyOpenAgentConfigRepository(fileManager: .default, configRootURL: sharedConfigRoot)
        let openCodeRepo = OpenCodeConfigRepository(fileManager: .default, configRootURL: sharedConfigRoot)

        guard case .success(let ohMyDocument) = ohMyRepo.load() else {
            XCTFail("Expected oh-my-openagent config to load")
            return
        }
        guard case .success(let openCodeDocument) = openCodeRepo.load() else {
            XCTFail("Expected OpenCode config to load")
            return
        }

        XCTAssertEqual(ohMyDocument, projectionResult.document)
        XCTAssertEqual((ohMyDocument.agents["oracle"] as? [String: Any])?["model"] as? String, "gpt-5.4")
        XCTAssertEqual((ohMyDocument.categories["quick"] as? [String: Any])?["model"] as? String, "minimax-m2.7")
        XCTAssertEqual((openCodeDocument.agents["karen"] as? [String: Any])?["model"] as? String, "openai/o3")

        let backupRepo = BackupRepository(fileManager: .default, configRootURL: harness.omoSwitchConfigURL)
        XCTAssertEqual(try backupRepo.listBackups(for: "opencode").count, 1)
        XCTAssertEqual(try backupRepo.listBackups(for: "oh-my-openagent").count, 1)
    }

    func testSwitchSkipsOpenCodeWhenNoEffectiveOverridesAndConfigMissing() async throws {
        let (harness, useCase) = try makeHarness()
        let sharedConfigRoot = makeSharedConfigRoot(harness)
        try seedOhMyOpenAgentConfig(configRootURL: sharedConfigRoot)

        let groupID = UUID()
        _ = try seedGroup(
            id: groupID,
            name: "SkipOpenCode",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "quick", modelRef: "minimax-m2.7"),
            ],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "karen", modelRef: "   "),
            ],
            configRootURL: harness.omoSwitchConfigURL
        )

        let result = await useCase.switchTo(groupID: groupID)

        guard case .success(let projectionResult) = result else {
            XCTFail("Expected success, got \(result)")
            return
        }

        XCTAssertTrue(projectionResult.warnings.isEmpty)
        XCTAssertFalse(FileManager.default.fileExists(atPath: sharedConfigRoot.appendingPathComponent("opencode/opencode.json").path))

        let backupRepo = BackupRepository(fileManager: .default, configRootURL: harness.omoSwitchConfigURL)
        XCTAssertTrue(try backupRepo.listBackups(for: "opencode").isEmpty)
        XCTAssertEqual(try backupRepo.listBackups(for: "oh-my-openagent").count, 1)
    }

    func testSwitchFailsWhenOpenCodeOverridesExistButConfigMissing() async throws {
        let (harness, useCase) = try makeHarness()
        let sharedConfigRoot = makeSharedConfigRoot(harness)
        try seedOhMyOpenAgentConfig(configRootURL: sharedConfigRoot)

        let groupID = UUID()
        _ = try seedGroup(
            id: groupID,
            name: "NeedsOpenCode",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "quick", modelRef: "minimax-m2.7"),
            ],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "karen", modelRef: "openai/o3"),
            ],
            configRootURL: harness.omoSwitchConfigURL
        )

        let result = await useCase.switchTo(groupID: groupID)

        guard case .failure(let message) = result else {
            XCTFail("Expected failure, got \(result)")
            return
        }

        XCTAssertEqual(message, "Failed to load opencode config")
    }

    func testSwitchFailsWhenOpenCodeOverridesExistAndConfigMalformed() async throws {
        let (harness, useCase) = try makeHarness()
        let sharedConfigRoot = makeSharedConfigRoot(harness)
        try seedOhMyOpenAgentConfig(configRootURL: sharedConfigRoot)

        let openCodeRepo = OpenCodeConfigRepository(fileManager: .default, configRootURL: sharedConfigRoot)
        try FileManager.default.createDirectory(
            at: openCodeRepo.openCodeConfigURL.deletingLastPathComponent(),
            withIntermediateDirectories: true
        )
        try "{ invalid json".write(to: openCodeRepo.openCodeConfigURL, atomically: true, encoding: .utf8)

        let groupID = UUID()
        _ = try seedGroup(
            id: groupID,
            name: "BrokenOpenCode",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "quick", modelRef: "minimax-m2.7"),
            ],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "karen", modelRef: "openai/o3"),
            ],
            configRootURL: harness.omoSwitchConfigURL
        )

        let result = await useCase.switchTo(groupID: groupID)

        guard case .failure(let message) = result else {
            XCTFail("Expected failure, got \(result)")
            return
        }

        XCTAssertEqual(message, "Failed to load opencode config")
    }

    func testSwitchRollsBackOpenCodeWhenSecondTargetFails() async throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()
        try harness.setupOpencodeConfig()

        let configRoot = harness.omoSwitchConfigURL
        let sharedConfigRoot = harness.homeURL.appendingPathComponent(".config", isDirectory: true)

        let modelGroupRepo = ModelGroupRepository(fileManager: .default, configRootURL: configRoot)
        let appStateRepo = AppStateRepository(fileManager: .default, configRootURL: configRoot)
        let backupRepo = BackupRepository(fileManager: .default, configRootURL: configRoot)
        let openCodeConfigRepo = OpenCodeConfigRepository(fileManager: .default, configRootURL: sharedConfigRoot)
        let ohMyConfigRepo = OhMyOpenAgentConfigRepository(fileManager: .default, configRootURL: sharedConfigRoot)

        try seedOhMyOpenAgentConfig(configRootURL: sharedConfigRoot)
        try seedOpenCodeConfig(configRootURL: sharedConfigRoot)

        let groupID = UUID()
        _ = try seedGroup(
            id: groupID,
            name: "RollbackGroup",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "deep", modelRef: "minimax-m2.7"),
            ],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "karen", modelRef: "openai/o3"),
            ],
            configRootURL: configRoot
        )

        let originalOpenCodeData = try Data(contentsOf: openCodeConfigRepo.openCodeConfigURL)
        let originalOhMyData = try Data(contentsOf: ohMyConfigRepo.ohMyOpenAgentConfigURL)

        try FileManager.default.setAttributes(
            [FileAttributeKey.posixPermissions: 0o000],
            ofItemAtPath: ohMyConfigRepo.ohMyOpenAgentConfigURL.deletingLastPathComponent().path
        )

        let useCase = SwitchGroupUseCase(
            modelGroupRepository: modelGroupRepo,
            appStateRepository: appStateRepo,
            backupRepository: backupRepo,
            openCodeConfigRepository: openCodeConfigRepo,
            ohMyConfigRepository: ohMyConfigRepo
        )

        let result = await useCase.switchTo(groupID: groupID)

        try FileManager.default.setAttributes(
            [FileAttributeKey.posixPermissions: 0o755],
            ofItemAtPath: ohMyConfigRepo.ohMyOpenAgentConfigURL.deletingLastPathComponent().path
        )

        guard case .failure = result else {
            XCTFail("Expected failure, got \(result)")
            return
        }

        let currentOpenCodeData = try Data(contentsOf: openCodeConfigRepo.openCodeConfigURL)
        let currentOhMyData = try Data(contentsOf: ohMyConfigRepo.ohMyOpenAgentConfigURL)
        XCTAssertEqual(currentOpenCodeData, originalOpenCodeData)
        XCTAssertEqual(currentOhMyData, originalOhMyData)
    }

    func testSwitchAggregatesStaleOpenCodeOverrideAsWarning() async throws {
        let (harness, useCase) = try makeHarness()
        let sharedConfigRoot = makeSharedConfigRoot(harness)
        try seedOhMyOpenAgentConfig(configRootURL: sharedConfigRoot)
        try seedOpenCodeConfig(configRootURL: sharedConfigRoot)

        let groupID = UUID()
        _ = try seedGroup(
            id: groupID,
            name: "WarningGroup",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "quick", modelRef: "minimax-m2.7"),
            ],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "missing-agent", modelRef: "openai/o3"),
            ],
            configRootURL: harness.omoSwitchConfigURL
        )

        let result = await useCase.switchTo(groupID: groupID)

        guard case .success(let projectionResult) = result else {
            XCTFail("Expected success, got \(result)")
            return
        }

        XCTAssertEqual(
            projectionResult.warnings,
            ["OpenCode agent 'missing-agent' was not found; skipped model override."]
        )

        let appStateRepo = AppStateRepository(fileManager: .default, configRootURL: harness.omoSwitchConfigURL)
        let state = try appStateRepo.load()
        XCTAssertEqual(state.lastWarningSummary?.count, 1)
        XCTAssertEqual(state.lastWarningSummary?.message, "OpenCode agent 'missing-agent' was not found; skipped model override.")
    }

    func testSwitchUpdatesSelectedGroupAndLastSuccessfulWriteMetadata() async throws {
        let (harness, useCase) = try makeHarness()
        let sharedConfigRoot = makeSharedConfigRoot(harness)
        try seedOhMyOpenAgentConfig(configRootURL: sharedConfigRoot)
        try seedOpenCodeConfig(configRootURL: sharedConfigRoot)

        let groupID = UUID()
        _ = try seedGroup(
            id: groupID,
            name: "StateGroup",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "deep", modelRef: "minimax-m2.7"),
            ],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "karen", modelRef: "openai/o3"),
            ],
            configRootURL: harness.omoSwitchConfigURL
        )

        let result = await useCase.switchTo(groupID: groupID)
        guard case .success = result else {
            XCTFail("Expected success, got \(result)")
            return
        }

        let appStateRepo = AppStateRepository(
            fileManager: .default,
            configRootURL: harness.omoSwitchConfigURL
        )
        let state = try appStateRepo.load()

        XCTAssertEqual(state.selectedGroupID, groupID)
        XCTAssertEqual(state.selectedGroupName, "StateGroup")
        XCTAssertNotNil(state.lastSuccessfulWrite)
        XCTAssertEqual(state.lastSuccessfulWrite?.target, "switch-group")
        XCTAssertNotNil(state.lastSuccessfulWrite?.wroteAt)

        let backupPath = try XCTUnwrap(state.lastSuccessfulWrite?.backupPath)
        XCTAssertTrue(backupPath.hasPrefix("opencode:"))
        XCTAssertTrue(backupPath.contains(";oh-my-openagent:"))
        XCTAssertNil(state.lastErrorSummary)
    }

    func testSwitchNoOpsWhenTargetAlreadyActive() async throws {
        let (harness, useCase) = try makeHarness()
        let sharedConfigRoot = makeSharedConfigRoot(harness)
        try seedOhMyOpenAgentConfig(configRootURL: sharedConfigRoot)

        let groupID = UUID()
        _ = try seedGroup(
            id: groupID,
            name: "ActiveGroup",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "deep", modelRef: "minimax-m2.7"),
            ],
            configRootURL: harness.omoSwitchConfigURL
        )

        let appStateRepo = AppStateRepository(
            fileManager: .default,
            configRootURL: harness.omoSwitchConfigURL
        )
        var state = try appStateRepo.load()
        state.selectedGroupID = groupID
        state.selectedGroupName = "ActiveGroup"
        try appStateRepo.save(state)

        let result = await useCase.switchTo(groupID: groupID)

        guard case .noOp = result else {
            XCTFail("Expected noOp, got \(result)")
            return
        }
    }

    func testDisabledGroupCannotBeSwitched() async throws {
        let (harness, useCase) = try makeHarness()
        let sharedConfigRoot = makeSharedConfigRoot(harness)
        try seedOhMyOpenAgentConfig(configRootURL: sharedConfigRoot)

        let groupID = UUID()
        _ = try seedGroup(
            id: groupID,
            name: "DisabledGroup",
            isEnabled: false,
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "deep", modelRef: "minimax-m2.7"),
            ],
            configRootURL: harness.omoSwitchConfigURL
        )

        let result = await useCase.switchTo(groupID: groupID)

        guard case .failure(let message) = result else {
            XCTFail("Expected failure, got \(result)")
            return
        }
        XCTAssertEqual(message, "Group is disabled")
    }
}
