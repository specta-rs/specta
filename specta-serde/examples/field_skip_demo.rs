//! Comprehensive example demonstrating field-level skip attributes in specta-serde
//!
//! This example shows how field-level serde attributes like `skip`, `skip_serializing`,
//! and `skip_deserializing` are now properly supported in specta-serde.

use serde::{Deserialize, Serialize};
use specta::Type;
use specta_macros::Type as TypeDerive;
use specta_serde::process_for_both;

#[derive(TypeDerive, Serialize, Deserialize, Debug)]
struct User {
    /// Public user ID - always included
    pub id: u64,

    /// Username - always included
    pub username: String,

    /// Email - always included
    pub email: String,

    /// Password hash - never serialized or deserialized
    #[serde(skip)]
    password_hash: String,

    /// Session token - only serialized, never deserialized
    #[serde(skip_deserializing)]
    pub session_token: Option<String>,

    /// Internal admin notes - only deserialized, never serialized
    #[serde(skip_serializing)]
    internal_notes: Option<String>,

    /// Renamed field
    #[serde(rename = "displayName")]
    pub display_name: String,

    /// Field with custom serialization
    #[serde(serialize_with = "serialize_timestamp")]
    pub created_at: u64,
}

#[derive(TypeDerive, Serialize, Deserialize, Debug)]
enum UserRole {
    /// Regular user
    User,

    /// Admin user
    Admin,

    /// This variant is skipped in serialization/deserialization
    #[serde(skip)]
    SystemInternal,

    /// Renamed variant
    #[serde(rename = "super_admin")]
    SuperAdmin,
}

#[derive(TypeDerive, Serialize, Deserialize, Debug)]
struct UserProfile {
    pub user: User,
    pub role: UserRole,

    /// Flattened preferences
    #[serde(flatten)]
    pub preferences: UserPreferences,

    /// Optional field with default
    #[serde(default)]
    pub is_verified: bool,
}

#[derive(TypeDerive, Serialize, Deserialize, Debug)]
struct UserPreferences {
    pub theme: String,
    pub language: String,

    /// This field is skipped
    #[serde(skip)]
    internal_cache: Option<String>,
}

// Custom serializer function
fn serialize_timestamp<S>(timestamp: &u64, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_u64(*timestamp)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Field-Level Skip Attributes Demo ===\n");

    // Create a type collection
    let types = specta::TypeCollection::default()
        .register::<User>()
        .register::<UserRole>()
        .register::<UserProfile>()
        .register::<UserPreferences>();

    println!("Original type collection has {} types", types.len());

    // Process for serialization and deserialization
    let (ser_types, de_types) = process_for_both(&types)?;

    println!("Serialization types: {}", ser_types.len());
    println!("Deserialization types: {}", de_types.len());

    // Demonstrate that the transformations preserve field attributes
    for ndt in ser_types.into_unsorted_iter() {
        println!("\n=== Serialization type: {} ===", ndt.name());
        print_type_info(ndt.ty());
    }

    for ndt in de_types.into_unsorted_iter() {
        println!("\n=== Deserialization type: {} ===", ndt.name());
        print_type_info(ndt.ty());
    }

    // Show how it works with actual data
    demonstrate_serialization()?;

    Ok(())
}

fn print_type_info(dt: &specta::DataType) {
    match dt {
        specta::DataType::Struct(s) => {
            if let specta::datatype::Fields::Named(fields) = s.fields() {
                println!("  Fields:");
                for (name, field) in fields.fields() {
                    let attrs_info = if field.attributes().is_empty() {
                        "no attributes".to_string()
                    } else {
                        format!("{} runtime attributes", field.attributes().len())
                    };

                    let ty_info = if let Some(ty) = field.ty() {
                        format!("type: {:?}", get_type_name(ty))
                    } else {
                        "skipped field".to_string()
                    };

                    println!("    - {}: {} ({})", name, ty_info, attrs_info);
                }
            }
        }
        specta::DataType::Enum(e) => {
            println!("  Variants:");
            for (name, variant) in e.variants() {
                let attrs_info = if variant.attributes().is_empty() {
                    "no attributes".to_string()
                } else {
                    format!("{} runtime attributes", variant.attributes().len())
                };

                let skip_info = if variant.skip() { " (skipped)" } else { "" };

                println!("    - {}: {} {}", name, attrs_info, skip_info);
            }
        }
        _ => {
            println!("  Other type: {:?}", get_type_name(dt));
        }
    }
}

fn get_type_name(dt: &specta::DataType) -> &'static str {
    match dt {
        specta::DataType::Primitive(_) => "Primitive",

        specta::DataType::Nullable(_) => "Nullable",
        specta::DataType::Map(_) => "Map",
        specta::DataType::List(_) => "List",
        specta::DataType::Tuple(_) => "Tuple",
        specta::DataType::Struct(_) => "Struct",
        specta::DataType::Enum(_) => "Enum",
        specta::DataType::Reference(_) => "Reference",
        specta::DataType::Generic(_) => "Generic",
    }
}

fn demonstrate_serialization() -> Result<(), Box<dyn std::error::Error>> {
    println!("\n=== Serialization Demo ===");

    let user = User {
        id: 1,
        username: "john_doe".to_string(),
        email: "john@example.com".to_string(),
        password_hash: "secret_hash_never_serialized".to_string(),
        session_token: Some("abc123".to_string()),
        internal_notes: Some("Admin notes - not serialized".to_string()),
        display_name: "John Doe".to_string(),
        created_at: 1640995200, // 2022-01-01
    };

    let preferences = UserPreferences {
        theme: "dark".to_string(),
        language: "en".to_string(),
        internal_cache: Some("cache_data".to_string()),
    };

    let profile = UserProfile {
        user,
        role: UserRole::Admin,
        preferences,
        is_verified: true,
    };

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&profile)?;
    println!("Serialized JSON (note missing skip fields):");
    println!("{}", json);

    // The JSON should not contain:
    // - password_hash (skip)
    // - internal_notes (skip_serializing)
    // - internal_cache (skip)
    // But should contain:
    // - session_token (skip_deserializing - still serialized)
    // - displayName instead of display_name (renamed)
    // - flattened preferences fields

    Ok(())
}
