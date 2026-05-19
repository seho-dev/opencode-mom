import Foundation
import XCTest
@testable import OMOSwitch

final class OpenCodeServeProcessManagerTests: XCTestCase {
    func testStartTransitionsStoppedStartingRunning() async {
        let runner = FakeServerProcessRunner()
        let readiness = FakeServerReadinessChecker(nextResult: nil)
        await runner.pauseLaunches()
        let manager = OpenCodeServeProcessManager(runner: runner, readinessChecker: readiness)

        let startTask = Task { await manager.start(arguments: ["serve", "--port", "4096"]) }
        await runner.waitForLaunchCount(1)

        await XCTAssertState(manager, .starting)

        let readinessCheckCountBeforeLaunchCompletes = await readiness.checkCount
        XCTAssertEqual(readinessCheckCountBeforeLaunchCompletes, 0)

        await runner.completeLaunch(at: 0)
        await readiness.waitForCheckCount(1)
        await XCTAssertState(manager, .starting)
        await readiness.completeCheck(at: 0, result: .success(()))
        await startTask.value

        await XCTAssertState(manager, .running)
    }

    func testReadinessFailureStopsLaunchedProcessAndTransitionsToFailed() async {
        let runner = FakeServerProcessRunner()
        let readiness = FakeServerReadinessChecker(nextResult: .failure(FakeServerReadinessError.notReady))
        let manager = OpenCodeServeProcessManager(runner: runner, readinessChecker: readiness)

        await manager.start(arguments: ["serve", "--port", "4096"])

        await XCTAssertState(manager, .failed(reason: "opencode serve did not become ready"))
        let stopCount = await runner.stopCount
        XCTAssertEqual(stopCount, 1)
    }

    func testStopTransitionsRunningStoppingStopped() async {
        let runner = FakeServerProcessRunner()
        let manager = OpenCodeServeProcessManager(runner: runner)
        await manager.start(arguments: ["serve"])
        await runner.pauseStops()

        let stopTask = Task { await manager.stop() }
        await runner.waitForStopCount(1)

        await XCTAssertState(manager, .stopping)

        await runner.completeStop(at: 0)
        await stopTask.value

        await XCTAssertState(manager, .stopped)
    }

    func testMissingExecutableStartupFailureTransitionsToFailed() async {
        let runner = FakeServerProcessRunner()
        await runner.setNextLaunchResult(.failure(FakeServerProcessError.missingExecutable))
        let manager = OpenCodeServeProcessManager(runner: runner)

        await manager.start(arguments: ["serve"])

        await XCTAssertState(manager, .failed(reason: "opencode executable not found"))
        let launchCount = await runner.launchCount
        XCTAssertEqual(launchCount, 1)
    }

    func testUnexpectedExitWhileRunningTransitionsToFailed() async {
        let runner = FakeServerProcessRunner()
        let manager = OpenCodeServeProcessManager(runner: runner)
        await manager.start(arguments: ["serve"])

        await runner.exitLaunch(at: 0, exit: ServerProcessExit(status: 9, reason: "process died"))

        await waitUntilState(manager, .failed(reason: "process died"))
        await XCTAssertState(manager, .failed(reason: "process died"))
    }

    func testRetryAfterFailureCanReachRunning() async {
        let runner = FakeServerProcessRunner()
        await runner.setNextLaunchResult(.failure(FakeServerProcessError.missingExecutable))
        let manager = OpenCodeServeProcessManager(runner: runner)

        await manager.start(arguments: ["serve"])
        await manager.start(arguments: ["serve", "--port", "5000"])

        await XCTAssertState(manager, .running)
        let launchCount = await runner.launchCount
        let secondArguments = await runner.arguments(at: 1)
        XCTAssertEqual(launchCount, 2)
        XCTAssertEqual(secondArguments, ["serve", "--port", "5000"])
    }

    func testDuplicateStartWhileStartingDoesNotSpawnTwice() async {
        let runner = FakeServerProcessRunner()
        await runner.pauseLaunches()
        let manager = OpenCodeServeProcessManager(runner: runner)

        let firstStart = Task { await manager.start(arguments: ["serve"]) }
        await runner.waitForLaunchCount(1)
        await manager.start(arguments: ["serve", "--port", "5000"])

        let launchCount = await runner.launchCount
        XCTAssertEqual(launchCount, 1)

        await runner.completeLaunch(at: 0)
        await firstStart.value
        await XCTAssertState(manager, .running)
    }

