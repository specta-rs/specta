#![allow(clippy::unwrap_used, dead_code, deprecated, missing_docs)]

use std::{borrow::Cow, fs};

use serde::{Deserialize, Serialize};
use specta::{
    Format, Type, Types,
    datatype::{
        Attributes, DataType, Deprecated, Enum, Field, Generic, GenericDefinition, List,
        Map as SpectaMap, NamedDataType, Primitive, Struct, Variant,
    },
};
use specta_jsonschema::{JsonSchema, SchemaVersion};

#[derive(Type)]
struct Primitives {
    bool_field: bool,
    string_field: String,
    char_field: char,
    i8_field: i8,
    u16_field: u16,
    i32_field: i32,
    u64_field: u64,
    f32_field: f32,
    f64_field: f64,
}

#[derive(Type)]
struct PointerPrimitives {
    isize_field: isize,
    usize_field: usize,
}

#[derive(Type)]
struct User {
    id: u32,
    name: String,
    email: Option<String>,
}

#[derive(Type, Serialize, Deserialize)]
struct PhaseSplitRoot {
    #[serde(skip_deserializing)]
    serialize_only: String,
}

#[derive(Type)]
struct Collections {
    tags: Vec<String>,
    tuple: (String, u32),
    map: std::collections::HashMap<String, u32>,
}

#[derive(Type, Eq, PartialEq, std::hash::Hash)]
enum Key {
    A,
    B,
}

#[derive(Type)]
struct MapKeys {
    chars: std::collections::HashMap<char, u32>,
    enums: std::collections::HashMap<Key, u32>,
}

#[derive(Type)]
enum Status {
    Active,
    Inactive,
    Pending,
}

#[derive(Type)]
enum Message {
    Quit,
    Move { x: i32, y: i32 },
    Write(String),
}

#[derive(Type)]
struct Wrapper<T> {
    value: T,
}

#[derive(Type)]
struct UsesGenerics {
    user: Wrapper<User>,
    count: Wrapper<u32>,
}

#[derive(Type)]
struct Nested<T> {
    value: T,
}

#[derive(Type)]
struct UsesNestedGenerics {
    string: Wrapper<Nested<String>>,
    number: Wrapper<Nested<u32>>,
}

#[derive(Type)]
struct GenericComposition<T> {
    wrapped: Wrapper<T>,
}

#[derive(Type)]
struct UsesGenericComposition {
    value: GenericComposition<String>,
}

#[derive(Type)]
struct Newtype(String);

/// Annotated newtype container documentation.
#[derive(Type)]
struct AnnotatedNewtype(
    /// Newtype field documentation.
    #[deprecated(note = "Use another field")]
    String,
);

#[derive(Type)]
struct AnnotatedTuple(
    /// First tuple field documentation.
    #[deprecated(note = "Use the second field")]
    String,
    /// Second tuple field documentation.
    u32,
);

#[derive(Serialize, Deserialize, Type)]
#[serde(untagged)]
enum OverlappingUntagged {
    Signed(i32),
    Unsigned(u32),
}

#[derive(Type, Eq, PartialEq, std::hash::Hash)]
struct InvalidMapKey {
    value: String,
}

#[derive(Type)]
struct UsesInvalidMapKey {
    values: std::collections::HashMap<InvalidMapKey, String>,
}

#[derive(Serialize, Deserialize, Type)]
#[serde(rename = "Escaped/Type~")]
struct EscapedType {
    value: String,
}

#[derive(Serialize, Deserialize, Type)]
#[serde(rename = "Escaped #/%雪~")]
struct UriEscapedType(String);

#[derive(Type)]
struct ReverseGenerics<Z, A> {
    z: Z,
    a: A,
}

#[derive(Type)]
struct UsesReverseGenerics {
    value: ReverseGenerics<String, u32>,
}

#[derive(Type, Serialize, Eq, PartialEq, std::hash::Hash)]
struct NumericKey(u32);

#[derive(Type, Serialize)]
struct LexicalMapKeys {
    numeric: std::collections::HashMap<NumericKey, String>,
    boolean: std::collections::HashMap<bool, String>,
}

#[derive(Type, Serialize)]
struct FixedWidthIntegerMapKeys {
    signed: std::collections::HashMap<i8, String>,
    unsigned: std::collections::HashMap<u8, String>,
    pointer_signed: std::collections::HashMap<isize, String>,
    pointer_unsigned: std::collections::HashMap<usize, String>,
}

#[derive(Type)]
enum RawNewtypeEnum {
    Value(String),
}

#[derive(Type, Deserialize)]
#[serde(tag = "kind")]
enum InternalOther {
    #[serde(rename = "known")]
    Known { value: u32 },
    #[serde(other)]
    Other,
}

/// Named map-key documentation.
#[deprecated(note = "Use another key")]
#[derive(Type, Eq, PartialEq, std::hash::Hash)]
enum DocumentedKey {
    A,
    B,
}

#[allow(deprecated)]
#[derive(Type)]
struct DocumentedKeyMap {
    values: std::collections::HashMap<DocumentedKey, String>,
}

