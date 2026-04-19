import SwiftUI

struct CategoryMappingEditor: View {
  @Binding var mappings: [ModelGroupCategoryMapping]
  @State private var draftRows: [DraftRow] = []
  @State private var warnings: [Int: String] = [:]

  struct DraftRow: Identifiable, Equatable {
    let id = UUID()
    var categoryName: String
    var modelRef: String
    var isKnownKey: Bool
  }

  var body: some View {
    VStack(alignment: .leading, spacing: 8) {
      ForEach(Array($draftRows.enumerated()), id: \.element.id) { index, $row in
        HStack(spacing: 8) {
          if row.isKnownKey {
            Text(row.categoryName)
              .frame(minWidth: 120, alignment: .leading)
              .padding(.horizontal, 6)
              .padding(.vertical, 4)
              .background(Color(nsColor: .controlBackgroundColor))
              .clipShape(RoundedRectangle(cornerRadius: 4))
          } else {
            TextField("Category Name", text: $row.categoryName)
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
        draftRows.append(DraftRow(categoryName: "", modelRef: "", isKnownKey: false))
      } label: {
        Label("Add Custom Category", systemImage: "plus")
      }
    }
    .onAppear { syncFromSource() }
    .onChange(of: mappings, perform: { _ in syncFromSource() })
    .onChange(of: draftRows, perform: { _ in
      rebuildWarnings()
      syncToSource()
    })
  }

  // MARK: - Sync

  private func syncFromSource() {
    let existingMappings = Dictionary(uniqueKeysWithValues: mappings.map { ($0.categoryName, $0.modelRef) })
    let knownSet = Set(KnownKeys.categoryNames)

    var rows: [DraftRow] = []

    for name in KnownKeys.categoryNames {
      let ref = existingMappings[name] ?? ""
      rows.append(DraftRow(categoryName: name, modelRef: ref, isKnownKey: true))
    }

    for mapping in mappings {
      if !knownSet.contains(mapping.categoryName) {
        rows.append(DraftRow(categoryName: mapping.categoryName, modelRef: mapping.modelRef, isKnownKey: false))
      }
    }

    draftRows = rows
    warnings.removeAll()
  }

  private func syncToSource() {
    let cleaned = draftRows.compactMap { row -> ModelGroupCategoryMapping? in
      let name = row.categoryName.trimmingCharacters(in: .whitespacesAndNewlines)
      guard !name.isEmpty else { return nil }
      return ModelGroupCategoryMapping(
        categoryName: name,
        modelRef: row.modelRef.trimmingCharacters(in: .whitespacesAndNewlines)
      )
    }
    mappings = cleaned
  }

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
      let name = draftRows[i].categoryName.trimmingCharacters(in: .whitespacesAndNewlines)

      if draftRows[i].isKnownKey { continue }

      if name.isEmpty { continue }

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
