import Foundation
#if os(macOS)
import Darwin
#endif

public enum OpenCodeServeProcessState: Equatable, Sendable {
    case stopped
    case starting
    case running
    case stopping
    case failed(reason: String)
}

public struct ServerProcessExit: Equatable, Sendable {
    public let status: Int32
    public let reason: String?

    public init(status: Int32, reason: String? = nil) {
        self.status = status
        self.reason = reason
    }
}

public protocol ServerProcessHandle: Sendable {
    func stop() async
}

public protocol ServerProcessRunning: Sendable {
    func launch(
        executablePath: String?,
        arguments: [String],
        onExit: @escaping @Sendable (ServerProcessExit) -> Void
    ) async throws -> any ServerProcessHandle
}

public protocol ServerProcessReadinessChecking: Sendable {
    func waitUntilReady(arguments: [String]) async throws
}

public protocol ServerProcessManaging: Sendable {
    func currentState() async -> OpenCodeServeProcessState
    func start(arguments: [String]) async
    func stop() async
    func restart(arguments: [String]) async
}

public protocol OpenCodeServeProcessManaging: ServerProcessManaging {
    func start(config: OpenCodeServeConfig) async
    func restart(config: OpenCodeServeConfig) async
}

public actor OpenCodeServeProcessManager: OpenCodeServeProcessManaging {
    private let runner: any ServerProcessRunning
    private let readinessChecker: any ServerProcessReadinessChecking
    private let argumentBuilder: OpenCodeServeArgumentBuilder
    private var state: OpenCodeServeProcessState = .stopped
    private var handle: (any ServerProcessHandle)?
    private var generation = 0

    public init(argumentBuilder: OpenCodeServeArgumentBuilder = OpenCodeServeArgumentBuilder()) {
        self.runner = OpenCodeServeProcessRunner()
        self.readinessChecker = OpenCodeServeHTTPReadinessChecker()
        self.argumentBuilder = argumentBuilder
    }

    public init(
        runner: any ServerProcessRunning,
        readinessChecker: any ServerProcessReadinessChecking = ImmediateServerProcessReadinessChecker(),
        argumentBuilder: OpenCodeServeArgumentBuilder = OpenCodeServeArgumentBuilder()
    ) {
        self.runner = runner
        self.readinessChecker = readinessChecker
        self.argumentBuilder = argumentBuilder
    }

    public func currentState() async -> OpenCodeServeProcessState {
        state
    }

    public func start(config: OpenCodeServeConfig) async {
        do {
            try await start(executablePath: config.executablePath, arguments: argumentBuilder.arguments(for: config))
        } catch {
            state = .failed(reason: error.localizedDescription)
        }
    }

    public func restart(config: OpenCodeServeConfig) async {
        do {
            try await restart(executablePath: config.executablePath, arguments: argumentBuilder.arguments(for: config))
        } catch {
            state = .failed(reason: error.localizedDescription)
        }
    }

    public func start(arguments: [String]) async {
        await start(executablePath: nil, arguments: arguments)
    }

    private func start(executablePath: String?, arguments: [String]) async {
        guard canStart else { return }

        state = .starting
        generation += 1
        let launchGeneration = generation
        var launchedHandle: (any ServerProcessHandle)?

        do {
            let processHandle = try await runner.launch(executablePath: executablePath, arguments: arguments) { [weak self] exit in
                Task { await self?.recordUnexpectedExit(exit, generation: launchGeneration) }
            }

            guard generation == launchGeneration else {
                await processHandle.stop()
                return
            }

            launchedHandle = processHandle
            handle = processHandle
            try await readinessChecker.waitUntilReady(arguments: arguments)

            guard generation == launchGeneration else {
                return
            }

            state = .running
        } catch {
            if generation == launchGeneration {
                let processHandle = handle ?? launchedHandle
                handle = nil
                await processHandle?.stop()
                state = .failed(reason: error.localizedDescription)
            }
        }
    }

    public func stop() async {
        guard let activeHandle = handle else {
            if state == .stopping {
                state = .stopped
            }
            return
        }

        state = .stopping
        generation += 1
        handle = nil
        await activeHandle.stop()
        state = .stopped
    }

    public func restart(arguments: [String]) async {
        await restart(executablePath: nil, arguments: arguments)
    }

    private func restart(executablePath: String?, arguments: [String]) async {
        switch state {
        case .starting, .running, .stopping:
            await stop()
        case .stopped, .failed:
            break
        }

        await start(executablePath: executablePath, arguments: arguments)
    }

    private var canStart: Bool {
        switch state {
        case .stopped, .failed:
            true
        case .starting, .running, .stopping:
            false
        }
    }

    private func recordUnexpectedExit(_ exit: ServerProcessExit, generation exitGeneration: Int) {
        guard generation == exitGeneration else { return }

        generation += 1
        handle = nil
        state = .failed(reason: exit.reason ?? "Process exited with status \(exit.status).")
    }
}

