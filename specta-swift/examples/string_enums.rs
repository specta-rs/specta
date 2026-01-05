use specta::{Type, TypeCollection};
use specta_swift::Swift;

/// Comprehensive example showcasing string enums and custom Codable implementations
///
/// This example demonstrates how specta-swift handles string enums, mixed enums,
/// and generates appropriate Codable implementations for different enum patterns.

/// Simple string enum (will be converted to Swift String enum with Codable)
#[derive(Type)]
enum HttpStatus {
    /// Request was successful
    Ok,
    /// Resource was created
    Created,
    /// Request was accepted
    Accepted,
    /// No content to return
    NoContent,
    /// Bad request
    BadRequest,
    /// Unauthorized access
    Unauthorized,
    /// Resource not found
    NotFound,
    /// Internal server error
    InternalServerError,
}

/// String enum with more complex values
#[derive(Type)]
enum Environment {
    /// Development environment
    Development,
    /// Staging environment
    Staging,
    /// Production environment
    Production,
    /// Testing environment
    Testing,
}

/// Mixed enum with both string-like and data variants
#[derive(Type)]
enum ApiResult {
    /// Simple success case
    Success,
    /// Success with data
    SuccessWithData { data: String, status_code: u16 },
    /// Error case
    Error { message: String, code: u32 },
    /// Loading state
    Loading,
}

/// Complex mixed enum
#[derive(Type)]
enum UserAction {
    /// Simple login action
    Login,
    /// Logout action
    Logout,
    /// Update profile with data
    UpdateProfile {
        name: String,
        email: String,
        avatar_url: Option<String>,
    },
    /// Change password
    ChangePassword {
        old_password: String,
        new_password: String,
    },
    /// Delete account
    DeleteAccount,
}

/// String enum for job states
#[derive(Type)]
enum JobState {
    /// Job is waiting in queue
    Queued,
    /// Job is currently running
    Running,
    /// Job is paused
    Paused,
    /// Job completed successfully
    Completed,
    /// Job failed with error
    Failed,
    /// Job was cancelled
    Cancelled,
}

/// Mixed enum with complex variants
#[derive(Type)]
enum NotificationType {
    /// Simple email notification
    Email,
    /// Push notification
    Push,
    /// SMS notification
    Sms,
    /// Webhook notification with payload
    Webhook {
        url: String,
        headers: Vec<(String, String)>,
        retry_count: u32,
    },
    /// In-app notification
    InApp {
        title: String,
        message: String,
        priority: String,
    },
}

/// Enum with generic type parameter
#[derive(Type)]
enum Result<T, E> {
    /// Success with data
    Ok(T),
    /// Error with error details
    Err(E),
}

/// Complex enum with multiple data variants
#[derive(Type)]
enum EventType {
    /// User created event
    UserCreated,
    /// User updated event
    UserUpdated {
        user_id: u32,
        changes: Vec<(String, String)>,
    },
    /// User deleted event
    UserDeleted { user_id: u32, reason: String },
    /// System event
    SystemEvent {
        component: String,
        level: String,
        message: String,
    },
}

/// String enum for file types
#[derive(Type)]
enum FileType {
    /// Image files
    Image,
    /// Video files
    Video,
    /// Audio files
    Audio,
    /// Document files
    Document,
    /// Archive files
    Archive,
    /// Unknown file type
    Unknown,
}

fn main() {
    println!("🚀 String Enums Example - String enums and custom Codable");
    println!("{}", "=".repeat(60));

    // Create type collection with all our enum types
    let types = TypeCollection::default()
        .register::<HttpStatus>()
        .register::<Environment>()
        .register::<ApiResult>()
        .register::<UserAction>()
        .register::<JobState>()
        .register::<NotificationType>()
        .register::<Result<String, String>>()
        .register::<EventType>()
        .register::<FileType>();

    // Export with default settings
    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("📝 Generated Swift code:\n");
    println!("{}", output);

    // Write to file for inspection
    swift
        .export_to("./examples/generated/StringEnums.swift", &types)
        .unwrap();
    println!("✅ String enums exported to StringEnums.swift");

    println!("\n🔍 Key Features Demonstrated:");
    println!("• Pure string enums (String, Codable)");
    println!("• Mixed enums with both simple and complex variants");
    println!("• Custom Codable implementations for complex enums");
    println!("• Struct generation for named field variants");
    println!("• Generic enum support");
    println!("• Proper Swift enum case naming");
    println!("• Automatic protocol conformance");

    println!("\n📋 String Enum Features:");
    println!("• Automatic String and Codable conformance");
    println!("• Simple enum cases without associated values");
    println!("• Clean Swift enum representation");

    println!("\n📋 Mixed Enum Features:");
    println!("• Custom Codable implementation generation");
    println!("• Struct generation for named field variants");
    println!("• Proper key mapping (Rust → Swift naming)");
    println!("• Error handling in Codable implementations");
    println!("• Support for both simple and complex variants");

    println!("\n💡 Generated Codable Features:");
    println!("• CodingKeys enum for key mapping");
    println!("• Custom init(from decoder:) implementation");
    println!("• Custom encode(to encoder:) implementation");
    println!("• Error handling for invalid data");
    println!("• Support for nested data structures");
}