#[derive(Type, Serialize, Deserialize)]
enum InnerChoices {
    Outer,
    Other,
}

#[derive(Type, Serialize, Deserialize)]
enum RewrittenOuterPayload {
    Outer(InnerChoices),
    Unit,
}

#[derive(Type)]
struct RawNewtypeEnumMap {
    values: std::collections::HashMap<RawNewtypeEnum, String>,
}

#[derive(Type, Serialize, Deserialize, Eq, PartialEq, std::hash::Hash)]
enum MixedMapKey {
    Unit,
    #[serde(untagged)]
    Number(u8),
}

#[derive(Type, Serialize, Deserialize)]
struct MixedMapKeyMap {
    values: std::collections::HashMap<MixedMapKey, String>,
}

#[derive(Type, Serialize)]
#[serde(untagged)]
enum DocumentedUntagged {
    /// Text branch.
    Text(String),
    Number(u32),
}

struct IdentityFormat;

impl Format for IdentityFormat {
    fn map_types(&self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(
        &self,
        _types: &Types,
        ty: &specta::datatype::DataType,
    ) -> Result<Cow<'_, specta::datatype::DataType>, specta::FormatError> {
        Ok(Cow::Owned(ty.clone()))
    }
}

#[derive(Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
struct SerdeUser {
    user_id: u32,
    full_name: String,
}

#[derive(Serialize, Deserialize, Type)]
#[serde(tag = "type", content = "data")]
enum SerdeMessage {
    Text { body: String },
    Count(u32),
}

#[derive(Serialize, Deserialize, Type)]
#[serde(untagged)]
enum UntaggedValue {
    Text(String),
    Object { id: u32 },
}

#[derive(Serialize, Deserialize, Type)]
struct FlattenA {
    a: String,
}

#[derive(Serialize, Deserialize, Type)]
struct FlattenB {
    b: String,
}

#[derive(Serialize, Deserialize, Type)]
struct Flattened {
    #[serde(flatten)]
    a: FlattenA,
    #[serde(flatten)]
    b: FlattenB,
}

#[derive(Serialize, Deserialize, Type)]
struct OptionalFlattened {
    base: String,
    #[serde(flatten)]
    extra: Option<FlattenA>,
}

#[derive(Type)]
struct DocumentedFields {
    /// GitHub issue #491 regression: field docs should become JSON Schema descriptions.
    name: String,
}

#[derive(Type)]
struct DocumentedReference {
    /// The referenced user.
    user: User,
}

#[test]
fn exports_metadata_and_primitives() {
    let types = Types::default().register::<Primitives>();
    let schema = JsonSchema::default()
        .title("Example")
        .description("Example schemas")
        .comment("Generated by Specta")
        .export_value(&types, specta_serde::Format)
        .unwrap();

    insta::assert_json_snapshot!(schema);
}

#[test]
fn exports_field_docs_as_descriptions() {
    let types = Types::default().register::<DocumentedFields>();
    let schema = JsonSchema::default()
        .export_value(&types, specta_serde::Format)
        .unwrap();

    insta::assert_json_snapshot!(schema);
}

#[test]
fn exports_structs_collections_and_enums() {
    let types = Types::default()
        .register::<User>()
        .register::<Collections>()
        .register::<Status>()
        .register::<Message>();
    let schema = JsonSchema::default()
        .export_value(&types, specta_serde::Format)
        .unwrap();

    insta::assert_json_snapshot!(schema);
}

#[test]
fn exports_draft7_tuple_syntax() {
    let types = Types::default().register::<Collections>();
    let schema = JsonSchema::default()
        .schema_version(SchemaVersion::Draft7)
        .export_value(&types, specta_serde::Format)
        .unwrap();

    insta::assert_json_snapshot!(schema);
}

#[test]
fn exports_generic_instantiations() {
    let types = Types::default()
        .register::<UsesGenerics>()
        .register::<UsesNestedGenerics>();
    let schema = JsonSchema::default()
        .export_value(&types, specta_serde::Format)
        .unwrap();

    insta::assert_json_snapshot!(schema);
}

#[test]
fn directly_registered_generic_root_is_materialized() {
    let schema = JsonSchema::default()
        .export_value(
            &Types::default().register::<Wrapper<String>>(),
            specta_serde::Format,
        )
        .unwrap();

    assert_eq!(
        schema["$defs"]["Wrapper<String>"]["properties"]["value"]["type"],
        "string"
    );
    assert!(schema["$defs"].get("Wrapper").is_none());
}

#[test]
fn exports_typed_anonymous_root_schema() {
    let schema = JsonSchema::default()
        .export_type_value::<Vec<User>>(IdentityFormat)
        .unwrap();

    assert_eq!(schema["type"], "array");
    assert_eq!(schema["items"]["$ref"], "#/$defs/User");
    assert!(schema["$defs"].get("User").is_some());
}

