import SwiftUI

struct GlobalSettingsView: View {
  @ObservedObject var appStore: AppStore
  @State private var persistenceMessage: String?
  @State private var persistenceMessageColor: Color = .secondary
  @State private var isUpdatingLaunchAtLogin = false

  var body: some View {
    VStack(alignment: .leading, spacing: 16) {
      VStack(alignment: .leading, spacing: 12) {
        Toggle("Launch at login", isOn: launchAtLoginToggle)
          .disabled(isUpdatingLaunchAtLogin)

        Text("Applies to omo-switch globally, not any individual group.")
          .font(.subheadline)
          .foregroundStyle(.secondary)

        if let launchAtLoginStatusMessage = appStore.launchAtLoginStatusMessage {
          Text(launchAtLoginStatusMessage)
            .font(.subheadline)
            .foregroundStyle(.orange)
        }

        if let persistenceMessage {
          Text(persistenceMessage)
            .font(.subheadline)
            .foregroundStyle(persistenceMessageColor)
        }
      }

      Spacer()
    }
    .padding(20)
    .frame(minWidth: 420, minHeight: 180)
    .onAppear {
      appStore.reload()
    }
  }

  private var launchAtLoginToggle: Binding<Bool> {
    Binding(
      get: { appStore.launchAtLoginEnabled },
      set: { newValue in
        Task { await updateLaunchAtLogin(newValue) }
      }
    )
  }

  private func updateLaunchAtLogin(_ isEnabled: Bool) async {
    guard isUpdatingLaunchAtLogin == false else { return }
    isUpdatingLaunchAtLogin = true
    defer { isUpdatingLaunchAtLogin = false }

    do {
      try appStore.setLaunchAtLoginEnabled(isEnabled)
      if let launchAtLoginStatusMessage = appStore.launchAtLoginStatusMessage {
        persistenceMessage = launchAtLoginStatusMessage
        persistenceMessageColor = .orange
      } else {
        persistenceMessage = isEnabled ? "Launch at login enabled." : "Launch at login disabled."
        persistenceMessageColor = .green
      }
    } catch {
      persistenceMessage = error.localizedDescription
      persistenceMessageColor = .red
    }
  }
}

struct SettingsView: View {
  struct SelectionSyncState: Equatable {
    let selectedGroupID: UUID?
    let activeGroupID: UUID?
    let baselineGroup: ModelGroup?
    let draftGroup: ModelGroup?
    let draftCategoryMappings: [ModelGroupCategoryMapping]
    let draftAgentOverrides: [ModelGroupAgentOverride]
    let draftOpenCodeAgentOverrides: [ModelGroupAgentOverride]
  }

  struct OpenCodeAgentOverrideSectionCounts: Equatable {
    let filled: Int
    let total: Int
  }

  struct CurrentGroupModelMatchCounts: Equatable {
    let categoryMappings: Int
    let agentOverrides: Int
    let openCodeAgentOverrides: Int

    var total: Int {
      categoryMappings + agentOverrides + openCodeAgentOverrides
    }
  }

  struct CurrentGroupModelReplaceResult: Equatable {
    let categoryMappings: [ModelGroupCategoryMapping]
    let agentOverrides: [ModelGroupAgentOverride]
    let openCodeAgentOverrides: [ModelGroupAgentOverride]
    let matchCounts: CurrentGroupModelMatchCounts
  }

  private enum PersistenceMessageTone {
    case success
    case warning
    case error

    var color: Color {
      switch self {
      case .success:
        .green
      case .warning:
        .orange
      case .error:
        .red
      }
    }
  }

  private static let lastUpdatedFormatter: DateFormatter = {
    let formatter = DateFormatter()
    formatter.locale = Locale(identifier: "en_US_POSIX")
    formatter.timeZone = .current
    formatter.dateFormat = "yyyy-MM-dd HH:mm"
    return formatter
  }()

  @ObservedObject var appStore: AppStore
  @State private var selectedGroupID: UUID?
  @State private var draftGroup: ModelGroup?
  @State private var baselineGroup: ModelGroup?
  @State private var validationMessage: String?
  @State private var persistenceMessage: String?
  @State private var persistenceMessageTone: PersistenceMessageTone = .error
  @State private var showingDeleteConfirmation = false
  @State private var draftCategoryMappings: [ModelGroupCategoryMapping] = []
  @State private var draftAgentOverrides: [ModelGroupAgentOverride] = []
  @State private var draftOpenCodeAgentOverrides: [ModelGroupAgentOverride] = []
  @State private var currentGroupModelSearchValue = ""
  @State private var currentGroupModelReplaceValue = ""

