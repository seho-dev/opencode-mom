import SwiftUI

struct QuickSwitchView: View {
  @ObservedObject var appStore: AppStore
  var onOpenGlobalSettings: (() -> Void)?
  var onOpenGroupSettings: (() -> Void)?

  private var currentGroup: ModelGroup? {
    guard let id = appStore.currentGroupID else { return nil }
    return appStore.groups.first { $0.id == id }
  }

  private var enabledGroups: [ModelGroup] {
    appStore.groups.filter(\.isEnabled)
  }

  var body: some View {
    ScrollView {
      VStack(alignment: .leading, spacing: 12) {
        currentGroupSection
        if let group = currentGroup {
          agentOverridesSection(group.agentOverrides)
          categoryMappingsSection(group.categoryMappings)
        }
        statusSection
        Divider()
        switchTargetsSection
        settingsButtons
      }
      .padding(16)
    }
    .frame(width: 320, height: 480)
  }

  // MARK: - Current Group

  private var currentGroupSection: some View {
    VStack(alignment: .leading, spacing: 4) {
      Text("Current Group")
        .font(.caption)
        .foregroundStyle(.secondary)
      if let group = currentGroup {
        Text(group.name)
          .font(.headline)
        if let desc = group.description, !desc.isEmpty {
          Text(desc)
            .font(.caption)
            .foregroundStyle(.secondary)
            .lineLimit(2)
        }
      } else {
        Text("None")
          .font(.headline)
          .foregroundStyle(.secondary)
      }
    }
  }

  // MARK: - Agent Overrides

  private func agentOverridesSection(_ overrides: [ModelGroupAgentOverride]) -> some View {
    Group {
      if !overrides.isEmpty {
        VStack(alignment: .leading, spacing: 4) {
          Text("Agent Overrides")
            .font(.caption)
            .foregroundStyle(.secondary)
          ForEach(overrides, id: \.agentName) { override in
            HStack {
              Text(override.agentName)
                .font(.system(size: 12, weight: .medium))
                .lineLimit(1)
              Spacer()
              Text(override.modelRef)
                .font(.system(size: 11, design: .monospaced))
                .foregroundStyle(.secondary)
                .lineLimit(1)
            }
          }
        }
      }
    }
  }

  // MARK: - Category Mappings

  private func categoryMappingsSection(_ mappings: [ModelGroupCategoryMapping]) -> some View {
    Group {
      if !mappings.isEmpty {
        VStack(alignment: .leading, spacing: 4) {
          Text("Category Mappings")
            .font(.caption)
            .foregroundStyle(.secondary)
          ForEach(mappings, id: \.categoryName) { mapping in
            HStack {
              Text(mapping.categoryName)
                .font(.system(size: 12, weight: .medium))
                .lineLimit(1)
              Spacer()
              Text(mapping.modelRef)
                .font(.system(size: 11, design: .monospaced))
                .foregroundStyle(.secondary)
                .lineLimit(1)
            }
          }
        }
      }
    }
  }

  // MARK: - Status

  private var statusSection: some View {
    Group {
      if let error = appStore.lastSwitchError, !error.isEmpty {
        HStack(alignment: .top, spacing: 6) {
          Image(systemName: "exclamationmark.triangle.fill")
            .foregroundStyle(.red)
            .font(.system(size: 12))
          Text(error)
            .font(.caption)
            .foregroundStyle(.red)
            .lineLimit(3)
        }
      }
      if let warning = appStore.lastSwitchWarning, !warning.isEmpty {
        HStack(alignment: .top, spacing: 6) {
          Image(systemName: "exclamationmark.circle.fill")
            .foregroundStyle(.yellow)
            .font(.system(size: 12))
          Text(warning)
            .font(.caption)
            .foregroundStyle(.secondary)
            .lineLimit(3)
        }
      }
    }
  }

  // MARK: - Switch Targets

  private var switchTargetsSection: some View {
    VStack(alignment: .leading, spacing: 6) {
      Text("Switch To")
        .font(.caption)
        .foregroundStyle(.secondary)

      if enabledGroups.isEmpty {
        Text("No enabled groups")
          .font(.system(size: 12))
          .foregroundStyle(.secondary)
      } else {
        ForEach(enabledGroups) { group in
          HStack {
            VStack(alignment: .leading, spacing: 1) {
              HStack(spacing: 4) {
                if group.id == appStore.currentGroupID {
                  Image(systemName: "checkmark.circle.fill")
                    .foregroundStyle(.green)
                    .font(.system(size: 11))
                }
                Text(group.name)
                  .font(.system(size: 12, weight: .medium))
                  .lineLimit(1)
              }
              if let desc = group.description, !desc.isEmpty {
                Text(desc)
                  .font(.system(size: 10))
                  .foregroundStyle(.secondary)
                  .lineLimit(1)
              }
            }
            Spacer()
            if group.id != appStore.currentGroupID {
              Button {
                Task { await appStore.switchTo(groupID: group.id) }
              } label: {
                if appStore.isLoading {
                  ProgressView()
                    .controlSize(.small)
                } else {
                  Text("Switch")
                    .font(.system(size: 11))
                }
              }
              .buttonStyle(.bordered)
              .controlSize(.small)
              .disabled(appStore.isLoading)
            }
          }
          .padding(.vertical, 2)
        }
      }
    }
  }

  // MARK: - Open Settings

  private var settingsButtons: some View {
    VStack(alignment: .leading, spacing: 8) {
      Button {
        onOpenGlobalSettings?()
      } label: {
        Label("Global Settings", systemImage: "gearshape")
          .font(.system(size: 12))
      }
      .buttonStyle(.bordered)
      .controlSize(.small)

      Button {
        onOpenGroupSettings?()
      } label: {
        Label("Group Settings", systemImage: "slider.horizontal.3")
          .font(.system(size: 12))
      }
      .buttonStyle(.bordered)
      .controlSize(.small)
    }
  }
}
