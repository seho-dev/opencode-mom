import Foundation
import XCTest
@testable import OMOSwitch

final class ModelGroupRepositoryTests: XCTestCase {
    func testPersistsNamedGroupsToAppOwnedConfigOnly() throws {
        let rootURL = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(rootURL) }

        let repository = ModelGroupRepository(configRootURL: rootURL)
        let group = ModelGroup(
            name: "Primary",
            description: "canonical",
            categoryMappings: [
                ModelGroupCategoryMapping(categoryName: "unspecified-high", modelRef: "cliproxyapi/gpt-5.4")
            ],
            agentOverrides: [
                ModelGroupAgentOverride(agentName: "oracle", modelRef: "cliproxyapi/gpt-5.4-xhigh")
            ],
            openCodeAgentOverrides: [
                ModelGroupAgentOverride(agentName: "general", modelRef: "cliproxyapi/gpt-5.4-mini")
            ],
            updatedAt: Date(timeIntervalSince1970: 1_700_000_000)
        )

        try repository.save([group])

        XCTAssertTrue(FileManager.default.fileExists(atPath: rootURL.appending(path: "groups.json").path()))
        XCTAssertFalse(FileManager.default.fileExists(atPath: rootURL.appending(path: "../opencode/oh-my-openagent.json", directoryHint: .notDirectory).standardizedFileURL.path()))

        let roundTrip = try repository.load()
        XCTAssertEqual(roundTrip, [group])

        let payload = try String(contentsOf: rootURL.appending(path: "groups.json"), encoding: .utf8)
        XCTAssertTrue(payload.contains("\"migrationVersion\""))
        XCTAssertTrue(payload.contains("\"agentOverrides\""))
        XCTAssertTrue(payload.contains("\"openCodeAgentOverrides\""))
        XCTAssertFalse(payload.contains("opencode"))
    }

    func testLoadsLegacyPayloadWithoutOpenCodeAgentOverridesAsEmptyArray() throws {
        let rootURL = try TestSupport.makeTemporaryDirectory()
        defer { TestSupport.removeIfExists(rootURL) }

        let repository = ModelGroupRepository(configRootURL: rootURL)
        let payload = #"""
        {
          "migrationVersion" : 1,
          "groups" : [
            {
              "agentOverrides" : [
                {
                  "agentName" : "oracle",
                  "modelRef" : "cliproxyapi/gpt-5.4-xhigh"
                }
              ],
              "categoryMappings" : [
                {
                  "categoryName" : "unspecified-high",
                  "modelRef" : "cliproxyapi/gpt-5.4"
                }
              ],
              "description" : "legacy",
              "id" : "11111111-1111-1111-1111-111111111111",
              "isEnabled" : true,
              "name" : "Primary",
              "updatedAt" : "2023-11-14T22:13:20Z"
            }
          ]
        }
        """#

        try payload.write(to: rootURL.appending(path: "groups.json"), atomically: true, encoding: .utf8)

        let groups = try repository.load()
        XCTAssertEqual(groups.count, 1)
        XCTAssertEqual(groups[0].openCodeAgentOverrides, [])
    }

    func testUsesCanonicalGroupsLocationUnderOmoSwitchConfig() {
        let rootURL = ModelGroupRepository.defaultConfigRootURL()
        let standardizedPath = URL(fileURLWithPath: rootURL.path).standardizedFileURL.path
        XCTAssertEqual(standardizedPath.hasSuffix("/.config/omo-switch"), true)
        let canonicalPath = URL(fileURLWithPath: ModelGroupRepository().canonicalLocation().path).standardizedFileURL.path
        XCTAssertEqual(canonicalPath.hasSuffix("/.config/omo-switch/groups.json"), true)
    }
}
