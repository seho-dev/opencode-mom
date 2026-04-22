import Foundation

public enum OpenCodeDocumentError: Error, Equatable, Sendable {
    case malformedJSON
}

public struct OpenCodeDocument: @unchecked Sendable {
    public var rawDictionary: [String: Any]

    public var agents: [String: Any] {
        (rawDictionary["agent"] as? [String: Any]) ?? [:]
    }

    public init(rawDictionary: [String: Any] = [:]) {
        self.rawDictionary = rawDictionary
    }

    public static func parse(jsonData: Data) -> Result<OpenCodeDocument, OpenCodeDocumentError> {
        do {
            guard let dict = try JSONSerialization.jsonObject(with: jsonData) as? [String: Any] else {
                return .failure(.malformedJSON)
            }
            return .success(OpenCodeDocument(rawDictionary: dict))
        } catch {
            return .failure(.malformedJSON)
        }
    }

    public static func parse(jsoncString: String) -> Result<OpenCodeDocument, OpenCodeDocumentError> {
        let stripped = JSONCStripper.stripComments(jsoncString)
        guard let data = stripped.data(using: .utf8) else {
            return .failure(.malformedJSON)
        }
        return parse(jsonData: data)
    }

    public func serialize() -> Data? {
        guard JSONSerialization.isValidJSONObject(rawDictionary) else { return nil }
        do {
            let data = try JSONSerialization.data(
                withJSONObject: rawDictionary,
                options: [.prettyPrinted, .sortedKeys]
            )
            guard let jsonString = String(data: data, encoding: .utf8) else {
                return data
            }
            return jsonString.replacingOccurrences(of: "\\/", with: "/").data(using: .utf8)
        } catch {
            return nil
        }
    }
}

extension OpenCodeDocument: Equatable {
    public static func == (lhs: OpenCodeDocument, rhs: OpenCodeDocument) -> Bool {
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
        return a.allSatisfy { key, value in
            guard let otherValue = b[key] else { return false }
            return areEqual(value, otherValue)
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
