import SwiftUI

struct OpenCodeAgentMappingEditor: View {
  @Binding var overrides: [ModelGroupAgentOverride]
  let discoveredAgentNames: [String]
  let discoveryError: String?

  struct Presentation: Equatable {
    let discoveredRows: [DiscoveredRow]
    let staleOverrides: [OverrideInfoRow]
    let preservedOverrides: [OverrideInfoRow]
    let discoveryError: String?
    let isReadOnly: Bool
    let allowsCustomAgentCreation: Bool
  }

  struct DiscoveredRow: Identifiable, Equatable {
    let id: String
    let agentName: String
    let modelRef: String
    let isEditable: Bool
  }

  struct OverrideInfoRow: Identifiable, Equatable {
    let id: String
    let agentName: String
    let modelRef: String
    let status: String
    let message: String
  }

  var body: some View {
    let presentation = Self.presentation(
      overrides: overrides,
      discoveredAgentNames: discoveredAgentNames,
      discoveryError: discoveryError
    )

    VStack(alignment: .leading, spacing: 8) {
      if presentation.isReadOnly {
        degradedState(presentation)
      } else {
        discoveredRows(presentation.discoveredRows)
      }
    }
  }

  static func presentation(
    overrides: [ModelGroupAgentOverride],
    discoveredAgentNames: [String],
    discoveryError: String?
  ) -> Presentation {
    let discoveredNames = uniqueNamesPreservingOrder(discoveredAgentNames)
    let isReadOnly = discoveryError != nil

    let discoveredRows: [DiscoveredRow]
    if isReadOnly {
      discoveredRows = []
    } else {
      discoveredRows = discoveredNames.map { name in
        DiscoveredRow(
          id: "discovered:\(name)",
          agentName: name,
          modelRef: modelRef(for: name, in: overrides),
          isEditable: true
        )
      }
    }

    let preservedOverrides = overrides.map { override in
      OverrideInfoRow(
        id: "preserved:\(override.agentName)",
        agentName: override.agentName,
        modelRef: override.modelRef,
        status: "Preserved",
        message: "Editing disabled until OpenCode agent discovery succeeds."
      )
    }

    return Presentation(
      discoveredRows: discoveredRows,
      staleOverrides: [],
      preservedOverrides: isReadOnly ? preservedOverrides : [],
      discoveryError: discoveryError,
      isReadOnly: isReadOnly,
      allowsCustomAgentCreation: false
    )
  }

  static func updatingModelRef(
    overrides: [ModelGroupAgentOverride],
    discoveredAgentNames: [String],
    discoveryError: String?,
    agentName: String,
    modelRef: String
  ) -> [ModelGroupAgentOverride] {
    guard discoveryError == nil else { return overrides }
    guard Set(discoveredAgentNames).contains(agentName) else { return overrides }

    let trimmedModelRef = modelRef.trimmingCharacters(in: .whitespacesAndNewlines)
    var didFindExistingOverride = false
    var updatedOverrides: [ModelGroupAgentOverride] = []

    for override in overrides {
      guard override.agentName == agentName else {
        updatedOverrides.append(override)
        continue
      }

      didFindExistingOverride = true
      if trimmedModelRef.isEmpty == false {
        updatedOverrides.append(ModelGroupAgentOverride(agentName: agentName, modelRef: trimmedModelRef))
      }
    }

    if didFindExistingOverride == false && trimmedModelRef.isEmpty == false {
      updatedOverrides.append(ModelGroupAgentOverride(agentName: agentName, modelRef: trimmedModelRef))
    }

    return updatedOverrides
  }

  private func discoveredRows(_ rows: [DiscoveredRow]) -> some View {
    VStack(alignment: .leading, spacing: 8) {
      if rows.isEmpty {
        Text("No OpenCode agents discovered.")
          .font(.caption)
          .foregroundStyle(.secondary)
      }

      ForEach(rows) { row in
        HStack(spacing: 8) {
          Text(row.agentName)
            .frame(minWidth: 160, alignment: .leading)
            .padding(.horizontal, 6)
            .padding(.vertical, 4)
            .background(Color(nsColor: .controlBackgroundColor))
            .clipShape(RoundedRectangle(cornerRadius: 4))

          TextField("Model Ref", text: modelRefBinding(for: row.agentName))
            .textFieldStyle(.roundedBorder)
            .frame(minWidth: 160)
            .placeholder(when: row.modelRef.isEmpty) {
              Text("Optional").foregroundStyle(.tertiary)
            }
        }
      }
    }
  }

  private func degradedState(_ presentation: Presentation) -> some View {
    VStack(alignment: .leading, spacing: 8) {
      if let discoveryError = presentation.discoveryError {
        Text("OpenCode agent discovery warning: \(discoveryError)")
          .font(.caption)
          .foregroundStyle(.orange)
      }

      Text("OpenCode agent overrides are read-only until discovery succeeds. Saved overrides are preserved.")
        .font(.caption)
        .foregroundStyle(.secondary)

      if presentation.preservedOverrides.isEmpty {
        Text("No saved OpenCode agent overrides.")
          .foregroundStyle(.secondary)
      } else {
        ForEach(presentation.preservedOverrides) { row in
          overrideInfoRow(row)
        }
      }
    }
  }

  private func overrideInfoRow(_ row: OverrideInfoRow) -> some View {
    VStack(alignment: .leading, spacing: 2) {
      HStack(spacing: 8) {
        Text(row.agentName.isEmpty ? "Unnamed OpenCode agent" : row.agentName)
          .frame(minWidth: 160, alignment: .leading)
          .padding(.horizontal, 6)
          .padding(.vertical, 4)
          .background(Color(nsColor: .controlBackgroundColor))
          .clipShape(RoundedRectangle(cornerRadius: 4))

        Text(row.modelRef.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? "Optional" : row.modelRef)
          .foregroundStyle(row.modelRef.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty ? .tertiary : .primary)

        Text(row.status)
          .font(.caption2)
          .foregroundStyle(.secondary)
      }

      Text(row.message)
        .font(.caption2)
        .foregroundStyle(.secondary)
    }
  }

  private func modelRefBinding(for agentName: String) -> Binding<String> {
    Binding(
      get: {
        Self.modelRef(for: agentName, in: overrides)
      },
      set: { newValue in
        overrides = Self.updatingModelRef(
          overrides: overrides,
          discoveredAgentNames: discoveredAgentNames,
          discoveryError: discoveryError,
          agentName: agentName,
          modelRef: newValue
        )
      }
    )
  }

  private static func modelRef(for agentName: String, in overrides: [ModelGroupAgentOverride]) -> String {
    overrides.first(where: { $0.agentName == agentName })?.modelRef ?? ""
  }

  private static func uniqueNamesPreservingOrder(_ names: [String]) -> [String] {
    var seen: Set<String> = []
    return names.filter { name in
      guard seen.contains(name) == false else { return false }
      seen.insert(name)
      return true
    }
  }
}