public struct ImmediateServerProcessReadinessChecker: ServerProcessReadinessChecking {
    public init() {}

    public func waitUntilReady(arguments: [String]) async throws {}
}

public enum OpenCodeServeReadinessError: LocalizedError, Sendable {
    case timedOut(endpoint: String, reason: String?)

    public var errorDescription: String? {
        switch self {
        case .timedOut(let endpoint, let reason):
            if let reason, reason.isEmpty == false {
                return "opencode serve did not become ready at \(endpoint): \(reason)"
            }
            return "opencode serve did not become ready at \(endpoint)."
        }
    }
}

public struct OpenCodeServeHTTPReadinessChecker: ServerProcessReadinessChecking {
    private let timeoutNanoseconds: UInt64
    private let pollIntervalNanoseconds: UInt64

    public init(
        timeoutNanoseconds: UInt64 = 5_000_000_000,
        pollIntervalNanoseconds: UInt64 = 100_000_000
    ) {
        self.timeoutNanoseconds = timeoutNanoseconds
        self.pollIntervalNanoseconds = pollIntervalNanoseconds
    }

    public func waitUntilReady(arguments: [String]) async throws {
        let endpoint = Self.healthURL(arguments: arguments)
        let deadline = DispatchTime.now().uptimeNanoseconds + timeoutNanoseconds
        var lastFailure: String?

        while DispatchTime.now().uptimeNanoseconds < deadline {
            try Task.checkCancellation()

            do {
                var request = URLRequest(url: endpoint)
                request.timeoutInterval = 1
                let (_, response) = try await URLSession.shared.data(for: request)
                if let httpResponse = response as? HTTPURLResponse {
                    if (200..<300).contains(httpResponse.statusCode) {
                        return
                    }
                    lastFailure = "HTTP \(httpResponse.statusCode)"
                } else {
                    lastFailure = "non-HTTP response"
                }
            } catch {
                lastFailure = error.localizedDescription
            }

            try await Task.sleep(nanoseconds: pollIntervalNanoseconds)
        }

        throw OpenCodeServeReadinessError.timedOut(endpoint: endpoint.absoluteString, reason: lastFailure)
    }

    private static func healthURL(arguments: [String]) -> URL {
        var hostname = "127.0.0.1"
        var port = 4096
        var index = 0

        while index < arguments.count {
            switch arguments[index] {
            case "--hostname" where index + 1 < arguments.count:
                hostname = arguments[index + 1]
                index += 1
            case "--port" where index + 1 < arguments.count:
                port = Int(arguments[index + 1]) ?? port
                index += 1
            default:
                break
            }
            index += 1
        }

        var components = URLComponents()
        components.scheme = "http"
        components.host = connectableHostname(hostname)
        components.port = port
        components.path = "/global/health"
        return components.url ?? URL(string: "http://127.0.0.1:\(port)/global/health")!
    }

    private static func connectableHostname(_ hostname: String) -> String {
        switch hostname.trimmingCharacters(in: .whitespacesAndNewlines) {
        case "", "0.0.0.0", "::", "[::]":
            "127.0.0.1"
        default:
            hostname
        }
    }
}

public enum OpenCodeServeProcessRunnerError: LocalizedError, Sendable {
    case launchFailed(String)
    case exitedDuringLaunch(String)

    public var errorDescription: String? {
        switch self {
        case .launchFailed(let reason), .exitedDuringLaunch(let reason):
            reason
        }
    }
}

