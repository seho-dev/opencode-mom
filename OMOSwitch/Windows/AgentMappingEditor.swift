import SwiftUI

struct AgentMappingEditor: View {
  @Binding var overrides: [ModelGroupAgentOverride]
  @State private var draftRows: [DraftRow] = []
  @State private var warnings: [Int: String] = [:]

  struct DraftRow: Identifiable, Equatable {
    let id = UUID()
    var agentName: String
    var modelRef: String
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      ForEach(Array($draftRows.enumerated()), id: \.element.id) { index, $row in
        HStack(spacing: 8) {
          TextField("Agent Name", text: $row.agentName)
            .textFieldStyle(.roundedBorder)
            .frame(minWidth: 120)
          TextField("Model Ref", text: $row.modelRef)
            .textFieldStyle(.roundedBorder)
            .frame(minWidth: 120)
          Button {
            draftRows.remove(at: index)
            warnings.removeValue(forKey: index)
            rebuildWarnings()
          } label: {
            Image(systemName: "trash")
              .foregroundStyle(.red)
          }
          .buttonStyle(.plain)
        }

        if let warning = warnings[index] {
          Text(warning)
            .font(.caption)
            .foregroundStyle(.orange)
        }
      }

      Button {
        draftRows.append(DraftRow(agentName: "", modelRef: ""))
      } label: {
        Label("Add Agent", systemImage: "plus")
      }
    }
    .onAppear { syncFromSource() }
    .onChange(of: overrides, perform: { _ in syncFromSource() })
    .onChange(of: draftRows, perform: { _ in rebuildWarnings() })
  }

  // MARK: - Sync

  private func syncFromSource() {
    draftRows = overrides.map {
      DraftRow(agentName: $0.agentName, modelRef: $0.modelRef)
    }
    warnings.removeAll()
  }

  /// Call this to push draft back to the binding. Returns true if valid.
  func commitToSource() -> Bool {
    rebuildWarnings()
    guard warnings.isEmpty else { return false }

    let cleaned = draftRows.filter { row in
      let name = row.agentName.trimmingCharacters(in: .whitespacesAndNewlines)
      let ref = row.modelRef.trimmingCharacters(in: .whitespacesAndNewlines)
      return !name.isEmpty && !ref.isEmpty
    }.map {
      ModelGroupAgentOverride(
        agentName: $0.agentName.trimmingCharacters(in: .whitespacesAndNewlines),
        modelRef: $0.modelRef.trimmingCharacters(in: .whitespacesAndNewlines)
      )
    }
    overrides = cleaned
    return true
  }

  // MARK: - Validation

  private func rebuildWarnings() {
    warnings.removeAll()
    for i in draftRows.indices {
      let name = draftRows[i].agentName.trimmingCharacters(in: .whitespacesAndNewlines)
      let ref = draftRows[i].modelRef.trimmingCharacters(in: .whitespacesAndNewlines)

      // Blank row → silently ignored
      if name.isEmpty && ref.isEmpty { continue }

      // Half-filled
      if name.isEmpty || ref.isEmpty {
        warnings[i] = "Both fields are required."
        continue
      }

      // Duplicate key (case-insensitive)
      let hasDup = draftRows.enumerated().contains { j, other in
        j != i
        && !other.agentName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        && other.agentName.trimmingCharacters(in: .whitespacesAndNewlines).caseInsensitiveCompare(name) == .orderedSame
      }
      if hasDup {
        warnings[i] = "Duplicate agent name \"\(name)\"."
      }
    }
  }
}
