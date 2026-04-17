import Foundation

public struct AppStateRepository {
    public let fileManager: FileManager
    public let stateFileURL: URL
    private let encoder: JSONEncoder
    private let decoder: JSONDecoder

    public init(
        fileManager: FileManager = .default,
        configRootURL: URL? = nil
    ) {
        self.fileManager = fileManager
        let rootURL = configRootURL ?? ModelGroupRepository.defaultConfigRootURL(fileManager: fileManager)
        self.stateFileURL = rootURL.appending(path: "state.json", directoryHint: .notDirectory)

        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        encoder.dateEncodingStrategy = .iso8601
        self.encoder = encoder

        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        self.decoder = decoder
    }

    public func load() throws -> AppSelectionState {
        guard fileManager.fileExists(atPath: stateFileURL.path()) else {
            return AppSelectionState()
        }

        let data = try Data(contentsOf: stateFileURL)
        return try decoder.decode(AppSelectionState.self, from: data)
    }

    public func save(_ state: AppSelectionState) throws {
        try ensureParentDirectory()
        let data = try encoder.encode(state)
        try data.write(to: stateFileURL, options: [.atomic])
    }

    private func ensureParentDirectory() throws {
        let parent = stateFileURL.deletingLastPathComponent()
        try fileManager.createDirectory(at: parent, withIntermediateDirectories: true)
    }
}
