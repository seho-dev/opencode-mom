import Foundation
import XCTest
@testable import OMOSwitch

final class AppStateRepositoryTests: XCTestCase {
    func testPersistsSelectionAndProjectionMetadataOnly() throws {
        let rootURL = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(rootURL) }

        let repository = AppStateRepository(configRootURL: rootURL)
        let groupID = UUID(uuidString: "4F648C2A-1D48-44D6-B0C0-4D340816E1F3")
        let state = AppSelectionState(
            selectedGroupID: groupID,
            selectedGroupName: "Primary",
            lastSuccessfulWrite: LastSuccessfulWriteMetadata(
                target: "oh-my-openagent.categories",
                wroteAt: Date(timeIntervalSince1970: 1_700_000_500),
                backupPath: "/tmp/backup.json"
            ),
            lastWarningSummary: ProjectionIssueSummary(message: "undeclared refs", count: 2),
            lastErrorSummary: ProjectionIssueSummary(message: "write failed", count: 1)
        )

        try repository.save(state)
        let saved = try repository.load()
        XCTAssertEqual(saved, state)

        let payload = try String(contentsOf: rootURL.appending(path: "state.json"), encoding: .utf8)
        XCTAssertTrue(payload.contains("selectedGroupID"))
        XCTAssertTrue(payload.contains("lastSuccessfulWrite"))
        XCTAssertFalse(payload.contains("windowFrame"))
        XCTAssertFalse(payload.contains("popover"))
    }

    func testMissingStateReturnsDefaultState() throws {
        let rootURL = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(rootURL) }

        let repository = AppStateRepository(configRootURL: rootURL)
        XCTAssertEqual(try repository.load(), AppSelectionState())
    }
}
