use serde::{Deserialize, Serialize};
use specta::{ResolvedTypes, Type, Types};
use specta_openapi::{GenericHandling, OpenAPI, OpenApiVersion};

#[derive(Type, Serialize, Deserialize)]
struct User {
    id: u32,
    name: String,
    email: Option<String>,
}

#[derive(Type, Serialize, Deserialize)]
struct ExtraFields {
    slug: String,
}

#[derive(Type, Serialize, Deserialize)]
struct FlattenedUser {
    id: u32,
    #[serde(flatten)]
    extra: ExtraFields,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data")]
enum TaggedEnum {
    Unit,
    Tuple(i32),
    Named { value: String },
}

#[derive(Type, Serialize, Deserialize)]
struct GenericBox<T> {
    value: T,
    values: Vec<T>,
}

#[derive(Type, Serialize, Deserialize)]
struct UsesGenerics {
    string_box: GenericBox<String>,
    number_box: GenericBox<i32>,
}

fn phase_collections() -> [(&'static str, Result<ResolvedTypes, specta_serde::Error>); 3] {
    let types = Types::default()
        .register::<User>()
        .register::<FlattenedUser>()
        .register::<TaggedEnum>()
        .register::<UsesGenerics>();

    [
        ("raw", Ok(ResolvedTypes::from_resolved_types(types.clone()))),
        ("serde", specta_serde::apply(types.clone())),
        ("serde_phases", specta_serde::apply_phases(types)),
    ]
}

fn phase_output(result: Result<ResolvedTypes, specta_serde::Error>, exporter: OpenAPI) -> String {
    result.map_or_else(
        |err| format!("ERROR: {err}"),
        |types| {
            exporter
                .export_json(&types)
                .unwrap_or_else(|err| format!("ERROR: {err}"))
        },
    )
}

#[test]
fn openapi_export() {
    for (mode, result) in phase_collections() {
        insta::assert_snapshot!(
            format!("openapi-export-{mode}"),
            phase_output(
                result,
                OpenAPI::default().title("Test API").version("1.0.0")
            )
        );
    }
}

#[test]
fn openapi_export_dynamic_ref() {
    for (mode, result) in phase_collections() {
        insta::assert_snapshot!(
            format!("openapi-export-dynamic-ref-{mode}"),
            phase_output(
                result,
                OpenAPI::default()
                    .title("Test API")
                    .version("1.0.0")
                    .openapi_version(OpenApiVersion::V3_1_0)
                    .generic_handling(GenericHandling::DynamicRef),
            )
        );
    }
}

#[test]
fn openapi_dynamic_ref_requires_openapi_31() {
    let types = ResolvedTypes::from_resolved_types(Types::default().register::<UsesGenerics>());

    insta::assert_snapshot!(
        OpenAPI::default()
            .generic_handling(GenericHandling::DynamicRef)
            .export(&types)
            .unwrap_err()
            .to_string(),
        @r#"`$dynamicRef` generic handling requires OpenAPI 3.1+, but exporter is configured for 3.0.3"#
    );
}
