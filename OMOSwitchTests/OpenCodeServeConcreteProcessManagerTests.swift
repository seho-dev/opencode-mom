import Foundation
import XCTest
@testable import OMOSwitch

final class OpenCodeServeConcreteProcessManagerTests: XCTestCase {
    func testStartConfigUsesEnvOpencodeAndTaskTwoArgumentBuilder() async {
        let process = FakeOpenCodeServeSystemProcess()
        let runner = OpenCodeServeProcessRunner(processFactory: { process }, stopTimeoutNanoseconds: 1_000_000)
        let manager = OpenCodeServeProcessManager(runner: runner)
        let config = OpenCodeServeConfig(
            port: 5000,
            hostname: "0.0.0.0",
            mdns: true,
            mdnsDomain: "dev.local",
            cors: ["https://a.example", "https://b.example"]
        )

        await manager.start(config: config)

        XCTAssertEqual(process.executableURL?.path, "/usr/bin/env")
        let processArguments = process.arguments ?? []
        XCTAssertTrue(processArguments.first?.hasPrefix("PATH=") == true)
        XCTAssertEqual(
            Array(processArguments.dropFirst()),
            [
                "opencode",
                "serve",
                "--port",
                "5000",
                "--hostname",
                "0.0.0.0",
                "--mdns",
                "--mdns-domain",
                "dev.local",
                "--cors",
                "https://a.example",
                "--cors",
                "https://b.example",
            ]
        )
        await XCTAssertConcreteState(manager, .running)
    }

    func testStartConfigAddsBunBinToEnvPathBeforeOpencode() async {
        let process = FakeOpenCodeServeSystemProcess()
        let runner = OpenCodeServeProcessRunner(processFactory: { process }, stopTimeoutNanoseconds: 1_000_000)
        let manager = OpenCodeServeProcessManager(runner: runner)

        await manager.start(config: OpenCodeServeConfig(port: 5000))

        let processArguments = process.arguments ?? []
        let pathArgument = processArguments.first ?? ""
        XCTAssertTrue(pathArgument.hasPrefix("PATH="), "Expected PATH assignment before opencode, got \(processArguments)")
        XCTAssertTrue(pathArgument.contains(FileManager.default.homeDirectoryForCurrentUser.appendingPathComponent(".bun/bin").path))
        XCTAssertEqual(processArguments.dropFirst().first, "opencode")
        XCTAssertEqual(Array(processArguments.dropFirst(2)), ["serve", "--port", "5000", "--hostname", "127.0.0.1"])
    }

    func testStartConfigUsesManualExecutablePathWithoutEnvLookup() async {
        let process = FakeOpenCodeServeSystemProcess()
        let runner = OpenCodeServeProcessRunner(processFactory: { process }, stopTimeoutNanoseconds: 1_000_000)
        let manager = OpenCodeServeProcessManager(runner: runner)
        let config = OpenCodeServeConfig(port: 5000, executablePath: "/opt/homebrew/bin/opencode")

        await manager.start(config: config)

        XCTAssertEqual(process.executableURL?.path, "/opt/homebrew/bin/opencode")
        XCTAssertEqual(process.arguments, ["serve", "--port", "5000", "--hostname", "127.0.0.1"])
    }

    func testLaunchFailureTransitionsToFailedWithNonEmptyReason() async {
        let process = FakeOpenCodeServeSystemProcess(runError: FakeOpenCodeServeSystemProcessError.missingExecutable)
        let runner = OpenCodeServeProcessRunner(processFactory: { process }, stopTimeoutNanoseconds: 1_000_000)
        let manager = OpenCodeServeProcessManager(runner: runner)

        await manager.start(arguments: ["serve"])

        let state = await manager.currentState()
        guard case .failed(let reason) = state else {
            XCTFail("Expected failed state, got \(state)")
            return
        }

        XCTAssertFalse(reason.isEmpty)
        XCTAssertTrue(reason.contains("mock env missing"), "Unexpected reason: \(reason)")
    }

