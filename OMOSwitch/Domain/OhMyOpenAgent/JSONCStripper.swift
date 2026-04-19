import Foundation

public enum JSONCStripper {
    /// Removes `//` single-line and `/* */` multi-line comments from JSONC input
    /// without altering content inside quoted string literals.
    public static func stripComments(_ input: String) -> String {
        var result = [Character]()
        var i = input.startIndex

        while i < input.endIndex {
            let char = input[i]

            // Inside a string literal — emit verbatim, handle escapes
            if char == "\"" {
                result.append(char)
                i = input.index(after: i)
                while i < input.endIndex {
                    let c = input[i]
                    result.append(c)
                    if c == "\\" {
                        i = input.index(after: i)
                        if i < input.endIndex {
                            result.append(input[i])
                        }
                    } else if c == "\"" {
                        i = input.index(after: i)
                        break
                    }
                    i = input.index(after: i)
                }
                continue
            }

            // Single-line comment
            if char == "/", i < input.index(before: input.endIndex) {
                let next = input[input.index(after: i)]
                if next == "/" {
                    // Skip to end of line
                    i = input.index(i, offsetBy: 2)
                    while i < input.endIndex, input[i] != "\n" {
                        i = input.index(after: i)
                    }
                    continue
                }
                // Multi-line comment
                if next == "*" {
                    i = input.index(i, offsetBy: 2)
                    while i < input.endIndex {
                        if input[i] == "*" {
                            let afterStar = input.index(after: i)
                            if afterStar < input.endIndex, input[afterStar] == "/" {
                                i = input.index(afterStar, offsetBy: 1)
                                break
                            }
                        }
                        i = input.index(after: i)
                    }
                    continue
                }
            }

            result.append(char)
            i = input.index(after: i)
        }

        return String(result)
    }
}
