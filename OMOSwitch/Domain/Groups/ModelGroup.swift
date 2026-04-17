import Foundation

public struct ModelGroup: Codable, Equatable, Identifiable, Sendable {
    public let id: UUID
    public var name: String
    public var description: String?
    public var categoryMappings: [ModelGroupCategoryMapping]
    public var agentOverrides: [ModelGroupAgentOverride]
    public var isEnabled: Bool
    public var updatedAt: Date

    public init(
        id: UUID = UUID(),
        name: String,
        description: String? = nil,
        categoryMappings: [ModelGroupCategoryMapping],
        agentOverrides: [ModelGroupAgentOverride] = [],
        isEnabled: Bool = true,
        updatedAt: Date = Date()
    ) {
        self.id = id
        self.name = name
        self.description = description
        self.categoryMappings = categoryMappings
        self.agentOverrides = agentOverrides
        self.isEnabled = isEnabled
        self.updatedAt = updatedAt
    }
}