    func testEarlyExitDuringLaunchTransitionsToFailedWithNonEmptyReason() async {
        let process = FakeOpenCodeServeSystemProcess(isRunningAfterRun: false, terminationStatus: 127)
        let runner = OpenCodeServeProcessRunner(processFactory: { process }, stopTimeoutNanoseconds: 1_000_000)
        let manager = OpenCodeServeProcessManager(runner: runner)

        await manager.start(arguments: ["serve"])

        await XCTAssertConcreteState(manager, .failed(reason: "opencode serve exited with status 127."))
    }

    func testStopTerminatesBeforeKillFallbackAfterBoundedTimeout() async {
        let process = FakeOpenCodeServeSystemProcess(waitUntilExitBehavior: .returnImmediatelyStillRunning)
        let runner = OpenCodeServeProcessRunner(processFactory: { process }, stopTimeoutNanoseconds: 1_000_000)
        let manager = OpenCodeServeProcessManager(runner: runner)
        await manager.start(arguments: ["serve"])

        await manager.stop()

        XCTAssertEqual(process.events, [.run, .terminate, .waitUntilExit, .kill])
        await XCTAssertConcreteState(manager, .stopped)
    }

    func testStopDoesNotKillWhenProcessExitsBeforeTimeout() async {
        let process = FakeOpenCodeServeSystemProcess(waitUntilExitBehavior: .markExitedAndReturn)
        let runner = OpenCodeServeProcessRunner(processFactory: { process }, stopTimeoutNanoseconds: 1_000_000_000)
        let manager = OpenCodeServeProcessManager(runner: runner)
        await manager.start(arguments: ["serve"])

        await manager.stop()

        XCTAssertEqual(process.events, [.run, .terminate, .waitUntilExit])
    }
}

private func XCTAssertConcreteState(
    _ manager: OpenCodeServeProcessManager,
    _ expectedState: OpenCodeServeProcessState,
    file: StaticString = #filePath,
    line: UInt = #line
) async {
    let state = await manager.currentState()
    XCTAssertEqual(state, expectedState, file: file, line: line)
}

private enum FakeOpenCodeServeSystemProcessEvent: Equatable, Sendable {
    case run
    case terminate
    case waitUntilExit
    case kill
}

private enum FakeOpenCodeServeSystemProcessWaitBehavior: Sendable {
    case markExitedAndReturn
    case returnImmediatelyStillRunning
}

private enum FakeOpenCodeServeSystemProcessError: Error, LocalizedError, Sendable {
    case missingExecutable

    var errorDescription: String? {
        switch self {
        case .missingExecutable:
            "mock env missing"
        }
    }
}

private final class FakeOpenCodeServeSystemProcess: OpenCodeServeSystemProcess, @unchecked Sendable {
    var executableURL: URL?
    var arguments: [String]?
    var standardOutput: Any?
    var standardError: Any?
    var terminationHandler: (@Sendable (any OpenCodeServeSystemProcess) -> Void)?
    private(set) var terminationStatus: Int32
    private(set) var events: [FakeOpenCodeServeSystemProcessEvent] = []

    private let runError: Error?
    private let isRunningAfterRun: Bool
    private let waitUntilExitBehavior: FakeOpenCodeServeSystemProcessWaitBehavior
    private var running = false

    init(
        runError: Error? = nil,
        isRunningAfterRun: Bool = true,
        terminationStatus: Int32 = 0,
        waitUntilExitBehavior: FakeOpenCodeServeSystemProcessWaitBehavior = .markExitedAndReturn
    ) {
        self.runError = runError
        self.isRunningAfterRun = isRunningAfterRun
        self.terminationStatus = terminationStatus
        self.waitUntilExitBehavior = waitUntilExitBehavior
    }

    var isRunning: Bool { running }

    func run() throws {
        events.append(.run)
        if let runError {
            throw runError
        }
        running = isRunningAfterRun
    }

    func terminate() {
        events.append(.terminate)
    }

    func waitUntilExit() {
        events.append(.waitUntilExit)
        switch waitUntilExitBehavior {
        case .markExitedAndReturn:
            running = false
        case .returnImmediatelyStillRunning:
            break
        }
    }

    func kill() {
        events.append(.kill)
        running = false
        terminationStatus = 9
    }
}
