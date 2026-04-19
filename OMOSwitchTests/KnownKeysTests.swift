import Foundation
import XCTest
@testable import OMOSwitch

final class KnownKeysTests: XCTestCase {

    func testAgentNamesIsNonEmpty() {
        XCTAssertFalse(KnownKeys.agentNames.isEmpty)
    }

    func testCategoryNamesIsNonEmpty() {
        XCTAssertFalse(KnownKeys.categoryNames.isEmpty)
    }

    func testAgentNamesContainsExpectedAgents() {
        let expected = ["hephaestus", "oracle", "librarian", "explore", "multimodal-looker",
                        "prometheus", "metis", "momus", "atlas", "sisyphus-junior", "sisyphus"]
        XCTAssertEqual(KnownKeys.agentNames, expected)
    }

    func testCategoryNamesContainsExpectedCategories() {
        let expected = ["visual-engineering", "ultrabrain", "deep", "artistry", "quick",
                        "unspecified-low", "unspecified-high", "writing"]
        XCTAssertEqual(KnownKeys.categoryNames, expected)
    }

    func testAgentNamesAreUnique() {
        XCTAssertEqual(KnownKeys.agentNames.count, Set(KnownKeys.agentNames).count)
    }

    func testCategoryNamesAreUnique() {
        XCTAssertEqual(KnownKeys.categoryNames.count, Set(KnownKeys.categoryNames).count)
    }
}
