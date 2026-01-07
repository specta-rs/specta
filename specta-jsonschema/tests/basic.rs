use specta::{Type, TypeCollection};
use specta_jsonschema::{JsonSchema, Layout, SchemaVersion};

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
    let types = TypeCollection::default()
        .register::<User>()
        .register::<Status>();

    let result = JsonSchema::default().export(&types);
    assert!(result.is_ok(), "Export should succeed: {:?}", result.err());

    let schema_str = result.unwrap();
    assert!(schema_str.contains("\"$schema\""));
    assert!(schema_str.contains("\"User\""));
    assert!(schema_str.contains("\"Status\""));
}

#[test]
fn test_schema_version() {
    let types = TypeCollection::default().register::<User>();

    let result = JsonSchema::default()
        .schema_version(SchemaVersion::Draft7)
        .export(&types);

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

    let types = TypeCollection::default().register::<Primitives>();
    let result = JsonSchema::default().export(&types);

    assert!(result.is_ok());
    let schema = result.unwrap();
    assert!(schema.contains("\"type\": \"string\""));
    assert!(schema.contains("\"type\": \"integer\""));
    assert!(schema.contains("\"type\": \"number\""));
    assert!(schema.contains("\"type\": \"boolean\""));
}

#[test]
fn test_nullable() {
    let types = TypeCollection::default().register::<User>();
    let result = JsonSchema::default().export(&types);

    assert!(result.is_ok());
    let schema = result.unwrap();
    // email is Option<String> so should have anyOf with null
    assert!(schema.contains("anyOf") || schema.contains("null"));
}

#[test]
fn test_enum() {
    let types = TypeCollection::default().register::<Status>();
    let result = JsonSchema::default().export(&types);

    assert!(result.is_ok());
    let schema = result.unwrap();
    assert!(schema.contains("\"Active\""));
    assert!(schema.contains("\"Inactive\""));
    assert!(schema.contains("\"Pending\""));
}
