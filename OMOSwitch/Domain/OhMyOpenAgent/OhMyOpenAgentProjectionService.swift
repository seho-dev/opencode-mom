import Foundation

public struct OhMyOpenAgentProjectionService: Sendable {

    public static func project(group: ModelGroup, onto existingDocument: OhMyOpenAgentDocument) -> ProjectionResult {
        var base = existingDocument.rawDictionary

        var agents: [String: Any] = [:]
        for override in group.agentOverrides {
            agents[override.agentName] = ["model": override.modelRef]
        }

        var categories: [String: Any] = [:]
        for mapping in group.categoryMappings {
            categories[mapping.categoryName] = ["model": mapping.modelRef]
        }

        base["agents"] = agents
        base["categories"] = categories

        return ProjectionResult(
            document: OhMyOpenAgentDocument(rawDictionary: base),
            warnings: []
        )
    }
}