#[test]
fn formats_typed_anonymous_roots_against_the_mapped_graph() {
    let schema = JsonSchema::default()
        .export_type_value::<Vec<PhaseSplitRoot>>(specta_serde::PhasesFormat)
        .unwrap();

    assert_eq!(schema["items"]["$ref"], "#/$defs/PhaseSplitRoot_Serialize");
    assert!(schema["$defs"].get("PhaseSplitRoot_Serialize").is_some());
    assert!(schema["$defs"].get("PhaseSplitRoot_Deserialize").is_some());
}

#[test]
fn draft7_typed_named_root_wraps_ref_and_keeps_document_metadata() {
    let schema = JsonSchema::default()
        .schema_version(SchemaVersion::Draft7)
        .id("https://example.com/user.schema.json")
        .title("Configured user")
        .description("Configured root description")
        .comment("Generated root")
        .export_type_value::<User>(IdentityFormat)
        .unwrap();

    assert!(schema.get("$ref").is_none());
    assert_eq!(schema["allOf"][0]["$ref"], "#/definitions/User");
    assert_eq!(schema["$id"], "https://example.com/user.schema.json");
    assert_eq!(schema["title"], "Configured user");
    assert_eq!(schema["description"], "Configured root description");
    assert_eq!(schema["$comment"], "Generated root");
}

#[test]
fn substitutes_generics_inside_nested_references() {
    let schema = JsonSchema::default()
        .export_value(
            &Types::default().register::<UsesGenericComposition>(),
            specta_serde::Format,
        )
        .unwrap();
    let reference = schema["$defs"]["GenericComposition<String>"]["properties"]["wrapped"]["$ref"]
        .as_str()
        .unwrap();
    assert_eq!(reference, "#/$defs/Wrapper%3CString%3E");
    assert_eq!(
        schema["$defs"]["Wrapper<String>"]["properties"]["value"]["type"],
        "string"
    );
}

#[test]
fn exports_newtype_struct_as_its_wire_value() {
    let schema = JsonSchema::default()
        .export_ref_value(
            &Types::default().register::<Newtype>(),
            specta_serde::Format,
            "Newtype",
        )
        .unwrap();

    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&serde_json::json!("value")));
    assert!(!validator.is_valid(&serde_json::json!(["value"])));
}

#[test]
fn preserves_unnamed_field_metadata() {
    let schema = JsonSchema::default()
        .export_value(
            &Types::default()
                .register::<AnnotatedNewtype>()
                .register::<AnnotatedTuple>(),
            IdentityFormat,
        )
        .unwrap();

    let newtype_description = schema["$defs"]["AnnotatedNewtype"]["description"]
        .as_str()
        .unwrap();
    assert!(newtype_description.contains("Annotated newtype container documentation."));
    assert!(newtype_description.contains("Newtype field documentation."));
    assert_eq!(schema["$defs"]["AnnotatedNewtype"]["deprecated"], true);
    assert!(
        schema["$defs"]["AnnotatedTuple"]["prefixItems"][0]["description"]
            .as_str()
            .is_some_and(|description| description.contains("First tuple field documentation."))
    );
    assert_eq!(
        schema["$defs"]["AnnotatedTuple"]["prefixItems"][0]["deprecated"],
        true
    );
    assert!(
        schema["$defs"]["AnnotatedTuple"]["prefixItems"][1]["description"]
            .as_str()
            .is_some_and(|description| description.contains("Second tuple field documentation."))
    );
}

#[test]
fn overlapping_untagged_variants_are_a_union() {
    let schema = JsonSchema::default()
        .export_ref_value(
            &Types::default().register::<OverlappingUntagged>(),
            specta_serde::Format,
            "OverlappingUntagged",
        )
        .unwrap();

    assert!(schema["$defs"]["OverlappingUntagged"]["anyOf"].is_array());
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&serde_json::json!(1)));
}

#[test]
fn rejects_object_map_keys() {
    let error = JsonSchema::default()
        .export_value(
            &Types::default().register::<UsesInvalidMapKey>(),
            specta_serde::Format,
        )
        .unwrap_err();

    assert!(matches!(
        error,
        specta_jsonschema::Error::InvalidMapKey { .. }
    ));
}

#[test]
fn constrains_fixed_width_integer_map_key_ranges() {
    let schema = JsonSchema::default()
        .export_ref_value(
            &Types::default().register::<FixedWidthIntegerMapKeys>(),
            specta_serde::Format,
            "FixedWidthIntegerMapKeys",
        )
        .unwrap();

    let validator = jsonschema::validator_for(&schema).unwrap();
    let mut valid = serde_json::json!({
        "signed": { "-128": "minimum", "127": "maximum" },
        "unsigned": { "0": "minimum", "255": "maximum" },
        "pointer_signed": {},
        "pointer_unsigned": {}
    });
    valid["pointer_signed"]
        .as_object_mut()
        .unwrap()
        .insert(isize::MIN.to_string(), serde_json::json!("minimum"));
    valid["pointer_unsigned"]
        .as_object_mut()
        .unwrap()
        .insert(usize::MAX.to_string(), serde_json::json!("maximum"));
    assert!(validator.is_valid(&valid));
    assert!(!validator.is_valid(&serde_json::json!({
        "signed": { "128": "out of range" },
        "unsigned": {},
        "pointer_signed": {},
        "pointer_unsigned": {}
    })));
    assert!(!validator.is_valid(&serde_json::json!({
        "signed": {},
        "unsigned": { "256": "out of range" },
        "pointer_signed": {},
        "pointer_unsigned": {}
    })));

    if let Some(out_of_range) = (usize::MAX as u128).checked_add(1) {
        let mut invalid = valid;
        invalid["pointer_unsigned"]
            .as_object_mut()
            .unwrap()
            .insert(out_of_range.to_string(), serde_json::json!("out of range"));
        assert!(!validator.is_valid(&invalid));
    }
}

