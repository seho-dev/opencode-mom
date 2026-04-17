import Foundation

public struct ModelGroupAgentOverride: Codable, Equatable, Sendable {
    public let agentName: String
    public let modelRef: String

    public init(agentName: String, modelRef: String) {
        self.agentName = agentName
        self.modelRef = modelRef
    }
}
