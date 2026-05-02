import Foundation

public struct GitHubRelease: Codable {
    public let tagName: String
    public let name: String?
    public let body: String?
    public let assets: [GitHubAsset]
    public let htmlUrl: String

    enum CodingKeys: String, CodingKey {
        case tagName = "tag_name"
        case name
        case body
        case assets
        case htmlUrl = "html_url"
    }
}

public struct GitHubAsset: Codable {
    public let name: String
    public let browserDownloadUrl: String
    public let contentType: String

    enum CodingKeys: String, CodingKey {
        case name
        case browserDownloadUrl = "browser_download_url"
        case contentType = "content_type"
    }
}

public struct UpdateInfo: Sendable {
    public let currentVersion: String
    public let latestVersion: String
    public let releaseURL: URL
    public let downloadURL: URL?
    public let releaseNotes: String?
    public let hasUpdate: Bool
}

public enum UpdateCheckerError: Error, LocalizedError {
    case networkError(Error)
    case invalidResponse
    case decodingError(Error)
    case noReleasesFound
    case downloadFailed(Error)
    case installationFailed(Error)

    public var errorDescription: String? {
        switch self {
        case .networkError(let error):
            return "Network error: \(error.localizedDescription)"
        case .invalidResponse:
            return "Invalid response from GitHub."
        case .decodingError(let error):
            return "Failed to decode release info: \(error.localizedDescription)"
        case .noReleasesFound:
            return "No releases found."
        case .downloadFailed(let error):
            return "Download failed: \(error.localizedDescription)"
        case .installationFailed(let error):
            return "Installation failed: \(error.localizedDescription)"
        }
    }
}

public protocol UpdateChecker: Sendable {
    func checkForUpdates() async throws -> UpdateInfo
    func downloadUpdate(_ updateInfo: UpdateInfo) async throws -> URL
    func installUpdate(at url: URL) throws
}

public class GitHubUpdateChecker: UpdateChecker, @unchecked Sendable {
    private let repositoryOwner: String
    private let repositoryName: String
    private let currentVersion: String
    private let urlSession: URLSession

    public init(
        repositoryOwner: String = "seho-dev",
        repositoryName: String = "omo-switch",
        currentVersion: String? = nil,
        urlSession: URLSession = .shared
    ) {
        self.repositoryOwner = repositoryOwner
        self.repositoryName = repositoryName
        self.currentVersion = currentVersion ?? Self.appVersion()
        self.urlSession = urlSession
    }

    public func checkForUpdates() async throws -> UpdateInfo {
        let urlString = "https://api.github.com/repos/\(repositoryOwner)/\(repositoryName)/releases/latest"
        guard let url = URL(string: urlString) else {
            throw UpdateCheckerError.invalidResponse
        }

        do {
            let (data, response) = try await urlSession.data(from: url)

            guard let httpResponse = response as? HTTPURLResponse,
                  (200...299).contains(httpResponse.statusCode) else {
                throw UpdateCheckerError.invalidResponse
            }

            let release = try JSONDecoder().decode(GitHubRelease.self, from: data)
            let latestVersion = release.tagName.hasPrefix("v") ? String(release.tagName.dropFirst()) : release.tagName

            let hasUpdate = compareVersions(current: currentVersion, latest: latestVersion) == .orderedAscending

            let downloadURLString = release.assets.first(where: { $0.name.hasSuffix(".dmg") })?.browserDownloadUrl
            let downloadURL = downloadURLString.flatMap { URL(string: $0) }

            return UpdateInfo(
                currentVersion: currentVersion,
                latestVersion: latestVersion,
                releaseURL: URL(string: release.htmlUrl) ?? url,
                downloadURL: downloadURL,
                releaseNotes: release.body,
                hasUpdate: hasUpdate
            )
        } catch let error as UpdateCheckerError {
            throw error
        } catch let error as DecodingError {
            throw UpdateCheckerError.decodingError(error)
        } catch {
            throw UpdateCheckerError.networkError(error)
        }
    }

    public func downloadUpdate(_ updateInfo: UpdateInfo) async throws -> URL {
        guard let downloadURL = updateInfo.downloadURL else {
            throw UpdateCheckerError.downloadFailed(NSError(domain: "UpdateChecker", code: 1, userInfo: [NSLocalizedDescriptionKey: "No download URL available."]))
        }

        do {
            let (tempURL, response) = try await urlSession.download(from: downloadURL)

            guard let httpResponse = response as? HTTPURLResponse,
                  (200...299).contains(httpResponse.statusCode) else {
                throw UpdateCheckerError.downloadFailed(NSError(domain: "UpdateChecker", code: 2, userInfo: [NSLocalizedDescriptionKey: "Invalid download response."]))
            }

            let fileManager = FileManager.default
            let downloadsDirectory = fileManager.urls(for: .downloadsDirectory, in: .userDomainMask).first!
            let destinationURL = downloadsDirectory.appendingPathComponent("omo-switch-\(updateInfo.latestVersion).dmg")

            if fileManager.fileExists(atPath: destinationURL.path) {
                try fileManager.removeItem(at: destinationURL)
            }

            try fileManager.moveItem(at: tempURL, to: destinationURL)
            return destinationURL
        } catch let error as UpdateCheckerError {
            throw error
        } catch {
            throw UpdateCheckerError.downloadFailed(error)
        }
    }

    public func installUpdate(at url: URL) throws {
        let fileManager = FileManager.default
        guard fileManager.fileExists(atPath: url.path) else {
            throw UpdateCheckerError.installationFailed(NSError(domain: "UpdateChecker", code: 1, userInfo: [NSLocalizedDescriptionKey: "DMG file not found."]))
        }

        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/hdiutil")
        process.arguments = ["attach", url.path, "-nobrowse", "-mountpoint", "/tmp/omo-switch-update"]

        do {
            try process.run()
            process.waitUntilExit()

            guard process.terminationStatus == 0 else {
                throw UpdateCheckerError.installationFailed(NSError(domain: "UpdateChecker", code: 2, userInfo: [NSLocalizedDescriptionKey: "Failed to mount DMG."]))
            }

            let appSource = URL(fileURLWithPath: "/tmp/omo-switch-update/omo-switch.app")
            let appDestination = URL(fileURLWithPath: "/Applications/omo-switch.app")

            if fileManager.fileExists(atPath: appDestination.path) {
                try fileManager.removeItem(at: appDestination)
            }

            try fileManager.copyItem(at: appSource, to: appDestination)

            let detachProcess = Process()
            detachProcess.executableURL = URL(fileURLWithPath: "/usr/bin/hdiutil")
            detachProcess.arguments = ["detach", "/tmp/omo-switch-update", "-force"]
            try detachProcess.run()
            detachProcess.waitUntilExit()

        } catch let error as UpdateCheckerError {
            throw error
        } catch {
            throw UpdateCheckerError.installationFailed(error)
        }
    }

    private func compareVersions(current: String, latest: String) -> ComparisonResult {
        let currentParts = current.split(separator: ".").compactMap { Int($0) }
        let latestParts = latest.split(separator: ".").compactMap { Int($0) }

        let maxLength = max(currentParts.count, latestParts.count)

        for i in 0..<maxLength {
            let currentPart = i < currentParts.count ? currentParts[i] : 0
            let latestPart = i < latestParts.count ? latestParts[i] : 0

            if currentPart < latestPart {
                return .orderedAscending
            } else if currentPart > latestPart {
                return .orderedDescending
            }
        }

        return .orderedSame
    }

    private static func appVersion() -> String {
        Bundle.main.infoDictionary?["CFBundleShortVersionString"] as? String ?? "1.0.0"
    }
}
