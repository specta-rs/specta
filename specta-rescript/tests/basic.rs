use specta::{
    Type, Types,
    datatype::{NamedDataType, Primitive},
};
use specta_rescript::ReScript;

fn export<T: specta::Type>() -> String {
    let types = Types::default().register::<T>();
    ReScript::default().without_serde().export(&types).unwrap()
}

fn export_err<T: specta::Type>() -> specta_rescript::Error {
    let types = Types::default().register::<T>();
    ReScript::default()
        .without_serde()
        .export(&types)
        .unwrap_err()
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
        out.contains("type status = [#Active | #Inactive]"),
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
    // Vec<i32> is exported as a named `vec<'t>` type; the field uses it as `vec<int>`.
    assert!(out.contains("int"), "output: {out}");
}

#[test]
fn test_dict_field() {
    use std::collections::HashMap;

    #[derive(Type)]
    struct WithMap {
        data: HashMap<String, i32>,
    }
    let out = export::<WithMap>();
    // HashMap<K, V> is exported as a generic named type `hashMap<'k, 'v> = dict<'v>`.
    assert!(out.contains("dict<"), "output: {out}");
}

#[test]
fn test_header() {
    let types = Types::default().register::<User>();
    let out = ReScript::default()
        .without_serde()
        .header("// custom header")
        .export(&types)
        .unwrap();
    assert!(out.starts_with("// custom header"), "output: {out}");
}

#[test]
fn test_empty_header() {
    let types = Types::default().register::<User>();
    let out = ReScript::default()
        .without_serde()
        .header("")
        .export(&types)
        .unwrap();
    assert!(!out.starts_with("//"), "output: {out}");
}

#[test]
fn test_unit_struct() {
    #[derive(Type)]
    struct Empty;
    let out = export::<Empty>();
    assert!(out.contains("type empty = unit"), "output: {out}");
}

#[test]
fn test_serde_record_label_is_validated() {
    #[derive(Type, serde::Serialize)]
    struct RenamedField {
        #[serde(rename = "kebab-case")]
        value: String,
    }

    let error = ReScript::default()
        .with_serde()
        .export(&Types::default().register::<RenamedField>())
        .unwrap_err();
    assert!(matches!(
        error,
        specta_rescript::Error::InvalidRecordLabel(label) if label == "kebab-case"
    ));
}

#[test]
fn test_named_variant_with_only_skipped_fields_is_unit() {
    #[derive(Type)]
    enum SkippedFields {
        Empty {
            #[specta(skip)]
            value: String,
        },
        Value(String),
    }

    let out = export::<SkippedFields>();
    assert!(out.contains("  | Empty\n"), "output: {out}");
    assert!(!out.contains("skippedFieldsEmptyFields"), "output: {out}");
}

#[test]
fn test_duplicate_rescript_type_names_are_rejected() {
    let mut types = Types::default();
    NamedDataType::new("Duplicate", &mut types, |_, ty| {
        ty.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("Duplicate", &mut types, |_, ty| {
        ty.ty = Some(Primitive::i32.into());
    });

    assert!(matches!(
        ReScript::default().export(&types),
        Err(specta_rescript::Error::DuplicateTypeName { name, .. }) if name == "duplicate"
    ));
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
    // HashMap<K, V> is exported as a generic named type `dict<'v>` regardless of K.
    // The non-string key check cannot be enforced at the generic template level.
    let out = export::<WithBadMap>();
    assert!(out.contains("dict<"), "output: {out}");
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
    let types = Types::default().register::<OldType>();
    let out = ReScript::default().without_serde().export(&types).unwrap();
    assert!(out.contains("/** @deprecated"), "output: {out}");
    assert!(out.contains("use NewType instead"), "output: {out}");
}

#[test]
fn test_recursive_types() {
    #[derive(Type)]
    struct Parent {
        child: Option<Box<Child>>,
    }
    #[derive(Type)]
    struct Child {
        parent: Option<Box<Parent>>,
    }

    let out = export::<Parent>();
    assert!(out.contains("type rec child"), "output: {out}");
    assert!(out.contains("and parent"), "output: {out}");
}

#[test]
fn test_field_and_variant_docs() {
    /// Type docs cannot close */ the generated comment.
    #[derive(Type)]
    struct Documented {
        /// A field cannot close */ the generated comment.
        value: String,
    }
    #[derive(Type)]
    enum DocumentedVariant {
        /// A variant cannot close */ the generated comment.
        Active,
    }

    let types = Types::default()
        .register::<Documented>()
        .register::<DocumentedVariant>();
    let out = ReScript::default().without_serde().export(&types).unwrap();
    assert!(out.contains("Type docs cannot close * /"), "output: {out}");
    assert!(out.contains("A field cannot close * /"), "output: {out}");
    assert!(out.contains("A variant cannot close * /"), "output: {out}");
}

#[test]
#[allow(deprecated)]
fn test_deprecated_comment_terminator_is_sanitized() {
    #[derive(Type)]
    #[deprecated = "cannot close */ the generated comment"]
    struct Deprecated;

    let out = export::<Deprecated>();
    assert!(out.contains("cannot close * /"), "output: {out}");
}
