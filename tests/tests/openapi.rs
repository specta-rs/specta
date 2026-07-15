use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use specta::{
    Type, Types,
    datatype::{DataType, NamedDataType, Primitive},
};
use specta_openapi::{OpenApi, OutputFormat, SchemaMode};

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase")]
struct ApiModel<T> {
    /// Stable identifier.
    id: u64,
    value: T,
    optional_value: Option<String>,
    fixed: [u8; 2],
    tuple: (String, bool),
    labels: HashMap<String, i32>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind", content = "data")]
enum ApiEvent {
    Started,
    Progress { percent: f32 },
    Message(String),
}

#[derive(Type)]
#[specta(collect = false)]
struct UsesGenerics {
    text: ApiModel<String>,
    number: ApiModel<u32>,
    event: ApiEvent,
}

#[derive(Type)]
#[specta(collect = false)]
struct EventAlias(ApiEvent);

#[derive(Type, PartialEq, Eq, std::hash::Hash)]
#[specta(collect = false)]
struct StringNewtype(String);

#[derive(Type)]
#[specta(collect = false)]
struct ReferencedField {
    /// Event payload documentation.
    #[deprecated]
    event: ApiEvent,
}

#[derive(Type)]
#[specta(collect = false)]
struct StrictTuple((String, bool));

#[derive(Type)]
#[specta(collect = false)]
struct StrictMap(HashMap<char, String>);

#[derive(Type)]
#[specta(collect = false)]
struct StringMap(HashMap<String, String>);

#[derive(Type)]
#[specta(collect = false)]
struct NamedStringMap(HashMap<StringNewtype, String>);

#[derive(Type, PartialEq, Eq, std::hash::Hash)]
#[specta(collect = false)]
enum EnumKey {
    A,
    B,
}

#[derive(Type)]
#[specta(collect = false)]
struct EnumMap(HashMap<EnumKey, String>);

#[derive(Type)]
#[specta(collect = false)]
struct StrictUnit;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenA {
    a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenB {
    b: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct StrictFlattened {
    #[serde(flatten)]
    a: FlattenA,
    #[serde(flatten)]
    b: FlattenB,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum OverlappingUntagged {
    First(u32),
    Second(u32),
}

#[test]
fn openapi_exports_full_type_corpus() {
    let (types, _) = crate::types();
    let output = OpenApi::default()
        .title("Specta corpus")
        .version("1.0.0")
        .schema_mode(SchemaMode::Compatible)
        .export(&types, specta_serde::Format)
        .expect("the shared type corpus should be representable in OpenAPI");

    let document: openapiv3::OpenAPI =
        serde_json::from_str(&output).expect("output should be a valid typed OpenAPI document");
    assert_eq!(document.openapi, "3.0.3");
    assert!(
        document
            .components
            .expect("components should exist")
            .schemas
            .len()
            > 20
    );
}

#[test]
fn openapi_exports_serde_phases() {
    let (mut types, _) = crate::types_phased();
    let (unified, _) = crate::types();
    types.extend(&unified);

    let output = OpenApi::default()
        .schema_mode(SchemaMode::Compatible)
        .export(&types, specta_serde::PhasesFormat)
        .expect("serialize and deserialize phase types should be representable in OpenAPI");
    let document: openapiv3::OpenAPI =
        serde_json::from_str(&output).expect("phase output should be valid OpenAPI");
    let schemas = &document
        .components
        .expect("components should exist")
        .schemas;
    assert!(schemas.keys().any(|name| name.contains("Serialize")));
    assert!(schemas.keys().any(|name| name.contains("Deserialize")));
}

#[test]
fn openapi_preserves_shapes_metadata_and_generics() {
    let types = Types::default()
        .register::<UsesGenerics>()
        .register::<EventAlias>()
        .register::<StringNewtype>()
        .register::<ReferencedField>()
        .register::<OverlappingUntagged>();
    let document = OpenApi::default()
        .schema_mode(SchemaMode::Compatible)
        .export_document(&types, specta_serde::Format)
        .expect("representative shapes should export");
    let value = serde_json::to_value(document).expect("document should serialize");
    let schemas = &value["components"]["schemas"];

    assert_eq!(
        schemas["ApiModel_String"]["properties"]["id"]["type"],
        "integer"
    );
    assert_eq!(
        schemas["ApiModel_String"]["properties"]["id"]["description"]
            .as_str()
            .map(str::trim),
        Some("Stable identifier.")
    );
    assert_eq!(
        schemas["ApiModel_String"]["properties"]["optionalValue"]["nullable"],
        true
    );
    assert_eq!(
        schemas["ApiModel_String"]["properties"]["fixed"]["minItems"],
        2
    );
    assert!(schemas.get("ApiModel_String").is_some());
    assert!(schemas.get("ApiModel_u32").is_some());
    assert!(schemas.get("ApiModel").is_none());
    assert_eq!(
        schemas["ApiEvent"]["oneOf"].as_array().map(Vec::len),
        Some(3)
    );
    assert_eq!(schemas["StringNewtype"]["type"], "string");
    assert_eq!(
        schemas["EventAlias"]["allOf"][0]["$ref"],
        "#/components/schemas/ApiEvent"
    );
    let referenced = &schemas["ReferencedField"]["properties"]["event"];
    assert_eq!(
        referenced["allOf"][0]["$ref"],
        "#/components/schemas/ApiEvent"
    );
    assert_eq!(
        referenced["description"].as_str().map(str::trim),
        Some("Event payload documentation.")
    );
    assert_eq!(referenced["deprecated"], true);
    assert_eq!(
        schemas["OverlappingUntagged"]["anyOf"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    insta::assert_snapshot!(
        "openapi-representative-shapes",
        serde_json::to_string_pretty(&value).expect("snapshot value should serialize")
    );
}

#[test]
fn openapi_strict_mode_rejects_lossy_openapi_3_shapes() {
    OpenApi::default()
        .export(
            &Types::default().register::<StringMap>(),
            specta_serde::Format,
        )
        .expect("ordinary string-key maps are exactly representable");
    OpenApi::default()
        .export(
            &Types::default().register::<NamedStringMap>(),
            specta_serde::Format,
        )
        .expect("transparent string-newtype map keys are exactly representable");

    let error = OpenApi::default()
        .schema_mode(SchemaMode::Strict)
        .export(
            &Types::default().register::<StrictTuple>(),
            specta_serde::Format,
        )
        .expect_err("OpenAPI 3.0 cannot represent positional tuples exactly");
    assert!(
        error
            .to_string()
            .contains("heterogeneous positional tuples")
    );

    let compatible = OpenApi::default()
        .schema_mode(SchemaMode::Compatible)
        .export_document(
            &Types::default().register::<StrictTuple>(),
            specta_serde::Format,
        )
        .expect("compatible mode should preserve tuple detail in an extension");
    let value = serde_json::to_value(compatible).unwrap();
    assert_eq!(
        value["components"]["schemas"]["StrictTuple"]["x-specta-prefix-items"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );

    let map_error = OpenApi::default()
        .schema_mode(SchemaMode::Strict)
        .export(
            &Types::default().register::<StrictMap>(),
            specta_serde::Format,
        )
        .expect_err("OpenAPI 3.0 cannot represent propertyNames");
    assert!(map_error.to_string().contains("constrained map keys"));

    let enum_map = OpenApi::default()
        .schema_mode(SchemaMode::Compatible)
        .export_document(
            &Types::default().register::<EnumMap>(),
            specta_serde::Format,
        )
        .expect("compatible mode should preserve enum-key constraints");
    let enum_map = serde_json::to_value(enum_map).unwrap();
    assert_eq!(
        enum_map["components"]["schemas"]["EnumMap"]["x-specta-property-names"]["$ref"],
        "#/components/schemas/EnumKey"
    );

    let null_error = OpenApi::default()
        .export(
            &Types::default().register::<StrictUnit>(),
            specta_serde::Format,
        )
        .expect_err("OpenAPI 3.0 cannot represent a null-only type");
    assert!(null_error.to_string().contains("null-only types"));

    let flattened_error = OpenApi::default()
        .export(
            &Types::default().register::<StrictFlattened>(),
            specta_serde::Format,
        )
        .expect_err("OpenAPI 3.0 cannot close flattened intersections exactly");
    assert!(
        flattened_error
            .to_string()
            .contains("closed flattened intersections")
    );
}

#[test]
fn openapi_rejects_component_name_collisions() {
    let mut types = Types::default();
    NamedDataType::new("A-B", &mut types, |_, ndt| {
        ndt.ty = Some(DataType::Primitive(Primitive::str));
    });
    NamedDataType::new("A_B", &mut types, |_, ndt| {
        ndt.ty = Some(DataType::Primitive(Primitive::bool));
    });

    let error = OpenApi::default()
        .export(&types, specta_serde::Format)
        .expect_err("sanitized component names must not overwrite each other");
    assert!(error.to_string().contains("definition name collision"));
}

#[test]
fn openapi_supports_yaml_export_to_and_document_merging() {
    let types = Types::default().register::<UsesGenerics>();
    let exporter = OpenApi::default()
        .schema_mode(SchemaMode::Compatible)
        .output_format(OutputFormat::Yaml);
    let yaml = exporter
        .export(&types, specta_serde::Format)
        .expect("YAML export should succeed");
    let parsed: openapiv3::OpenAPI =
        serde_yaml::from_str(&yaml).expect("YAML should be a valid OpenAPI document");
    assert_eq!(parsed.openapi, "3.0.3");

    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("openapi")
        .join("document.yaml");
    exporter
        .export_to(&path, &types, specta_serde::Format)
        .expect("export_to should create parent directories");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), yaml);
    std::fs::remove_file(path).unwrap();

    let mut document = openapiv3::OpenAPI::default();
    exporter
        .add_to_document(&mut document, &types, specta_serde::Format)
        .expect("components should merge into an empty document");
    assert!(
        exporter
            .add_to_document(&mut document, &types, specta_serde::Format)
            .is_err(),
        "duplicate component names should never be overwritten"
    );
}
