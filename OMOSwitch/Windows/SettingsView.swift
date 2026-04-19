import SwiftUI

struct SettingsView: View {
  @ObservedObject var appStore: AppStore
  @State private var selectedGroupID: UUID?
  @State private var draftGroup: ModelGroup?
  @State private var baselineGroup: ModelGroup?
  @State private var validationMessage: String?
  @State private var persistenceMessage: String?
  @State private var showingDeleteConfirmation = false
  @State private var draftCategoryMappings: [ModelGroupCategoryMapping] = []
  @State private var draftAgentOverrides: [ModelGroupAgentOverride] = []

  var body: some View {
    NavigationSplitView {
      sidebar
    } detail: {
      detail
    }
    .frame(minWidth: 700, minHeight: 500)
    .onAppear { appStore.reload() }
    .onChange(of: selectedGroupID) { newValue in
      loadDraft(for: newValue)
    }
  }

  // MARK: - Sidebar

  private var sidebar: some View {
    List(displayGroups, selection: $selectedGroupID) { group in
      HStack {
        Circle()
          .fill(group.isEnabled ? Color.green : Color.gray.opacity(0.4))
          .frame(width: 8, height: 8)
        Text(group.name.isEmpty ? "Untitled Group" : group.name)
          .lineLimit(1)
        Spacer()
        if group.id == appStore.currentGroupID {
          Text("Active")
            .font(.caption2)
            .foregroundStyle(.secondary)
            .padding(.horizontal, 6)
            .padding(.vertical, 2)
            .background(Color.accentColor.opacity(0.15))
            .clipShape(Capsule())
        }
      }
      .tag(group.id)
    }
    .listStyle(.sidebar)
    .overlay {
      if displayGroups.isEmpty {
        Text("No groups yet.\nCreate one to get started.")
          .foregroundStyle(.secondary)
          .multilineTextAlignment(.center)
      }
    }
  }

  // MARK: - Detail

  private var detail: some View {
    Group {
      if let group = selectedGroup {
        groupDetail(group)
      } else {
        emptyDetail
      }
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity)
  }

  private var emptyDetail: some View {
    VStack(spacing: 12) {
      Image(systemName: "sidebar.left")
        .font(.system(size: 36))
        .foregroundStyle(.secondary)
      Text("Select a group")
        .font(.title3)
        .foregroundStyle(.secondary)
      Button {
        let newGroup = ModelGroup(
          name: "",
          description: nil,
          categoryMappings: [],
          agentOverrides: [],
          isEnabled: true
        )
        baselineGroup = nil
        draftGroup = newGroup
        draftCategoryMappings = []
        draftAgentOverrides = []
        validationMessage = validationError(for: newGroup)
        persistenceMessage = nil
        selectedGroupID = newGroup.id
      } label: {
        Label("New Group", systemImage: "plus")
      }
      .buttonStyle(.borderedProminent)
    }
  }

  private func groupDetail(_ group: ModelGroup) -> some View {
    VStack(alignment: .leading, spacing: 0) {
      toolbar(group: group)
      Divider()
      ScrollView {
        VStack(alignment: .leading, spacing: 20) {
          if let validationMessage {
            Text(validationMessage)
              .foregroundStyle(.red)
          }

          if let persistenceMessage {
            Text(persistenceMessage)
              .foregroundStyle(.red)
          }

          formSection(title: "Group Metadata") {
            TextField("Name", text: draftName)
              .textFieldStyle(.roundedBorder)

            TextField("Description", text: draftDescription, axis: .vertical)
              .textFieldStyle(.roundedBorder)
              .lineLimit(2...4)

            Toggle("Enabled", isOn: draftIsEnabled)
          }

          LabeledContent("Last Updated") {
            Text(group.updatedAt, style: .date)
          }

          Divider()

          sectionHeader("Category Mappings", count: draftCategoryMappings.count)
          CategoryMappingEditor(mappings: $draftCategoryMappings)

          Divider()

          sectionHeader("Agent Overrides", count: draftAgentOverrides.count)
          AgentMappingEditor(overrides: $draftAgentOverrides)
        }
        .padding(20)
      }
    }
  }

  private func formSection<Content: View>(title: String, @ViewBuilder content: () -> Content) -> some View {
    VStack(alignment: .leading, spacing: 12) {
      Text(title)
        .font(.headline)
      content()
    }
  }

  private func sectionHeader(_ title: String, count: Int) -> some View {
    HStack {
      Text(title)
        .font(.headline)
      Text("(\(count))")
        .foregroundStyle(.secondary)
        .font(.subheadline)
    }
  }

  // MARK: - Toolbar

  private func toolbar(group: ModelGroup) -> some View {
    HStack(spacing: 8) {
      Button {
        let newGroup = ModelGroup(
          name: "",
          description: nil,
          categoryMappings: [],
          agentOverrides: [],
          isEnabled: true
        )
        baselineGroup = nil
        draftGroup = newGroup
        draftCategoryMappings = []
        draftAgentOverrides = []
        validationMessage = validationError(for: newGroup)
        persistenceMessage = nil
        selectedGroupID = newGroup.id
      } label: {
        Label("New Group", systemImage: "plus")
      }

      Spacer()

      Button("Save") {
        saveDraft()
      }
      .disabled(!canSave)

      Button("Cancel") {
        cancelEditing()
      }
      .disabled(draftGroup == nil)

      Button("Delete") {
        showingDeleteConfirmation = true
      }
      .disabled(selectedGroupID == nil)
      .foregroundStyle(.red)

      if appStore.groups.contains(where: { $0.id == group.id }) && group.id != appStore.currentGroupID {
        Button("Switch To This Group") {
          Task { await appStore.switchTo(groupID: group.id) }
        }
      }
    }
    .padding(.horizontal, 20)
    .padding(.vertical, 10)
    .background(.bar)
    .confirmationDialog(
      "Delete this group?",
      isPresented: $showingDeleteConfirmation,
      titleVisibility: .visible
    ) {
      Button("Delete", role: .destructive) {
        deleteSelectedGroup()
      }
      Button("Cancel", role: .cancel) {}
    } message: {
      Text("This action cannot be undone.")
    }
  }

