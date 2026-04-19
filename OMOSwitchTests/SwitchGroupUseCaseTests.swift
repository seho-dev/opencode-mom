import Foundation
import XCTest
@testable import OMOSwitch

final class SwitchGroupUseCaseTests: XCTestCase {

    private func makeHarness() throws -> (harness: TemporaryHomeHarness, useCase: SwitchGroupUseCase) {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()
        try harness.setupOpencodeConfig()

        let configRoot = harness.omoSwitchConfigURL
        let opencodeRoot = harness.opencodeConfigURL

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
        let ohMyConfigRepo = OhMyOpenAgentConfigRepository(
            fileManager: .default,
            configRootURL: opencodeRoot
        )

        let useCase = SwitchGroupUseCase(
            modelGroupRepository: modelGroupRepo,
            appStateRepository: appStateRepo,
            backupRepository: backupRepo,
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
        configRootURL: URL
    ) throws -> ModelGroup {
        let group = ModelGroup(
            id: id,
            name: name,
            categoryMappings: categoryMappings,
            agentOverrides: agentOverrides,
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

    func testSwitchWritesSelectedGroupIntoOhMyOpenAgentConfig() async throws {
        let (harness, useCase) = try makeHarness()
        try seedOhMyOpenAgentConfig(configRootURL: harness.opencodeConfigURL)

        let groupID = UUID()
        _ = try seedGroup(
            id: groupID,
            name: "FastGroup",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "quick", modelRef: "minimax-m2.7"),
            ],
            agentOverrides: [
                ModelGroupAgentOverride(agentName: "oracle", modelRef: "gpt-5.4"),
            ],
            configRootURL: harness.omoSwitchConfigURL
        )

        let result = await useCase.switchTo(groupID: groupID)

        guard case .success(let projectionResult) = result else {
            XCTFail("Expected success, got \(result)")
            return
        }

        let repo = OhMyOpenAgentConfigRepository(
            fileManager: .default,
            configRootURL: harness.opencodeConfigURL
        )
        let loadResult = repo.load()
        guard case .success(let writtenDoc) = loadResult else {
            XCTFail("Expected to load written config")
            return
        }

        XCTAssertEqual(writtenDoc, projectionResult.document)

        let agents = writtenDoc.agents
        XCTAssertEqual((agents["oracle"] as? [String: Any])?["model"] as? String, "gpt-5.4")

        let categories = writtenDoc.categories
        XCTAssertEqual((categories["quick"] as? [String: Any])?["model"] as? String, "minimax-m2.7")
    }

    func testSwitchCreatesBackupBeforeWrite() async throws {
        let (harness, useCase) = try makeHarness()
        try seedOhMyOpenAgentConfig(configRootURL: harness.opencodeConfigURL)

        let groupID = UUID()
        _ = try seedGroup(
            id: groupID,
            name: "BackupGroup",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "deep", modelRef: "minimax-m2.7"),
            ],
            configRootURL: harness.omoSwitchConfigURL
        )

        let backupRepo = BackupRepository(
            fileManager: .default,
            configRootURL: harness.omoSwitchConfigURL
        )
        let backupsBefore = try backupRepo.listBackups(for: "oh-my-openagent")
        XCTAssertTrue(backupsBefore.isEmpty)

        let result = await useCase.switchTo(groupID: groupID)
        guard case .success = result else {
            XCTFail("Expected success, got \(result)")
            return
        }

        let backupsAfter = try backupRepo.listBackups(for: "oh-my-openagent")
        XCTAssertEqual(backupsAfter.count, 1)
    }

    func testSwitchUpdatesSelectedGroupAndLastSuccessfulWrite() async throws {
        let (harness, useCase) = try makeHarness()
        try seedOhMyOpenAgentConfig(configRootURL: harness.opencodeConfigURL)

        let groupID = UUID()
        _ = try seedGroup(
            id: groupID,
            name: "StateGroup",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "deep", modelRef: "minimax-m2.7"),
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
        XCTAssertEqual(state.lastSuccessfulWrite?.target, "oh-my-openagent")
        XCTAssertNil(state.lastErrorSummary)
    }

    func testSwitchRollsBackConfigAndStoresErrorWhenWriteFails() async throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()
        try harness.setupOpencodeConfig()

        let configRoot = harness.omoSwitchConfigURL
        let opencodeRoot = harness.opencodeConfigURL

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
        let ohMyConfigRepo = OhMyOpenAgentConfigRepository(
            fileManager: .default,
            configRootURL: opencodeRoot
        )

        try seedOhMyOpenAgentConfig(configRootURL: opencodeRoot)

        let groupID = UUID()
        _ = try seedGroup(
            id: groupID,
            name: "FailGroup",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "deep", modelRef: "minimax-m2.7"),
            ],
            configRootURL: configRoot
        )

        let originalData = try Data(
            contentsOf: ohMyConfigRepo.ohMyOpenAgentConfigURL
        )

        try FileManager.default.setAttributes(
            [FileAttributeKey.posixPermissions: 0o000],
            ofItemAtPath: ohMyConfigRepo.ohMyOpenAgentConfigURL.deletingLastPathComponent().path
        )

        let useCase = SwitchGroupUseCase(
            modelGroupRepository: modelGroupRepo,
            appStateRepository: appStateRepo,
            backupRepository: backupRepo,
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

        let currentData = try Data(
            contentsOf: ohMyConfigRepo.ohMyOpenAgentConfigURL
        )
        XCTAssertEqual(currentData, originalData)
    }

    func testSwitchNoOpsWhenTargetAlreadyActive() async throws {
        let (harness, useCase) = try makeHarness()
        try seedOhMyOpenAgentConfig(configRootURL: harness.opencodeConfigURL)

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
        try seedOhMyOpenAgentConfig(configRootURL: harness.opencodeConfigURL)

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
