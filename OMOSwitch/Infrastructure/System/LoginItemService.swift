import Foundation
import ServiceManagement

public enum LoginItemStatus: Equatable {
  case enabled
  case requiresApproval
  case disabled
}

@MainActor
protocol LoginItemService {
  func currentStatus() throws -> LoginItemStatus
  func setEnabled(_ isEnabled: Bool) throws
}

@MainActor
struct SMAppServiceLoginItemService: LoginItemService {
  func currentStatus() throws -> LoginItemStatus {
    switch SMAppService.mainApp.status {
    case .enabled:
      return .enabled
    case .requiresApproval:
      return .requiresApproval
    case .notRegistered, .notFound:
      return .disabled
    @unknown default:
      return .disabled
    }
  }

  func setEnabled(_ isEnabled: Bool) throws {
    let service = SMAppService.mainApp
    let currentlyEnabled = try currentStatus()
    if isEnabled, currentlyEnabled == .enabled || currentlyEnabled == .requiresApproval {
      return
    }

    if isEnabled == false, currentlyEnabled == .disabled {
      return
    }

    if isEnabled {
      try service.register()
      return
    }

    try service.unregister()
  }
}