  var body: some View {
    NavigationSplitView {
      sidebar
    } detail: {
      detail
    }
    .frame(minWidth: 700, minHeight: 500)
    .onAppear {
      appStore.reload()
      syncSelectionFromActiveGroupIfSafe()
    }
    .onChange(of: appStore.currentGroupID) { _ in
      syncSelectionFromActiveGroupIfSafe()
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
    .onChange(of: selectedGroupID) { newValue in
      loadDraft(for: newValue)
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
          .id(group.id)
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
          openCodeAgentOverrides: [],
          isEnabled: true
        )
        baselineGroup = nil
        draftGroup = newGroup
        draftCategoryMappings = []
        draftAgentOverrides = []
        draftOpenCodeAgentOverrides = []
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
              .foregroundStyle(persistenceMessageTone.color)
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
            Text(Self.lastUpdatedFormatter.string(from: group.updatedAt))
          }

          Divider()

          currentGroupModelSearchReplaceSection

          Divider()

          sectionHeader("Category Mappings", filled: draftCategoryMappings.filter { !$0.modelRef.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }.count, total: KnownKeys.categoryNames.count + draftCategoryMappings.filter { !KnownKeys.categoryNames.contains($0.categoryName) }.count)
          CategoryMappingEditor(mappings: $draftCategoryMappings)

          Divider()

          sectionHeader("Agent Overrides", filled: draftAgentOverrides.filter { !$0.modelRef.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty }.count, total: KnownKeys.agentNames.count + draftAgentOverrides.filter { !KnownKeys.agentNames.contains($0.agentName) }.count)
          AgentMappingEditor(overrides: $draftAgentOverrides)

          Divider()

          let openCodeCounts = SettingsView.openCodeAgentOverrideSectionCounts(
            overrides: draftOpenCodeAgentOverrides,
            discoveredAgentNames: appStore.discoveredOpenCodeAgentNames
          )
          sectionHeader("OpenCode Agent Overrides", filled: openCodeCounts.filled, total: openCodeCounts.total)
          OpenCodeAgentMappingEditor(
            overrides: $draftOpenCodeAgentOverrides,
            discoveredAgentNames: appStore.discoveredOpenCodeAgentNames,
            discoveryError: appStore.openCodeAgentDiscoveryError
          )
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

  private func sectionHeader(_ title: String, filled: Int, total: Int) -> some View {
    HStack {
      Text(title)
        .font(.headline)
      Text("(\(filled)/\(total))")
        .foregroundStyle(.secondary)
        .font(.subheadline)
    }
  }

  private var currentGroupModelSearchReplaceSection: some View {
    let matchCounts = SettingsView.currentGroupModelMatchCounts(
      searchValue: currentGroupModelSearchValue,
      draftCategoryMappings: draftCategoryMappings,
      draftAgentOverrides: draftAgentOverrides,
      draftOpenCodeAgentOverrides: draftOpenCodeAgentOverrides
    )

    return formSection(title: "Current Group Model Batch Replace") {
      Text("Searches only the currently edited draft group across category mappings, agent overrides, and OpenCode agent overrides.")
        .font(.subheadline)
        .foregroundStyle(.secondary)

      HStack(alignment: .top, spacing: 12) {
        TextField("Search model", text: $currentGroupModelSearchValue)
          .textFieldStyle(.roundedBorder)

        TextField("Replace with", text: $currentGroupModelReplaceValue)
          .textFieldStyle(.roundedBorder)
      }

      HStack(alignment: .firstTextBaseline, spacing: 12) {
        Text(currentGroupModelMatchSummary(for: matchCounts))
          .font(.subheadline)
          .foregroundStyle(.secondary)

        Button("Replace All Exact Matches") {
          replaceAllCurrentGroupModelMatches()
        }
        .disabled(matchCounts.total == 0)
      }
    }
  }

  private func replaceAllCurrentGroupModelMatches() {
    let result = SettingsView.replacingCurrentGroupModelMatches(
      searchValue: currentGroupModelSearchValue,
      replaceValue: currentGroupModelReplaceValue,
      draftCategoryMappings: draftCategoryMappings,
      draftAgentOverrides: draftAgentOverrides,
      draftOpenCodeAgentOverrides: draftOpenCodeAgentOverrides
    )

    guard result.matchCounts.total > 0 else { return }

    draftCategoryMappings = result.categoryMappings
    draftAgentOverrides = result.agentOverrides
    draftOpenCodeAgentOverrides = result.openCodeAgentOverrides
    setPersistenceMessage(
      "Replaced \(result.matchCounts.total) exact model match\(result.matchCounts.total == 1 ? "" : "es") in the current draft group.",
      tone: .success
    )
  }

  private func currentGroupModelMatchSummary(for counts: CurrentGroupModelMatchCounts) -> String {
    let trimmedSearchValue = SettingsView.trimmedModelSearchReplaceValue(currentGroupModelSearchValue)

    guard trimmedSearchValue.isEmpty == false else {
      return "Enter a model to count exact matches in this draft group."
    }

    return "\(counts.total) match\(counts.total == 1 ? "" : "es") total · Categories \(counts.categoryMappings) · Agents \(counts.agentOverrides) · OpenCode \(counts.openCodeAgentOverrides)"
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
          openCodeAgentOverrides: [],
          isEnabled: true
        )
        baselineGroup = nil
        draftGroup = newGroup
        draftCategoryMappings = []
        draftAgentOverrides = []
        draftOpenCodeAgentOverrides = []
        validationMessage = validationError(for: newGroup)
        persistenceMessage = nil
        selectedGroupID = newGroup.id
      } label: {
        Label("New Group", systemImage: "plus")
      }