public struct OpenCodeServeProcessRunner: ServerProcessRunning {
    private let processFactory: @Sendable () -> any OpenCodeServeSystemProcess
    private let stopTimeoutNanoseconds: UInt64
    private let outputLimit: Int

    public init(
        stopTimeoutNanoseconds: UInt64 = 2_000_000_000,
        outputLimit: Int = 4_096
    ) {
        self.init(
            processFactory: { FoundationProcessAdapter() },
            stopTimeoutNanoseconds: stopTimeoutNanoseconds,
            outputLimit: outputLimit
        )
    }

    init(
        processFactory: @escaping @Sendable () -> any OpenCodeServeSystemProcess,
        stopTimeoutNanoseconds: UInt64 = 2_000_000_000,
        outputLimit: Int = 4_096
    ) {
        self.processFactory = processFactory
        self.stopTimeoutNanoseconds = stopTimeoutNanoseconds
        self.outputLimit = outputLimit
    }

    public func launch(
        executablePath: String? = nil,
        arguments: [String],
        onExit: @escaping @Sendable (ServerProcessExit) -> Void
    ) async throws -> any ServerProcessHandle {
        let process = processFactory()
        let output = BoundedProcessOutput(limit: outputLimit)

        if let executablePath = Self.trimmedExecutablePath(executablePath) {
            process.executableURL = URL(fileURLWithPath: executablePath)
            process.arguments = arguments
        } else {
            process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
            process.arguments = ["PATH=\(Self.opencodeSearchPath())", "opencode"] + arguments
        }
        process.standardOutput = output.makePipe()
        process.standardError = output.makePipe()
        process.terminationHandler = { terminatedProcess in
            onExit(
                ServerProcessExit(
                    status: terminatedProcess.terminationStatus,
                    reason: Self.exitReason(for: terminatedProcess, output: output)
                )
            )
        }

        do {
            try process.run()
        } catch {
            throw OpenCodeServeProcessRunnerError.launchFailed(Self.launchFailureReason(error: error, output: output))
        }

        guard process.isRunning else {
            throw OpenCodeServeProcessRunnerError.exitedDuringLaunch(Self.exitReason(for: process, output: output))
        }

        return OpenCodeServeProcessHandle(process: process, timeoutNanoseconds: stopTimeoutNanoseconds)
    }

    private static func opencodeSearchPath() -> String {
        let defaultPath = "/usr/bin:/bin:/usr/sbin:/sbin"
        let inheritedPath = ProcessInfo.processInfo.environment["PATH"]?.trimmingCharacters(in: .whitespacesAndNewlines)
        let basePath = inheritedPath?.isEmpty == false ? inheritedPath! : defaultPath
        let bunBinPath = FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".bun/bin").path
        var seen = Set<String>()

        return ([bunBinPath] + basePath.components(separatedBy: ":"))
            .filter { component in
                let trimmed = component.trimmingCharacters(in: .whitespacesAndNewlines)
                guard trimmed.isEmpty == false, seen.contains(trimmed) == false else { return false }
                seen.insert(trimmed)
                return true
            }
            .joined(separator: ":")
    }

    private static func trimmedExecutablePath(_ executablePath: String?) -> String? {
        let trimmedPath = executablePath?.trimmingCharacters(in: .whitespacesAndNewlines) ?? ""
        return trimmedPath.isEmpty ? nil : trimmedPath
    }

    private static func launchFailureReason(error: Error, output: BoundedProcessOutput) -> String {
        let capturedOutput = output.text.trimmingCharacters(in: .whitespacesAndNewlines)
        if capturedOutput.isEmpty == false {
            return capturedOutput
        }

        let errorDescription = error.localizedDescription.trimmingCharacters(in: .whitespacesAndNewlines)
        if errorDescription.isEmpty == false {
            return "Failed to launch opencode serve: \(errorDescription)"
        }

        return "Failed to launch opencode serve."
    }

    private static func exitReason(for process: any OpenCodeServeSystemProcess, output: BoundedProcessOutput) -> String {
        let capturedOutput = output.text.trimmingCharacters(in: .whitespacesAndNewlines)
        if capturedOutput.isEmpty == false {
            return capturedOutput
        }

        return "opencode serve exited with status \(process.terminationStatus)."
    }
}

