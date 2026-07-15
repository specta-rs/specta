use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use specta::{
    Type, Types,
    datatype::{DataType, Field, NamedDataType, Primitive, Struct},
};
use specta_openapi::{OpenApi, Operation, OutputFormat, SchemaMode};

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
struct RootWrapper<T> {
    value: T,
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
struct StrictHomogeneousTuple((u8, u8));

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

#[derive(Type)]
#[specta(collect = false)]
struct StrictOptionalPrimitive {
    value: Option<String>,
    nested: Option<Option<String>>,
}

#[derive(Type)]
#[specta(collect = false)]
struct WideIntegers {
    signed: i64,
    unsigned: u64,
}

#[derive(Type)]
#[specta(collect = false)]
struct StrictOptionalReference {
    value: Option<StrictUnit>,
}

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
fn openapi_materializes_directly_registered_generic_root() {
    let document = OpenApi::default()
        .export_document(
            &Types::default().register::<RootWrapper<String>>(),
            specta_serde::Format,
        )
        .expect("a directly registered concrete generic should export");
    let value = serde_json::to_value(document).unwrap();

    assert_eq!(
        value["components"]["schemas"]["RootWrapper_String"]["properties"]["value"]["type"],
        "string"
    );
    assert!(value["components"]["schemas"].get("RootWrapper").is_none());
}

#[test]
fn openapi_strict_mode_rejects_lossy_openapi_3_shapes() {
    let optional = OpenApi::default()
        .export_document(
            &Types::default().register::<StrictOptionalPrimitive>(),
            specta_serde::Format,
        )
        .expect("strict mode should represent nullable primitive fields exactly");
    let optional = serde_json::to_value(optional).unwrap();
    assert_eq!(
        optional["components"]["schemas"]["StrictOptionalPrimitive"]["properties"]["value"]["type"],
        "string"
    );
    assert_eq!(
        optional["components"]["schemas"]["StrictOptionalPrimitive"]["properties"]["value"]["nullable"],
        true
    );
    assert_eq!(
        optional["components"]["schemas"]["StrictOptionalPrimitive"]["properties"]["nested"]["type"],
        "string"
    );
    assert_eq!(
        optional["components"]["schemas"]["StrictOptionalPrimitive"]["properties"]["nested"]["nullable"],
        true
    );

    let homogeneous = OpenApi::default()
        .export_document(
            &Types::default().register::<StrictHomogeneousTuple>(),
            specta_serde::Format,
        )
        .expect("strict mode should represent homogeneous fixed tuples exactly");
    let homogeneous = serde_json::to_value(homogeneous).unwrap();
    let homogeneous = &homogeneous["components"]["schemas"]["StrictHomogeneousTuple"];
    assert_eq!(homogeneous["items"]["type"], "integer");
    assert_eq!(homogeneous["minItems"], 2);
    assert_eq!(homogeneous["maxItems"], 2);

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

    let compatible_unit = OpenApi::default()
        .schema_mode(SchemaMode::Compatible)
        .export_document(
            &Types::default().register::<StrictUnit>(),
            specta_serde::Format,
        )
        .expect("compatible mode should emit a legal nullable approximation");
    let compatible_unit = serde_json::to_value(compatible_unit).unwrap();
    let compatible_unit = &compatible_unit["components"]["schemas"]["StrictUnit"];
    assert_eq!(compatible_unit["type"], "object");
    assert_eq!(compatible_unit["nullable"], true);
    assert_eq!(compatible_unit["maxProperties"], 0);
    assert_eq!(compatible_unit["additionalProperties"], false);
    assert_eq!(compatible_unit["x-specta-type"], "null");

    let flattened = OpenApi::default()
        .export_document(
            &Types::default().register::<StrictFlattened>(),
            specta_serde::Format,
        )
        .expect("mergeable flattened objects are represented exactly");
    let flattened = serde_json::to_value(flattened).unwrap();
    let flattened = &flattened["components"]["schemas"]["StrictFlattened"];
    assert_eq!(flattened["additionalProperties"], false);
    assert!(flattened["properties"].get("a").is_some());
    assert!(flattened["properties"].get("b").is_some());

    // `Option<T>` over a named type is the most common shape strict mode rejects: OpenAPI 3.0
    // cannot mark a `$ref` nullable.
    let reference_error = OpenApi::default()
        .export_document(
            &Types::default().register::<StrictOptionalReference>(),
            specta_serde::Format,
        )
        .expect_err("OpenAPI 3.0 cannot represent a nullable reference exactly");
    assert!(
        reference_error
            .to_string()
            .contains("nullable references or composed schemas")
    );
    let compatible_reference = OpenApi::default()
        .schema_mode(SchemaMode::Compatible)
        .export_document(
            &Types::default().register::<StrictOptionalReference>(),
            specta_serde::Format,
        )
        .expect("compatible mode should approximate a nullable reference");
    let compatible_reference = serde_json::to_value(compatible_reference).unwrap();
    assert!(
        compatible_reference["components"]["schemas"]["StrictOptionalReference"]["properties"]
            ["value"]
            .get("x-specta-nullable")
            .is_some()
    );

    // Strict is the default, so every rejection names the way out.
    for error in [&error, &map_error, &null_error, &reference_error] {
        assert!(
            error.to_string().contains("SchemaMode::Compatible"),
            "strict-mode error should point at the escape hatch: {error}"
        );
    }
}

#[test]
fn openapi_preserves_exact_wide_integer_bounds_in_extensions() {
    let types = Types::default().register::<WideIntegers>();
    let error = OpenApi::default()
        .schema_mode(SchemaMode::Strict)
        .export_document(&types, specta_serde::Format)
        .expect_err("OpenAPI 3.0 cannot represent every 64-bit integer bound exactly");
    assert!(error.to_string().contains("exact 64-bit integer bounds"));

    let document = OpenApi::default()
        .schema_mode(SchemaMode::Compatible)
        .export_document(&types, specta_serde::Format)
        .expect("compatible mode should retain exact bounds in extensions");
    let document = serde_json::to_value(document).unwrap();
    let properties = &document["components"]["schemas"]["WideIntegers"]["properties"];

    assert_eq!(properties["signed"]["maximum"], i64::MAX);
    assert_eq!(properties["signed"]["minimum"], i64::MIN);
    assert!(properties["signed"].get("x-specta-maximum").is_none());
    assert!(properties["unsigned"].get("maximum").is_none());
    assert_eq!(properties["unsigned"]["x-specta-maximum"], u64::MAX);
    assert_eq!(properties["unsigned"]["minimum"], 0.0);
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
fn openapi_sanitizes_escaped_definition_names_and_rewrites_refs() {
    let mut types = Types::default();
    let escaped = NamedDataType::new("A/B~%#é", &mut types, |_, ndt| {
        ndt.ty = Some(DataType::Primitive(Primitive::str));
    });
    NamedDataType::new("EscapedHolder", &mut types, |_, ndt| {
        ndt.ty = Some(
            Struct::named()
                .field("value", Field::new(escaped.reference(vec![]).into()))
                .build(),
        );
    });

    let document = OpenApi::default()
        .export_document(&types, specta_serde::Format)
        .unwrap();
    let document = serde_json::to_value(document).unwrap();
    let schemas = &document["components"]["schemas"];
    assert!(schemas.get("A_B").is_some());
    assert_eq!(
        schemas["EscapedHolder"]["properties"]["value"]["$ref"],
        "#/components/schemas/A_B"
    );
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

#[derive(Type)]
#[specta(collect = false)]
struct Recipe {
    slug: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct NewRecipe {
    slug: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct ApiError {
    message: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct Wrapper<T> {
    value: T,
}

mod other {
    use specta::Type;

    // Same name as the outer `Recipe`, which is what makes the exporter disambiguate both by
    // module path.
    #[derive(Type)]
    #[specta(collect = false)]
    pub struct Recipe {
        pub id: u32,
    }
}

fn recipe_types() -> Types {
    Types::default()
        .register::<Recipe>()
        .register::<NewRecipe>()
        .register::<ApiError>()
}

#[test]
fn openapi_exports_operations_into_paths() {
    let document = OpenApi::default()
        .operation(
            Operation::get("/recipes/{slug}")
                .summary("Fetch one recipe")
                .operation_id("getRecipe")
                .tag("recipes")
                .path_param::<String>("slug")
                .response::<Recipe>(200, "The recipe")
                .response::<ApiError>(404, "No such recipe"),
        )
        .operation(
            Operation::post("/recipes")
                .request_body::<NewRecipe>()
                .response::<Recipe>(201, "The created recipe")
                .empty_response(204, "Nothing to do"),
        )
        .export_document(&recipe_types(), specta_serde::Format)
        .expect("operations should export");
    let value = serde_json::to_value(&document).unwrap();

    let get = &value["paths"]["/recipes/{slug}"]["get"];
    assert_eq!(get["summary"], "Fetch one recipe");
    assert_eq!(get["operationId"], "getRecipe");
    assert_eq!(get["tags"][0], "recipes");
    assert_eq!(get["parameters"][0]["name"], "slug");
    assert_eq!(get["parameters"][0]["in"], "path");
    assert_eq!(get["parameters"][0]["required"], true);
    assert_eq!(
        get["responses"]["200"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/Recipe"
    );
    assert_eq!(
        get["responses"]["404"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/ApiError"
    );

    let post = &value["paths"]["/recipes"]["post"];
    assert_eq!(
        post["requestBody"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/NewRecipe"
    );
    assert_eq!(
        post["responses"]["201"]["content"]["application/json"]["schema"]["$ref"],
        "#/components/schemas/Recipe"
    );
    // A response with no body carries no content at all.
    assert!(post["responses"]["204"].get("content").is_none());

    // Every `$ref` an operation emits resolves to a component that was exported.
    let schemas = document.components.as_ref().unwrap();
    for reference in collect_refs(&value["paths"]) {
        let name = reference.trim_start_matches("#/components/schemas/");
        assert!(
            schemas.schemas.contains_key(name),
            "operation $ref {reference:?} does not resolve"
        );
    }
}

fn collect_refs(value: &serde_json::Value) -> Vec<String> {
    let mut refs = Vec::new();
    fn walk(value: &serde_json::Value, refs: &mut Vec<String>) {
        match value {
            serde_json::Value::Object(map) => {
                for (key, value) in map {
                    match (key.as_str(), value.as_str()) {
                        ("$ref", Some(reference)) => refs.push(reference.to_string()),
                        _ => walk(value, refs),
                    }
                }
            }
            serde_json::Value::Array(items) => items.iter().for_each(|item| walk(item, refs)),
            _ => {}
        }
    }
    walk(value, &mut refs);
    refs
}

/// Operation names are resolved by reproducing the JSON Schema exporter's naming, which
/// disambiguates by module path only when two definitions share a name. This pins the reproduction
/// against the real thing for both the plain and the disambiguated case.
#[test]
fn openapi_resolves_operations_against_disambiguated_component_names() {
    let types = Types::default()
        .register::<Recipe>()
        .register::<other::Recipe>();

    let document = OpenApi::default()
        .operation(Operation::get("/a").response::<Recipe>(200, "outer"))
        .operation(Operation::get("/b").response::<other::Recipe>(200, "inner"))
        .export_document(&types, specta_serde::Format)
        .expect("colliding names should still resolve");
    let value = serde_json::to_value(&document).unwrap();

    let schemas = document.components.as_ref().unwrap();
    let refs = collect_refs(&value["paths"]);
    assert_eq!(refs.len(), 2);
    for reference in &refs {
        let name = reference.trim_start_matches("#/components/schemas/");
        assert!(
            schemas.schemas.contains_key(name),
            "disambiguated $ref {reference:?} does not resolve"
        );
    }
    // Both were disambiguated, so the two operations cannot have landed on one component.
    assert_ne!(refs[0], refs[1]);
}

#[test]
fn openapi_rejects_operations_it_cannot_resolve() {
    // A type absent from the exported collection.
    let unregistered = OpenApi::default()
        .operation(Operation::get("/a").response::<Recipe>(200, "ok"))
        .export_document(
            &Types::default().register::<ApiError>(),
            specta_serde::Format,
        )
        .expect_err("an unexported type must not produce a dangling $ref");
    assert!(unregistered.to_string().contains("Recipe"));

    // The same method and path twice.
    let duplicate = OpenApi::default()
        .operation(Operation::get("/a").response::<Recipe>(200, "ok"))
        .operation(Operation::get("/a").response::<Recipe>(200, "ok"))
        .export_document(&recipe_types(), specta_serde::Format)
        .expect_err("one method and path describes one operation");
    assert!(duplicate.to_string().contains("duplicate operation"));

    // An operation that says nothing about what it returns.
    let empty = OpenApi::default()
        .operation(Operation::get("/a"))
        .export_document(&recipe_types(), specta_serde::Format)
        .expect_err("an operation must declare at least one response");
    assert!(empty.to_string().contains("declares no responses"));
}

/// Generic instantiations are exported one component per instantiation, with the arguments folded
/// into the name and then sanitised. Operations resolve to those components by asking the exporter
/// rather than reproducing the naming, so an `ApiResponse<T>`-shaped API is describable.
#[test]
fn openapi_resolves_generic_operation_types() {
    let types = Types::default()
        .register::<Wrapper<String>>()
        .register::<Wrapper<Recipe>>();

    let document = OpenApi::default()
        .operation(Operation::get("/text").response::<Wrapper<String>>(200, "wrapped text"))
        .operation(Operation::get("/recipe").response::<Wrapper<Recipe>>(200, "wrapped recipe"))
        .export_document(&types, specta_serde::Format)
        .expect("generic instantiations should resolve");
    let value = serde_json::to_value(&document).unwrap();

    // Each instantiation is its own component, so the two operations must not collapse onto one.
    let text =
        value["paths"]["/text"]["get"]["responses"]["200"]["content"]["application/json"]["schema"]
            ["$ref"]
            .as_str()
            .unwrap()
            .to_string();
    let recipe = value["paths"]["/recipe"]["get"]["responses"]["200"]["content"]
        ["application/json"]["schema"]["$ref"]
        .as_str()
        .unwrap()
        .to_string();
    assert_ne!(text, recipe);

    let schemas = document.components.as_ref().unwrap();
    for reference in [&text, &recipe] {
        let name = reference.trim_start_matches("#/components/schemas/");
        assert!(
            schemas.schemas.contains_key(name),
            "generic $ref {reference:?} does not resolve"
        );
    }
}

/// The probe used to ask the exporter for component names must not appear in the output, nor change
/// what the document would otherwise contain.
#[test]
fn openapi_resolution_probe_does_not_leak_into_the_document() {
    let types = recipe_types();
    let with_operations = OpenApi::default()
        .operation(Operation::get("/recipes/{slug}").response::<Recipe>(200, "The recipe"))
        .export_document(&types, specta_serde::Format)
        .expect("operations should export");
    let without_operations = OpenApi::default()
        .export_document(&types, specta_serde::Format)
        .expect("types should export");

    let components = with_operations.components.as_ref().unwrap();
    assert!(
        components
            .schemas
            .keys()
            .all(|name| !name.contains("probe")),
        "the probe leaked into the components"
    );
    assert_eq!(
        serde_json::to_value(&components).unwrap(),
        serde_json::to_value(without_operations.components.as_ref().unwrap()).unwrap(),
        "describing operations changed the exported components"
    );
}

#[derive(Type)]
#[specta(collect = false, inline)]
struct InlinedBody {
    value: String,
}

/// A type is a component only when the exporter gives it one. An `#[specta(inline)]` type is
/// written out at its use site instead, so an operation carries its schema rather than a `$ref` to
/// a component that was never exported.
#[test]
fn openapi_inlines_operation_types_that_have_no_component() {
    let document = OpenApi::default()
        .operation(Operation::get("/inlined").response::<InlinedBody>(200, "inlined"))
        .export_document(
            &Types::default().register::<InlinedBody>(),
            specta_serde::Format,
        )
        .expect("an inlined type should be written in place");
    let value = serde_json::to_value(&document).unwrap();

    let schema = &value["paths"]["/inlined"]["get"]["responses"]["200"]["content"]["application/json"]
        ["schema"];
    assert!(
        schema.get("$ref").is_none(),
        "inlined types have no component to reference"
    );
    assert_eq!(schema["properties"]["value"]["type"], "string");
}

/// Parameters carry the schema of whatever the extractor parses the segment into, so `/users/{id}`
/// served by a `Path<u32>` exports as an integer rather than as text.
#[test]
fn openapi_types_parameters_from_their_extracted_type() {
    let document = OpenApi::default()
        .operation(
            Operation::get("/users/{id}")
                .path_param::<u32>("id")
                .query_param::<String>("q")
                .response::<Recipe>(200, "ok"),
        )
        .export_document(&recipe_types(), specta_serde::Format)
        .expect("typed parameters should export");
    let value = serde_json::to_value(&document).unwrap();
    let parameters = &value["paths"]["/users/{id}"]["get"]["parameters"];

    assert_eq!(parameters[0]["name"], "id");
    assert_eq!(parameters[0]["in"], "path");
    assert_eq!(parameters[0]["required"], true);
    assert_eq!(parameters[0]["schema"]["type"], "integer");
    assert_eq!(parameters[0]["schema"]["maximum"], u32::MAX);

    assert_eq!(parameters[1]["name"], "q");
    assert_eq!(parameters[1]["in"], "query");
    assert_eq!(parameters[1]["schema"]["type"], "string");
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "snake_case")]
enum PlainStringEnum {
    /// Automatic per-location blend.
    Auto,
    Gfs,
    Ecmwf,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct NumericFormats {
    small: u16,
    lead: u32,
    wide: i64,
    ratio: f32,
    value: f64,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ProblemBody {
    title: String,
}

/// Numeric schemas carry OpenAPI formats, and plain string enums render in
/// the compact `type: string, enum: [...]` form with variant docs retained in
/// an extension.
#[test]
fn numeric_formats_and_compact_string_enums() {
    let types = Types::default()
        .register::<NumericFormats>()
        .register::<PlainStringEnum>();
    let document = OpenApi::default()
        .export_document(&types, specta_serde::Format)
        .unwrap();
    let json = serde_json::to_value(&document).unwrap();
    let schemas = &json["components"]["schemas"];

    let numeric = &schemas["NumericFormats"]["properties"];
    assert_eq!(numeric["small"]["format"], "int32");
    assert_eq!(numeric["lead"]["format"], "int64");
    assert_eq!(numeric["wide"]["format"], "int64");
    assert_eq!(numeric["ratio"]["format"], "float");
    assert_eq!(numeric["value"]["format"], "double");

    let string_enum = &schemas["PlainStringEnum"];
    assert_eq!(string_enum["type"], "string");
    assert_eq!(
        string_enum["enum"],
        serde_json::json!(["auto", "gfs", "ecmwf"])
    );
    assert!(string_enum.get("oneOf").is_none());
    assert_eq!(
        string_enum["x-specta-enum-descriptions"]["auto"],
        "Automatic per-location blend."
    );
}

/// The `Param` builder covers what the bare conveniences cannot: required
/// query parameters, descriptions, and example values.
#[test]
fn parameters_carry_required_description_and_example() {
    use specta_openapi::Param;

    let types = Types::default().register::<ProblemBody>();
    let document = OpenApi::default()
        .operation(
            Operation::get("/v1/weather/forecast")
                .parameter(
                    Param::query::<f64>("lat")
                        .required()
                        .description("Latitude, WGS84 degrees, -90 to 90")
                        .example(serde_json::json!(35.0)),
                )
                .query_param::<u32>("horizon_hours")
                .response::<ProblemBody>(200, "ok"),
        )
        .export_document(&types, specta_serde::Format)
        .unwrap();
    let json = serde_json::to_value(&document).unwrap();
    let parameters = &json["paths"]["/v1/weather/forecast"]["get"]["parameters"];

    assert_eq!(parameters[0]["name"], "lat");
    assert_eq!(parameters[0]["required"], true);
    assert_eq!(
        parameters[0]["description"],
        "Latitude, WGS84 degrees, -90 to 90"
    );
    assert_eq!(parameters[0]["example"], 35.0);
    assert_eq!(parameters[0]["schema"]["format"], "double");

    // The bare convenience stays optional (openapiv3 omits `required: false`)
    // with no annotations.
    assert_eq!(parameters[1]["name"], "horizon_hours");
    assert_ne!(parameters[1]["required"], serde_json::json!(true));
    assert!(parameters[1].get("example").is_none());
}

/// Error responses can be served as `application/problem+json`, security
/// schemes register on the document, and operations state their security
/// alternatives, the anonymous option included.
#[test]
fn content_types_security_and_document_metadata() {
    let types = Types::default().register::<ProblemBody>();
    let document = OpenApi::default()
        .title("Orrery API")
        .version("1.0.0")
        .server_described("https://api.orr.sh", "Production")
        .contact("Orrery", "https://orreryhq.com")
        .tag("weather", "Weather intelligence")
        .bearer_security_scheme("api_key", "opaque")
        .operation(
            Operation::get("/v1/me")
                .tag("weather")
                .response::<ProblemBody>(200, "ok")
                .response_as::<ProblemBody>(
                    401,
                    "Missing or unknown key",
                    "application/problem+json",
                )
                .security([("api_key", Vec::new())])
                .security_optional(),
        )
        .export_document(&types, specta_serde::Format)
        .unwrap();
    let json = serde_json::to_value(&document).unwrap();

    assert_eq!(json["servers"][0]["url"], "https://api.orr.sh");
    assert_eq!(json["servers"][0]["description"], "Production");
    assert_eq!(json["info"]["contact"]["name"], "Orrery");
    assert_eq!(json["tags"][0]["description"], "Weather intelligence");
    assert_eq!(
        json["components"]["securitySchemes"]["api_key"]["scheme"],
        "bearer"
    );

    let operation = &json["paths"]["/v1/me"]["get"];
    assert!(operation["responses"]["200"]["content"]["application/json"].is_object());
    assert!(operation["responses"]["401"]["content"]["application/problem+json"].is_object());
    assert_eq!(
        operation["security"],
        serde_json::json!([{ "api_key": [] }, {}])
    );
}
