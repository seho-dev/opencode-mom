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
    XCTAssertEqual(presentation.staleOverrides, [])
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

  func testStaleOverridesAreAbsentFromSuccessfulDiscoveryPresentation() {
    let presentation = OpenCodeAgentMappingEditor.presentation(
      overrides: [
        ModelGroupAgentOverride(agentName: "stale", modelRef: "openai/o3"),
        ModelGroupAgentOverride(agentName: "alpha", modelRef: "openai/gpt-5.4"),
      ],
      discoveredAgentNames: ["alpha"],
      discoveryError: nil
    )

    XCTAssertFalse(presentation.isReadOnly)
    XCTAssertEqual(presentation.discoveredRows.map(\.agentName), ["alpha"])
    XCTAssertEqual(presentation.staleOverrides, [])
    XCTAssertEqual(presentation.preservedOverrides, [])
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
    XCTAssertEqual(presentation.staleOverrides, [])
    XCTAssertEqual(presentation.preservedOverrides.map(\.agentName), ["alpha", "stale"])
    XCTAssertFalse(presentation.allowsCustomAgentCreation)
    XCTAssertEqual(updated, overrides)
  }
}
