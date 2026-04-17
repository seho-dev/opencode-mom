import Foundation

public struct ModelGroupRepository {
    public static let currentSchemaVersion = 1

    public let fileManager: FileManager
    public let groupsFileURL: URL
    private let encoder: JSONEncoder
    private let decoder: JSONDecoder

    public init(
        fileManager: FileManager = .default,
        configRootURL: URL? = nil
    ) {
        self.fileManager = fileManager
        let rootURL = configRootURL ?? Self.defaultConfigRootURL(fileManager: fileManager)
        self.groupsFileURL = rootURL.appending(path: "groups.json", directoryHint: .notDirectory)

        let encoder = JSONEncoder()
        encoder.outputFormatting = [.prettyPrinted, .sortedKeys]
        encoder.dateEncodingStrategy = .iso8601
        self.encoder = encoder

        let decoder = JSONDecoder()
        decoder.dateDecodingStrategy = .iso8601
        self.decoder = decoder
    }

    public func load() throws -> [ModelGroup] {
        guard fileManager.fileExists(atPath: groupsFileURL.path()) else {
            return []
        }

        let data = try Data(contentsOf: groupsFileURL)
        let payload = try decoder.decode(ModelGroupStore.self, from: data)
        return payload.groups
    }

    public func save(_ groups: [ModelGroup]) throws {
        try ensureParentDirectory()
        let payload = ModelGroupStore(migrationVersion: Self.currentSchemaVersion, groups: groups)
        let data = try encoder.encode(payload)
        try atomicWrite(data: data, to: groupsFileURL)
    }

    public func canonicalLocation() -> URL {
        groupsFileURL
    }

    public static func defaultConfigRootURL(fileManager: FileManager = .default) -> URL {
        fileManager.homeDirectoryForCurrentUser
            .appending(path: ".config", directoryHint: .isDirectory)
            .appending(path: "omo-switch", directoryHint: .isDirectory)
    }

    private func ensureParentDirectory() throws {
        let parent = groupsFileURL.deletingLastPathComponent()
        try fileManager.createDirectory(at: parent, withIntermediateDirectories: true)
    }

    private func atomicWrite(data: Data, to url: URL) throws {
        try data.write(to: url, options: [.atomic])
    }
}

private struct ModelGroupStore: Codable, Equatable {
    let migrationVersion: Int
    let groups: [ModelGroup]
}
