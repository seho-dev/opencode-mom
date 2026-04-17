import Foundation
import XCTest

enum FixtureAssertions {

    static func assertNoLeakedSecrets(in data: Data, file: StaticString = #file, line: UInt = #line) {
        guard let content = String(data: data, encoding: .utf8) else { return }
        assertNoLeakedSecrets(in: content, file: file, line: line)
    }

    static func assertNoLeakedSecrets(in content: String, file: StaticString = #file, line: UInt = #line) {
        let leakedPatterns = [
            "\\bsk-[A-Za-z0-9]{20,}\\b",
            "apiKey\":\\s*\"(?!(PLACEHOLDER|SANITIZED|\\$))[^\"]{10,}\"",
        ]
        for pattern in leakedPatterns {
            if let regex = try? NSRegularExpression(pattern: pattern),
               regex.firstMatch(in: content, range: NSRange(content.startIndex..., in: content)) != nil {
                XCTFail("Found leaked secret pattern: \(pattern)", file: file, line: line)
            }
        }
    }

    static func assertValidJSON(_ data: Data, file: StaticString = #file, line: UInt = #line) {
        do {
            _ = try JSONSerialization.jsonObject(with: data)
        } catch {
            XCTFail("Invalid JSON: \(error.localizedDescription)", file: file, line: line)
        }
    }

    static func assertHasKeys(_ data: Data, keys: [String], file: StaticString = #file, line: UInt = #line) {
        do {
            guard let json = try JSONSerialization.jsonObject(with: data) as? [String: Any] else {
                XCTFail("Fixture is not a JSON object", file: file, line: line)
                return
            }
            for key in keys {
                if json[key] == nil {
                    XCTFail("Missing expected key: \(key)", file: file, line: line)
                }
            }
        } catch {
            XCTFail("Invalid JSON: \(error.localizedDescription)", file: file, line: line)
        }
    }

    static func assertFileContentsEqual(_ url1: URL, _ url2: URL, file: StaticString = #file, line: UInt = #line) {
        do {
            let data1 = try Data(contentsOf: url1)
            let data2 = try Data(contentsOf: url2)
            XCTAssertEqual(data1, data2, file: file, line: line)
        } catch {
            XCTFail("Failed to read files: \(error.localizedDescription)", file: file, line: line)
        }
    }
}
