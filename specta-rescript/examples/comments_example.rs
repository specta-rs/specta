use specta::{Type, TypeCollection};

/// A response returned by API commands.
///
/// This enum demonstrates multi-line doc comments on both the type
/// and its individual variants. specta-rescript emits these as `//`
/// line comments immediately before each definition.
#[derive(Type)]
enum ApiResponse<T> {
    /// Successful response containing the requested data.
    ///
    /// The `status` field holds the HTTP status code (200, 201, etc.)
    /// and `headers` carries optional response metadata.
    Success {
        /// The data payload returned by the command.
        data: T,
        /// HTTP status code.
        status: u16,
        /// Optional response headers as key-value pairs.
        headers: Option<Vec<(String, String)>>,
    },

    /// Error response indicating a failure.
    ///
    /// The `code` is machine-readable; `message` is human-readable.
    Error {
        /// Human-readable error message.
        message: String,
        /// Machine-readable error code for programmatic handling.
        code: u32,
        /// Optional additional details about the error.
        details: Option<String>,
    },
}

/// A user account in the system.
///
/// Demonstrates field-level doc comments on a struct.
/// All timestamps are ISO-8601 strings in UTC.
///
/// # Security Notes
/// The `api_key` field is sensitive — do not log it.
#[derive(Type)]
struct User {
    /// Unique identifier, assigned at registration and never changed.
    id: u32,

    /// The user's chosen username (unique, case-insensitive).
    username: String,

    /// Email address used for authentication and notifications.
    email: String,

    /// Whether the account is currently active.
    ///
    /// Inactive users cannot log in. Set to `false` when suspended
    /// or when the user requests deletion.
    is_active: bool,

    /// UTC timestamp of account creation (ISO-8601).
    created_at: String,

    /// UTC timestamp of most recent login, or `None` if never logged in.
    last_login: Option<String>,
}

/// Severity levels for application events.
///
/// Used in log entries and alert thresholds.
#[derive(Type)]
enum Severity {
    /// Verbose diagnostic information.
    Debug,
    /// Normal operational messages.
    Info,
    /// Non-critical issues that may need attention.
    Warn,
    /// Errors that require immediate attention.
    Error,
}

pub fn types() -> TypeCollection {
    TypeCollection::default()
        .register::<ApiResponse<String>>()
        .register::<User>()
        .register::<Severity>()
}

#[cfg(not(test))]
fn main() {
    use specta_rescript::ReScript;

    println!("Comments Example - Doc comment emission in ReScript");
    println!("{}", "=".repeat(60));

    let types = types();

    let output = ReScript::default().export(&types).unwrap();

    println!("Generated ReScript code:\n\n{}", output);

    ReScript::default()
        .export_to(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/examples/generated/CommentsExample.res"
            ),
            &types,
        )
        .unwrap();
    println!("Comments example exported to CommentsExample.res");

    println!("\nKey Features Demonstrated:");
    println!("• Multi-line type-level doc comments -> // line comments");
    println!("• Field-level doc comments on struct fields");
    println!("• Variant-level doc comments on enum variants");
    println!("• Doc comments on named-field variant auxiliary record fields");
}
