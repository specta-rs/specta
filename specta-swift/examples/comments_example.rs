use specta::{Type, TypeCollection};
use specta_swift::Swift;

/// A comprehensive example demonstrating multi-line comment support
///
/// This example shows how Specta Swift handles complex documentation
/// including:
/// - Multi-line type documentation
/// - Bullet points and formatting
/// - Complex technical descriptions
///
/// The generated Swift code will have properly formatted doc comments
/// that are compatible with Swift's documentation system.
#[derive(Type)]
enum ApiResponse<T> {
    /// Successful response containing the requested data
    ///
    /// This variant is returned when the API call completes successfully
    /// and contains the expected data type. The status code indicates
    /// the HTTP response status (200, 201, etc.).
    Success {
        /// The actual data returned by the API
        data: T,
        /// HTTP status code (200, 201, 204, etc.)
        status: u16,
        /// Optional response headers
        headers: Option<Vec<(String, String)>>,
    },

    /// Error response indicating a failure
    ///
    /// This variant is returned when the API call fails for any reason.
    /// The error contains detailed information about what went wrong
    /// and how to potentially resolve the issue.
    Error {
        /// Human-readable error message
        message: String,
        /// Machine-readable error code
        code: u32,
        /// Optional additional error details
        details: Option<String>,
    },

    /// Loading state for asynchronous operations
    ///
    /// This variant is used for long-running operations where the client
    /// needs to show progress to the user. The progress value ranges
    /// from 0.0 (not started) to 1.0 (completed).
    Loading {
        /// Progress value between 0.0 and 1.0
        progress: f32,
        /// Optional estimated time remaining in seconds
        estimated_time: Option<u64>,
    },
}

/// A user account in the system
///
/// This struct represents a complete user account with all necessary
/// information for authentication, authorization, and personalization.
///
/// # Security Notes
/// - The `password_hash` field should never be logged or exposed
/// - The `api_key` is sensitive and should be treated as a secret
/// - All timestamps are in UTC
#[derive(Type)]
struct User {
    /// Unique identifier for the user
    ///
    /// This ID is generated when the user first registers and never
    /// changes throughout the user's lifetime in the system.
    id: u32,

    /// The user's chosen username
    ///
    /// Must be unique across the entire system. Usernames are
    /// case-insensitive and can contain letters, numbers, and underscores.
    username: String,

    /// The user's email address
    ///
    /// Used for authentication, password resets, and notifications.
    /// Must be a valid email format and unique across the system.
    email: String,

    /// Whether the user account is currently active
    ///
    /// Inactive users cannot log in or perform any actions.
    /// This is typically set to false when an account is suspended
    /// or when the user requests account deletion.
    is_active: bool,

    /// When the user account was created
    ///
    /// Timestamp in UTC when the user first registered.
    created_at: String,

    /// When the user last logged in
    ///
    /// Updated on every successful login. Can be None if the user
    /// has never logged in (e.g., account created but not activated).
    last_login: Option<String>,
}

fn main() {
    let types = TypeCollection::default()
        .register::<ApiResponse<String>>()
        .register::<User>();

    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!(
        "Generated Swift code with comprehensive comments:\n{}",
        output
    );

    // Also write to file for inspection
    swift
        .export_to("./examples/generated/CommentsExample.swift", &types)
        .unwrap();
    println!("\nComments example written to CommentsExample.swift");
}
