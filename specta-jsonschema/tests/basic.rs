use std::{borrow::Cow, fs};

use specta::{Type, Types};
use specta_jsonschema::{JsonSchema, SchemaVersion};

fn raw_map_types(types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
    Ok(Cow::Borrowed(types))
}

fn raw_map_datatype<'a>(
    _types: &'a Types,
    dt: &'a specta::datatype::DataType,
) -> Result<Cow<'a, specta::datatype::DataType>, specta::FormatError> {
    Ok(Cow::Borrowed(dt))
}

const RAW_FORMAT: specta::Format = specta::Format::new(raw_map_types, raw_map_datatype);

#[derive(Type)]
struct User {
    id: u32,
    name: String,
    email: Option<String>,
}

#[derive(Type)]
enum Status {
    Active,
    Inactive,
    Pending,
}

#[test]
fn test_basic_export() {
    let types = Types::default().register::<User>().register::<Status>();

    let result = JsonSchema::default().export(&types, RAW_FORMAT);
    assert!(result.is_ok(), "Export should succeed: {:?}", result.err());

    let schema_str = result.unwrap();
    assert!(schema_str.contains("\"$schema\""));
    assert!(schema_str.contains("\"User\""));
    assert!(schema_str.contains("\"Status\""));
}

#[test]
fn test_schema_version() {
    let types = Types::default().register::<User>();

    let result = JsonSchema::default()
        .schema_version(SchemaVersion::Draft7)
        .export(&types, RAW_FORMAT);

    assert!(result.is_ok());
    let schema = result.unwrap();
    assert!(schema.contains("http://json-schema.org/draft-07/schema#"));
}

#[test]
fn test_primitives() {
    #[derive(Type)]
    struct Primitives {
        string_field: String,
        int_field: i32,
        float_field: f64,
        bool_field: bool,
    }

    let types = Types::default().register::<Primitives>();
    let result = JsonSchema::default().export(&types, RAW_FORMAT);

    assert!(result.is_ok());
    let schema = result.unwrap();
    assert!(schema.contains("\"type\": \"string\""));
    assert!(schema.contains("\"type\": \"integer\""));
    assert!(schema.contains("\"type\": \"number\""));
    assert!(schema.contains("\"type\": \"boolean\""));
}

#[test]
fn test_nullable() {
    let types = Types::default().register::<User>();
    let result = JsonSchema::default().export(&types, RAW_FORMAT);

    assert!(result.is_ok());
    let schema = result.unwrap();
    // email is Option<String> so should have anyOf with null
    assert!(schema.contains("anyOf") || schema.contains("null"));
}

#[test]
fn test_enum() {
    let types = Types::default().register::<Status>();
    let result = JsonSchema::default().export(&types, RAW_FORMAT);

    assert!(result.is_ok());
    let schema = result.unwrap();
    assert!(schema.contains("\"Active\""));
    assert!(schema.contains("\"Inactive\""));
    assert!(schema.contains("\"Pending\""));
}

#[test]
fn test_export_uses_format() {
    #[derive(Type, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SerdeUser {
        user_id: u32,
    }

    let types = Types::default().register::<SerdeUser>();
    let schema = JsonSchema::default()
        .export(&types, specta_serde::format)
        .unwrap();

    assert!(schema.contains("userId"));
    assert!(!schema.contains("user_id"));
}

#[test]
fn test_export_to_uses_format() {
    #[derive(Type, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct SerdeUser {
        user_id: u32,
    }

    let types = Types::default().register::<SerdeUser>();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("jsonschema-test-{}.json", std::process::id()));

    JsonSchema::default()
        .export_to(&path, &types, specta_serde::format)
        .unwrap();

    let schema = fs::read_to_string(&path).unwrap();
    fs::remove_file(&path).unwrap();

    assert!(schema.contains("userId"));
    assert!(!schema.contains("user_id"));
}
