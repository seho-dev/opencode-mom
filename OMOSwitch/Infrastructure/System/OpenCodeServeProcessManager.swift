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
        arguments: [String],
        onExit: @escaping @Sendable (ServerProcessExit) -> Void
    ) async throws -> any ServerProcessHandle
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
    private let argumentBuilder: OpenCodeServeArgumentBuilder
    private var state: OpenCodeServeProcessState = .stopped
    private var handle: (any ServerProcessHandle)?
    private var generation = 0

    public init(argumentBuilder: OpenCodeServeArgumentBuilder = OpenCodeServeArgumentBuilder()) {
        self.runner = OpenCodeServeProcessRunner()
        self.argumentBuilder = argumentBuilder
    }

    public init(
        runner: any ServerProcessRunning,
        argumentBuilder: OpenCodeServeArgumentBuilder = OpenCodeServeArgumentBuilder()
    ) {
        self.runner = runner
        self.argumentBuilder = argumentBuilder
    }

    public func currentState() async -> OpenCodeServeProcessState {
        state
    }

    public func start(config: OpenCodeServeConfig) async {
        do {
            try await start(arguments: argumentBuilder.arguments(for: config))
        } catch {
            state = .failed(reason: error.localizedDescription)
        }
    }

    public func restart(config: OpenCodeServeConfig) async {
        do {
            try await restart(arguments: argumentBuilder.arguments(for: config))
        } catch {
            state = .failed(reason: error.localizedDescription)
        }
    }

    public func start(arguments: [String]) async {
        guard canStart else { return }

        state = .starting
        generation += 1
        let launchGeneration = generation

        do {
            let launchedHandle = try await runner.launch(arguments: arguments) { [weak self] exit in
                Task { await self?.recordUnexpectedExit(exit, generation: launchGeneration) }
            }

            guard generation == launchGeneration else {
                await launchedHandle.stop()
                return
            }

            handle = launchedHandle
            state = .running
        } catch {
            if generation == launchGeneration {
                handle = nil
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
        switch state {
        case .starting, .running, .stopping:
            await stop()
        case .stopped, .failed:
            break
        }

        await start(arguments: arguments)
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

        handle = nil
        state = .failed(reason: exit.reason ?? "Process exited with status \(exit.status).")
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
        arguments: [String],
        onExit: @escaping @Sendable (ServerProcessExit) -> Void
    ) async throws -> any ServerProcessHandle {
        let process = processFactory()
        let output = BoundedProcessOutput(limit: outputLimit)

        process.executableURL = URL(fileURLWithPath: "/usr/bin/env")
        process.arguments = ["opencode"] + arguments
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
