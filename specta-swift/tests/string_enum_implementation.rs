use serde::{Deserialize, Serialize};
use specta::datatype::DataType;
use specta::{Type, Types};
use specta_swift::Swift;

fn raw_format() -> (
    impl for<'a> Fn(&'a Types) -> Result<std::borrow::Cow<'a, Types>, specta_swift::FormatError>,
    impl for<'a> Fn(
        &'a Types,
        &'a DataType,
    ) -> Result<std::borrow::Cow<'a, DataType>, specta_swift::FormatError>,
) {
    (
        |types| Ok(std::borrow::Cow::Borrowed(types)),
        |_, dt| Ok(std::borrow::Cow::Borrowed(dt)),
    )
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
pub enum Priority {
    Low,
    Medium,
    High,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LogLevel {
    Debug,
    Info,
    Warning,
    Error,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub enum UserRole {
    Admin,
    Moderator,
    User,
    Guest,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ApiStatus {
    Online,
    Offline,
    Maintenance,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING-KEBAB-CASE")]
pub enum DatabaseStatus {
    Connected,
    Disconnected,
    Reconnecting,
}

// This should NOT be a string enum (has data fields)
#[derive(Type, Serialize, Deserialize)]
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
    let types = Types::default().register::<JobStatus>();

    let swift = Swift::default();
    let result = swift.export(&types, specta_swift::raw_format()).unwrap();

    println!("Generated Swift for JobStatus:");
    println!("{}", result);

    assert!(result.contains("enum JobStatus: Codable"));
    assert!(result.contains("case queued"));
    assert!(result.contains("case running"));
    assert!(result.contains("case completed"));
    assert!(result.contains("case failed"));
    assert!(result.contains("case cancelled"));
}

#[test]
fn test_string_enum_uppercase() {
    let types = Types::default().register::<Priority>();

    let swift = Swift::default();
    let result = swift.export(&types, specta_swift::raw_format()).unwrap();

    println!("Generated Swift for Priority:");
    println!("{}", result);

    assert!(result.contains("enum Priority: Codable"));
    assert!(result.contains("case low"));
    assert!(result.contains("case medium"));
    assert!(result.contains("case high"));
}

#[test]
fn test_string_enum_camel_case() {
    let types = Types::default().register::<LogLevel>();

    let swift = Swift::default();
    let result = swift.export(&types, specta_swift::raw_format()).unwrap();

    println!("Generated Swift for LogLevel:");
    println!("{}", result);

    assert!(result.contains("enum LogLevel: Codable"));
    assert!(result.contains("case debug"));
    assert!(result.contains("case info"));
    assert!(result.contains("case warning"));
    assert!(result.contains("case error"));
}

#[test]
fn test_string_enum_pascal_case() {
    let types = Types::default().register::<UserRole>();

    let swift = Swift::default();
    let result = swift.export(&types, specta_swift::raw_format()).unwrap();

    println!("Generated Swift for UserRole:");
    println!("{}", result);

    assert!(result.contains("enum UserRole: Codable"));
    assert!(result.contains("case admin"));
    assert!(result.contains("case moderator"));
    assert!(result.contains("case user"));
    assert!(result.contains("case guest"));
}

#[test]
fn test_string_enum_kebab_case() {
    let types = Types::default().register::<ApiStatus>();

    let swift = Swift::default();
    let result = swift.export(&types, specta_swift::raw_format()).unwrap();

    println!("Generated Swift for ApiStatus:");
    println!("{}", result);

    assert!(result.contains("enum ApiStatus: Codable"));
    assert!(result.contains("case online"));
    assert!(result.contains("case offline"));
    assert!(result.contains("case maintenance"));
}

#[test]
fn test_string_enum_screaming_kebab_case() {
    let types = Types::default().register::<DatabaseStatus>();

    let swift = Swift::default();
    let result = swift.export(&types, specta_swift::raw_format()).unwrap();

    println!("Generated Swift for DatabaseStatus:");
    println!("{}", result);

    assert!(result.contains("enum DatabaseStatus: Codable"));
    assert!(result.contains("case connected"));
    assert!(result.contains("case disconnected"));
    assert!(result.contains("case reconnecting"));
}

#[test]
fn test_mixed_enum_not_string() {
    let types = Types::default().register::<MixedEnum>();
    let swift = Swift::default();
    let result = swift.export(&types, specta_swift::raw_format()).unwrap();

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
    let types = Types::default().register::<RegularEnum>();
    let swift = Swift::default();
    let result = swift.export(&types, specta_swift::raw_format()).unwrap();

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
    let string_types = Types::default()
        .register::<JobStatus>()
        .register::<Priority>()
        .register::<LogLevel>()
        .register::<UserRole>()
        .register::<ApiStatus>()
        .register::<DatabaseStatus>();
    let other_types = Types::default()
        .register::<MixedEnum>()
        .register::<RegularEnum>();
    let swift = Swift::default();
    let result = swift.export(&string_types, specta_swift::raw_format()).unwrap();
    let raw_result = swift.export(&other_types, raw_format()).unwrap();

    println!("Generated Swift for all enums:");
    println!("{}", result);

    assert!(result.contains("enum JobStatus: Codable"));
    assert!(result.contains("enum Priority: Codable"));
    assert!(result.contains("enum LogLevel: Codable"));
    assert!(result.contains("enum UserRole: Codable"));
    assert!(result.contains("enum ApiStatus: Codable"));
    assert!(result.contains("enum DatabaseStatus: Codable"));

    // Check that non-string enums are generated correctly
    assert!(raw_result.contains("enum MixedEnum")); // No redundant Codable in declaration
    assert!(raw_result.contains("enum RegularEnum: Codable")); // Simple enum can have Codable in declaration
    assert!(raw_result.contains("extension MixedEnum: Codable")); // Complex enum has Codable in extension
}
