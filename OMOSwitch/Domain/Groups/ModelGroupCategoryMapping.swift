import Foundation

public struct ModelGroupCategoryMapping: Codable, Equatable, Sendable {
    public let categoryName: String
    public let modelRef: String

    public init(categoryName: String, modelRef: String) {
        self.categoryName = categoryName
        self.modelRef = modelRef
    }
}
