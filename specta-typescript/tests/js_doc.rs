use specta::{Type, TypeCollection};
use specta_typescript::{BigIntExportBehavior, JSDoc};

#[derive(Type)]
struct SimpleStruct {
    name: String,
    age: u32,
    active: bool,
}

#[derive(Type)]
struct GenericStruct<T> {
    value: T,
    count: u32,
}

#[derive(Type)]
enum SimpleEnum {
    Variant1,
    Variant2(String),
    Variant3 { field: i32 },
}

#[derive(Type)]
struct ComplexStruct {
    simple: SimpleStruct,
    generic: GenericStruct<String>,
    optional: Option<String>,
    list: Vec<i32>,
    tuple: (String, i32, bool),
}

#[test]
fn jsdoc_basic_export() {
    let types = TypeCollection::default().register::<SimpleStruct>();

    let result = JSDoc::default().export(&types);
    assert!(result.is_ok());

    let output = result.unwrap();
    // Should contain JSDoc typedef syntax
    assert!(output.contains("@typedef"));
    assert!(output.contains("SimpleStruct"));
    // Should not contain TypeScript export syntax
    assert!(!output.contains("export type"));
}

#[test]
fn jsdoc_with_generics() {
    let types = TypeCollection::default().register::<GenericStruct<String>>();

    let result = JSDoc::default().export(&types);
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.contains("@typedef"));
    assert!(output.contains("GenericStruct"));
}

#[test]
fn jsdoc_with_enum() {
    let types = TypeCollection::default().register::<SimpleEnum>();

    let result = JSDoc::default().export(&types);
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.contains("@typedef"));
    assert!(output.contains("SimpleEnum"));
}

#[test]
fn jsdoc_complex_types() {
    let types = TypeCollection::default().register::<ComplexStruct>();

    let result = JSDoc::default().export(&types);
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.contains("@typedef"));
    assert!(output.contains("ComplexStruct"));
}

#[test]
fn jsdoc_bigint_behavior() {
    #[derive(Type)]
    struct BigIntStruct {
        big_number: i64,
    }

    let types = TypeCollection::default().register::<BigIntStruct>();

    // Test string behavior
    let result = JSDoc::default()
        .bigint(BigIntExportBehavior::String)
        .export(&types);
    assert!(result.is_ok());

    // Test number behavior
    let result = JSDoc::default()
        .bigint(BigIntExportBehavior::Number)
        .export(&types);
    assert!(result.is_ok());

    // Test bigint behavior
    let result = JSDoc::default()
        .bigint(BigIntExportBehavior::BigInt)
        .export(&types);
    assert!(result.is_ok());

    // Test fail behavior
    let result = JSDoc::default()
        .bigint(BigIntExportBehavior::Fail)
        .export(&types);
    assert!(result.is_err());
}

#[test]
fn jsdoc_with_header() {
    let types = TypeCollection::default().register::<SimpleStruct>();

    let result = JSDoc::default()
        .header("/* eslint-disable */")
        .export(&types);
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.contains("/* eslint-disable */"));
}

#[test]
fn jsdoc_conversion_from_typescript() {
    use specta_typescript::Typescript;

    let ts = Typescript::default()
        .header("// TypeScript header")
        .bigint(BigIntExportBehavior::String);

    let jsdoc = JSDoc::from(ts);

    let types = TypeCollection::default().register::<SimpleStruct>();
    let result = jsdoc.export(&types);
    assert!(result.is_ok());

    let output = result.unwrap();
    assert!(output.contains("// TypeScript header"));
    assert!(output.contains("@typedef"));
}

#[test]
fn jsdoc_output_demonstration() {
    #[derive(Type)]
    /// A user in the system
    struct User {
        /// The user's full name
        name: String,
        /// The user's age in years
        age: u32,
        /// Whether the user account is active
        active: bool,
        /// Optional email address
        email: Option<String>,
    }

    let types = TypeCollection::default().register::<User>();
    let output = JSDoc::default().export(&types).unwrap();

    println!("JSDoc Output:\n{}", output);

    // Verify basic structure
    assert!(output.contains("@typedef"));
    assert!(output.contains("User"));
    assert!(output.contains("Object"));
}
