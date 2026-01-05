use specta::{Type, TypeCollection};
use specta_swift::Swift;

#[derive(Type)]
struct User {
    id: u32,
    name: String,
    email: Option<String>,
    age: u8,
    is_active: bool,
    metadata: UserMetadata,
    tags: Vec<String>,
    scores: Vec<f64>,
}

#[derive(Type)]
struct UserMetadata {
    created_at: String,
    last_login: Option<String>,
    preferences: UserPreferences,
}

#[derive(Type)]
struct UserPreferences {
    theme: String,
    notifications: bool,
    language: String,
}

#[derive(Type)]
enum UserRole {
    Admin,
    User,
    Guest,
    Moderator { permissions: Vec<String> },
    Custom { name: String, level: u8 },
}

#[derive(Type)]
enum ApiResponse<T> {
    Success(T),
    Error { code: u32, message: String },
    Loading,
}

#[derive(Type)]
struct ApiResult<T, E> {
    data: Option<T>,
    error: Option<E>,
    status: u16,
}

#[test]
fn test_comprehensive_export() {
    let types = TypeCollection::default()
        .register::<User>()
        .register::<UserMetadata>()
        .register::<UserPreferences>()
        .register::<UserRole>()
        .register::<ApiResponse<String>>()
        .register::<ApiResult<String, String>>();

    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Generated Swift code:\n{}", output);

    // Test struct generation
    assert!(output.contains("struct User"));
    assert!(output.contains("struct UserMetadata"));
    assert!(output.contains("struct UserPreferences"));
    assert!(output.contains("struct ApiResult"));

    // Test enum generation
    assert!(output.contains("enum UserRole"));
    assert!(output.contains("enum ApiResponse"));

    // Test field types
    assert!(output.contains("let id: UInt32"));
    assert!(output.contains("let name: String"));
    assert!(output.contains("let email: String?"));
    assert!(output.contains("let age: UInt8"));
    assert!(output.contains("let isActive: Bool"));
    assert!(output.contains("let metadata: UserMetadata"));
    assert!(output.contains("let tags: [String]"));
    assert!(output.contains("let scores: [Double]"));

    // Test enum cases
    assert!(output.contains("case admin"));
    assert!(output.contains("case user"));
    assert!(output.contains("case guest"));
    assert!(output.contains("case moderator"));
    assert!(output.contains("case custom"));

    // Test generic types (they appear as generic definitions, not concrete instantiations)
    assert!(output.contains("enum ApiResponse<T>"));
    assert!(output.contains("struct ApiResult<T, E>"));

    // Test optional types
    assert!(output.contains("let email: String?"));
    assert!(output.contains("let lastLogin: String?"));
    assert!(output.contains("let data: T?"));
    assert!(output.contains("let error: E?"));

    // Test array types
    assert!(output.contains("let tags: [String]"));
    assert!(output.contains("let scores: [Double]"));
    assert!(output.contains("permissions: [String]"));
}

#[test]
fn test_naming_conventions() {
    let types = TypeCollection::default().register::<User>();

    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    // Test PascalCase for type names
    assert!(output.contains("struct User"));

    // Test camelCase for field names
    assert!(output.contains("let isActive: Bool")); // snake_case -> camelCase
    assert!(output.contains("let createdAt: String")); // snake_case -> camelCase
}

#[test]
fn test_swift_configuration() {
    let types = TypeCollection::default().register::<User>();

    // Test with custom configuration
    let swift = Swift::new()
        .header("// Custom header")
        .naming(specta_swift::NamingConvention::SnakeCase)
        .optionals(specta_swift::OptionalStyle::Optional);

    let output = swift.export(&types).unwrap();

    println!("Snake case output:\n{}", output);

    assert!(output.contains("// Custom header"));
    assert!(output.contains("struct user")); // snake_case type names
    assert!(output.contains("let is_active: Bool")); // snake_case field names
    assert!(output.contains("let email: Optional<String>")); // Optional<T> style
}