    func testDuplicateStartWhileRunningDoesNotSpawnTwice() async {
        let runner = FakeServerProcessRunner()
        let manager = OpenCodeServeProcessManager(runner: runner)

        await manager.start(arguments: ["serve"])
        await manager.start(arguments: ["serve", "--port", "5000"])

        await XCTAssertState(manager, .running)
        let launchCount = await runner.launchCount
        XCTAssertEqual(launchCount, 1)
    }

    func testStopWhileStoppedIsNoOp() async {
        let runner = FakeServerProcessRunner()
        let manager = OpenCodeServeProcessManager(runner: runner)

        await manager.stop()

        await XCTAssertState(manager, .stopped)
        let launchCount = await runner.launchCount
        let stopCount = await runner.stopCount
        XCTAssertEqual(launchCount, 0)
        XCTAssertEqual(stopCount, 0)
    }

    func testRestartWhileRunningStopsOldProcessBeforeStartingNewProcess() async {
        let runner = FakeServerProcessRunner()
        let manager = OpenCodeServeProcessManager(runner: runner)
        await manager.start(arguments: ["serve", "--port", "4096"])

        await manager.restart(arguments: ["serve", "--port", "5000"])

        await XCTAssertState(manager, .running)
        let stopCount = await runner.stopCount
        let launchCount = await runner.launchCount
        let events = await runner.events
        let secondArguments = await runner.arguments(at: 1)
        XCTAssertEqual(stopCount, 1)
        XCTAssertEqual(launchCount, 2)
        XCTAssertEqual(events, [.launch, .stop, .launch])
        XCTAssertEqual(secondArguments, ["serve", "--port", "5000"])
    }

    func testStartConfigUsesOpenCodeServeArgumentBuilder() async {
        let runner = FakeServerProcessRunner()
        let manager = OpenCodeServeProcessManager(runner: runner)
        let config = OpenCodeServeConfig(port: 5000, hostname: "0.0.0.0", mdns: true, mdnsDomain: "dev.local", cors: ["https://a.example"])

        await manager.start(config: config)

        let arguments = await runner.arguments(at: 0)
        XCTAssertEqual(arguments, ["serve", "--port", "5000", "--hostname", "0.0.0.0", "--mdns", "--mdns-domain", "dev.local", "--cors", "https://a.example"])
    }
}

private func XCTAssertState(
    _ manager: OpenCodeServeProcessManager,
    _ expectedState: OpenCodeServeProcessState,
    file: StaticString = #filePath,
    line: UInt = #line
) async {
    let state = await manager.currentState()
    XCTAssertEqual(state, expectedState, file: file, line: line)
}

private func waitUntilState(
    _ manager: OpenCodeServeProcessManager,
    _ expectedState: OpenCodeServeProcessState,
    timeoutNanoseconds: UInt64 = 1_000_000_000,
    file: StaticString = #filePath,
    line: UInt = #line
) async {
    let deadline = DispatchTime.now().uptimeNanoseconds + timeoutNanoseconds
    while DispatchTime.now().uptimeNanoseconds < deadline {
        let state = await manager.currentState()
        if state == expectedState { return }
        try? await Task.sleep(nanoseconds: 10_000_000)
    }
    let state = await manager.currentState()
    XCTFail("Timed out waiting for state \(expectedState), got \(state)", file: file, line: line)
}

private enum FakeServerProcessEvent: Equatable, Sendable {
    case launch
    case stop
}

private enum FakeServerProcessError: Error, LocalizedError, Sendable {
    case missingExecutable

    var errorDescription: String? {
        switch self {
        case .missingExecutable:
            "opencode executable not found"
        }
    }
}

private enum FakeServerReadinessError: Error, LocalizedError, Sendable {
    case notReady

    var errorDescription: String? {
        switch self {
        case .notReady:
            "opencode serve did not become ready"
        }
    }
}