#[test]
fn preserves_definition_names_and_escapes_json_pointers() {
    let schema = JsonSchema::default()
        .id("https://example.com/schema.json")
        .export_ref_value(
            &Types::default().register::<EscapedType>(),
            specta_serde::Format,
            "Escaped/Type~",
        )
        .unwrap();

    assert_eq!(schema["$id"], "https://example.com/schema.json");
    assert!(schema["$defs"].get("Escaped/Type~").is_some());
    assert_eq!(schema["$ref"], "#/$defs/Escaped~1Type~0");
}

#[test]
fn percent_encodes_definition_ref_fragments() {
    let schema = JsonSchema::default()
        .export_ref_value(
            &Types::default().register::<UriEscapedType>(),
            specta_serde::Format,
            "Escaped #/%雪~",
        )
        .unwrap();

    assert_eq!(schema["$ref"], "#/$defs/Escaped%20%23~1%25%E9%9B%AA~0");
}

#[test]
fn generic_keys_follow_declaration_order() {
    let schema = JsonSchema::default()
        .export_ref_value(
            &Types::default().register::<UsesReverseGenerics>(),
            specta_serde::Format,
            "UsesReverseGenerics",
        )
        .unwrap();

    assert!(
        schema["$defs"]
            .get("ReverseGenerics<String, u32>")
            .is_some()
    );
    assert_eq!(
        schema["$defs"]["UsesReverseGenerics"]["properties"]["value"]["$ref"],
        "#/$defs/ReverseGenerics%3CString%2C%20u32%3E"
    );
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&serde_json::json!({
        "value": { "z": "value", "a": 1 }
    })));
}

#[test]
fn rejects_expanding_recursive_generics() {
    let mut types = Types::default();
    NamedDataType::new("Grow", &mut types, |_, ndt| {
        let generic = Generic::new("T".into());
        ndt.generics = vec![GenericDefinition::new(
            "T".into(),
            Some(Primitive::str.into()),
        )]
        .into();
        let nested = List::new(generic.clone().into()).into();
        ndt.ty = Some(
            Struct::named()
                .field(
                    "next",
                    Field::new(ndt.reference(vec![(generic, nested)]).into()),
                )
                .build(),
        );
    });

    let error = JsonSchema::default()
        .export_value(&types, IdentityFormat)
        .unwrap_err();
    assert!(matches!(
        error,
        specta_jsonschema::Error::ExpandingRecursiveGeneric { .. }
    ));
}

#[test]
fn allows_finite_recursive_generic_cycles() {
    let mut types = Types::default();
    NamedDataType::new("Flip", &mut types, |_, ndt| {
        let a = GenericDefinition::new("A".into(), Some(Primitive::str.into()));
        let b = GenericDefinition::new("B".into(), Some(Primitive::u32.into()));
        ndt.generics = vec![a.clone(), b.clone()].into();
        ndt.ty = Some(
            Struct::named()
                .field(
                    "next",
                    Field::new(
                        ndt.reference(vec![
                            (a.reference(), DataType::Generic(b.reference())),
                            (b.reference(), DataType::Generic(a.reference())),
                        ])
                        .into(),
                    ),
                )
                .build(),
        );
    });

    let schema = JsonSchema::default()
        .export_value(&types, IdentityFormat)
        .unwrap();
    assert!(schema["$defs"].get("Flip<str, u32>").is_some());
    assert!(schema["$defs"].get("Flip<u32, str>").is_some());
    assert_eq!(
        schema["$defs"]["Flip<str, u32>"]["properties"]["next"]["$ref"],
        "#/$defs/Flip%3Cu32%2C%20str%3E"
    );
    assert_eq!(
        schema["$defs"]["Flip<u32, str>"]["properties"]["next"]["$ref"],
        "#/$defs/Flip%3Cstr%2C%20u32%3E"
    );
}