      Spacer()

      Button("Save") {
        Task { await saveDraft() }
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

      if group.id != appStore.currentGroupID {
        Button("Switch To This Group") {
          Task { await switchToDraftGroup() }
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

  static func openCodeAgentOverrideSectionCounts(
    overrides: [ModelGroupAgentOverride],
    discoveredAgentNames: [String]
  ) -> OpenCodeAgentOverrideSectionCounts {
    let discoveredAgentNameSet = Set(discoveredAgentNames)
    let filled = overrides.filter { override in
      discoveredAgentNameSet.contains(override.agentName)
        && override.modelRef.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty == false
    }.count

    return OpenCodeAgentOverrideSectionCounts(filled: filled, total: discoveredAgentNames.count)
  }

  static func retainedOpenCodeAgentOverrides(
    from group: ModelGroup,
    discoveredAgentNames _: [String],
    discoveryError _: String?
  ) -> [ModelGroupAgentOverride] {
    return group.openCodeAgentOverrides
  }

  static func trimmedModelSearchReplaceValue(_ value: String) -> String {
    value.trimmingCharacters(in: .whitespacesAndNewlines)
  }

  static func currentGroupModelMatchCounts(
    searchValue: String,
    draftCategoryMappings: [ModelGroupCategoryMapping],
    draftAgentOverrides: [ModelGroupAgentOverride],
    draftOpenCodeAgentOverrides: [ModelGroupAgentOverride]
  ) -> CurrentGroupModelMatchCounts {
    let trimmedSearchValue = trimmedModelSearchReplaceValue(searchValue)

    guard trimmedSearchValue.isEmpty == false else {
      return CurrentGroupModelMatchCounts(categoryMappings: 0, agentOverrides: 0, openCodeAgentOverrides: 0)
    }

    return CurrentGroupModelMatchCounts(
      categoryMappings: draftCategoryMappings.filter { trimmedModelSearchReplaceValue($0.modelRef) == trimmedSearchValue }.count,
      agentOverrides: draftAgentOverrides.filter { trimmedModelSearchReplaceValue($0.modelRef) == trimmedSearchValue }.count,
      openCodeAgentOverrides: draftOpenCodeAgentOverrides.filter { trimmedModelSearchReplaceValue($0.modelRef) == trimmedSearchValue }.count
    )
  }

  static func replacingCurrentGroupModelMatches(
    searchValue: String,
    replaceValue: String,
    draftCategoryMappings: [ModelGroupCategoryMapping],
    draftAgentOverrides: [ModelGroupAgentOverride],
    draftOpenCodeAgentOverrides: [ModelGroupAgentOverride]
  ) -> CurrentGroupModelReplaceResult {
    let trimmedSearchValue = trimmedModelSearchReplaceValue(searchValue)
    let trimmedReplaceValue = trimmedModelSearchReplaceValue(replaceValue)
    let matchCounts = currentGroupModelMatchCounts(
      searchValue: trimmedSearchValue,
      draftCategoryMappings: draftCategoryMappings,
      draftAgentOverrides: draftAgentOverrides,
      draftOpenCodeAgentOverrides: draftOpenCodeAgentOverrides
    )

    guard trimmedSearchValue.isEmpty == false else {
      return CurrentGroupModelReplaceResult(
        categoryMappings: draftCategoryMappings,
        agentOverrides: draftAgentOverrides,
        openCodeAgentOverrides: draftOpenCodeAgentOverrides,
        matchCounts: matchCounts
      )
    }

    let updatedCategoryMappings = draftCategoryMappings.map { mapping in
      guard trimmedModelSearchReplaceValue(mapping.modelRef) == trimmedSearchValue else { return mapping }
      return ModelGroupCategoryMapping(categoryName: mapping.categoryName, modelRef: trimmedReplaceValue)
    }

    let updatedAgentOverrides = draftAgentOverrides.map { override in
      guard trimmedModelSearchReplaceValue(override.modelRef) == trimmedSearchValue else { return override }
      return ModelGroupAgentOverride(agentName: override.agentName, modelRef: trimmedReplaceValue)
    }

    let updatedOpenCodeAgentOverrides = draftOpenCodeAgentOverrides.map { override in
      guard trimmedModelSearchReplaceValue(override.modelRef) == trimmedSearchValue else { return override }
      return ModelGroupAgentOverride(agentName: override.agentName, modelRef: trimmedReplaceValue)
    }

    return CurrentGroupModelReplaceResult(
      categoryMappings: updatedCategoryMappings,
      agentOverrides: updatedAgentOverrides,
      openCodeAgentOverrides: updatedOpenCodeAgentOverrides,
      matchCounts: matchCounts
    )
  }

  static func persistedDraftGroup(
    draftGroup: ModelGroup,
    draftCategoryMappings: [ModelGroupCategoryMapping],
    draftAgentOverrides: [ModelGroupAgentOverride],
    draftOpenCodeAgentOverrides: [ModelGroupAgentOverride],
    updatedAt: Date
  ) -> ModelGroup {
    var persistedGroup = draftGroup
    persistedGroup.name = persistedGroup.name.trimmingCharacters(in: .whitespacesAndNewlines)
    let trimmedDescription = persistedGroup.description?.trimmingCharacters(in: .whitespacesAndNewlines)
    persistedGroup.description = trimmedDescription?.isEmpty == true ? nil : trimmedDescription
    persistedGroup.categoryMappings = draftCategoryMappings
    persistedGroup.agentOverrides = draftAgentOverrides
    persistedGroup.openCodeAgentOverrides = draftOpenCodeAgentOverrides
    persistedGroup.updatedAt = updatedAt
    return persistedGroup
  }

  private var selectionSyncState: SelectionSyncState {
    SelectionSyncState(
      selectedGroupID: selectedGroupID,
      activeGroupID: appStore.currentGroupID,
      baselineGroup: baselineGroup,
      draftGroup: draftGroup,
      draftCategoryMappings: draftCategoryMappings,
      draftAgentOverrides: draftAgentOverrides,
      draftOpenCodeAgentOverrides: draftOpenCodeAgentOverrides,
    )
  }

  private var hasUnsavedLocalDraft: Bool {
    SettingsView.shouldPreserveSelectedGroupID(for: selectionSyncState)
  }

  private func syncSelectionFromActiveGroupIfSafe() {
    guard hasUnsavedLocalDraft == false else { return }
    guard selectedGroupID != appStore.currentGroupID else { return }
    selectedGroupID = appStore.currentGroupID
  }

  static func shouldPreserveSelectedGroupID(for state: SelectionSyncState) -> Bool {
    if state.baselineGroup == nil {
      return state.draftGroup != nil
    }

    guard let baselineGroup = state.baselineGroup, let draftGroup = state.draftGroup else {
      return false
    }

    return draftGroup.name != baselineGroup.name
      || draftGroup.description != baselineGroup.description
      || draftGroup.isEnabled != baselineGroup.isEnabled
      || state.draftCategoryMappings != baselineGroup.categoryMappings
      || state.draftAgentOverrides != baselineGroup.agentOverrides
      || state.draftOpenCodeAgentOverrides != baselineGroup.openCodeAgentOverrides
  }

  private func loadDraft(for id: UUID?) {
    persistenceMessage = nil

    guard let id, let group = appStore.groups.first(where: { $0.id == id }) else {
      if baselineGroup != nil || draftGroup?.id != id {
        baselineGroup = nil
        draftGroup = nil
        draftCategoryMappings = []
        draftAgentOverrides = []
        draftOpenCodeAgentOverrides = []
        validationMessage = nil
      }
      return
    }

    baselineGroup = group
    draftGroup = group
    draftCategoryMappings = group.categoryMappings
    draftAgentOverrides = group.agentOverrides
    draftOpenCodeAgentOverrides = SettingsView.retainedOpenCodeAgentOverrides(
      from: group,
      discoveredAgentNames: appStore.discoveredOpenCodeAgentNames,
      discoveryError: appStore.openCodeAgentDiscoveryError
    )
    validationMessage = validationError(for: group)
  }

  private func updateDraft(_ mutate: (inout ModelGroup) -> Void) {
    guard var draftGroup else { return }
    mutate(&draftGroup)
    self.draftGroup = draftGroup
    validationMessage = validationError(for: draftGroup)
    persistenceMessage = nil
  }

  private func setPersistenceMessage(_ message: String?, tone: PersistenceMessageTone = .error) {
    persistenceMessage = message
    persistenceMessageTone = tone
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

  @discardableResult
  private func persistDraft(showSuccessMessage: Bool) async -> ModelGroup? {
    guard var draftGroup else { return nil }

    if let validationMessage = validationError(for: draftGroup) {
      self.validationMessage = validationMessage
      setPersistenceMessage("Fix validation errors before saving.")
      return nil
    }

    draftGroup = SettingsView.persistedDraftGroup(
      draftGroup: draftGroup,
      draftCategoryMappings: draftCategoryMappings,
      draftAgentOverrides: draftAgentOverrides,
      draftOpenCodeAgentOverrides: draftOpenCodeAgentOverrides,
      updatedAt: Date()
    )

    do {
      try await appStore.saveGroup(draftGroup)
      baselineGroup = draftGroup
      self.draftGroup = draftGroup
      selectedGroupID = draftGroup.id
      validationMessage = nil
      if showSuccessMessage {
        setPersistenceMessage("Saved \(draftGroup.name).", tone: .success)
      } else {
        persistenceMessage = nil
      }
      return draftGroup
    } catch {
      setPersistenceMessage(error.localizedDescription)
      return nil
    }
  }

  private func saveDraft() async {
    _ = await persistDraft(showSuccessMessage: true)
  }

  private func switchFeedbackMessage(for result: SwitchResult, groupName: String) -> (message: String, tone: PersistenceMessageTone) {
    switch result {
    case .success(let projectionResult):
      if projectionResult.warnings.isEmpty {
        return ("Switched to \(groupName).", .success)
      }
      return ("Switched to \(groupName) with warnings: \(projectionResult.warnings.joined(separator: "; "))", .warning)
    case .noOp:
      return ("\(groupName) is already active.", .warning)
    case .failure(let message):
      return (message, .error)
    }
  }

  private func switchToDraftGroup() async {
    guard let draftGroup else { return }

    if let validationMessage = validationError(for: draftGroup) {
      self.validationMessage = validationMessage
      setPersistenceMessage("Fix validation errors before switching.")
      return
    }

    guard let savedGroup = await persistDraft(showSuccessMessage: false) else {
      if validationMessage == nil {
        setPersistenceMessage("Save failed. Group was not switched.")
      }
      return
    }

    let result = await appStore.switchTo(groupID: savedGroup.id)
    let feedback = switchFeedbackMessage(for: result, groupName: savedGroup.name)
    setPersistenceMessage(feedback.message, tone: feedback.tone)
  }

  private func cancelEditing() {
    persistenceMessage = nil

    if let baselineGroup {
      draftGroup = baselineGroup
      draftCategoryMappings = baselineGroup.categoryMappings
      draftAgentOverrides = baselineGroup.agentOverrides
      draftOpenCodeAgentOverrides = SettingsView.retainedOpenCodeAgentOverrides(
        from: baselineGroup,
        discoveredAgentNames: appStore.discoveredOpenCodeAgentNames,
        discoveryError: appStore.openCodeAgentDiscoveryError
      )
      validationMessage = validationError(for: baselineGroup)
      return
    }

    draftGroup = nil
    draftCategoryMappings = []
    draftAgentOverrides = []
    draftOpenCodeAgentOverrides = []
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
      draftOpenCodeAgentOverrides = []
      validationMessage = nil
      persistenceMessage = nil
      selectedGroupID = nil
    } catch {
      setPersistenceMessage(error.localizedDescription)
    }
  }
}
