use specta::{Type, TypeCollection};
use specta_swift::Swift;

/// Comprehensive example showcasing basic Rust types and their Swift equivalents
///
/// This example demonstrates how specta-swift handles fundamental Rust types
/// and converts them to appropriate Swift types.

/// Basic primitive types
#[derive(Type)]
struct Primitives {
    // Integer types
    small_int: i8,
    unsigned_small: u8,
    short_int: i16,
    unsigned_short: u16,
    regular_int: i32,
    unsigned_int: u32,
    long_int: i64,
    unsigned_long: u64,

    // Float types
    single_precision: f32,
    double_precision: f64,

    // Boolean and character
    is_active: bool,
    single_char: char,

    // String types
    name: String,
    optional_name: Option<String>,

    // Collections
    tags: Vec<String>,
    scores: Vec<f64>,
    user_ids: Vec<u32>,

    // Nested collections
    matrix: Vec<Vec<f64>>,
    string_pairs: Vec<(String, String)>,
}

/// Enum with different variant types
#[derive(Type)]
enum Status {
    /// Simple unit variant
    Active,
    /// Tuple variant with single value
    Pending(String),
    /// Tuple variant with multiple values
    Error(String, u32),
    /// Named field variant
    Loading {
        progress: f32,
        message: Option<String>,
    },
}

/// Generic struct demonstrating type parameters
#[derive(Type)]
struct ApiResponse<T, E> {
    data: Option<T>,
    error: Option<E>,
    status_code: u16,
    headers: Vec<(String, String)>,
}

/// Nested struct demonstrating complex relationships
#[derive(Type)]
struct User {
    id: u32,
    username: String,
    email: String,
    profile: UserProfile,
    preferences: UserPreferences,
    status: Status,
    metadata: Option<UserMetadata>,
}

#[derive(Type)]
struct UserProfile {
    first_name: String,
    last_name: String,
    bio: Option<String>,
    avatar_url: Option<String>,
    birth_date: Option<String>,
}

#[derive(Type)]
struct UserPreferences {
    theme: String,
    language: String,
    notifications_enabled: bool,
    privacy_level: u8,
}

#[derive(Type)]
struct UserMetadata {
    created_at: String,
    last_login: Option<String>,
    login_count: u32,
    is_verified: bool,
}

fn main() {
    println!("🚀 Basic Types Example - Generating Swift from Rust types");
    println!("{}", "=".repeat(60));

    // Create type collection with all our types
    let types = TypeCollection::default()
        .register::<Primitives>()
        .register::<Status>()
        .register::<ApiResponse<String, String>>()
        .register::<User>()
        .register::<UserProfile>()
        .register::<UserPreferences>()
        .register::<UserMetadata>();

    // Export with default settings
    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("📝 Generated Swift code:\n");
    println!("{}", output);

    // Write to file for inspection
    swift
        .export_to("./examples/generated/BasicTypes.swift", &types)
        .unwrap();
    println!("✅ Basic types exported to BasicTypes.swift");

    println!("\n🔍 Key Features Demonstrated:");
    println!("• Primitive type mappings (i32 → Int32, f64 → Double, etc.)");
    println!("• Optional types (Option<String> → String?)");
    println!("• Collections (Vec<T> → [T])");
    println!("• Nested collections (Vec<Vec<f64>> → [[Double]])");
    println!("• Enum variants (unit, tuple, named fields)");
    println!("• Generic types with type parameters");
    println!("• Complex nested struct relationships");
    println!("• Tuple types ((String, String) → (String, String))");
}
