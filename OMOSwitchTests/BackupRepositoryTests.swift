import Foundation
import XCTest
@testable import OMOSwitch

final class BackupRepositoryTests: XCTestCase {
    func testCreatesTimestampedBackupsPerTarget() throws {
        let rootURL = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(rootURL) }

        let repository = BackupRepository(
            configRootURL: rootURL,
            now: { Date(timeIntervalSince1970: 1_700_000_000) }
        )

        let first = try repository.createBackup(
            for: "groups",
            sourceFileURL: URL(fileURLWithPath: "/tmp/groups.json"),
            contents: Data("group".utf8)
        )

        let second = try repository.createBackup(
            for: "oh-my-openagent.categories",
            sourceFileURL: URL(fileURLWithPath: "/tmp/oh-my-openagent.json"),
            contents: Data("projection".utf8)
        )

        XCTAssertTrue(FileManager.default.fileExists(atPath: first.path))
        XCTAssertTrue(FileManager.default.fileExists(atPath: second.path))
        XCTAssertTrue(first.path.contains("backups/groups/"))
        XCTAssertTrue(second.path.contains("backups/oh-my-openagent.categories/"))
        XCTAssertTrue(first.lastPathComponent.hasPrefix("20231114T221320Z-"))
        XCTAssertTrue(second.lastPathComponent.hasPrefix("20231114T221320Z-"))

        let groupsBackups = try repository.listBackups(for: "groups")
        XCTAssertEqual(groupsBackups.map(\.target), ["groups"])
        let groupsFirstPath = groupsBackups.first?.fileURL.path ?? ""
        let groupsFirstResolved = (groupsFirstPath as NSString).resolvingSymlinksInPath
        let firstResolvedFromCreate = (first.path as NSString).resolvingSymlinksInPath
        XCTAssertEqual(groupsFirstResolved, firstResolvedFromCreate)
    }

    func testCleanupKeepsLatestBackupsPerTarget() throws {
        let rootURL = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(rootURL) }

        final class Tick: @unchecked Sendable {
            private var value: Int = 0
            func next() -> Date {
                defer { value += 1 }
                return Date(timeIntervalSince1970: 1_700_000_000 + TimeInterval(value))
            }
        }
        let tick = Tick()

        let repository = BackupRepository(
            configRootURL: rootURL,
            now: { tick.next() }
        )

        _ = try repository.createBackup(for: "groups", sourceFileURL: URL(fileURLWithPath: "/tmp/groups.json"), contents: Data("1".utf8))
        _ = try repository.createBackup(for: "groups", sourceFileURL: URL(fileURLWithPath: "/tmp/groups.json"), contents: Data("2".utf8))
        _ = try repository.createBackup(for: "groups", sourceFileURL: URL(fileURLWithPath: "/tmp/groups.json"), contents: Data("3".utf8))

        try repository.cleanup(target: "groups", keepingLatest: 2)

        let remaining = try repository.listBackups(for: "groups")
        XCTAssertEqual(remaining.count, 2)
    }
}