private actor FakeServerProcessRunner: ServerProcessRunning {
    private var launches: [FakeLaunch] = []
    private var nextLaunchResult: Result<Void, Error> = .success(())
    private var shouldPauseLaunches = false
    private var shouldPauseStops = false
    private var launchContinuations: [Int: CheckedContinuation<Void, Never>] = [:]
    private var stopContinuations: [Int: CheckedContinuation<Void, Never>] = [:]
    private var launchWaiters: [(Int, CheckedContinuation<Void, Never>)] = []
    private var stopWaiters: [(Int, CheckedContinuation<Void, Never>)] = []
    private var recordedEvents: [FakeServerProcessEvent] = []
    private var recordedStopCount = 0

    var launchCount: Int { launches.count }
    var stopCount: Int { recordedStopCount }
    var events: [FakeServerProcessEvent] { recordedEvents }

    func launch(executablePath: String?, arguments: [String], onExit: @escaping @Sendable (ServerProcessExit) -> Void) async throws -> any ServerProcessHandle {
        let index = launches.count
        launches.append(FakeLaunch(arguments: arguments, onExit: onExit))
        recordedEvents.append(.launch)
        resumeReadyLaunchWaiters()

        let result = nextLaunchResult
        nextLaunchResult = .success(())
        try result.get()

        if shouldPauseLaunches {
            await withCheckedContinuation { continuation in
                launchContinuations[index] = continuation
            }
        }

        return FakeServerProcessHandle(runner: self, launchIndex: index)
    }

    func setNextLaunchResult(_ result: Result<Void, Error>) {
        nextLaunchResult = result
    }

    func pauseLaunches() {
        shouldPauseLaunches = true
    }

    func pauseStops() {
        shouldPauseStops = true
    }

    func completeLaunch(at index: Int) {
        shouldPauseLaunches = false
        launchContinuations.removeValue(forKey: index)?.resume()
    }

    func completeStop(at index: Int) {
        shouldPauseStops = false
        stopContinuations.removeValue(forKey: index)?.resume()
    }

    func exitLaunch(at index: Int, exit: ServerProcessExit) {
        launches[index].onExit(exit)
    }

    func arguments(at index: Int) -> [String] {
        launches[index].arguments
    }

    func waitForLaunchCount(_ count: Int) async {
        guard launches.count < count else { return }
        await withCheckedContinuation { continuation in
            launchWaiters.append((count, continuation))
        }
    }

    func waitForStopCount(_ count: Int) async {
        guard recordedStopCount < count else { return }
        await withCheckedContinuation { continuation in
            stopWaiters.append((count, continuation))
        }
    }

    fileprivate func recordStop(launchIndex: Int) async {
        recordedStopCount += 1
        recordedEvents.append(.stop)
        resumeReadyStopWaiters()

        if shouldPauseStops {
            await withCheckedContinuation { continuation in
                stopContinuations[launchIndex] = continuation
            }
        }
    }

    private func resumeReadyLaunchWaiters() {
        let ready = launchWaiters.filter { launches.count >= $0.0 }
        launchWaiters.removeAll { launches.count >= $0.0 }
        ready.forEach { $0.1.resume() }
    }

    private func resumeReadyStopWaiters() {
        let ready = stopWaiters.filter { recordedStopCount >= $0.0 }
        stopWaiters.removeAll { recordedStopCount >= $0.0 }
        ready.forEach { $0.1.resume() }
    }
}

private actor FakeServerReadinessChecker: ServerProcessReadinessChecking {
    private var nextResult: Result<Void, Error>?
    private var checks: [FakeServerReadinessCheck] = []
    private var checkWaiters: [(Int, CheckedContinuation<Void, Never>)] = []

    init(nextResult: Result<Void, Error>? = .success(())) {
        self.nextResult = nextResult
    }

    var checkCount: Int { checks.count }

    func waitUntilReady(arguments: [String]) async throws {
        if let nextResult {
            self.nextResult = .success(())
            try nextResult.get()
            return
        }

        try await withCheckedThrowingContinuation { continuation in
            checks.append(FakeServerReadinessCheck(arguments: arguments, continuation: continuation))
            resumeReadyCheckWaiters()
        }
    }

    func waitForCheckCount(_ count: Int) async {
        guard checks.count < count else { return }
        await withCheckedContinuation { continuation in
            checkWaiters.append((count, continuation))
        }
    }

    func completeCheck(at index: Int, result: Result<Void, Error>) {
        let continuation = checks[index].continuation
        switch result {
        case .success:
            continuation.resume()
        case .failure(let error):
            continuation.resume(throwing: error)
        }
    }

    private func resumeReadyCheckWaiters() {
        let ready = checkWaiters.filter { checks.count >= $0.0 }
        checkWaiters.removeAll { checks.count >= $0.0 }
        ready.forEach { $0.1.resume() }
    }
}

private struct FakeServerReadinessCheck: Sendable {
    let arguments: [String]
    let continuation: CheckedContinuation<Void, any Error>
}

private struct FakeLaunch: Sendable {
    let arguments: [String]
    let onExit: @Sendable (ServerProcessExit) -> Void
}

private struct FakeServerProcessHandle: ServerProcessHandle {
    let runner: FakeServerProcessRunner
    let launchIndex: Int

    func stop() async {
        await runner.recordStop(launchIndex: launchIndex)
    }
}