protocol OpenCodeServeSystemProcess: AnyObject, Sendable {
    var executableURL: URL? { get set }
    var arguments: [String]? { get set }
    var standardOutput: Any? { get set }
    var standardError: Any? { get set }
    var terminationHandler: (@Sendable (any OpenCodeServeSystemProcess) -> Void)? { get set }
    var terminationStatus: Int32 { get }
    var isRunning: Bool { get }

    func run() throws
    func terminate()
    func waitUntilExit()
    func kill()
}

private final class FoundationProcessAdapter: OpenCodeServeSystemProcess, @unchecked Sendable {
    private let process = Process()

    var executableURL: URL? {
        get { process.executableURL }
        set { process.executableURL = newValue }
    }

    var arguments: [String]? {
        get { process.arguments }
        set { process.arguments = newValue }
    }

    var standardOutput: Any? {
        get { process.standardOutput }
        set { process.standardOutput = newValue }
    }

    var standardError: Any? {
        get { process.standardError }
        set { process.standardError = newValue }
    }

    var terminationHandler: (@Sendable (any OpenCodeServeSystemProcess) -> Void)? {
        get { nil }
        set {
            process.terminationHandler = { [weak self] _ in
                guard let self else { return }
                newValue?(self)
            }
        }
    }

    var terminationStatus: Int32 { process.terminationStatus }
    var isRunning: Bool { process.isRunning }

    func run() throws {
        try process.run()
    }

    func terminate() {
        process.terminate()
    }

    func waitUntilExit() {
        process.waitUntilExit()
    }

    func kill() {
        #if os(macOS)
        let pid = process.processIdentifier
        if pid > 0 {
            Darwin.kill(pid, SIGKILL)
        }
        #endif
    }
}

private struct OpenCodeServeProcessHandle: ServerProcessHandle {
    let process: any OpenCodeServeSystemProcess
    let timeoutNanoseconds: UInt64

    func stop() async {
        process.terminate()

        _ = await withCheckedContinuation { continuation in
            let resume = OneShotProcessStopResume(continuation: continuation)

            DispatchQueue.global(qos: .utility).async { [process] in
                process.waitUntilExit()
                resume.resume(true)
            }

            DispatchQueue.global(qos: .utility).asyncAfter(deadline: .now() + .nanosecondsClamped(timeoutNanoseconds)) {
                resume.resume(false)
            }
        }

        if process.isRunning {
            process.kill()
        }
    }
}

private final class OneShotProcessStopResume: @unchecked Sendable {
    private let lock = NSLock()
    private var continuation: CheckedContinuation<Bool, Never>?

    init(continuation: CheckedContinuation<Bool, Never>) {
        self.continuation = continuation
    }

    func resume(_ value: Bool) {
        lock.lock()
        let continuation = continuation
        self.continuation = nil
        lock.unlock()

        continuation?.resume(returning: value)
    }
}

private extension DispatchTimeInterval {
    static func nanosecondsClamped(_ value: UInt64) -> DispatchTimeInterval {
        .nanoseconds(Int(min(value, UInt64(Int.max))))
    }
}

private final class BoundedProcessOutput: @unchecked Sendable {
    private let limit: Int
    private let lock = NSLock()
    private var storage = Data()

    init(limit: Int) {
        self.limit = max(0, limit)
    }

    var text: String {
        lock.lock()
        defer { lock.unlock() }
        return String(data: storage, encoding: .utf8) ?? String(decoding: storage, as: UTF8.self)
    }

    func makePipe() -> Pipe {
        let pipe = Pipe()
        pipe.fileHandleForReading.readabilityHandler = { [weak self] handle in
            self?.append(handle.availableData)
        }
        return pipe
    }

    private func append(_ data: Data) {
        guard data.isEmpty == false, limit > 0 else { return }

        lock.lock()
        defer { lock.unlock() }

        let remaining = limit - storage.count
        guard remaining > 0 else { return }
        storage.append(data.prefix(remaining))
    }
}
