use specta::{Type, TypeCollection};
use specta_rescript::ReScript;

fn export<T: specta::Type>() -> String {
    let types = TypeCollection::default().register::<T>();
    ReScript::default().without_serde().export(&types).unwrap()
}

fn export_err<T: specta::Type>() -> specta_rescript::Error {
    let types = TypeCollection::default().register::<T>();
    ReScript::default().without_serde().export(&types).unwrap_err()
}

#[derive(Type)]
struct User {
    name: String,
    age: u32,
}

#[derive(Type)]
enum Status {
    Active,
    Inactive,
}

#[derive(Type)]
enum Shape {
    Circle(f64),
    Rect { width: f64, height: f64 },
}

#[derive(Type)]
struct Wrapper<T>(T);

#[derive(Type)]
struct Pair<A, B> {
    first: A,
    second: B,
}

#[test]
fn test_basic_struct() {
    let out = export::<User>();
    assert!(out.contains("type user = {"), "output: {out}");
    assert!(out.contains("name: string"), "output: {out}");
    assert!(out.contains("age: int"), "output: {out}");
}

#[test]
fn test_unit_enum_polymorphic_variants() {
    let out = export::<Status>();
    assert!(
        out.contains("type status = [ #Active | #Inactive ]"),
        "output: {out}"
    );
}

#[test]
fn test_data_enum_with_auxiliary_record() {
    let out = export::<Shape>();
    // Should have an auxiliary record type for the Rect variant
    assert!(out.contains("type shapeRectFields = {"), "output: {out}");
    assert!(out.contains("width: float"), "output: {out}");
    assert!(out.contains("height: float"), "output: {out}");
    // Main enum type
    assert!(out.contains("type shape ="), "output: {out}");
    assert!(out.contains("| Circle(float)"), "output: {out}");
    assert!(out.contains("| Rect(shapeRectFields)"), "output: {out}");
}

#[test]
fn test_newtype_wrapper() {
    let out = export::<Wrapper<String>>();
    assert!(out.contains("type wrapper"), "output: {out}");
}

#[test]
fn test_generic_struct() {
    let out = export::<Pair<String, u32>>();
    // Pair<A, B> should use apostrophe generics
    assert!(out.contains("type pair<"), "output: {out}");
    assert!(out.contains("'a"), "output: {out}");
    assert!(out.contains("'b"), "output: {out}");
}

#[test]
fn test_result_type() {
    // Manually build a Result-shaped enum to test the detection logic
    #[derive(Type)]
    enum MyResult {
        Ok(String),
        Err(i32),
    }
    let out = export::<MyResult>();
    assert!(out.contains("result<string, int>"), "output: {out}");
}

#[test]
fn test_option_field() {
    #[derive(Type)]
    struct WithOption {
        value: Option<String>,
    }
    let out = export::<WithOption>();
    assert!(out.contains("option<string>"), "output: {out}");
}

#[test]
fn test_array_field() {
    #[derive(Type)]
    struct WithVec {
        items: Vec<i32>,
    }
    let out = export::<WithVec>();
    assert!(out.contains("array<int>"), "output: {out}");
}

#[test]
fn test_dict_field() {
    use std::collections::HashMap;

    #[derive(Type)]
    struct WithMap {
        data: HashMap<String, i32>,
    }
    let out = export::<WithMap>();
    assert!(out.contains("dict<int>"), "output: {out}");
}

#[test]
fn test_header() {
    let types = TypeCollection::default().register::<User>();
    let out = ReScript::default()
        .without_serde()
        .header("// custom header")
        .export(&types)
        .unwrap();
    assert!(out.starts_with("// custom header"), "output: {out}");
}

#[test]
fn test_empty_header() {
    let types = TypeCollection::default().register::<User>();
    let out = ReScript::default().without_serde().header("").export(&types).unwrap();
    assert!(!out.starts_with("//"), "output: {out}");
}

#[test]
fn test_unit_struct() {
    #[derive(Type)]
    struct Empty;
    let out = export::<Empty>();
    assert!(out.contains("type empty = unit"), "output: {out}");
}

// ---------------------------------------------------------------------------
// Error paths
// ---------------------------------------------------------------------------

#[test]
fn test_i128_error() {
    #[derive(Type)]
    struct WithI128 {
        value: i128,
    }
    let err = export_err::<WithI128>();
    assert!(matches!(err, specta_rescript::Error::UnsupportedType(_)));
}

#[test]
fn test_u128_error() {
    #[derive(Type)]
    struct WithU128 {
        value: u128,
    }
    let err = export_err::<WithU128>();
    assert!(matches!(err, specta_rescript::Error::UnsupportedType(_)));
}

#[test]
fn test_non_string_map_key_error() {
    use std::collections::HashMap;
    #[derive(Type)]
    struct WithBadMap {
        data: HashMap<i32, String>,
    }
    let err = export_err::<WithBadMap>();
    assert!(matches!(err, specta_rescript::Error::InvalidType(_)));
}

// ---------------------------------------------------------------------------
// Deprecated rendering
// ---------------------------------------------------------------------------

#[test]
#[allow(deprecated)]
fn test_deprecated_type() {
    #[derive(Type)]
    #[deprecated = "use NewType instead"]
    struct OldType {
        value: String,
    }
    let types = TypeCollection::default().register::<OldType>();
    let out = ReScript::default().without_serde().export(&types).unwrap();
    assert!(out.contains("// @deprecated"), "output: {out}");
    assert!(out.contains("use NewType instead"), "output: {out}");
}

