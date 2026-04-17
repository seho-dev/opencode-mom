import Foundation

final class FixtureLoader {
    static var fixturesBaseURL: URL {
        let bundle = Bundle(for: FixtureLoader.self)
        if let resourceURL = bundle.resourceURL {
            let direct = resourceURL.appendingPathComponent("Fixtures")
            if FileManager.default.fileExists(atPath: direct.path) {
                return direct
            }
            let bundleInResources = bundle.bundleURL.appendingPathComponent("Contents/Resources/Fixtures")
            if FileManager.default.fileExists(atPath: bundleInResources.path) {
                return bundleInResources
            }
        }
        let xctestResources = bundle.bundleURL
            .appendingPathComponent("Contents/Resources/Fixtures")
        if FileManager.default.fileExists(atPath: xctestResources.path) {
            return xctestResources
        }
        let bundlePath = Bundle.module.bundleURL.appendingPathComponent("Fixtures")
        if FileManager.default.fileExists(atPath: bundlePath.path) {
            return bundlePath
        }
        return bundle.resourceURL?.appendingPathComponent("Fixtures")
            ?? bundle.bundleURL.appendingPathComponent("Fixtures")
    }

    static func fixtureURL(named name: String, subdirectory: String) -> URL {
        fixturesBaseURL
            .appendingPathComponent(subdirectory)
            .appendingPathComponent(name)
    }

    static func fixtureData(named name: String, subdirectory: String) throws -> Data {
        let url = fixtureURL(named: name, subdirectory: subdirectory)
        return try Data(contentsOf: url)
    }

    static func fixtureString(named name: String, subdirectory: String) throws -> String {
        let data = try fixtureData(named: name, subdirectory: subdirectory)
        guard let string = String(data: data, encoding: .utf8) else {
            throw FixtureError.invalidEncoding
        }
        return string
    }

    static func fixtureJSON(named name: String, subdirectory: String) throws -> [String: Any] {
        let data = try fixtureData(named: name, subdirectory: subdirectory)
        guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
            throw FixtureError.notJSONObject
        }
        return json
    }

    static func listFixtures(in subdirectory: String) -> [String] {
        let dirURL = fixturesBaseURL.appendingPathComponent(subdirectory)
        guard let contents = try? FileManager.default.contentsOfDirectory(atPath: dirURL.path) else {
            return []
        }
        return contents
    }

    enum FixtureError: Error {
        case invalidEncoding
        case notJSONObject
    }
}
