import Foundation

public struct AppSelectionState: Codable, Equatable, Sendable {
    public static let currentSchemaVersion = 2

    public var selectedGroupID: UUID?
    public var selectedGroupName: String?
    public var launchAtLoginEnabled: Bool
    public var lastSuccessfulWrite: LastSuccessfulWriteMetadata?
    public var lastWarningSummary: ProjectionIssueSummary?
    public var lastErrorSummary: ProjectionIssueSummary?
    public var migrationVersion: Int

    private enum CodingKeys: String, CodingKey {
        case selectedGroupID
        case selectedGroupName
        case launchAtLoginEnabled
        case lastSuccessfulWrite
        case lastWarningSummary
        case lastErrorSummary
        case migrationVersion
    }

    public init(
        selectedGroupID: UUID? = nil,
        selectedGroupName: String? = nil,
        launchAtLoginEnabled: Bool = false,
        lastSuccessfulWrite: LastSuccessfulWriteMetadata? = nil,
        lastWarningSummary: ProjectionIssueSummary? = nil,
        lastErrorSummary: ProjectionIssueSummary? = nil,
        migrationVersion: Int = AppSelectionState.currentSchemaVersion
    ) {
        self.selectedGroupID = selectedGroupID
        self.selectedGroupName = selectedGroupName
        self.launchAtLoginEnabled = launchAtLoginEnabled
        self.lastSuccessfulWrite = lastSuccessfulWrite
        self.lastWarningSummary = lastWarningSummary
        self.lastErrorSummary = lastErrorSummary
        self.migrationVersion = migrationVersion
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.selectedGroupID = try container.decodeIfPresent(UUID.self, forKey: .selectedGroupID)
        self.selectedGroupName = try container.decodeIfPresent(String.self, forKey: .selectedGroupName)
        self.launchAtLoginEnabled = try container.decodeIfPresent(Bool.self, forKey: .launchAtLoginEnabled) ?? false
        self.lastSuccessfulWrite = try container.decodeIfPresent(LastSuccessfulWriteMetadata.self, forKey: .lastSuccessfulWrite)
        self.lastWarningSummary = try container.decodeIfPresent(ProjectionIssueSummary.self, forKey: .lastWarningSummary)
        self.lastErrorSummary = try container.decodeIfPresent(ProjectionIssueSummary.self, forKey: .lastErrorSummary)
        self.migrationVersion = try container.decodeIfPresent(Int.self, forKey: .migrationVersion) ?? 1
    }
}

public struct LastSuccessfulWriteMetadata: Codable, Equatable, Sendable {
    public var target: String
    public var wroteAt: Date
    public var backupPath: String?

    public init(target: String, wroteAt: Date, backupPath: String? = nil) {
        self.target = target
        self.wroteAt = wroteAt
        self.backupPath = backupPath
    }
}

public struct ProjectionIssueSummary: Codable, Equatable, Sendable {
    public var message: String
    public var count: Int

    public init(message: String, count: Int) {
        self.message = message
        self.count = count
    }
}
