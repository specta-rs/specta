use specta::{Type, Types, datatype::DataType};
use specta_typescript::{
    Typescript, define, primitives,
    semantic::{Configuration, Transform},
};

#[derive(Type)]
#[specta(collect = false)]
struct Website(String);

#[derive(Type)]
#[specta(collect = false)]
struct SemanticPayload {
    site: Website,
    sites: Vec<Website>,
    optional_site: Option<Website>,
}

#[derive(Type)]
#[specta(collect = false)]
struct NumberPayload {
    id: u64,
    signed: i128,
    scores: Vec<f64>,
    maybe_score: Option<f32>,
}

#[derive(Type)]
#[specta(collect = false)]
struct CombinedPayload {
    site: Website,
    id: u64,
    nested: Vec<Option<Website>>,
}

fn semantic_config() -> Configuration {
    Configuration::empty().define::<Website>(
        |_| define("URL").into(),
        Some(Transform::new(|value| format!("{value}.toString()"))),
        Some(Transform::new(|value| format!("new URL({value})"))),
    )
}

fn transformed_snapshot(types: &Types, transform: Option<(Option<DataType>, String)>) -> String {
    let (ty, runtime) = transform.expect("semantic transform should apply");
    let ty = ty
        .as_ref()
        .map(|ty| primitives::inline(&Typescript::default(), types, ty).unwrap())
        .unwrap_or_else(|| "<unchanged>".to_owned());

    format!("type: {ty}\nruntime: {runtime}")
}

#[test]
fn semantic_apply_types_exports_runtime_shapes() {
    let semantic = semantic_config();
    let types = Types::default().register::<SemanticPayload>();
    let types = semantic.apply_types(&types).into_owned();
    let rendered = Typescript::default()
        .export(&types, specta_serde::Format)
        .expect("semantic types should export");

    insta::assert_snapshot!("semantic-apply-types-exports-runtime-shapes", rendered);
}

#[test]
fn semantic_custom_transforms_snapshot_nested_runtime_expressions() {
    let semantic = semantic_config();
    let mut types = Types::default();
    let dt = SemanticPayload::definition(&mut types);

    insta::assert_snapshot!(
        "semantic-custom-serialize-transform",
        transformed_snapshot(&types, semantic.apply_serialize(&types, &dt, "payload"),)
    );
    insta::assert_snapshot!(
        "semantic-custom-deserialize-transform",
        transformed_snapshot(&types, semantic.apply_deserialize(&types, &dt, "payload"),)
    );
}

#[test]
fn semantic_lossless_numbers_snapshot_export_and_transforms() {
    let semantic = Configuration::empty()
        .enable_lossless_bigints()
        .enable_lossless_floats();

    let mut types = Types::default();
    let dt = NumberPayload::definition(&mut types);
    let semantic_types = semantic.apply_types(&types).into_owned();
    let rendered = Typescript::default()
        .export(&semantic_types, specta_serde::Format)
        .expect("lossless semantic numbers should export");

    insta::assert_snapshot!("semantic-lossless-numbers-export", rendered);
    insta::assert_snapshot!(
        "semantic-lossless-numbers-serialize-transform",
        transformed_snapshot(&types, semantic.apply_serialize(&types, &dt, "payload"),)
    );
    insta::assert_snapshot!(
        "semantic-lossless-numbers-deserialize-transform",
        transformed_snapshot(&types, semantic.apply_deserialize(&types, &dt, "payload"),)
    );
}

#[test]
fn semantic_custom_rules_compose_with_lossless_builtin_remaps() {
    let semantic = semantic_config().enable_lossless_bigints();

    let mut types = Types::default();
    let dt = CombinedPayload::definition(&mut types);
    let semantic_types = semantic.apply_types(&types).into_owned();
    let rendered = Typescript::default()
        .export(&semantic_types, specta_serde::Format)
        .expect("custom and built-in semantic remaps should export together");

    insta::assert_snapshot!(
        "semantic-custom-rules-compose-with-lossless-builtins",
        rendered
    );
    insta::assert_snapshot!(
        "semantic-composed-deserialize-transform",
        transformed_snapshot(&types, semantic.apply_deserialize(&types, &dt, "payload"),)
    );
}
