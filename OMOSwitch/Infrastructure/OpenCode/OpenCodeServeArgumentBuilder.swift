import Foundation

public enum OpenCodeServeValidationError: Equatable, LocalizedError, Sendable {
    case emptyPort
    case nonNumericPort(String)
    case portOutOfRange(Int)
    case emptyHostname
    case emptyMDNSDomain

    public var errorDescription: String? {
        switch self {
        case .emptyPort:
            "Port is required."
        case .nonNumericPort:
            "Port must be a whole number."
        case .portOutOfRange:
            "Port must be between 1 and 65535."
        case .emptyHostname:
            "Hostname is required."
        case .emptyMDNSDomain:
            "mDNS domain is required when mDNS is enabled."
        }
    }
}

public struct OpenCodeServeArgumentBuilder: Sendable {
    public init() {}

    public func arguments(for config: OpenCodeServeConfig) throws -> [String] {
        let validationErrors = Self.validationErrors(for: config)
        guard validationErrors.isEmpty else { throw validationErrors[0] }

        var arguments = [
            "serve",
            "--port",
            String(config.port),
            "--hostname",
            config.hostname.trimmingCharacters(in: .whitespacesAndNewlines),
        ]

        if config.mdns {
            arguments.append("--mdns")
            arguments.append("--mdns-domain")
            arguments.append(config.mdnsDomain.trimmingCharacters(in: .whitespacesAndNewlines))
        }

        for origin in config.cors {
            arguments.append("--cors")
            arguments.append(origin)
        }

        return arguments
    }

    public static func validationErrors(for config: OpenCodeServeConfig) -> [OpenCodeServeValidationError] {
        var errors: [OpenCodeServeValidationError] = []

        if isValidPort(config.port) == false {
            errors.append(.portOutOfRange(config.port))
        }

        if trimmed(config.hostname).isEmpty {
            errors.append(.emptyHostname)
        }

        if config.mdns, trimmed(config.mdnsDomain).isEmpty {
            errors.append(.emptyMDNSDomain)
        }

        return errors
    }

    public static func config(
        portText: String,
        hostname: String,
        mdns: Bool,
        mdnsDomain: String,
        corsText: String,
        executablePath: String = "",
        autoStart: Bool = false
    ) throws -> OpenCodeServeConfig {
        let trimmedPort = trimmed(portText)
        guard trimmedPort.isEmpty == false else { throw OpenCodeServeValidationError.emptyPort }
        guard let port = Int(trimmedPort) else { throw OpenCodeServeValidationError.nonNumericPort(portText) }

        let config = OpenCodeServeConfig(
            port: port,
            hostname: trimmed(hostname),
            mdns: mdns,
            mdnsDomain: trimmed(mdnsDomain),
            cors: parseCORSTextArea(corsText),
            executablePath: optionalTrimmed(executablePath),
            autoStart: autoStart
        )
        let validationErrors = validationErrors(for: config)
        guard validationErrors.isEmpty else { throw validationErrors[0] }
        return config
    }

    public static func parseCORSTextArea(_ text: String) -> [String] {
        text.components(separatedBy: .newlines)
            .map(trimmed)
            .filter { $0.isEmpty == false }
    }

    private static func isValidPort(_ port: Int) -> Bool {
        (1...65_535).contains(port)
    }

    private static func trimmed(_ value: String) -> String {
        value.trimmingCharacters(in: .whitespacesAndNewlines)
    }

    private static func optionalTrimmed(_ value: String) -> String? {
        let trimmedValue = trimmed(value)
        return trimmedValue.isEmpty ? nil : trimmedValue
    }
}
