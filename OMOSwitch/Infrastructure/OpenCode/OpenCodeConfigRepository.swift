import Foundation

public enum OpenCodeConfigError: Error, Equatable, Sendable {
    case fileNotFound
    case malformedConfig
    case writeFailed(Error)

    public static func == (lhs: OpenCodeConfigError, rhs: OpenCodeConfigError) -> Bool {
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

public struct OpenCodeConfigRepository: @unchecked Sendable {
    public let configRootURL: URL
    public let fileManager: FileManager

    public var openCodeConfigURL: URL {
        configRootURL
            .appending(path: "opencode", directoryHint: .isDirectory)
            .appending(path: "opencode.json", directoryHint: .notDirectory)
    }

    public init(
        fileManager: FileManager = .default,
        configRootURL: URL? = nil
    ) {
        self.fileManager = fileManager
        self.configRootURL = configRootURL ?? fileManager.homeDirectoryForCurrentUser
            .appending(path: ".config", directoryHint: .isDirectory)
    }

    public func load() -> Result<OpenCodeDocument, OpenCodeConfigError> {
        guard fileManager.fileExists(atPath: openCodeConfigURL.path()) else {
            return .failure(.fileNotFound)
        }

        do {
            let data = try Data(contentsOf: openCodeConfigURL)
            if let string = String(data: data, encoding: .utf8) {
                let result = OpenCodeDocument.parse(jsoncString: string)
                if case .failure = result {
                    return .failure(.malformedConfig)
                }
                return result.mapError { _ in .malformedConfig }
            }
            let result = OpenCodeDocument.parse(jsonData: data)
            if case .failure = result {
                return .failure(.malformedConfig)
            }
            return result.mapError { _ in .malformedConfig }
        } catch {
            return .failure(.malformedConfig)
        }
    }

    public func save(_ document: OpenCodeDocument) throws {
        try ensureParentDirectory()
        guard let data = document.serialize() else {
            throw OpenCodeConfigError.malformedConfig
        }
        do {
            try data.write(to: openCodeConfigURL, options: [.atomic])
        } catch {
            throw OpenCodeConfigError.writeFailed(error)
        }
    }

    private func ensureParentDirectory() throws {
        let parent = openCodeConfigURL.deletingLastPathComponent()
        try fileManager.createDirectory(at: parent, withIntermediateDirectories: true)
    }
}
