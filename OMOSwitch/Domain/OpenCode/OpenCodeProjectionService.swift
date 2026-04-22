import Foundation

public struct OpenCodeProjectionService: Sendable {

    public static func project(group: ModelGroup, onto existingDocument: OpenCodeDocument) -> OpenCodeProjectionResult {
        var base = existingDocument.rawDictionary
        let effectiveOverrides = group.openCodeAgentOverrides.compactMap { override -> (agentName: String, modelRef: String)? in
            let ref = override.modelRef.trimmingCharacters(in: .whitespacesAndNewlines)
            guard !ref.isEmpty else { return nil }
            return (override.agentName, ref)
        }

        guard var agents = base["agent"] as? [String: Any] else {
            let warnings = effectiveOverrides.map {
                "OpenCode config has no valid top-level 'agent' object; skipped model override for '\($0.agentName)'."
            }
            return OpenCodeProjectionResult(document: existingDocument, warnings: warnings)
        }

        var warnings: [String] = []

        for override in effectiveOverrides {
            guard var entry = agents[override.agentName] as? [String: Any] else {
                warnings.append("OpenCode agent '\(override.agentName)' was not found; skipped model override.")
                continue
            }

            entry["model"] = override.modelRef
            agents[override.agentName] = entry
        }

        base["agent"] = agents

        return OpenCodeProjectionResult(
            document: OpenCodeDocument(rawDictionary: base),
            warnings: warnings
        )
    }
}

public struct OpenCodeProjectionResult: @unchecked Sendable, Equatable {
    public let document: OpenCodeDocument
    public let warnings: [String]

    public init(document: OpenCodeDocument, warnings: [String] = []) {
        self.document = document
        self.warnings = warnings
    }
}
