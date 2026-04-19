import SwiftUI

struct SettingsView: View {
  @ObservedObject var appStore: AppStore
  @State private var selectedGroupID: UUID?

  var body: some View {
    NavigationSplitView {
      sidebar
    } detail: {
      detail
    }
    .frame(minWidth: 700, minHeight: 500)
    .onAppear { appStore.reload() }
  }

  // MARK: - Sidebar

  private var sidebar: some View {
    List(appStore.groups, selection: $selectedGroupID) { group in
      HStack {
        Circle()
          .fill(group.isEnabled ? Color.green : Color.gray.opacity(0.4))
          .frame(width: 8, height: 8)
        Text(group.name)
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
      if appStore.groups.isEmpty {
        Text("No groups yet.\nCreate one to get started.")
          .foregroundStyle(.secondary)
          .multilineTextAlignment(.center)
      }
    }
  }

  // MARK: - Detail

  private var detail: some View {
    Group {
      if let group = appStore.groups.first(where: { $0.id == selectedGroupID }) {
        groupDetail(group)
      } else {
        emptyDetail
      }
    }
    .frame(maxWidth: .infinity, maxHeight: .infinity)
  }

  private var emptyDetail: some View {
    VStack(spacing: 8) {
      Image(systemName: "sidebar.left")
        .font(.system(size: 36))
        .foregroundStyle(.secondary)
      Text("Select a group")
        .font(.title3)
        .foregroundStyle(.secondary)
    }
  }

  private func groupDetail(_ group: ModelGroup) -> some View {
    VStack(alignment: .leading, spacing: 0) {
      toolbar(group: group)
      Divider()
      ScrollView {
        VStack(alignment: .leading, spacing: 16) {
          // Name
          LabeledContent("Name") {
            Text(group.name)
          }
          // Description
          LabeledContent("Description") {
            Text(group.description ?? "—")
              .foregroundStyle(group.description == nil ? .secondary : .primary)
          }
          // Enabled
          LabeledContent("Enabled") {
            Text(group.isEnabled ? "Yes" : "No")
              .foregroundStyle(group.isEnabled ? .green : .red)
          }
          // Updated
          LabeledContent("Last Updated") {
            Text(group.updatedAt, style: .date)
          }

          Divider()

          // Category Mappings
          sectionHeader("Category Mappings", count: group.categoryMappings.count)
          if group.categoryMappings.isEmpty {
            Text("No mappings")
              .foregroundStyle(.secondary)
          } else {
            ForEach(group.categoryMappings, id: \.categoryName) { mapping in
              HStack {
                Text(mapping.categoryName)
                  .fontWeight(.medium)
                Spacer()
                Text(mapping.modelRef)
                  .foregroundStyle(.secondary)
              }
              .padding(.vertical, 2)
            }
          }

          Divider()

          // Agent Overrides
          sectionHeader("Agent Overrides", count: group.agentOverrides.count)
          if group.agentOverrides.isEmpty {
            Text("No overrides")
              .foregroundStyle(.secondary)
          } else {
            ForEach(group.agentOverrides, id: \.agentName) { override in
              HStack {
                Text(override.agentName)
                  .fontWeight(.medium)
                Spacer()
                Text(override.modelRef)
                  .foregroundStyle(.secondary)
              }
              .padding(.vertical, 2)
            }
          }
        }
        .padding(20)
      }
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
          name: "New Group",
          categoryMappings: []
        )
        try? appStore.saveGroup(newGroup)
        selectedGroupID = newGroup.id
      } label: {
        Label("New Group", systemImage: "plus")
      }

      Spacer()

      Button("Save") {
        // Placeholder — Task 8 implements full save
      }
      .disabled(true)

      Button("Delete") {
        if let id = selectedGroupID {
          try? appStore.deleteGroup(id: id)
          selectedGroupID = nil
        }
      }
      .foregroundStyle(.red)

      if group.id != appStore.currentGroupID {
        Button("Switch To This Group") {
          Task { await appStore.switchTo(groupID: group.id) }
        }
      }
    }
    .padding(.horizontal, 20)
    .padding(.vertical, 10)
    .background(.bar)
  }
}
