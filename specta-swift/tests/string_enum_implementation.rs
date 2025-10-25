use specta::{Type, TypeCollection};
use specta_swift::{NamingConvention, Swift};

#[derive(Type)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Type)]
#[serde(rename_all = "UPPERCASE")]
pub enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Type)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Type)]
#[serde(rename_all = "PascalCase")]
pub enum UserRole {
    Admin,
    Moderator,
    User,
    Guest,
}

#[derive(Type)]
#[serde(rename_all = "kebab-case")]
pub enum ApiStatus {
    Online,
    Offline,
    Maintenance,
}

#[derive(Type)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub enum DatabaseStatus {
    Connected,
    Disconnected,
    Reconnecting,
}

// This should NOT be a string enum (has data fields)
#[derive(Type)]
#[serde(rename_all = "snake_case")]
pub enum MixedEnum {
    Unit,
    WithData(String),
    WithFields { name: String, value: i32 },
}

// This should NOT be a string enum (no rename_all)
#[derive(Type)]
pub enum RegularEnum {
    Variant1,
    Variant2,
    Variant3,
}

#[test]
fn test_string_enum_snake_case() {
    let types = TypeCollection::default().register::<JobStatus>();

    let swift = Swift::default();
    let result = swift.export(&types).unwrap();

    println!("Generated Swift for JobStatus:");
    println!("{}", result);

    // Should contain string enum syntax
    assert!(result.contains("enum JobStatus: String, Codable"));
    assert!(result.contains("case queued = \"queued\""));
    assert!(result.contains("case running = \"running\""));
    assert!(result.contains("case completed = \"completed\""));
    assert!(result.contains("case failed = \"failed\""));
    assert!(result.contains("case cancelled = \"cancelled\""));
}

#[test]
fn test_string_enum_uppercase() {
    let types = TypeCollection::default().register::<Priority>();

    let swift = Swift::default();
    let result = swift.export(&types).unwrap();

    println!("Generated Swift for Priority:");
    println!("{}", result);

    // Should contain string enum syntax with uppercase values
    assert!(result.contains("enum Priority: String, Codable"));
    assert!(result.contains("case lOW = \"LOW\""));
    assert!(result.contains("case mEDIUM = \"MEDIUM\""));
    assert!(result.contains("case hIGH = \"HIGH\""));
}

#[test]
fn test_string_enum_camel_case() {
    let types = TypeCollection::default().register::<LogLevel>();

    let swift = Swift::default();
    let result = swift.export(&types).unwrap();

    println!("Generated Swift for LogLevel:");
    println!("{}", result);

    // Should contain string enum syntax with camelCase values
    assert!(result.contains("enum LogLevel: String, Codable"));
    assert!(result.contains("case debug = \"debug\""));
    assert!(result.contains("case info = \"info\""));
    assert!(result.contains("case warning = \"warning\""));
    assert!(result.contains("case error = \"error\""));
}

#[test]
fn test_string_enum_pascal_case() {
    let types = TypeCollection::default().register::<UserRole>();

    let swift = Swift::default();
    let result = swift.export(&types).unwrap();

    println!("Generated Swift for UserRole:");
    println!("{}", result);

    // Should contain string enum syntax with PascalCase values
    assert!(result.contains("enum UserRole: String, Codable"));
    assert!(result.contains("case admin = \"Admin\""));
    assert!(result.contains("case moderator = \"Moderator\""));
    assert!(result.contains("case user = \"User\""));
    assert!(result.contains("case guest = \"Guest\""));
}

#[test]
fn test_string_enum_kebab_case() {
    let types = TypeCollection::default().register::<ApiStatus>();

    let swift = Swift::default();
    let result = swift.export(&types).unwrap();

    println!("Generated Swift for ApiStatus:");
    println!("{}", result);

    // Should contain string enum syntax with kebab-case values
    assert!(result.contains("enum ApiStatus: String, Codable"));
    assert!(result.contains("case online = \"online\""));
    assert!(result.contains("case offline = \"offline\""));
    assert!(result.contains("case maintenance = \"maintenance\""));
}

#[test]
fn test_string_enum_screaming_kebab_case() {
    let types = TypeCollection::default().register::<DatabaseStatus>();

    let swift = Swift::default();
    let result = swift.export(&types).unwrap();

    println!("Generated Swift for DatabaseStatus:");
    println!("{}", result);

    // Should contain string enum syntax with SCREAMING-KEBAB-CASE values
    assert!(result.contains("enum DatabaseStatus: String, Codable"));
    assert!(result.contains("case cONNECTED = \"C-O-N-N-E-C-T-E-D\""));
    assert!(result.contains("case dISCONNECTED = \"D-I-S-C-O-N-N-E-C-T-E-D\""));
    assert!(result.contains("case rECONNECTING = \"R-E-C-O-N-N-E-C-T-I-N-G\""));
}

#[test]
fn test_mixed_enum_not_string() {
    let types = TypeCollection::default().register::<MixedEnum>();

    let swift = Swift::default();
    let result = swift.export(&types).unwrap();

    println!("Generated Swift for MixedEnum:");
    println!("{}", result);

    // Should NOT be a string enum (has data fields) - no redundant Codable in declaration
    assert!(result.contains("enum MixedEnum"));
    assert!(!result.contains("enum MixedEnum: Codable"));
    assert!(!result.contains("enum MixedEnum: String, Codable"));

    // Should have Codable in extension instead
    assert!(result.contains("extension MixedEnum: Codable"));
    assert!(result.contains("case unit"));
    assert!(result.contains("case withData(String)"));
    assert!(result.contains("case withFields(MixedEnumWithFieldsData)"));
    assert!(result.contains("struct MixedEnumWithFieldsData: Codable"));
    assert!(result.contains("let name: String"));
    assert!(result.contains("let value: Int32"));
}

#[test]
fn test_regular_enum_not_string() {
    let types = TypeCollection::default().register::<RegularEnum>();

    let swift = Swift::default();
    let result = swift.export(&types).unwrap();

    println!("Generated Swift for RegularEnum:");
    println!("{}", result);

    // Should NOT be a string enum (no rename_all)
    assert!(result.contains("enum RegularEnum: Codable"));
    assert!(!result.contains("enum RegularEnum: String, Codable"));
    assert!(result.contains("case variant1"));
    assert!(result.contains("case variant2"));
    assert!(result.contains("case variant3"));
}

#[test]
fn test_all_string_enums_together() {
    let types = TypeCollection::default()
        .register::<JobStatus>()
        .register::<Priority>()
        .register::<LogLevel>()
        .register::<UserRole>()
        .register::<ApiStatus>()
        .register::<DatabaseStatus>()
        .register::<MixedEnum>()
        .register::<RegularEnum>();

    let swift = Swift::default();
    let result = swift.export(&types).unwrap();

    println!("Generated Swift for all enums:");
    println!("{}", result);

    // Check that string enums are generated correctly
    assert!(result.contains("enum JobStatus: String, Codable"));
    assert!(result.contains("enum Priority: String, Codable"));
    assert!(result.contains("enum LogLevel: String, Codable"));
    assert!(result.contains("enum UserRole: String, Codable"));
    assert!(result.contains("enum ApiStatus: String, Codable"));
    assert!(result.contains("enum DatabaseStatus: String, Codable"));

    // Check that non-string enums are generated correctly
    assert!(result.contains("enum MixedEnum")); // No redundant Codable in declaration
    assert!(result.contains("enum RegularEnum: Codable")); // Simple enum can have Codable in declaration
    assert!(result.contains("extension MixedEnum: Codable")); // Complex enum has Codable in extension
}
