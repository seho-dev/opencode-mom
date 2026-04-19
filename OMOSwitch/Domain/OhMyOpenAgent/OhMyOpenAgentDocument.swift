import Foundation

public enum OhMyOpenAgentDocumentError: Error, Equatable, Sendable {
    case malformedJSON
}

public struct OhMyOpenAgentDocument: @unchecked Sendable {
    private static let bootstrapSchema = "https://raw.githubusercontent.com/code-yeongyu/oh-my-openagent/dev/assets/oh-my-opencode.schema.json"

    public var rawDictionary: [String: Any]

    public var agents: [String: Any] {
        (rawDictionary["agents"] as? [String: Any]) ?? [:]
    }

    public var categories: [String: Any] {
        (rawDictionary["categories"] as? [String: Any]) ?? [:]
    }

    public init(rawDictionary: [String: Any] = [:]) {
        self.rawDictionary = rawDictionary
    }

    public static func bootstrap() -> OhMyOpenAgentDocument {
        OhMyOpenAgentDocument(rawDictionary: [
            "$schema": bootstrapSchema,
            "agents": [String: Any](),
            "categories": [String: Any](),
        ])
    }

    public static func parse(jsonData: Data) -> Result<OhMyOpenAgentDocument, OhMyOpenAgentDocumentError> {
        do {
            guard let dict = try JSONSerialization.jsonObject(with: jsonData) as? [String: Any] else {
                return .failure(.malformedJSON)
            }
            return .success(OhMyOpenAgentDocument(rawDictionary: dict))
        } catch {
            return .failure(.malformedJSON)
        }
    }

    public static func parse(jsoncString: String) -> Result<OhMyOpenAgentDocument, OhMyOpenAgentDocumentError> {
        let stripped = JSONCStripper.stripComments(jsoncString)
        guard let data = stripped.data(using: .utf8) else {
            return .failure(.malformedJSON)
        }
        return parse(jsonData: data)
    }

    public func serialize() -> Data? {
        guard JSONSerialization.isValidJSONObject(rawDictionary) else { return nil }
        do {
            return try JSONSerialization.data(
                withJSONObject: rawDictionary,
                options: [.prettyPrinted, .sortedKeys]
            )
        } catch {
            return nil
        }
    }
}

extension OhMyOpenAgentDocument: Equatable {
    public static func == (lhs: OhMyOpenAgentDocument, rhs: OhMyOpenAgentDocument) -> Bool {
        areEqual(lhs.rawDictionary, rhs.rawDictionary)
    }
}

private func areEqual(_ lhs: Any, _ rhs: Any) -> Bool {
    switch (lhs, rhs) {
    case let (a as String, b as String): return a == b
    case let (a as Bool, b as Bool): return a == b
    case let (a as Int, b as Int): return a == b
    case let (a as Double, b as Double): return a == b
    case let (a as [Any], b as [Any]):
        guard a.count == b.count else { return false }
        return zip(a, b).allSatisfy { areEqual($0, $1) }
    case let (a as [String: Any], b as [String: Any]):
        guard a.count == b.count else { return false }
        return a.allSatisfy { key, val in
            guard let otherVal = b[key] else { return false }
            return areEqual(val, otherVal)
        }
    case (_ as NSNull, _):
        return rhs is NSNull
    default:
        if let a = lhs as? NSNumber, let b = rhs as? NSNumber {
            return a == b
        }
        return false
    }
}
