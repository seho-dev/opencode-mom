import Foundation

public struct ModelGroup: Codable, Equatable, Identifiable, Sendable {
    public let id: UUID
    public var name: String
    public var description: String?
    public var categoryMappings: [ModelGroupCategoryMapping]
    public var agentOverrides: [ModelGroupAgentOverride]
    public var openCodeAgentOverrides: [ModelGroupAgentOverride]
    public var isEnabled: Bool
    public var updatedAt: Date

    private enum CodingKeys: String, CodingKey {
        case id
        case name
        case description
        case categoryMappings
        case agentOverrides
        case openCodeAgentOverrides
        case isEnabled
        case updatedAt
    }

    public init(
        id: UUID = UUID(),
        name: String,
        description: String? = nil,
        categoryMappings: [ModelGroupCategoryMapping],
        agentOverrides: [ModelGroupAgentOverride] = [],
        openCodeAgentOverrides: [ModelGroupAgentOverride] = [],
        isEnabled: Bool = true,
        updatedAt: Date = Date()
    ) {
        self.id = id
        self.name = name
        self.description = description
        self.categoryMappings = categoryMappings
        self.agentOverrides = agentOverrides
        self.openCodeAgentOverrides = openCodeAgentOverrides
        self.isEnabled = isEnabled
        self.updatedAt = updatedAt
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        id = try container.decode(UUID.self, forKey: .id)
        name = try container.decode(String.self, forKey: .name)
        description = try container.decodeIfPresent(String.self, forKey: .description)
        categoryMappings = try container.decode([ModelGroupCategoryMapping].self, forKey: .categoryMappings)
        agentOverrides = try container.decode([ModelGroupAgentOverride].self, forKey: .agentOverrides)
        openCodeAgentOverrides = try container.decodeIfPresent([ModelGroupAgentOverride].self, forKey: .openCodeAgentOverrides) ?? []
        isEnabled = try container.decode(Bool.self, forKey: .isEnabled)
        updatedAt = try container.decode(Date.self, forKey: .updatedAt)
    }
}