#[test]
fn resolves_dependent_defaults_after_explicit_generics() {
    let mut types = Types::default();
    let dependent = NamedDataType::new("Dependent", &mut types, |_, ndt| {
        let t = GenericDefinition::new("T".into(), Some(Primitive::str.into()));
        let u = GenericDefinition::new("U".into(), Some(DataType::Generic(t.reference())));
        ndt.generics = vec![t.clone(), u.clone()].into();
        ndt.ty = Some(
            Struct::named()
                .field("first", Field::new(DataType::Generic(t.reference())))
                .field("second", Field::new(DataType::Generic(u.reference())))
                .build(),
        );
    });
    let t = dependent.generics[0].reference();
    NamedDataType::new("UsesDependent", &mut types, move |_, ndt| {
        ndt.ty = Some(
            Struct::named()
                .field(
                    "value",
                    Field::new(dependent.reference(vec![(t, Primitive::i32.into())]).into()),
                )
                .build(),
        );
    });

    let schema = JsonSchema::default()
        .export_value(&types, IdentityFormat)
        .unwrap();
    assert_eq!(
        schema["$defs"]["Dependent<i32, i32>"]["properties"]["first"]["type"],
        "integer"
    );
    assert_eq!(
        schema["$defs"]["Dependent<i32, i32>"]["properties"]["second"]["type"],
        "integer"
    );
    assert!(schema["$defs"].get("Dependent<i32, str>").is_none());
}

#[test]
fn rejects_raw_newtype_enum_map_keys() {
    let error = JsonSchema::default()
        .export_value(
            &Types::default().register::<RawNewtypeEnumMap>(),
            IdentityFormat,
        )
        .unwrap_err();

    assert!(matches!(
        error,
        specta_jsonschema::Error::InvalidMapKey { .. }
    ));
}

#[test]
fn variant_untagged_map_keys_use_lexical_schemas() {
    let schema = JsonSchema::default()
        .export_ref_value(
            &Types::default().register::<MixedMapKeyMap>(),
            specta_serde::Format,
            "MixedMapKeyMap",
        )
        .unwrap();
    let property_names =
        &schema["$defs"]["MixedMapKeyMap"]["properties"]["values"]["propertyNames"];
    assert!(property_names["anyOf"].as_array().is_some_and(|branches| {
        branches.iter().any(|branch| branch["pattern"].is_string())
            && branches.iter().all(|branch| branch["type"] != "integer")
    }));

    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&serde_json::json!({
        "values": { "Unit": "unit", "255": "number" }
    })));
    assert!(!validator.is_valid(&serde_json::json!({
        "values": { "256": "out of range" }
    })));
    assert!(!validator.is_valid(&serde_json::json!({
        "values": { "not-a-number": "invalid" }
    })));
}

#[test]
fn empty_inline_enum_map_keys_reject_all_property_names() {
    let mut types = Types::default();
    NamedDataType::new("EmptyEnumMap", &mut types, |_, ndt| {
        ndt.ty = Some(SpectaMap::new(Enum::default().into(), Primitive::str.into()).into());
    });

    let schema = JsonSchema::default()
        .export_ref_value(&types, IdentityFormat, "EmptyEnumMap")
        .unwrap();
    assert_eq!(schema["$defs"]["EmptyEnumMap"]["propertyNames"], false);
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&serde_json::json!({})));
    assert!(!validator.is_valid(&serde_json::json!({ "anything": "value" })));
}

#[test]
fn draft201909_uses_array_items_for_tuples() {
    let schema = JsonSchema::default()
        .schema_version(SchemaVersion::Draft201909)
        .export_ref_value(
            &Types::default().register::<Collections>(),
            specta_serde::Format,
            "Collections",
        )
        .unwrap();

    let tuple = &schema["$defs"]["Collections"]["properties"]["tuple"];
    assert!(tuple["items"].is_array());
    assert_eq!(tuple["additionalItems"], false);
    assert!(tuple.get("prefixItems").is_none());
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&serde_json::json!({
        "tags": [], "tuple": ["value", 1], "map": {}
    })));
}

#[test]
fn map_keys_use_lexical_string_schemas() {
    let schema = JsonSchema::default()
        .export_ref_value(
            &Types::default().register::<LexicalMapKeys>(),
            specta_serde::Format,
            "LexicalMapKeys",
        )
        .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    assert!(validator.is_valid(&serde_json::json!({
        "numeric": { "42": "ok" }, "boolean": { "true": "ok" }
    })));
    assert!(!validator.is_valid(&serde_json::json!({
        "numeric": { "nope": "bad" }, "boolean": {}
    })));
    assert!(!validator.is_valid(&serde_json::json!({
        "numeric": {}, "boolean": { "yes": "bad" }
    })));
}

#[test]
fn identity_format_preserves_external_newtype_enum_tag() {
    let schema = JsonSchema::default()
        .export_ref_value(
            &Types::default().register::<RawNewtypeEnum>(),
            IdentityFormat,
            "RawNewtypeEnum",
        )
        .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    assert!(validator.is_valid(&serde_json::json!({ "Value": "text" })));
    assert!(!validator.is_valid(&serde_json::json!("text")));
}

