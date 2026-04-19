import Foundation

public struct OhMyOpenAgentProjectionService: Sendable {

    public static func project(group: ModelGroup, onto existingDocument: OhMyOpenAgentDocument) -> ProjectionResult {
        var base = existingDocument.rawDictionary

        var agents: [String: Any] = [:]
        for override in group.agentOverrides {
            let ref = override.modelRef.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !ref.isEmpty else { continue }
            agents[override.agentName] = ["model": ref]
        }

        var categories: [String: Any] = [:]
        for mapping in group.categoryMappings {
            let ref = mapping.modelRef.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !ref.isEmpty else { continue }
            categories[mapping.categoryName] = ["model": ref]
        }

        base["agents"] = agents
        base["categories"] = categories

        return ProjectionResult(
            document: OhMyOpenAgentDocument(rawDictionary: base),
            warnings: []
        )
    }
}
