import AppKit
import SwiftUI

enum SettingsCategory: CaseIterable, Identifiable {
  case groupSettings
  case serverConfig
  case globalSettings

  var id: Self { self }

  var title: String {
    switch self {
    case .groupSettings:
      "Group Settings"
    case .serverConfig:
      "Server Config"
    case .globalSettings:
      "Global Settings"
    }
  }
}

private enum SettingsWindowLayout {
  static let defaultSize = NSSize(width: 980, height: 620)
  static let minimumWidth: CGFloat = 900
  static let minimumHeight: CGFloat = 560
  static let sidebarMinimumWidth: CGFloat = 180
  static let sidebarIdealWidth: CGFloat = 200
}

private struct SettingsContainerView: View {
  @ObservedObject var appStore: AppStore
  @State private var selectedCategory: SettingsCategory? = .groupSettings

  var body: some View {
    NavigationSplitView {
      List(SettingsCategory.allCases, selection: $selectedCategory) { category in
        Text(category.title)
          .tag(category as SettingsCategory?)
      }
      .listStyle(.sidebar)
      .navigationSplitViewColumnWidth(
        min: SettingsWindowLayout.sidebarMinimumWidth,
        ideal: SettingsWindowLayout.sidebarIdealWidth
      )
    } detail: {
      detail
        .frame(maxWidth: .infinity, maxHeight: .infinity)
    }
    .frame(
      minWidth: SettingsWindowLayout.minimumWidth,
      minHeight: SettingsWindowLayout.minimumHeight
    )
    .onAppear {
      if selectedCategory == nil {
        selectedCategory = .groupSettings
      }
    }
  }

  @ViewBuilder
  private var detail: some View {
    switch selectedCategory ?? .groupSettings {
    case .groupSettings:
      SettingsView(appStore: appStore)
    case .serverConfig:
      ServerConfigView(appStore: appStore)
    case .globalSettings:
      GlobalSettingsView(appStore: appStore)
    }
  }
}

@MainActor
final class SettingsWindowController: NSWindowController {
  enum Kind {
    case global
    case group
    case serverConfig

    var title: String {
      "Settings"
    }

    var defaultSize: NSSize {
      SettingsWindowLayout.defaultSize
    }
  }

  let appStore: AppStore

  convenience init() {
    self.init(appStore: .livePreview)
  }

  convenience init(appStore: AppStore, kind: Kind) {
    self.init(appStore: appStore)
  }

  init(appStore: AppStore) {
    self.appStore = appStore
    let rootView = SettingsContainerView(appStore: appStore)
    let hostingController = NSHostingController(rootView: rootView)
    let window = NSWindow(contentViewController: hostingController)
    window.title = "Settings"
    window.setContentSize(SettingsWindowLayout.defaultSize)
    window.styleMask = [.titled, .closable, .miniaturizable, .resizable]
    window.isReleasedWhenClosed = false
    super.init(window: window)
    shouldCascadeWindows = true
  }

  @available(*, unavailable)
  required init?(coder: NSCoder) {
    fatalError("init(coder:) has not been implemented")
  }

  override func showWindow(_ sender: Any?) {
    super.showWindow(sender)
    window?.makeKeyAndOrderFront(sender)
  }
}