#[test]
fn rewritten_other_variants_keep_their_wire_shape() {
    let schema = JsonSchema::default()
        .export_ref_value(
            &Types::default().register::<InternalOther>(),
            specta_serde::PhasesFormat,
            "InternalOther_Deserialize",
        )
        .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    assert!(validator.is_valid(&serde_json::json!({ "kind": "known", "value": 1 })));
    assert!(!validator.is_valid(&serde_json::json!({ "kind": "known" })));
    assert!(validator.is_valid(&serde_json::json!({ "kind": "unknown" })));
    assert!(!validator.is_valid(&serde_json::json!({
        "Other": { "kind": "unknown" }
    })));
}

#[test]
fn serde_other_exclusions_do_not_constrain_authored_untagged_variants() {
    fn string_literal(value: &'static str) -> DataType {
        let mut enm = Enum::default();
        enm.attributes
            .insert("specta_serde:enum_repr_rewritten", true);
        enm.variants.push((value.into(), Variant::unit()));
        enm.into()
    }

    let mut types = Types::default();
    NamedDataType::new("MixedUntaggedOther", &mut types, |_, ndt| {
        let mut enm = Enum::default();
        enm.attributes
            .insert("specta_serde:enum_repr_rewritten", true);
        enm.variants.push((
            "Known".into(),
            Variant::named()
                .field("kind", Field::new(string_literal("Known")))
                .field("value", Field::new(Primitive::u32.into()))
                .build(),
        ));
        enm.variants.push((
            "Raw".into(),
            Variant::named()
                .field("kind", Field::new(Primitive::str.into()))
                .build(),
        ));
        let mut attributes = Attributes::default();
        attributes.insert("specta_serde:variant_other", true);
        enm.variants.push((
            "Other".into(),
            Variant::named()
                .field("kind", Field::new(Primitive::str.into()))
                .attributes(attributes)
                .build(),
        ));
        ndt.ty = Some(enm.into());
    });
    let schema = JsonSchema::default()
        .export_ref_value(&types, IdentityFormat, "MixedUntaggedOther")
        .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    assert!(validator.is_valid(&serde_json::json!({ "kind": "Known", "value": 1 })));
    assert!(validator.is_valid(&serde_json::json!({ "kind": "Known" })));
}

#[test]
fn boolean_schemas_preserve_named_and_field_metadata() {
    let mut types = Types::default();
    NamedDataType::new("Impossible", &mut types, |_, ndt| {
        ndt.docs = "No values exist.".into();
        ndt.deprecated = Some(Deprecated::with_note("Use another type".into()));
        ndt.ty = Some(Enum::default().into());
    });
    NamedDataType::new("ImpossibleField", &mut types, |_, ndt| {
        let mut field = Field::new(Enum::default().into());
        field.docs = "Impossible field.".into();
        field.deprecated = Some(Deprecated::with_note("Remove this field".into()));
        ndt.ty = Some(Struct::named().field("value", field).build());
    });

    let schema = JsonSchema::default()
        .export_value(&types, IdentityFormat)
        .unwrap();
    let impossible = &schema["$defs"]["Impossible"];
    assert_eq!(impossible["allOf"][0], false);
    assert_eq!(impossible["title"], "Impossible");
    assert_eq!(impossible["deprecated"], true);
    assert!(
        impossible["description"]
            .as_str()
            .unwrap()
            .contains("No values")
    );
    let field = &schema["$defs"]["ImpossibleField"]["properties"]["value"];
    assert_eq!(field["allOf"][0], false);
    assert_eq!(field["deprecated"], true);
    assert!(
        field["description"]
            .as_str()
            .unwrap()
            .contains("Impossible field")
    );
}

#[test]
#[allow(deprecated)]
fn serde_named_enum_map_keys_keep_definition_metadata() {
    fn assert_schema(schema: serde_json::Value) {
        let property_names =
            &schema["$defs"]["DocumentedKeyMap"]["properties"]["values"]["propertyNames"];
        assert_eq!(property_names["$ref"], "#/$defs/DocumentedKey");
        assert_eq!(schema["$defs"]["DocumentedKey"]["deprecated"], true);
        assert!(
            schema["$defs"]["DocumentedKey"]["description"]
                .as_str()
                .unwrap()
                .contains("Named map-key documentation")
        );
    }

    let types = Types::default().register::<DocumentedKeyMap>();
    assert_schema(
        JsonSchema::default()
            .export_value(&types, specta_serde::Format)
            .unwrap(),
    );
    assert_schema(
        JsonSchema::default()
            .export_value(&types, specta_serde::PhasesFormat)
            .unwrap(),
    );
}

#[test]
fn skipped_single_field_tuple_struct_is_an_empty_sequence() {
    let mut types = Types::default();
    NamedDataType::new("SkippedSingleTuple", &mut types, |_, ndt| {
        ndt.ty = Some(Struct::unnamed().field(Field::default()).build());
    });
    let schema = JsonSchema::default()
        .export_ref_value(&types, IdentityFormat, "SkippedSingleTuple")
        .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    assert!(validator.is_valid(&serde_json::json!([])));
    assert!(!validator.is_valid(&serde_json::json!(null)));
    assert!(!validator.is_valid(&serde_json::json!([1])));
}

