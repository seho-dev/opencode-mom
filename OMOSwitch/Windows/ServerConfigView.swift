import SwiftUI

struct ServerConfigDraft: Equatable {
  var portText: String
  var hostname: String
  var mdns: Bool
  var mdnsDomain: String
  var corsText: String
  var autoStart: Bool

  init(
    portText: String,
    hostname: String,
    mdns: Bool,
    mdnsDomain: String,
    corsText: String,
    autoStart: Bool
  ) {
    self.portText = portText
    self.hostname = hostname
    self.mdns = mdns
    self.mdnsDomain = mdnsDomain
    self.corsText = corsText
    self.autoStart = autoStart
  }

  init(config: OpenCodeServeConfig = OpenCodeServeConfig()) {
    self.portText = String(config.port)
    self.hostname = config.hostname
    self.mdns = config.mdns
    self.mdnsDomain = config.mdnsDomain
    self.corsText = config.cors.joined(separator: "\n")
    self.autoStart = config.autoStart
  }

  func validatedConfig() throws -> OpenCodeServeConfig {
    let config = try OpenCodeServeArgumentBuilder.config(
      portText: portText,
      hostname: hostname,
      mdns: mdns,
      mdnsDomain: mdnsDomain,
      corsText: corsText,
      autoStart: autoStart
    )
    let validationErrors = OpenCodeServeArgumentBuilder.validationErrors(for: config)
    guard validationErrors.isEmpty else { throw validationErrors[0] }
    return config
  }
}

struct ServerConfigView: View {
  @ObservedObject var appStore: AppStore
  @State private var draft = ServerConfigDraft()
  @State private var validationMessage: String?
  @State private var persistenceMessage: String?
  @State private var persistenceMessageColor: Color = .secondary

  var body: some View {
    VStack(alignment: .leading, spacing: 0) {
      ScrollView {
        VStack(alignment: .leading, spacing: 16) {
          VStack(alignment: .leading, spacing: 4) {
            Text("Server Config")
              .font(.title2)
              .fontWeight(.semibold)
            Text("Controls the arguments used when launching `opencode serve`.")
              .font(.subheadline)
              .foregroundStyle(.secondary)
          }

          Divider()

          VStack(alignment: .leading, spacing: 12) {
            fieldRow("Port") {
              TextField("4096", text: $draft.portText)
                .textFieldStyle(.roundedBorder)
                .frame(maxWidth: 160)
            }

            fieldRow("Hostname") {
              TextField("127.0.0.1", text: $draft.hostname)
                .textFieldStyle(.roundedBorder)
            }

            Toggle("Enable mDNS", isOn: $draft.mdns)

            fieldRow("mDNS Domain") {
              TextField("opencode.local", text: $draft.mdnsDomain)
                .textFieldStyle(.roundedBorder)
            }

            VStack(alignment: .leading, spacing: 6) {
              Text("CORS Origins")
                .font(.headline)
              Text("One origin per line. Blank lines are ignored on save.")
                .font(.caption)
                .foregroundStyle(.secondary)
              TextEditor(text: $draft.corsText)
                .font(.body)
                .frame(minHeight: 90)
                .padding(4)
                .background(Color(nsColor: .textBackgroundColor))
                .clipShape(RoundedRectangle(cornerRadius: 4))
                .overlay(
                  RoundedRectangle(cornerRadius: 4)
                    .stroke(Color(nsColor: .separatorColor), lineWidth: 1)
                )
            }

            Toggle("Auto-start server on app launch", isOn: $draft.autoStart)
          }

          if let validationMessage {
            Text(validationMessage)
              .font(.subheadline)
              .foregroundStyle(.red)
          }

          if let persistenceMessage {
            Text(persistenceMessage)
              .font(.subheadline)
              .foregroundStyle(persistenceMessageColor)
          }
        }
        .padding(20)
        .frame(maxWidth: .infinity, alignment: .leading)
      }

      Divider()

      HStack {
        Spacer()
        Button("Reload") {
          loadCurrentConfig()
        }
        Button("Save") {
          save()
        }
        .keyboardShortcut(.defaultAction)
      }
      .padding(20)
    }
    .frame(minWidth: 520, minHeight: 480)
    .onAppear {
      loadCurrentConfig()
    }
  }

  private func fieldRow<Content: View>(_ title: String, @ViewBuilder content: () -> Content) -> some View {
    VStack(alignment: .leading, spacing: 6) {
      Text(title)
        .font(.headline)
      content()
    }
  }

  private func loadCurrentConfig() {
    appStore.loadServerConfig()
    draft = ServerConfigDraft(config: appStore.openCodeServeConfig)
    validationMessage = nil
    persistenceMessage = nil
  }

  private func save() {
    validationMessage = nil
    persistenceMessage = nil

    do {
      let config = try draft.validatedConfig()
      try appStore.saveServerConfig(config)
      draft = ServerConfigDraft(config: config)
      persistenceMessage = "Server config saved."
      persistenceMessageColor = .green
    } catch {
      validationMessage = error.localizedDescription
      persistenceMessageColor = .red
    }
  }
}
