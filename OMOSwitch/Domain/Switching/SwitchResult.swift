import Foundation

public enum SwitchResult: Equatable, Sendable {
    case success(ProjectionResult)
    case noOp
    case failure(String)
}
