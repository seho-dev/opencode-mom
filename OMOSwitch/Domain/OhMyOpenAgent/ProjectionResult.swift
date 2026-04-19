import Foundation

public struct ProjectionResult: @unchecked Sendable, Equatable {
    public let document: OhMyOpenAgentDocument
    public let warnings: [String]

    public init(document: OhMyOpenAgentDocument, warnings: [String] = []) {
        self.document = document
        self.warnings = warnings
    }
}
