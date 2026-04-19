import Foundation
import XCTest
@testable import OMOSwitch

final class OhMyOpenAgentConfigRepositoryTests: XCTestCase {

    func testMissingConfigReturnsBootstrapDocument() throws {
        let tempDir = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(tempDir) }

        let repo = OhMyOpenAgentConfigRepository(configRootURL: tempDir)
        let result = repo.load()

        guard case .success(let doc) = result else {
            XCTFail("Expected success when no config file exists")
            return
        }

        let bootstrap = OhMyOpenAgentDocument.bootstrap()
        XCTAssertEqual(doc, bootstrap)
        XCTAssertNotNil(doc.rawDictionary["$schema"])
        XCTAssertTrue(doc.agents.isEmpty)
        XCTAssertTrue(doc.categories.isEmpty)
    }

    func testLoadsExistingConfig() throws {
        let tempDir = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(tempDir) }

        let repo = OhMyOpenAgentConfigRepository(configRootURL: tempDir)
        let opencodeDir = tempDir.appending(path: "opencode", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: opencodeDir, withIntermediateDirectories: true)

        let json = """
        {"$schema":"https://example.com/schema.json","agents":{"oracle":{"model":"gpt-5"}},"categories":{"high":["oracle"]}}
        """
        try json.write(
            to: opencodeDir.appending(path: "oh-my-openagent.json"),
            atomically: true,
            encoding: .utf8
        )

        let result = repo.load()

        guard case .success(let doc) = result else {
            XCTFail("Expected success loading existing config")
            return
        }

        XCTAssertEqual(doc.rawDictionary["$schema"] as? String, "https://example.com/schema.json")
        XCTAssertFalse(doc.agents.isEmpty)
        XCTAssertFalse(doc.categories.isEmpty)
    }

    func testLoadFailsForMalformedConfig() throws {
        let tempDir = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(tempDir) }

        let repo = OhMyOpenAgentConfigRepository(configRootURL: tempDir)
        let opencodeDir = tempDir.appending(path: "opencode", directoryHint: .isDirectory)
        try FileManager.default.createDirectory(at: opencodeDir, withIntermediateDirectories: true)

        try "NOT VALID JSON {{{".write(
            to: opencodeDir.appending(path: "oh-my-openagent.json"),
            atomically: true,
            encoding: .utf8
        )

        let result = repo.load()

        guard case .failure = result else {
            XCTFail("Expected failure for malformed config")
            return
        }
    }

    func testSaveCreatesCanonicalPathUnderOpencodeConfig() throws {
        let tempDir = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(tempDir) }

        let repo = OhMyOpenAgentConfigRepository(configRootURL: tempDir)
        let doc = OhMyOpenAgentDocument.bootstrap()

        try repo.save(doc)

        let expectedPath = tempDir
            .appending(path: "opencode", directoryHint: .isDirectory)
            .appending(path: "oh-my-openagent.json", directoryHint: .notDirectory)
        XCTAssertTrue(
            FileManager.default.fileExists(atPath: expectedPath.path()),
            "Save should create file at {configRoot}/opencode/oh-my-openagent.json"
        )
        XCTAssertEqual(repo.ohMyOpenAgentConfigURL, expectedPath)
    }

    func testSavePersistsCanonicalJSONAtomically() throws {
        let tempDir = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(tempDir) }

        let repo = OhMyOpenAgentConfigRepository(configRootURL: tempDir)
        let doc = OhMyOpenAgentDocument(rawDictionary: [
            "$schema": "https://example.com/schema.json",
            "agents": ["oracle": ["model": "gpt-5"]] as [String: Any],
            "categories": ["high": ["oracle"]] as [String: Any],
        ])

        try repo.save(doc)

        let loadResult = repo.load()

        guard case .success(let roundTrip) = loadResult else {
            XCTFail("Expected success on round-trip load")
            return
        }

        XCTAssertEqual(roundTrip, doc)
    }
}
