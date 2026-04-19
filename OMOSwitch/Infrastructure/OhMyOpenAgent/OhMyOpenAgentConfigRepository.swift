import Foundation

public enum OhMyOpenAgentConfigError: Error, Equatable, Sendable {
    case fileNotFound
    case malformedConfig
    case writeFailed(Error)

    public static func == (lhs: OhMyOpenAgentConfigError, rhs: OhMyOpenAgentConfigError) -> Bool {
        switch (lhs, rhs) {
        case (.fileNotFound, .fileNotFound),
             (.malformedConfig, .malformedConfig):
            return true
        case let (.writeFailed(a), .writeFailed(b)):
            return a.localizedDescription == b.localizedDescription
        default:
            return false
        }
    }
}

public struct OhMyOpenAgentConfigRepository: @unchecked Sendable {
    public let configRootURL: URL
    public let fileManager: FileManager

    public var ohMyOpenAgentConfigURL: URL {
        configRootURL
            .appending(path: "opencode", directoryHint: .isDirectory)
            .appending(path: "oh-my-openagent.json", directoryHint: .notDirectory)
    }

    public init(
        fileManager: FileManager = .default,
        configRootURL: URL? = nil
    ) {
        self.fileManager = fileManager
        self.configRootURL = configRootURL ?? fileManager.homeDirectoryForCurrentUser
            .appending(path: ".config", directoryHint: .isDirectory)
    }

    public func load() -> Result<OhMyOpenAgentDocument, OhMyOpenAgentConfigError> {
        guard fileManager.fileExists(atPath: ohMyOpenAgentConfigURL.path()) else {
            return .success(OhMyOpenAgentDocument.bootstrap())
        }

        do {
            let data = try Data(contentsOf: ohMyOpenAgentConfigURL)
            if let string = String(data: data, encoding: .utf8) {
                let result = OhMyOpenAgentDocument.parse(jsoncString: string)
                if case .failure = result {
                    return .failure(.malformedConfig)
                }
                return result.mapError { _ in .malformedConfig }
            }
            let result = OhMyOpenAgentDocument.parse(jsonData: data)
            if case .failure = result {
                return .failure(.malformedConfig)
            }
            return result.mapError { _ in .malformedConfig }
        } catch {
            return .failure(.malformedConfig)
        }
    }

    public func save(_ document: OhMyOpenAgentDocument) throws {
        try ensureParentDirectory()
        guard let data = document.serialize() else {
            throw OhMyOpenAgentConfigError.malformedConfig
        }
        do {
            try data.write(to: ohMyOpenAgentConfigURL, options: [.atomic])
        } catch {
            throw OhMyOpenAgentConfigError.writeFailed(error)
        }
    }

    private func ensureParentDirectory() throws {
        let parent = ohMyOpenAgentConfigURL.deletingLastPathComponent()
        try fileManager.createDirectory(at: parent, withIntermediateDirectories: true)
    }
}
