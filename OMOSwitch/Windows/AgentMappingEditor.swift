import SwiftUI

struct AgentMappingEditor: View {
  @Binding var overrides: [ModelGroupAgentOverride]
  @State private var draftRows: [DraftRow] = []
  @State private var warnings: [Int: String] = [:]
  @State private var isSyncingFromSource = false
  @State private var isSyncingToSource = false

  struct DraftRow: Identifiable, Equatable {
    let id: String
    var agentName: String
    var modelRef: String
    var isKnownKey: Bool
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      ForEach(Array($draftRows.enumerated()), id: \.element.id) { index, $row in
        HStack(spacing: 8) {
          if row.isKnownKey {
            Text(row.agentName)
              .frame(minWidth: 120, alignment: .leading)
              .padding(.horizontal, 6)
              .padding(.vertical, 4)
              .background(Color(nsColor: .controlBackgroundColor))
              .clipShape(RoundedRectangle(cornerRadius: 4))
          } else {
            TextField("Agent Name", text: $row.agentName)
              .textFieldStyle(.roundedBorder)
              .frame(minWidth: 120)
          }
          TextField("Model Ref", text: $row.modelRef)
            .textFieldStyle(.roundedBorder)
            .frame(minWidth: 120)
            .placeholder(when: row.modelRef.isEmpty) {
              Text("Optional").foregroundStyle(.tertiary)
            }
          if !row.isKnownKey {
            Button {
              draftRows.remove(at: index)
              warnings.removeValue(forKey: index)
              rebuildWarnings()
              syncToSource()
            } label: {
              Image(systemName: "trash")
                .foregroundStyle(.red)
            }
            .buttonStyle(.plain)
          }
        }

        if let warning = warnings[index] {
          Text(warning)
            .font(.caption)
            .foregroundStyle(.orange)
        }
      }

      Button {
        draftRows.append(DraftRow(id: UUID().uuidString, agentName: "", modelRef: "", isKnownKey: false))
      } label: {
        Label("Add Custom Agent", systemImage: "plus")
      }
    }
    .onAppear { syncFromSource() }
    .onChange(of: overrides, perform: { _ in
      guard !isSyncingToSource else {
        isSyncingToSource = false
        return
      }
      syncFromSource()
    })
    .onChange(of: draftRows, perform: { _ in
      guard !isSyncingFromSource else { return }
      rebuildWarnings()
      syncToSource()
    })
  }

  // MARK: - Sync

  private func syncFromSource() {
    isSyncingFromSource = true
    defer { isSyncingFromSource = false }

    let existingOverrides = Dictionary(uniqueKeysWithValues: overrides.map { ($0.agentName, $0.modelRef) })
    let knownSet = Set(KnownKeys.agentNames)
    var existingCustomIDs = draftRows
      .filter { !$0.isKnownKey }
      .map(\.id)

    var rows: [DraftRow] = []

    // Known keys first
    for name in KnownKeys.agentNames {
      let ref = existingOverrides[name] ?? ""
      rows.append(DraftRow(id: "known:\(name)", agentName: name, modelRef: ref, isKnownKey: true))
    }

    // Custom keys not in known list
    for override in overrides {
      if !knownSet.contains(override.agentName) {
        rows.append(
          DraftRow(
            id: existingCustomIDs.isEmpty ? UUID().uuidString : existingCustomIDs.removeFirst(),
            agentName: override.agentName,
            modelRef: override.modelRef,
            isKnownKey: false
          )
        )
      }
    }

    draftRows = rows
    warnings.removeAll()
  }

  private func syncToSource() {
    let cleaned = draftRows.compactMap { row -> ModelGroupAgentOverride? in
      let name = row.agentName.trimmingCharacters(in: .whitespacesAndNewlines)
      guard !name.isEmpty else { return nil }
      return ModelGroupAgentOverride(
        agentName: name,
        modelRef: row.modelRef.trimmingCharacters(in: .whitespacesAndNewlines)
      )
    }
    isSyncingToSource = true
    overrides = cleaned
  }

  /// Call this to push draft back to the binding. Returns true if valid.
  func commitToSource() -> Bool {
    rebuildWarnings()
    guard warnings.isEmpty else { return false }
    syncToSource()
    return true
  }

  // MARK: - Validation

  private func rebuildWarnings() {
    warnings.removeAll()
    for i in draftRows.indices {
      let name = draftRows[i].agentName.trimmingCharacters(in: .whitespacesAndNewlines)

      // Known keys skip name validation
      if draftRows[i].isKnownKey { continue }

      // Blank custom row → silently ignored
      if name.isEmpty { continue }

      // Duplicate key among custom rows (case-insensitive)
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

// MARK: - TextField placeholder extension

extension View {
  func placeholder<Content: View>(when shouldShow: Bool, alignment: Alignment = .leading, @ViewBuilder placeholder: () -> Content) -> some View {
    ZStack(alignment: alignment) {
      placeholder().opacity(shouldShow ? 1 : 0)
      self
    }
  }
}
