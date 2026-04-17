import Foundation

public struct BackupArtifact: Equatable, Sendable {
    public let target: String
    public let fileURL: URL
    public let createdAt: Date?

    public init(target: String, fileURL: URL, createdAt: Date?) {
        self.target = target
        self.fileURL = fileURL
        self.createdAt = createdAt
    }
}

public struct BackupRepository {
    public let fileManager: FileManager
    public let backupsRootURL: URL
    public var now: @Sendable () -> Date

    public init(
        fileManager: FileManager = .default,
        configRootURL: URL? = nil,
        now: @escaping @Sendable () -> Date = Date.init
    ) {
        self.fileManager = fileManager
        let rootURL = configRootURL ?? ModelGroupRepository.defaultConfigRootURL(fileManager: fileManager)
        self.backupsRootURL = rootURL.appending(path: "backups", directoryHint: .isDirectory)
        self.now = now
    }

    @discardableResult
    public func createBackup(for target: String, sourceFileURL: URL, contents: Data) throws -> URL {
        let directory = backupsRootURL.appending(path: sanitized(target), directoryHint: .isDirectory)
        try fileManager.createDirectory(at: directory, withIntermediateDirectories: true)

        let timestamp = Self.timestampFormatter.string(from: now())
        let baseName = sourceFileURL.lastPathComponent.isEmpty ? sanitized(target) : sourceFileURL.lastPathComponent
        let backupURL = directory.appending(path: "\(timestamp)-\(baseName)", directoryHint: .notDirectory)

        try contents.write(to: backupURL, options: [.atomic])
        return backupURL
    }

    public func listBackups(for target: String? = nil) throws -> [BackupArtifact] {
        guard fileManager.fileExists(atPath: backupsRootURL.path()) else {
            return []
        }

        if let target {
            return try listBackups(in: backupsRootURL.appending(path: sanitized(target), directoryHint: .isDirectory), target: sanitized(target))
        }

        let directories = try fileManager.contentsOfDirectory(
            at: backupsRootURL,
            includingPropertiesForKeys: [.isDirectoryKey],
            options: [.skipsHiddenFiles]
        )

        return try directories.flatMap { directory in
            try listBackups(in: directory, target: directory.lastPathComponent)
        }.sorted { $0.fileURL.lastPathComponent > $1.fileURL.lastPathComponent }
    }

    public func cleanup(target: String, keepingLatest limit: Int) throws {
        precondition(limit >= 0, "limit must be non-negative")

        let artifacts = try listBackups(for: target)
        guard artifacts.count > limit else {
            return
        }

        for artifact in artifacts.dropFirst(limit) {
            try fileManager.removeItem(at: artifact.fileURL)
        }
    }

    private func listBackups(in directory: URL, target: String) throws -> [BackupArtifact] {
        guard fileManager.fileExists(atPath: directory.path()) else {
            return []
        }

        let urls = try fileManager.contentsOfDirectory(
            at: directory,
            includingPropertiesForKeys: [.creationDateKey],
            options: [.skipsHiddenFiles]
        )

        return try urls.map { url in
            let values = try url.resourceValues(forKeys: [.creationDateKey])
            return BackupArtifact(target: target, fileURL: url, createdAt: values.creationDate)
        }.sorted { $0.fileURL.lastPathComponent > $1.fileURL.lastPathComponent }
    }

    private func sanitized(_ value: String) -> String {
        let trimmed = value.trimmingCharacters(in: .whitespacesAndNewlines)
        let sanitized = trimmed.replacingOccurrences(of: #"[^A-Za-z0-9._-]+"#, with: "-", options: .regularExpression)
        return sanitized.isEmpty ? "unknown-target" : sanitized
    }

    private static let timestampFormatter: DateFormatter = {
        let formatter = DateFormatter()
        formatter.calendar = Calendar(identifier: .gregorian)
        formatter.locale = Locale(identifier: "en_US_POSIX")
        formatter.timeZone = TimeZone(secondsFromGMT: 0)
        formatter.dateFormat = "yyyyMMdd'T'HHmmss'Z'"
        return formatter
    }()
}