#[test]
fn rewritten_newtype_payloads_are_not_collapsed_by_nested_literals() {
    let schema = JsonSchema::default()
        .export_ref_value(
            &Types::default().register::<RewrittenOuterPayload>(),
            specta_serde::Format,
            "RewrittenOuterPayload",
        )
        .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    assert!(validator.is_valid(&serde_json::json!({ "Outer": "Outer" })));
    assert!(validator.is_valid(&serde_json::json!({ "Outer": "Other" })));
    assert!(!validator.is_valid(&serde_json::json!("Outer")));
}

#[test]
fn untagged_variant_metadata_is_preserved() {
    let schema = JsonSchema::default()
        .export_value(
            &Types::default().register::<DocumentedUntagged>(),
            IdentityFormat,
        )
        .unwrap();

    assert_eq!(
        schema["$defs"]["DocumentedUntagged"]["anyOf"][0]["description"],
        " Text branch."
    );
}

#[test]
fn can_allow_additional_struct_properties() {
    let schema = JsonSchema::default()
        .allow_additional_properties(true)
        .export_ref_value(
            &Types::default().register::<User>(),
            specta_serde::Format,
            "User",
        )
        .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&serde_json::json!({
        "id": 1, "name": "name", "email": null, "extra": true
    })));
}

#[test]
fn permissive_objects_allow_present_optional_flatten_fields() {
    let schema = JsonSchema::default()
        .allow_additional_properties(true)
        .export_ref_value(
            &Types::default().register::<OptionalFlattened>(),
            specta_serde::Format,
            "OptionalFlattened",
        )
        .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();

    assert!(validator.is_valid(&serde_json::json!({ "base": "base" })));
    assert!(validator.is_valid(&serde_json::json!({ "base": "base", "a": "extra" })));
}

#[test]
fn permissive_objects_allow_unknown_fields_in_intersection_fallbacks() {
    let mut types = Types::default();
    NamedDataType::new("FallbackIntersection", &mut types, |_, ndt| {
        ndt.ty = Some(DataType::Intersection(vec![
            Struct::named()
                .field("known", Field::new(Primitive::str.into()))
                .build(),
            DataType::Generic(Generic::new("T".into())),
        ]));
    });

    let schema = JsonSchema::default()
        .allow_additional_properties(true)
        .export_value(&types, IdentityFormat)
        .unwrap();
    let fallback = &schema["$defs"]["FallbackIntersection"];
    assert!(fallback["allOf"].is_array());
    assert!(fallback.get("unevaluatedProperties").is_none());

    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&serde_json::json!({
        "known": "value",
        "extra": true
    })));
}

#[test]
fn draft7_closes_finite_intersection_fallbacks() {
    let mut types = Types::default();
    NamedDataType::new("Draft7Fallback", &mut types, |_, ndt| {
        ndt.ty = Some(DataType::Intersection(vec![
            Struct::named()
                .field("known", Field::new(Primitive::str.into()))
                .build(),
            DataType::Generic(Generic::new("T".into())),
        ]));
    });

    let schema = JsonSchema::default()
        .schema_version(SchemaVersion::Draft7)
        .export_ref_value(&types, IdentityFormat, "Draft7Fallback")
        .unwrap();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&serde_json::json!({ "known": "value" })));
    assert!(!validator.is_valid(&serde_json::json!({
        "known": "value",
        "extra": true
    })));
}

#[test]
fn draft7_rejects_dynamic_intersection_fallbacks_that_cannot_be_closed() {
    let mut types = Types::default();
    NamedDataType::new("DynamicDraft7Fallback", &mut types, |_, ndt| {
        ndt.ty = Some(DataType::Intersection(vec![
            Struct::named()
                .field("known", Field::new(Primitive::str.into()))
                .build(),
            SpectaMap::new(Primitive::str.into(), Primitive::str.into()).into(),
        ]));
    });

    let error = JsonSchema::default()
        .schema_version(SchemaVersion::Draft7)
        .export_value(&types, IdentityFormat)
        .unwrap_err();
    assert!(matches!(
        error,
        specta_jsonschema::Error::UnsupportedClosedIntersection { .. }
    ));
}

#[test]
fn fixed_width_integer_bounds_are_exported() {
    let schema = JsonSchema::default()
        .export_value(
            &Types::default()
                .register::<Primitives>()
                .register::<PointerPrimitives>(),
            specta_serde::Format,
        )
        .unwrap();
    let properties = &schema["$defs"]["Primitives"]["properties"];
    assert_eq!(properties["i32_field"]["minimum"], i32::MIN);
    assert_eq!(properties["i32_field"]["maximum"], i32::MAX);
    assert_eq!(properties["u64_field"]["maximum"], u64::MAX);
    let pointer = &schema["$defs"]["PointerPrimitives"]["properties"];
    assert_eq!(pointer["isize_field"]["minimum"], isize::MIN);
    assert_eq!(pointer["isize_field"]["maximum"], isize::MAX);
    assert_eq!(pointer["usize_field"]["maximum"], usize::MAX);
}

