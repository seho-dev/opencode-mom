import SwiftUI

struct CategoryMappingEditor: View {
  @Binding var mappings: [ModelGroupCategoryMapping]
  @State private var draftRows: [DraftRow] = []
  @State private var warnings: [Int: String] = [:]

  struct DraftRow: Identifiable, Equatable {
    let id = UUID()
    var categoryName: String
    var modelRef: String
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      ForEach(Array($draftRows.enumerated()), id: \.element.id) { index, $row in
        HStack(spacing: 8) {
          TextField("Category Name", text: $row.categoryName)
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
        draftRows.append(DraftRow(categoryName: "", modelRef: ""))
      } label: {
        Label("Add Category", systemImage: "plus")
      }
    }
    .onAppear { syncFromSource() }
    .onChange(of: mappings, perform: { _ in syncFromSource() })
    .onChange(of: draftRows, perform: { _ in rebuildWarnings() })
  }

  // MARK: - Sync

  private func syncFromSource() {
    draftRows = mappings.map {
      DraftRow(categoryName: $0.categoryName, modelRef: $0.modelRef)
    }
    warnings.removeAll()
  }

  /// Call this to push draft back to the binding. Returns true if valid.
  func commitToSource() -> Bool {
    rebuildWarnings()
    guard warnings.isEmpty else { return false }

    let cleaned = draftRows.filter { row in
      let name = row.categoryName.trimmingCharacters(in: .whitespacesAndNewlines)
      let ref = row.modelRef.trimmingCharacters(in: .whitespacesAndNewlines)
      return !name.isEmpty && !ref.isEmpty
    }.map {
      ModelGroupCategoryMapping(
        categoryName: $0.categoryName.trimmingCharacters(in: .whitespacesAndNewlines),
        modelRef: $0.modelRef.trimmingCharacters(in: .whitespacesAndNewlines)
      )
    }
    mappings = cleaned
    return true
  }

  // MARK: - Validation

  private func rebuildWarnings() {
    warnings.removeAll()
    for i in draftRows.indices {
      let name = draftRows[i].categoryName.trimmingCharacters(in: .whitespacesAndNewlines)
      let ref = draftRows[i].modelRef.trimmingCharacters(in: .whitespacesAndNewlines)

      if name.isEmpty && ref.isEmpty { continue }

      if name.isEmpty || ref.isEmpty {
        warnings[i] = "Both fields are required."
        continue
      }

      let hasDup = draftRows.enumerated().contains { j, other in
        j != i
        && !other.categoryName.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
        && other.categoryName.trimmingCharacters(in: .whitespacesAndNewlines).caseInsensitiveCompare(name) == .orderedSame
      }
      if hasDup {
        warnings[i] = "Duplicate category name \"\(name)\"."
      }
    }
  }
}
