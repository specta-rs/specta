use specta::{
    Type, Types,
    datatype::{DataType, Field, NamedDataType, Primitive, Struct},
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
    assert!(out.contains("age: bigint"), "output: {out}");
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
fn test_empty_named_struct_is_rejected() {
    #[derive(Type)]
    struct EmptyNamed {}

    assert!(matches!(
        export_err::<EmptyNamed>(),
        specta_rescript::Error::UnsupportedType(message)
            if message.contains("Empty named structs")
    ));
}

#[test]
fn test_optional_unnamed_field_is_rejected() {
    #[derive(Type)]
    struct OptionalTuple(i32, #[specta(optional)] i32);

    assert!(matches!(
        export_err::<OptionalTuple>(),
        specta_rescript::Error::UnsupportedType(message)
            if message.contains("Optional unnamed fields")
    ));
}

#[test]
fn test_empty_unnamed_fields_are_rejected() {
    #[derive(Type)]
    struct EmptyTuple();

    #[derive(Type)]
    enum EmptyTupleVariant {
        Empty(),
        Unit,
    }

    for error in [
        export_err::<EmptyTuple>(),
        export_err::<EmptyTupleVariant>(),
    ] {
        assert!(matches!(
            error,
            specta_rescript::Error::UnsupportedType(message)
                if message.contains("Empty unnamed fields")
        ));
    }
}

#[test]
fn test_skipped_unnamed_field_is_rejected() {
    #[derive(Type)]
    struct SkippedTuple(#[specta(skip)] i32, String);

    assert!(matches!(
        export_err::<SkippedTuple>(),
        specta_rescript::Error::UnsupportedType(message)
            if message.contains("Skipped unnamed fields")
    ));
}

#[test]
fn test_result_detection_validates_unnamed_fields() {
    #[derive(Type)]
    enum OptionalResult {
        Ok(#[specta(optional)] String),
        Err(String),
    }

    #[derive(Type)]
    enum SkippedResult {
        Ok(#[specta(skip)] i32, String),
        Err(String),
    }

    assert!(matches!(
        export_err::<OptionalResult>(),
        specta_rescript::Error::UnsupportedType(message)
            if message.contains("Optional unnamed fields")
    ));
    assert!(matches!(
        export_err::<SkippedResult>(),
        specta_rescript::Error::UnsupportedType(message)
            if message.contains("Skipped unnamed fields")
    ));
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
fn test_serde_type_name_is_validated() {
    #[derive(Type, serde::Serialize)]
    #[serde(rename = "api-response")]
    struct RenamedType {
        value: String,
    }

    let error = ReScript::default()
        .with_serde()
        .export(&Types::default().register::<RenamedType>())
        .unwrap_err();
    assert!(matches!(
        error,
        specta_rescript::Error::InvalidTypeName(name) if name == "api-response"
    ));
}

#[test]
fn test_serde_externally_tagged_data_enum_is_rejected() {
    #[derive(Type, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    enum RenamedVariant {
        FooBar(String),
    }

    assert!(matches!(
        ReScript::default()
            .with_serde()
            .export(&Types::default().register::<RenamedVariant>()),
        Err(specta_rescript::Error::UnsupportedType(message))
            if message.contains("externally tagged enums")
    ));
}

#[test]
fn test_standard_result_uses_builtin_without_alias() {
    #[derive(Type)]
    struct WithResult {
        value: Result<String, i32>,
    }

    let out = export::<WithResult>();
    assert!(out.contains("value: result<string, int>"), "output: {out}");
    assert!(!out.contains("type result"), "output: {out}");
}

#[test]
fn test_serde_untagged_enum_is_rejected() {
    #[derive(Type, serde::Serialize)]
    #[serde(untagged)]
    enum UntaggedValue {
        Text(String),
        Number(i32),
    }

    assert!(matches!(
        ReScript::default()
            .with_serde()
            .export(&Types::default().register::<UntaggedValue>()),
        Err(specta_rescript::Error::UnsupportedType(message))
            if message.contains("Serde untagged enums")
    ));
}

#[test]
fn test_serde_tagged_enums_are_rejected() {
    #[derive(Type, serde::Serialize)]
    #[serde(tag = "kind")]
    enum InternallyTagged {
        Text { value: String },
        Empty,
    }

    #[derive(Type, serde::Serialize)]
    #[serde(tag = "kind", content = "data")]
    enum AdjacentlyTagged {
        Text(String),
        Empty,
    }

    for types in [
        Types::default().register::<InternallyTagged>(),
        Types::default().register::<AdjacentlyTagged>(),
    ] {
        assert!(matches!(
            ReScript::default().with_serde().export(&types),
            Err(specta_rescript::Error::UnsupportedType(message))
                if message.contains("tagged enums")
        ));
    }
}

#[test]
fn test_standard_result_with_serde_uses_builtin() {
    #[derive(Type, serde::Serialize)]
    struct WithResult {
        value: Result<String, String>,
    }

    let out = ReScript::default()
        .with_serde()
        .export(&Types::default().register::<WithResult>())
        .unwrap();
    assert!(
        out.contains("value: result<string, string>"),
        "output: {out}"
    );
    assert!(!out.contains("type result"), "output: {out}");
}

#[test]
fn test_rescript_keywords_are_rejected() {
    #[derive(Type, serde::Serialize)]
    #[serde(rename = "type")]
    struct KeywordType;

    assert!(matches!(
        ReScript::default()
            .with_serde()
            .export(&Types::default().register::<KeywordType>()),
        Err(specta_rescript::Error::InvalidTypeName(name)) if name == "type"
    ));

    #[derive(Type, serde::Serialize)]
    struct KeywordField {
        #[serde(rename = "let")]
        value: String,
    }

    assert!(matches!(
        ReScript::default()
            .with_serde()
            .export(&Types::default().register::<KeywordField>()),
        Err(specta_rescript::Error::InvalidRecordLabel(name)) if name == "let"
    ));

    #[derive(Type)]
    struct Match;

    assert!(matches!(
        export_err::<Match>(),
        specta_rescript::Error::InvalidTypeName(name) if name == "match"
    ));

    #[derive(Type, serde::Serialize)]
    struct LegacyKeywordField {
        #[serde(rename = "class")]
        value: String,
    }

    assert!(matches!(
        ReScript::default()
            .with_serde()
            .export(&Types::default().register::<LegacyKeywordField>()),
        Err(specta_rescript::Error::InvalidRecordLabel(name)) if name == "class"
    ));
}

#[test]
fn test_rescript_builtin_type_names_are_rejected() {
    #[derive(Type, serde::Serialize)]
    #[serde(rename = "option")]
    struct BuiltinType;

    assert!(matches!(
        ReScript::default()
            .with_serde()
            .export(&Types::default().register::<BuiltinType>()),
        Err(specta_rescript::Error::InvalidTypeName(name)) if name == "option"
    ));
}

#[test]
fn test_auxiliary_record_name_collisions_are_rejected() {
    #[derive(Type)]
    struct CollisionEnumDataFields {
        other: String,
    }

    #[derive(Type)]
    enum CollisionEnum {
        Data { value: String },
    }

    let types = Types::default()
        .register::<CollisionEnum>()
        .register::<CollisionEnumDataFields>();
    assert!(matches!(
        ReScript::default().export(&types),
        Err(specta_rescript::Error::DuplicateTypeName { name, .. })
            if name == "collisionEnumDataFields"
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

#[test]
fn test_hidden_named_references_are_rejected() {
    let mut types = Types::default();
    let hidden = NamedDataType::new("Hidden", &mut types, |_, _| {});
    NamedDataType::new("UsesHidden", &mut types, |_, datatype| {
        datatype.ty = Some(
            Struct::named()
                .field(
                    "hidden",
                    Field::new(DataType::Reference(hidden.reference(Vec::new()))),
                )
                .build(),
        );
    });

    assert!(matches!(
        ReScript::default().without_serde().export(&types),
        Err(specta_rescript::Error::UnsupportedType(message))
            if message.contains("Hidden") && message.contains("no definition")
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
fn test_wide_integers_use_bigint() {
    #[derive(Type)]
    struct WideIntegers {
        signed: i64,
        unsigned: u32,
        pointer_sized: usize,
    }

    let out = export::<WideIntegers>();
    assert!(out.contains("signed: bigint"), "output: {out}");
    assert!(out.contains("unsigned: bigint"), "output: {out}");
    assert!(out.contains("pointer_sized: bigint"), "output: {out}");
}

#[test]
fn test_recursive_unnamed_struct_is_rejected() {
    #[derive(Type)]
    struct RecursiveTuple(Box<RecursiveTuple>);

    assert!(matches!(
        export_err::<RecursiveTuple>(),
        specta_rescript::Error::UnsupportedType(message)
            if message.contains("Recursive type aliases")
    ));
}

#[test]
fn test_recursive_transparent_alias_is_rejected() {
    #[derive(Type)]
    #[specta(type = Option<Box<RecursiveAlias>>)]
    struct RecursiveAlias;

    assert!(matches!(
        export_err::<RecursiveAlias>(),
        specta_rescript::Error::UnsupportedType(message)
            if message.contains("Recursive type aliases")
    ));
}

#[test]
#[allow(non_camel_case_types)]
fn test_colliding_generic_parameter_names_are_rejected() {
    #[derive(Type)]
    struct CaseSensitiveGenerics<T, t> {
        upper: T,
        lower: t,
    }

    assert!(matches!(
        export_err::<CaseSensitiveGenerics<String, i32>>(),
        specta_rescript::Error::InvalidType(message)
            if message.contains("both render as '\'t'")
    ));
}

#[test]
fn test_non_string_map_key_error() {
    use std::collections::HashMap;
    #[derive(Type)]
    struct WithBadMap {
        data: HashMap<i32, String>,
    }
    let error = export_err::<WithBadMap>();
    assert!(matches!(error, specta_rescript::Error::UnsupportedType(_)));
}

#[test]
fn test_one_element_tuple_error() {
    #[derive(Type)]
    struct WithUnaryTuple {
        value: (String,),
    }

    let error = export_err::<WithUnaryTuple>();
    assert!(matches!(error, specta_rescript::Error::UnsupportedType(_)));
}

#[test]
fn test_serde_externally_tagged_unit_enum_is_renamed() {
    #[derive(Type, serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    enum RenamedUnitVariant {
        InProgress,
    }

    let out = ReScript::default()
        .with_serde()
        .export(&Types::default().register::<RenamedUnitVariant>())
        .unwrap();
    assert!(out.contains("type renamedUnitVariant = [#inProgress]"));
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
fn test_enum_docs_follow_generated_helper_records() {
    /// Documentation for the enum declaration.
    #[derive(Type)]
    enum DocumentedDataEnum {
        Value { value: String },
    }

    let out = export::<DocumentedDataEnum>();
    let helper = out
        .find("type documentedDataEnumValueFields")
        .expect("generated helper record");
    let docs = out
        .find("Documentation for the enum declaration")
        .expect("enum documentation");
    let enumeration = out
        .find("type documentedDataEnum =")
        .expect("enum declaration");

    assert!(helper < docs && docs < enumeration, "output: {out}");
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
