// My Custom App Types
// Generated with love ❤️
import Foundation

public enum apiResponse<T>: Equatable, Hashable where T: Codable & Equatable & Hashable {
    case success(apiResponseSuccessData<T>)
    case error(apiResponseErrorData<T>)
    case loading(apiResponseLoadingData<T>)
}
public struct apiResponseSuccessData<T>: Codable, Equatable, Hashable where T: Codable & Equatable & Hashable {
    public let data: T
    public let statusCode: UInt16

    private enum CodingKeys: String, CodingKey {
        case data = "data"
        case statusCode = "status_code"
    }
}


extension apiResponseSuccessData: Sendable where T: Sendable {}
public struct apiResponseErrorData<T>: Codable, Equatable, Hashable where T: Codable & Equatable & Hashable {
    public let message: String
    public let errorCode: UInt32

    private enum CodingKeys: String, CodingKey {
        case message = "message"
        case errorCode = "error_code"
    }
}


extension apiResponseErrorData: Sendable where T: Sendable {}
public struct apiResponseLoadingData<T>: Codable, Equatable, Hashable where T: Codable & Equatable & Hashable {
    public let progress: Float
}


extension apiResponseLoadingData: Sendable where T: Sendable {}
// MARK: - apiResponse Codable Implementation
extension apiResponse: Codable {
    private enum CodingKeys: String, CodingKey {
        case success = "Success"
        case error = "Error"
        case loading = "Loading"
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        
        if container.allKeys.count != 1 {
            throw DecodingError.dataCorrupted(
                DecodingError.Context(codingPath: decoder.codingPath, debugDescription: "Invalid number of keys found, expected one.")
            )
        }

        let key = container.allKeys.first!
        switch key {
        case .success:
            let data = try container.decode(apiResponseSuccessData<T>.self, forKey: .success)
            self = .success(data)
        case .error:
            let data = try container.decode(apiResponseErrorData<T>.self, forKey: .error)
            self = .error(data)
        case .loading:
            let data = try container.decode(apiResponseLoadingData<T>.self, forKey: .loading)
            self = .loading(data)
        }
    }

    public func encode(to encoder: Encoder) throws {
        switch self {
        case .success(let data):
            var container = encoder.container(keyedBy: CodingKeys.self)
            try container.encode(data, forKey: .success)
        case .error(let data):
            var container = encoder.container(keyedBy: CodingKeys.self)
            try container.encode(data, forKey: .error)
        case .loading(let data):
            var container = encoder.container(keyedBy: CodingKeys.self)
            try container.encode(data, forKey: .loading)
        }
    }
}


extension apiResponse: Sendable where T: Sendable {}

public struct genericContainer<T, U>: Codable, Equatable, Hashable where T: Codable & Equatable & Hashable, U: Codable & Equatable & Hashable {
    public let primary: T
    public let secondary: U
    public let metadata: Optional<[(String, String)]>

    private enum CodingKeys: String, CodingKey {
        case primary = "primary"
        case secondary = "secondary"
        case metadata = "metadata"
    }

    public init(primary: T, secondary: U, metadata: Optional<[(String, String)]>) {
        self.primary = primary
        self.secondary = secondary
        self.metadata = metadata
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.primary = try container.decode(T.self, forKey: .primary)
        self.secondary = try container.decode(U.self, forKey: .secondary)
        self.metadata = try container.decodeIfPresent([(String, String)].self, forKey: .metadata)
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(primary, forKey: .primary)
        try container.encode(secondary, forKey: .secondary)
        try container.encode(metadata, forKey: .metadata)
    }
}

extension genericContainer: Sendable where T: Sendable, U: Sendable {}

/// Comprehensive example showcasing ALL configuration options for specta-swift
/// 
/// This example demonstrates every configuration option available in the Swift exporter,
/// showing how different settings affect the generated Swift code.
/// 
/// Sample types for demonstration
public struct user: Codable, Sendable, Equatable, Hashable {
    public let userId: UInt32
    public let fullName: String
    public let emailAddress: Optional<String>
    public let isVerified: Bool

    private enum CodingKeys: String, CodingKey {
        case userId = "user_id"
        case fullName = "full_name"
        case emailAddress = "email_address"
        case isVerified = "is_verified"
    }

    public init(userId: UInt32, fullName: String, emailAddress: Optional<String>, isVerified: Bool) {
        self.userId = userId
        self.fullName = fullName
        self.emailAddress = emailAddress
        self.isVerified = isVerified
    }

    public init(from decoder: Decoder) throws {
        let container = try decoder.container(keyedBy: CodingKeys.self)
        self.userId = try container.decode(UInt32.self, forKey: .userId)
        self.fullName = try container.decode(String.self, forKey: .fullName)
        self.emailAddress = try container.decodeIfPresent(String.self, forKey: .emailAddress)
        self.isVerified = try container.decode(Bool.self, forKey: .isVerified)
    }

    public func encode(to encoder: Encoder) throws {
        var container = encoder.container(keyedBy: CodingKeys.self)
        try container.encode(userId, forKey: .userId)
        try container.encode(fullName, forKey: .fullName)
        try container.encode(emailAddress, forKey: .emailAddress)
        try container.encode(isVerified, forKey: .isVerified)
    }
}

