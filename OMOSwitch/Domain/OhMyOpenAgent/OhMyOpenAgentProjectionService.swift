import Foundation

public struct OhMyOpenAgentProjectionService: Sendable {

    public static func project(group: ModelGroup, onto existingDocument: OhMyOpenAgentDocument) -> ProjectionResult {
        var base = existingDocument.rawDictionary

        let agents = projectedSection(
            from: existingDocument.agents,
            selectedModels: Dictionary(
                uniqueKeysWithValues: group.agentOverrides.compactMap { override in
                    let ref = override.modelRef.trimmingCharacters(in: .whitespacesAndNewlines)
                    guard !ref.isEmpty else { return nil }
                    return (override.agentName, ref)
                }
            )
        )

        let categories = projectedSection(
            from: existingDocument.categories,
            selectedModels: Dictionary(
                uniqueKeysWithValues: group.categoryMappings.compactMap { mapping in
                    let ref = mapping.modelRef.trimmingCharacters(in: .whitespacesAndNewlines)
                    guard !ref.isEmpty else { return nil }
                    return (mapping.categoryName, ref)
                }
            )
        )

        base["agents"] = agents
        base["categories"] = categories

        return ProjectionResult(
            document: OhMyOpenAgentDocument(rawDictionary: base),
            warnings: []
        )
    }

    private static func projectedSection(
        from existingSection: [String: Any],
        selectedModels: [String: String]
    ) -> [String: Any] {
        var projected: [String: Any] = [:]

        for (name, modelRef) in selectedModels {
            var entry = existingSection[name] as? [String: Any] ?? [:]
            entry["model"] = modelRef
            projected[name] = entry
        }

        return projected
    }
}