#[test]
fn rejects_a_missing_root_definition() {
    let error = JsonSchema::default()
        .export_ref_value(&Types::default(), specta_serde::Format, "Missing")
        .unwrap_err();

    assert!(matches!(
        error,
        specta_jsonschema::Error::MissingDefinition { definition }
            if definition == "Missing"
    ));
}

#[test]
fn exports_root_ref() {
    let types = Types::default().register::<User>();
    let schema = JsonSchema::default()
        .export_ref_value(&types, specta_serde::Format, "User")
        .unwrap();

    assert_eq!(schema["$ref"], "#/$defs/User");
    insta::assert_json_snapshot!(schema);
}

#[test]
fn exports_map_key_constraints() {
    let types = Types::default().register::<MapKeys>();
    let schema = JsonSchema::default()
        .export_value(&types, specta_serde::Format)
        .unwrap();

    insta::assert_json_snapshot!(schema);
}

#[test]
fn exports_untagged_enums() {
    let types = Types::default().register::<UntaggedValue>();
    let schema = JsonSchema::default()
        .export_ref_value(&types, specta_serde::Format, "UntaggedValue")
        .unwrap();

    insta::assert_json_snapshot!(schema);

    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&serde_json::json!("hello")));
    assert!(validator.is_valid(&serde_json::json!({ "id": 1 })));
    assert!(!validator.is_valid(&serde_json::json!({ "Object": { "id": 1 } })));
}

#[test]
fn exports_flattened_structs_without_conflicting_additional_properties() {
    let types = Types::default().register::<Flattened>();
    let schema = JsonSchema::default()
        .export_ref_value(&types, specta_serde::Format, "Flattened")
        .unwrap();

    let flattened = &schema["$defs"]["Flattened"];
    assert_eq!(flattened["additionalProperties"], false);
    assert!(flattened.get("allOf").is_none());
    insta::assert_json_snapshot!(schema);

    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&serde_json::json!({ "a": "a", "b": "b" })));
    assert!(!validator.is_valid(&serde_json::json!({ "a": "a", "b": "b", "c": "c" })));
}

#[test]
fn draft7_flattens_object_intersections() {
    let schema = JsonSchema::default()
        .schema_version(SchemaVersion::Draft7)
        .export_ref_value(
            &Types::default().register::<Flattened>(),
            specta_serde::Format,
            "Flattened",
        )
        .unwrap();

    let flattened = &schema["definitions"]["Flattened"];
    assert_eq!(flattened["additionalProperties"], false);
    assert!(flattened.get("allOf").is_none());
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&serde_json::json!({ "a": "a", "b": "b" })));
    assert!(!validator.is_valid(&serde_json::json!({ "a": "a", "b": "b", "c": true })));
}

#[test]
fn draft7_wraps_documented_refs_for_annotation_siblings() {
    let schema = JsonSchema::default()
        .schema_version(SchemaVersion::Draft7)
        .export_value(
            &Types::default().register::<DocumentedReference>(),
            specta_serde::Format,
        )
        .unwrap();

    let user = &schema["definitions"]["DocumentedReference"]["properties"]["user"];
    assert!(user.get("$ref").is_none());
    assert_eq!(user["allOf"][0]["$ref"], "#/definitions/User");
    assert_eq!(user["description"], " The referenced user.");
}

#[test]
fn rejects_duplicate_definition_names() {
    let mut types = Types::default();
    NamedDataType::new("Duplicate", &mut types, |_, ndt| {
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("Duplicate", &mut types, |_, ndt| {
        ndt.ty = Some(Primitive::i32.into());
    });

    let error = JsonSchema::default()
        .export_value(&types, specta_serde::Format)
        .unwrap_err();
    assert!(matches!(
        error,
        specta_jsonschema::Error::DuplicateDefinitionName { .. }
    ));
}

#[test]
fn rejects_duplicate_definition_names_from_same_callsite() {
    let mut types = Types::default();
    for ty in [DataType::Primitive(Primitive::str), Primitive::i32.into()] {
        NamedDataType::new("SameCallsite", &mut types, |_, ndt| {
            ndt.ty = Some(ty);
        });
    }

    let error = JsonSchema::default()
        .export_value(&types, IdentityFormat)
        .unwrap_err();
    assert!(matches!(
        error,
        specta_jsonschema::Error::DuplicateDefinitionName { .. }
    ));
}

#[test]
fn applies_serde_format() {
    let types = Types::default()
        .register::<SerdeUser>()
        .register::<SerdeMessage>();
    let schema = JsonSchema::default()
        .export_value(&types, specta_serde::Format)
        .unwrap();

    insta::assert_json_snapshot!(schema);
}

#[test]
fn export_to_writes_single_file() {
    let types = Types::default().register::<User>();
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join(format!("jsonschema-test-{}.json", std::process::id()));

    JsonSchema::default()
        .export_to(&path, &types, specta_serde::Format)
        .unwrap();

    let schema = fs::read_to_string(&path).unwrap();
    fs::remove_file(&path).unwrap();
    assert!(schema.contains("\"$schema\""));
    assert!(schema.contains("\"User\""));
}
