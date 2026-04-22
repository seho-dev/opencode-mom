import Foundation
import XCTest
@testable import OMOSwitch

@MainActor
final class OpenCodeAgentMappingEditorTests: XCTestCase {
  func testDiscoveredRowsAreRenderedFromProvidedNamesOnly() {
    let presentation = OpenCodeAgentMappingEditor.presentation(
      overrides: [
        ModelGroupAgentOverride(agentName: "alpha", modelRef: " openai/gpt-5.4 "),
        ModelGroupAgentOverride(agentName: "stale", modelRef: "openai/o3"),
      ],
      discoveredAgentNames: ["beta", "alpha"],
      discoveryError: nil
    )

    XCTAssertEqual(presentation.discoveredRows.map(\.agentName), ["beta", "alpha"])
    XCTAssertEqual(presentation.discoveredRows.map(\.modelRef), ["", " openai/gpt-5.4 "])
    XCTAssertTrue(presentation.discoveredRows.allSatisfy(\.isEditable))
    XCTAssertEqual(presentation.staleOverrides.map(\.agentName), ["stale"])
  }

  func testCustomAgentCreationIsNotExposedOrAccepted() {
    let overrides = [ModelGroupAgentOverride(agentName: "alpha", modelRef: "openai/gpt-5.4")]
    let presentation = OpenCodeAgentMappingEditor.presentation(
      overrides: overrides,
      discoveredAgentNames: ["alpha"],
      discoveryError: nil
    )

    let updated = OpenCodeAgentMappingEditor.updatingModelRef(
      overrides: overrides,
      discoveredAgentNames: ["alpha"],
      discoveryError: nil,
      agentName: "custom-agent",
      modelRef: "openai/o3"
    )

    XCTAssertFalse(presentation.allowsCustomAgentCreation)
    XCTAssertEqual(updated, overrides)
  }

  func testStaleOverridesRemainInSourceAndAreReportedAsUndiscovered() {
    let staleOverride = ModelGroupAgentOverride(agentName: "stale", modelRef: "openai/o3")
    let updated = OpenCodeAgentMappingEditor.updatingModelRef(
      overrides: [staleOverride],
      discoveredAgentNames: ["alpha"],
      discoveryError: nil,
      agentName: "alpha",
      modelRef: " openai/gpt-5.4 "
    )

    let presentation = OpenCodeAgentMappingEditor.presentation(
      overrides: updated,
      discoveredAgentNames: ["alpha"],
      discoveryError: nil
    )

    XCTAssertEqual(updated, [
      staleOverride,
      ModelGroupAgentOverride(agentName: "alpha", modelRef: "openai/gpt-5.4"),
    ])
    XCTAssertEqual(presentation.staleOverrides.map(\.agentName), ["stale"])
    XCTAssertEqual(presentation.staleOverrides.map(\.status), ["Undiscovered"])
    XCTAssertEqual(
      presentation.staleOverrides.map(\.message),
      ["Ignored during switching until this agent is discovered again."]
    )
  }

  func testDiscoveryErrorRendersDegradedReadOnlyStateWithoutEditableRows() {
    let overrides = [
      ModelGroupAgentOverride(agentName: "alpha", modelRef: "openai/gpt-5.4"),
      ModelGroupAgentOverride(agentName: "stale", modelRef: "openai/o3"),
    ]
    let presentation = OpenCodeAgentMappingEditor.presentation(
      overrides: overrides,
      discoveredAgentNames: ["alpha"],
      discoveryError: "OpenCode config is malformed."
    )
    let updated = OpenCodeAgentMappingEditor.updatingModelRef(
      overrides: overrides,
      discoveredAgentNames: ["alpha"],
      discoveryError: "OpenCode config is malformed.",
      agentName: "alpha",
      modelRef: "openai/o3"
    )

    XCTAssertTrue(presentation.isReadOnly)
    XCTAssertEqual(presentation.discoveryError, "OpenCode config is malformed.")
    XCTAssertEqual(presentation.discoveredRows, [])
    XCTAssertEqual(presentation.preservedOverrides.map(\.agentName), ["alpha", "stale"])
    XCTAssertFalse(presentation.allowsCustomAgentCreation)
    XCTAssertEqual(updated, overrides)
  }
}
