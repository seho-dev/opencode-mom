import SwiftUI

struct CategoryMappingEditor: View {
  @Binding var mappings: [ModelGroupCategoryMapping]
  @State private var draftRows: [DraftRow] = []
  @State private var warnings: [Int: String] = [:]
  @State private var isSyncingFromSource = false
  @State private var isSyncingToSource = false

  struct DraftRow: Identifiable, Equatable {
    let id: String
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
        draftRows.append(DraftRow(id: UUID().uuidString, categoryName: "", modelRef: "", isKnownKey: false))
      } label: {
        Label("Add Custom Category", systemImage: "plus")
      }
    }
    .onAppear { syncFromSource() }
    .onChange(of: mappings, perform: { _ in
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

    let existingMappings = Dictionary(uniqueKeysWithValues: mappings.map { ($0.categoryName, $0.modelRef) })
    let knownSet = Set(KnownKeys.categoryNames)
    var existingCustomIDs = draftRows
      .filter { !$0.isKnownKey }
      .map(\.id)

    var rows: [DraftRow] = []

    for name in KnownKeys.categoryNames {
      let ref = existingMappings[name] ?? ""
      rows.append(DraftRow(id: "known:\(name)", categoryName: name, modelRef: ref, isKnownKey: true))
    }

    for mapping in mappings {
      if !knownSet.contains(mapping.categoryName) {
        rows.append(
          DraftRow(
            id: existingCustomIDs.isEmpty ? UUID().uuidString : existingCustomIDs.removeFirst(),
            categoryName: mapping.categoryName,
            modelRef: mapping.modelRef,
            isKnownKey: false
          )
        )
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
    isSyncingToSource = true
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
