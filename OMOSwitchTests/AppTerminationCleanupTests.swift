import AppKit
import XCTest
@testable import OMOSwitch

@MainActor
final class AppTerminationCleanupTests: XCTestCase {
  nonisolated(unsafe) private var tempRootURLs: [URL] = []

  override func tearDown() {
    tempRootURLs.forEach(TestSupport.removeIfExists)
    tempRootURLs.removeAll()
    super.tearDown()
  }

  func testAppDelegateStopsServerWhenApplicationTerminates() async throws {
    let manager = TerminationStubOpenCodeServeProcessManager(initialState: .running)
    let appDelegate = AppDelegate(container: makeContainer(processManager: manager))

    (appDelegate as NSApplicationDelegate).applicationWillTerminate?(Notification(name: NSApplication.willTerminateNotification))

    try await waitUntil { await manager.recordedStopCount() == 1 }
    let state = await manager.currentState()
    XCTAssertEqual(state, .stopped)
  }

  func testAppDelegateTerminationIsSafeWhenServerAlreadyStopped() async throws {
    let manager = TerminationStubOpenCodeServeProcessManager(initialState: .stopped)
    let appDelegate = AppDelegate(container: makeContainer(processManager: manager))

    (appDelegate as NSApplicationDelegate).applicationWillTerminate?(Notification(name: NSApplication.willTerminateNotification))

    try await waitUntil { await manager.recordedStopCount() == 1 }
    let state = await manager.currentState()
    XCTAssertEqual(state, .stopped)
  }

  func testAppDelegateTerminationIsSafeWhenServerFailed() async throws {
    let manager = TerminationStubOpenCodeServeProcessManager(initialState: .failed(reason: "boom"))
    let appDelegate = AppDelegate(container: makeContainer(processManager: manager))

    (appDelegate as NSApplicationDelegate).applicationWillTerminate?(Notification(name: NSApplication.willTerminateNotification))

    try await waitUntil { await manager.recordedStopCount() == 1 }
    let state = await manager.currentState()
    XCTAssertEqual(state, .stopped)
  }

  func testAppDelegateTerminationDoesNotHangWhenStopIsSlow() async throws {
    let manager = SlowTerminationStubOpenCodeServeProcessManager()
    let appDelegate = AppDelegate(container: makeContainer(processManager: manager))

    let startedAt = Date()
    (appDelegate as NSApplicationDelegate).applicationWillTerminate?(Notification(name: NSApplication.willTerminateNotification))
    let elapsed = Date().timeIntervalSince(startedAt)

    XCTAssertLessThan(elapsed, 1.0)
    try await waitUntil { await manager.recordedStopCount() == 1 }
  }

  private func makeContainer(processManager: any OpenCodeServeProcessManaging) -> DependencyContainer {
    let rootURL = try! TestSupport.makeTemporaryDirectory()
    tempRootURLs.append(rootURL)
    return DependencyContainer(
      statusBarProvider: TerminationFakeStatusBarProvider(),
      popoverControllerFactory: { QuickSwitchPopoverController(popover: NSPopover()) },
      globalSettingsWindowController: SettingsWindowController(appStore: .livePreview, kind: .global),
      groupSettingsWindowController: SettingsWindowController(appStore: .livePreview, kind: .group),
      processManager: processManager,
      configRootURL: rootURL
    )
  }
}

private actor TerminationStubOpenCodeServeProcessManager: OpenCodeServeProcessManaging {
  private var stopCount = 0
  private var state: OpenCodeServeProcessState

  init(initialState: OpenCodeServeProcessState) {
    state = initialState
  }

  func currentState() async -> OpenCodeServeProcessState {
    state
  }

  func recordedStopCount() -> Int {
    stopCount
  }

  func start(config: OpenCodeServeConfig) async {
    state = .running
  }

  func restart(config: OpenCodeServeConfig) async {
    state = .running
  }

  func start(arguments: [String]) async {
    state = .running
  }

  func stop() async {
    stopCount += 1
    state = .stopped
  }

  func restart(arguments: [String]) async {
    state = .running
  }
}

private actor SlowTerminationStubOpenCodeServeProcessManager: OpenCodeServeProcessManaging {
  private var stopCount = 0

  func currentState() async -> OpenCodeServeProcessState {
    .running
  }

  func recordedStopCount() -> Int {
    stopCount
  }

  func start(config: OpenCodeServeConfig) async {}
  func restart(config: OpenCodeServeConfig) async {}
  func start(arguments: [String]) async {}

  func stop() async {
    stopCount += 1
    try? await Task.sleep(nanoseconds: 2_000_000_000)
  }

  func restart(arguments: [String]) async {}
}

@MainActor
private final class TerminationFakeStatusBarProvider: StatusBarProviding {
  func makeStatusItem(length: CGFloat) -> StatusItemType {
    TerminationFakeStatusItem()
  }
}

@MainActor
private final class TerminationFakeStatusItem: StatusItemType {
  let button: NSStatusBarButton? = NSStatusBarButton(frame: NSRect(x: 0, y: 0, width: 24, height: 24))
  var menu: NSMenu?
}

@MainActor
private func waitUntil(
  timeoutNanoseconds: UInt64 = 1_000_000_000,
  condition: @escaping @MainActor () async throws -> Bool
) async throws {
  let deadline = DispatchTime.now().uptimeNanoseconds + timeoutNanoseconds
  while DispatchTime.now().uptimeNanoseconds < deadline {
    if try await condition() { return }
    try await Task.sleep(nanoseconds: 10_000_000)
  }
  XCTFail("Timed out waiting for condition")
}
