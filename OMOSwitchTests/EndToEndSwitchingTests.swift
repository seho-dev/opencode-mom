import Foundation
import XCTest
@testable import OMOSwitch

@MainActor
final class EndToEndSwitchingTests: XCTestCase {
    private final class Tick: @unchecked Sendable {
        private var value: Int = 0

        func next() -> Date {
            defer { value += 1 }
            return Date(timeIntervalSince1970: 1_700_000_000 + TimeInterval(value))
        }
    }

    func testSwitchFromTemporaryHomeRewritesOhMyConfigAndState() async throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()
        try harness.setupOpencodeConfig()

        let group = makeGroup(
            id: UUID(uuidString: "44444444-4444-4444-4444-444444444444")!,
            name: "Shipping",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "quick", modelRef: "cliproxyapi/minimax-m2.7"),
                ModelGroupCategoryMapping(categoryName: "unspecified-high", modelRef: "cliproxyapi/gpt-5.4"),
            ],
            agentOverrides: [
                ModelGroupAgentOverride(agentName: "oracle", modelRef: "cliproxyapi/gpt-5.4"),
                ModelGroupAgentOverride(agentName: "explore", modelRef: "cliproxyapi/gpt-5.4"),
            ]
        )
        try makeModelGroupRepository(harness).save([group])
        try harness.installFixture(named: "current-oh-my-openagent.json", subdirectory: "ohmy", to: "oh-my-openagent.json")

        let useCase = makeSwitchUseCase(harness)
        let result = await useCase.switchTo(groupID: group.id)

        guard case .success = result else {
            XCTFail("Expected success, got \(result)")
            return
        }

        let configRepo = makeOhMyConfigRepository(harness)
        let loadResult = configRepo.load()
        guard case .success(let document) = loadResult else {
            XCTFail("Expected written config to load")
            return
        }

        XCTAssertEqual((document.agents["oracle"] as? [String: Any])?["model"] as? String, "cliproxyapi/gpt-5.4")
        XCTAssertEqual((document.agents["explore"] as? [String: Any])?["model"] as? String, "cliproxyapi/gpt-5.4")
        XCTAssertEqual((document.categories["quick"] as? [String: Any])?["model"] as? String, "cliproxyapi/minimax-m2.7")
        XCTAssertEqual((document.categories["unspecified-high"] as? [String: Any])?["model"] as? String, "cliproxyapi/gpt-5.4")

        let state = try makeAppStateRepository(harness).load()
        XCTAssertEqual(state.selectedGroupID, group.id)
        XCTAssertEqual(state.selectedGroupName, "Shipping")
        XCTAssertEqual(state.lastSuccessfulWrite?.target, "switch-group")
        XCTAssertNotNil(state.lastSuccessfulWrite?.wroteAt)
    }

    func testSwitchWithOpenCodeOverridesRewritesBothTargets() async throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()
        try harness.setupOpencodeConfig()

        let group = makeGroup(
            id: UUID(uuidString: "99999999-9999-9999-9999-999999999999")!,
            name: "Dual Target",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "quick", modelRef: "cliproxyapi/minimax-m2.7"),
                ModelGroupCategoryMapping(categoryName: "unspecified-high", modelRef: "cliproxyapi/gpt-5.4-xhigh"),
            ],
            agentOverrides: [
                ModelGroupAgentOverride(agentName: "oracle", modelRef: "cliproxyapi/gpt-5.4-xhigh")
            ],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "creative-ui-coder", modelRef: "cliproxyapi/gpt-5.4"),
                ModelGroupAgentOverride(agentName: "Jenny", modelRef: "cliproxyapi/gpt-5.4-xhigh"),
            ]
        )
        try makeModelGroupRepository(harness).save([group])
        try harness.installFixture(named: "current-oh-my-openagent.json", subdirectory: "ohmy", to: "oh-my-openagent.json")
        try harness.installFixture(named: "current-opencode.json", subdirectory: "opencode", to: "opencode.json")

        let useCase = makeSwitchUseCase(harness)
        let result = await useCase.switchTo(groupID: group.id)

        guard case .success = result else {
            XCTFail("Expected success, got \(result)")
            return
        }

        let ohMyRepository = makeOhMyConfigRepository(harness)
        let openCodeRepository = makeOpenCodeConfigRepository(harness)

        guard case .success(let ohMyDocument) = ohMyRepository.load() else {
            XCTFail("Expected written oh-my-openagent config to load")
            return
        }
        guard case .success(let openCodeDocument) = openCodeRepository.load() else {
            XCTFail("Expected written opencode config to load")
            return
        }

        XCTAssertEqual((ohMyDocument.agents["oracle"] as? [String: Any])?["model"] as? String, "cliproxyapi/gpt-5.4-xhigh")
        XCTAssertEqual((ohMyDocument.categories["unspecified-high"] as? [String: Any])?["model"] as? String, "cliproxyapi/gpt-5.4-xhigh")
        XCTAssertEqual((openCodeDocument.agents["creative-ui-coder"] as? [String: Any])?["model"] as? String, "cliproxyapi/gpt-5.4")
        XCTAssertEqual((openCodeDocument.agents["Jenny"] as? [String: Any])?["model"] as? String, "cliproxyapi/gpt-5.4-xhigh")

        let state = try makeAppStateRepository(harness).load()
        XCTAssertEqual(state.selectedGroupID, group.id)
        XCTAssertEqual(state.selectedGroupName, "Dual Target")
        XCTAssertEqual(state.lastSuccessfulWrite?.target, "switch-group")
        XCTAssertTrue(state.lastSuccessfulWrite?.backupPath?.contains("opencode:") == true)
        XCTAssertTrue(state.lastSuccessfulWrite?.backupPath?.contains("oh-my-openagent:") == true)
    }

    func testSwitchPreservesSiblingFieldsInOhMyOpenAgentEntries() async throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()
        try harness.setupOpencodeConfig()

        let group = makeGroup(
            id: UUID(uuidString: "12121212-3434-5656-7878-909090909090")!,
            name: "Preserve Siblings",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "quick", modelRef: "cliproxyapi/minimax-m2.7")
            ],
            agentOverrides: [
                ModelGroupAgentOverride(agentName: "librarian", modelRef: "cliproxyapi/gpt-5.4")
            ]
        )
        try makeModelGroupRepository(harness).save([group])

        let customOhMyDocument: [String: Any] = [
            "$schema": "https://example.com/schema.json",
            "customTopLevel": ["keep": true],
            "agents": [
                "librarian": [
                    "model": "cliproxyapi/legacy-model",
                    "variant": "medium",
                    "temperature": 0.2
                ],
                "oracle": [
                    "model": "cliproxyapi/legacy-model",
                    "variant": "xhigh"
                ]
            ],
            "categories": [
                "quick": [
                    "model": "cliproxyapi/legacy-model",
                    "variant": "balanced"
                ],
                "deep": [
                    "model": "cliproxyapi/legacy-model",
                    "variant": "slow"
                ]
            ]
        ]
        let ohMyData = try XCTUnwrap(JSONSerialization.data(withJSONObject: customOhMyDocument, options: [.prettyPrinted, .sortedKeys]))
        try ohMyData.write(to: makeOhMyConfigRepository(harness).ohMyOpenAgentConfigURL, options: [.atomic])

        let result = await makeSwitchUseCase(harness).switchTo(groupID: group.id)

        guard case .success = result else {
            XCTFail("Expected success, got \(result)")
            return
        }

        guard case .success(let document) = makeOhMyConfigRepository(harness).load() else {
            XCTFail("Expected written oh-my-openagent config to load")
            return
        }

        let librarian = try XCTUnwrap(document.agents["librarian"] as? [String: Any])
        XCTAssertEqual(librarian["model"] as? String, "cliproxyapi/gpt-5.4")
        XCTAssertEqual(librarian["variant"] as? String, "medium")
        XCTAssertEqual(librarian["temperature"] as? Double, 0.2)
        XCTAssertNil(document.agents["oracle"])

        let quick = try XCTUnwrap(document.categories["quick"] as? [String: Any])
        XCTAssertEqual(quick["model"] as? String, "cliproxyapi/minimax-m2.7")
        XCTAssertEqual(quick["variant"] as? String, "balanced")
        XCTAssertNil(document.categories["deep"])

        let customTopLevel = try XCTUnwrap(document.rawDictionary["customTopLevel"] as? [String: Any])
        XCTAssertEqual(customTopLevel["keep"] as? Bool, true)
    }

    func testSwitchWithOpenCodeOverridesOnlyChangesAgentModelFields() async throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()
        try harness.setupOpencodeConfig()

        let group = makeGroup(
            id: UUID(uuidString: "aaaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa")!,
            name: "Selective OpenCode",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "quick", modelRef: "cliproxyapi/minimax-m2.7")
            ],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "creative-ui-coder", modelRef: "cliproxyapi/gpt-5.4-xhigh")
            ]
        )
        try makeModelGroupRepository(harness).save([group])
        try harness.installFixture(named: "current-oh-my-openagent.json", subdirectory: "ohmy", to: "oh-my-openagent.json")
        try harness.installFixture(named: "current-opencode.json", subdirectory: "opencode", to: "opencode.json")

        let beforeDocument = try XCTUnwrap(loadJSONObject(from: harness.opencodeConfigURL.appendingPathComponent("opencode.json")))
        let beforeAgents = try XCTUnwrap(beforeDocument["agent"] as? [String: Any])
        let beforeCreative = try XCTUnwrap(beforeAgents["creative-ui-coder"] as? [String: Any])
        let beforeKaren = try XCTUnwrap(beforeAgents["karen"] as? [String: Any])
        let beforePlugin = try XCTUnwrap(beforeDocument["plugin"] as? [AnyHashable])

        let result = await makeSwitchUseCase(harness).switchTo(groupID: group.id)

        guard case .success = result else {
            XCTFail("Expected success, got \(result)")
            return
        }

        let afterDocument = try XCTUnwrap(loadJSONObject(from: harness.opencodeConfigURL.appendingPathComponent("opencode.json")))
        let afterAgents = try XCTUnwrap(afterDocument["agent"] as? [String: Any])
        let afterCreative = try XCTUnwrap(afterAgents["creative-ui-coder"] as? [String: Any])
        let afterKaren = try XCTUnwrap(afterAgents["karen"] as? [String: Any])
        let afterPlugin = try XCTUnwrap(afterDocument["plugin"] as? [AnyHashable])

        XCTAssertEqual(afterCreative["model"] as? String, "cliproxyapi/gpt-5.4-xhigh")
        XCTAssertEqual(afterCreative["mode"] as? String, beforeCreative["mode"] as? String)
        XCTAssertEqual(afterCreative["description"] as? String, beforeCreative["description"] as? String)
        XCTAssertEqual(afterCreative["prompt"] as? String, beforeCreative["prompt"] as? String)
        XCTAssertEqual(afterKaren as NSDictionary, beforeKaren as NSDictionary)
        XCTAssertEqual(afterPlugin, beforePlugin)
        XCTAssertEqual(afterDocument["$schema"] as? String, beforeDocument["$schema"] as? String)
        XCTAssertEqual((afterDocument["provider"] as? NSDictionary), (beforeDocument["provider"] as? NSDictionary))
    }

    func testSwitchWithoutEffectiveOpenCodeOverridesSkipsMissingOpenCodeConfig() async throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()
        try harness.setupOpencodeConfig()

        let group = makeGroup(
            id: UUID(uuidString: "bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb")!,
            name: "Skip OpenCode",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "quick", modelRef: "cliproxyapi/gpt-5.4")
            ],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "creative-ui-coder", modelRef: "   ")
            ]
        )
        try makeModelGroupRepository(harness).save([group])
        try harness.installFixture(named: "current-oh-my-openagent.json", subdirectory: "ohmy", to: "oh-my-openagent.json")

        let opencodeURL = harness.opencodeConfigURL.appendingPathComponent("opencode.json")
        XCTAssertFalse(FileManager.default.fileExists(atPath: opencodeURL.path))

        let result = await makeSwitchUseCase(harness).switchTo(groupID: group.id)

        guard case .success = result else {
            XCTFail("Expected success, got \(result)")
            return
        }

        XCTAssertFalse(FileManager.default.fileExists(atPath: opencodeURL.path))

        let ohMyLoadResult = makeOhMyConfigRepository(harness).load()
        guard case .success(let document) = ohMyLoadResult else {
            XCTFail("Expected written oh-my-openagent config")
            return
        }

        XCTAssertEqual((document.categories["quick"] as? [String: Any])?["model"] as? String, "cliproxyapi/gpt-5.4")
    }

    func testSwitchCreatesAndCleansUpBackups() async throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()
        try harness.setupOpencodeConfig()
        try harness.installFixture(named: "current-oh-my-openagent.json", subdirectory: "ohmy", to: "oh-my-openagent.json")

        let groups = (0..<6).map { index in
            makeGroup(
                id: UUID(uuidString: String(format: "55555555-5555-5555-5555-%012d", index + 1))!,
                name: "Group \(index)",
                categoryMappings: [
                    ModelGroupCategoryMapping(categoryName: "quick", modelRef: "model-\(index)"),
                ]
            )
        }
        try makeModelGroupRepository(harness).save(groups)

        let tick = Tick()
        let useCase = makeSwitchUseCase(harness, now: { tick.next() })
        for group in groups {
            let result = await useCase.switchTo(groupID: group.id)
            guard case .success = result else {
                XCTFail("Expected success, got \(result)")
                return
            }
        }

        let backupRepo = makeBackupRepository(harness)
        let backups = try backupRepo.listBackups(for: "oh-my-openagent")
        XCTAssertEqual(backups.count, 5)
    }

    func testReloadHydratesStoreFromDiskAfterExternalGroupChange() throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()
        try harness.setupOpencodeConfig()

        let store = makeAppStore(harness)
        let initialGroup = makeGroup(
            id: UUID(uuidString: "66666666-6666-6666-6666-666666666666")!,
            name: "Initial",
            categoryMappings: [ModelGroupCategoryMapping(categoryName: "quick", modelRef: "initial-model")]
        )
        try makeModelGroupRepository(harness).save([initialGroup])
        store.reload()
        XCTAssertEqual(store.groups.map(\.name), ["Initial"])

        let externalGroups = [
            makeGroup(
                id: UUID(uuidString: "77777777-7777-7777-7777-777777777777")!,
                name: "External",
                categoryMappings: [ModelGroupCategoryMapping(categoryName: "deep", modelRef: "external-model")]
            )
        ]
        try makeModelGroupRepository(harness).save(externalGroups)

        store.reload()

        XCTAssertEqual(store.groups, externalGroups)
        XCTAssertEqual(store.groups.map(\.name), ["External"])
    }

    func testMalformedTargetConfigLeavesDiskUnchanged() async throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()
        try harness.setupOpencodeConfig()

        let group = makeGroup(
            id: UUID(uuidString: "88888888-8888-8888-8888-888888888888")!,
            name: "Broken Input",
            categoryMappings: [ModelGroupCategoryMapping(categoryName: "quick", modelRef: "safe-model")]
        )
        try makeModelGroupRepository(harness).save([group])

        let malformedURL = makeOhMyConfigRepository(harness).ohMyOpenAgentConfigURL
        let malformedContent = "{ invalid json".data(using: .utf8)!
        try malformedContent.write(to: malformedURL, options: [.atomic])

        let useCase = makeSwitchUseCase(harness)
        let result = await useCase.switchTo(groupID: group.id)

        guard case .failure = result else {
            XCTFail("Expected failure, got \(result)")
            return
        }

        let diskData = try Data(contentsOf: malformedURL)
        XCTAssertEqual(diskData, malformedContent)
    }

    func testMalformedOpenCodeConfigLeavesBothTargetsUnchanged() async throws {
        let harness = TemporaryHomeHarness()
        try harness.setupOmoSwitchConfig()
        try harness.setupOpencodeConfig()

        let group = makeGroup(
            id: UUID(uuidString: "cccccccc-cccc-cccc-cccc-cccccccccccc")!,
            name: "Broken OpenCode",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "quick", modelRef: "cliproxyapi/gpt-5.4")
            ],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "creative-ui-coder", modelRef: "cliproxyapi/gpt-5.4-xhigh")
            ]
        )
        try makeModelGroupRepository(harness).save([group])
        try harness.installFixture(named: "current-oh-my-openagent.json", subdirectory: "ohmy", to: "oh-my-openagent.json")

        let malformedURL = harness.opencodeConfigURL.appendingPathComponent("opencode.json")
        let malformedContent = Data("{ invalid json".utf8)
        try malformedContent.write(to: malformedURL, options: [.atomic])

        let originalOhMyData = try Data(contentsOf: makeOhMyConfigRepository(harness).ohMyOpenAgentConfigURL)

        let result = await makeSwitchUseCase(harness).switchTo(groupID: group.id)

        guard case .failure = result else {
            XCTFail("Expected failure, got \(result)")
            return
        }

        XCTAssertEqual(try Data(contentsOf: malformedURL), malformedContent)
        XCTAssertEqual(try Data(contentsOf: makeOhMyConfigRepository(harness).ohMyOpenAgentConfigURL), originalOhMyData)
    }

    private func makeAppStore(_ harness: TemporaryHomeHarness) -> AppStore {
        let modelGroupRepository = makeModelGroupRepository(harness)
        let appStateRepository = makeAppStateRepository(harness)
        let switchUseCase = makeSwitchUseCase(harness)
        return AppStore(
            modelGroupRepository: modelGroupRepository,
            appStateRepository: appStateRepository,
            openCodeConfigRepository: makeOpenCodeConfigRepository(harness),
            switchUseCase: switchUseCase,
            loginItemService: StubLoginItemService()
        )
    }

    private func makeSwitchUseCase(_ harness: TemporaryHomeHarness, now: @escaping @Sendable () -> Date = Date.init) -> SwitchGroupUseCase {
        SwitchGroupUseCase(
            modelGroupRepository: makeModelGroupRepository(harness),
            appStateRepository: makeAppStateRepository(harness),
            backupRepository: makeBackupRepository(harness, now: now),
            openCodeConfigRepository: makeOpenCodeConfigRepository(harness),
            ohMyConfigRepository: makeOhMyConfigRepository(harness)
        )
    }

    private func makeModelGroupRepository(_ harness: TemporaryHomeHarness) -> ModelGroupRepository {
        ModelGroupRepository(fileManager: .default, configRootURL: harness.omoSwitchConfigURL)
    }

    private func makeAppStateRepository(_ harness: TemporaryHomeHarness) -> AppStateRepository {
        AppStateRepository(fileManager: .default, configRootURL: harness.omoSwitchConfigURL)
    }

    private func makeBackupRepository(_ harness: TemporaryHomeHarness, now: @escaping @Sendable () -> Date = Date.init) -> BackupRepository {
        BackupRepository(fileManager: .default, configRootURL: harness.omoSwitchConfigURL, now: now)
    }

    private func makeOhMyConfigRepository(_ harness: TemporaryHomeHarness) -> OhMyOpenAgentConfigRepository {
        let configRootURL = harness.homeURL.appendingPathComponent(".config", isDirectory: true)
        return OhMyOpenAgentConfigRepository(fileManager: .default, configRootURL: configRootURL)
    }

    private func makeOpenCodeConfigRepository(_ harness: TemporaryHomeHarness) -> OpenCodeConfigRepository {
        let configRootURL = harness.homeURL.appendingPathComponent(".config", isDirectory: true)
        return OpenCodeConfigRepository(fileManager: .default, configRootURL: configRootURL)
    }

    private func makeGroup(
        id: UUID,
        name: String,
        categoryMappings: [ModelGroupCategoryMapping],
        agentOverrides: [ModelGroupAgentOverride] = [],
        openCodeAgentOverrides: [ModelGroupAgentOverride] = [],
        isEnabled: Bool = true
    ) -> ModelGroup {
        ModelGroup(
            id: id,
            name: name,
            categoryMappings: categoryMappings,
            agentOverrides: agentOverrides,
            openCodeAgentOverrides: openCodeAgentOverrides,
            isEnabled: isEnabled,
            updatedAt: Date(timeIntervalSince1970: 1_700_000_000)
        )
    }

    private func loadJSONObject(from url: URL) throws -> [String: Any]? {
        let data = try Data(contentsOf: url)
        return try JSONSerialization.jsonObject(with: data) as? [String: Any]
    }
}

@MainActor
private struct StubLoginItemService: LoginItemService {
    func currentStatus() throws -> LoginItemStatus { .disabled }
    func setEnabled(_ isEnabled: Bool) throws {}
}
