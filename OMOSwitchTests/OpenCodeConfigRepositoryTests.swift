import Foundation
import XCTest
@testable import OMOSwitch

final class OpenCodeConfigRepositoryTests: XCTestCase {

    func testCanonicalPathUsesOpencodeJSON() throws {
        let tempDir = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(tempDir) }

        let repository = OpenCodeConfigRepository(configRootURL: tempDir)
        let expectedURL = tempDir
            .appending(path: "opencode", directoryHint: .isDirectory)
            .appending(path: "opencode.json", directoryHint: .notDirectory)

        XCTAssertEqual(repository.openCodeConfigURL, expectedURL)
    }

    func testMissingConfigReturnsFileNotFoundFailure() throws {
        let tempDir = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(tempDir) }

        let repository = OpenCodeConfigRepository(configRootURL: tempDir)
        XCTAssertEqual(repository.load(), .failure(.fileNotFound))
    }

    func testLoadFailsForMalformedConfig() throws {
        let tempDir = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(tempDir) }

        let repository = OpenCodeConfigRepository(configRootURL: tempDir)
        let opencodeDirectory = tempDir.appending(path: "opencode", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: opencodeDirectory, withIntermediateDirectories: true)
        try "NOT VALID JSON {{{".write(
            to: repository.openCodeConfigURL,
            atomically: true,
            encoding: .utf8
        )

        XCTAssertEqual(repository.load(), .failure(.malformedConfig))
    }

    func testLoadsExistingFixtureFromCanonicalPath() throws {
        let tempDir = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(tempDir) }

        let repository = OpenCodeConfigRepository(configRootURL: tempDir)
        let opencodeDirectory = tempDir.appending(path: "opencode", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: opencodeDirectory, withIntermediateDirectories: true)

        let fixtureData = try FixtureLoader.fixtureData(
            named: "current-opencode.json",
            subdirectory: "opencode"
        )
        try fixtureData.write(to: repository.openCodeConfigURL, options: [.atomic])

        let result = repository.load()
        guard case .success(let document) = result else {
            XCTFail("Expected successful load of fixture-backed OpenCode config")
            return
        }

        XCTAssertTrue(document.agents.keys.contains("creative-ui-coder"))
        XCTAssertTrue(document.agents.keys.contains("Jenny"))
        XCTAssertTrue(document.agents.keys.contains("karen"))
    }

    func testSaveAndLoadRoundTripPreservesDocument() throws {
        let tempDir = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(tempDir) }

        let repository = OpenCodeConfigRepository(configRootURL: tempDir)
        let document = OpenCodeDocument(rawDictionary: [
            "$schema": "https://opencode.ai/config.json",
            "plugin": ["oh-my-openagent"],
            "agent": [
                "karen": [
                    "model": "cliproxyapi/minimax-m2.7",
                    "description": "Assess completion state",
                    "mode": "subagent",
                ]
            ],
            "provider": [
                "cliproxyapi": [
                    "name": "CLIProxyAPI"
                ]
            ],
        ])

        try repository.save(document)

        let loadResult = repository.load()
        guard case .success(let roundTripped) = loadResult else {
            XCTFail("Expected successful load after save")
            return
        }

        XCTAssertEqual(roundTripped, document)
        XCTAssertTrue(FileManager.default.fileExists(atPath: repository.openCodeConfigURL.path()))
    }
}