  private var displayGroups: [ModelGroup] {
    guard let draftGroup, baselineGroup == nil else {
      return appStore.groups
    }

    let existingDraftIDs = Set(appStore.groups.map(\.id))
    guard existingDraftIDs.contains(draftGroup.id) == false else {
      return appStore.groups
    }

    return appStore.groups + [draftGroup]
  }

  private var selectedGroup: ModelGroup? {
    if let draftGroup, draftGroup.id == selectedGroupID {
      return draftGroup
    }

    return appStore.groups.first(where: { $0.id == selectedGroupID })
  }

  private var draftName: Binding<String> {
    Binding(
      get: { draftGroup?.name ?? "" },
      set: { newValue in
        updateDraft { draft in
          draft.name = newValue
        }
      }
    )
  }

  private var draftDescription: Binding<String> {
    Binding(
      get: { draftGroup?.description ?? "" },
      set: { newValue in
        updateDraft { draft in
          let trimmed = newValue.trimmingCharacters(in: .whitespacesAndNewlines)
          draft.description = trimmed.isEmpty ? nil : newValue
        }
      }
    )
  }

  private var draftIsEnabled: Binding<Bool> {
    Binding(
      get: { draftGroup?.isEnabled ?? false },
      set: { newValue in
        updateDraft { draft in
          draft.isEnabled = newValue
        }
      }
    )
  }

  private var canSave: Bool {
    draftGroup != nil && validationMessage == nil
  }

  private func loadDraft(for id: UUID?) {
    persistenceMessage = nil

    guard let id, let group = appStore.groups.first(where: { $0.id == id }) else {
      if baselineGroup != nil || draftGroup?.id != id {
        baselineGroup = nil
        draftGroup = nil
        draftCategoryMappings = []
        draftAgentOverrides = []
        validationMessage = nil
      }
      return
    }

    baselineGroup = group
    draftGroup = group
    draftCategoryMappings = group.categoryMappings
    draftAgentOverrides = group.agentOverrides
    validationMessage = validationError(for: group)
  }

  private func updateDraft(_ mutate: (inout ModelGroup) -> Void) {
    guard var draftGroup else { return }
    mutate(&draftGroup)
    self.draftGroup = draftGroup
    validationMessage = validationError(for: draftGroup)
    persistenceMessage = nil
  }

  private func validationError(for group: ModelGroup) -> String? {
    let trimmedName = group.name.trimmingCharacters(in: .whitespacesAndNewlines)
    if trimmedName.isEmpty {
      return "Name is required."
    }

    let duplicateExists = appStore.groups.contains { existing in
      let existingTrimmedName = existing.name.trimmingCharacters(in: .whitespacesAndNewlines)
      return existing.id != group.id
        && existingTrimmedName.localizedCaseInsensitiveCompare(trimmedName) == .orderedSame
    }

    if duplicateExists {
      return "Name must be unique."
    }

    return nil
  }

  private func saveDraft() {
    guard var draftGroup else { return }

    if let validationMessage = validationError(for: draftGroup) {
      self.validationMessage = validationMessage
      return
    }

    draftGroup.name = draftGroup.name.trimmingCharacters(in: .whitespacesAndNewlines)
    let trimmedDescription = draftGroup.description?.trimmingCharacters(in: .whitespacesAndNewlines)
    draftGroup.description = trimmedDescription?.isEmpty == true ? nil : trimmedDescription
    draftGroup.categoryMappings = draftCategoryMappings
    draftGroup.agentOverrides = draftAgentOverrides
    draftGroup.updatedAt = Date()

    do {
      try appStore.saveGroup(draftGroup)
      baselineGroup = draftGroup
      self.draftGroup = draftGroup
      selectedGroupID = draftGroup.id
      validationMessage = nil
      persistenceMessage = nil
    } catch {
      persistenceMessage = error.localizedDescription
    }
  }

  private func cancelEditing() {
    persistenceMessage = nil

    if let baselineGroup {
      draftGroup = baselineGroup
      draftCategoryMappings = baselineGroup.categoryMappings
      draftAgentOverrides = baselineGroup.agentOverrides
      validationMessage = validationError(for: baselineGroup)
      return
    }

    draftGroup = nil
    draftCategoryMappings = []
    draftAgentOverrides = []
    validationMessage = nil
    selectedGroupID = nil
  }

  private func deleteSelectedGroup() {
    guard let id = selectedGroupID else { return }

    do {
      try appStore.deleteGroup(id: id)
      baselineGroup = nil
      draftGroup = nil
      draftCategoryMappings = []
      draftAgentOverrides = []
      validationMessage = nil
      persistenceMessage = nil
      selectedGroupID = nil
    } catch {
      persistenceMessage = error.localizedDescription
    }
  }
}
