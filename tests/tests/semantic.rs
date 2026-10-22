use std::collections::HashMap;

use specta::{Type, Types, datatype::DataType};
use specta_typescript::{
    Typescript, define, primitives,
    semantic::{Configuration, Transform},
};

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
struct Website(String);

#[derive(Type)]
#[specta(collect = false)]
struct Newtype(Option<Website>);

#[derive(Type)]
#[specta(collect = false)]
struct SemanticTuple(Option<Website>, Website);

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct TransparentWebsite {
    value: Option<Website>,
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
struct TupleWithSkippedFirst(#[serde(skip)] String, Option<Website>);

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
struct TupleWithSkippedLast(Option<Website>, #[serde(skip)] String);

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

#[derive(Type)]
#[specta(collect = false)]
struct StructWithF64 {
    value: f64,
}

#[derive(Type)]
#[specta(collect = false)]
struct IdentityFloatPayload {
    value: f64,
    nested: Option<StructWithF64>,
    values: Vec<f64>,
}

#[derive(Type)]
#[specta(collect = false)]
struct MixedIdentityPayload {
    value: f64,
    nested: Option<StructWithF64>,
    values: Vec<f64>,
    site: Website,
}

#[derive(Type)]
#[specta(collect = false)]
struct Wrapper<T> {
    value: T,
}

#[derive(Type)]
#[specta(collect = false)]
struct SemanticMapPayload {
    sites: HashMap<String, Website>,
    wide: HashMap<u64, u64>,
}

fn semantic_config() -> Configuration {
    Configuration::empty().define::<Website>(
        |_| define("URL").into(),
        Some(Transform::new(|value| format!("{value}.toString()"))),
        Some(Transform::new(|value| format!("new URL({value})"))),
    )
}

fn identity_semantic_config() -> Configuration {
    Configuration::empty().define::<Website>(
        |_| define("URL").into(),
        Some(Transform::identity()),
        Some(Transform::identity()),
    )
}

// Regression coverage for https://github.com/specta-rs/specta/issues/578.
#[test]
fn semantic_newtype_transforms_use_scalar_shape() {
    let semantic = semantic_config();
    let mut types = Types::default();
    let newtype = Newtype::definition(&mut types);

    let (_, serialize) = semantic
        .apply_serialize(&types, &newtype, "value")
        .expect("newtype should apply its nested semantic transform");
    assert_eq!(serialize, "value==null?value:value.toString()");

    let (_, deserialize) = semantic
        .apply_deserialize(&types, &newtype, "value")
        .expect("newtype should apply its nested semantic transform");
    assert_eq!(deserialize, "value==null?value:new URL(value)");

    let tuple = SemanticTuple::definition(&mut types);
    let (_, serialize) = semantic
        .apply_serialize(&types, &tuple, "value")
        .expect("tuple should apply its nested semantic transform");
    assert_eq!(
        serialize,
        "([value[0]==null?value[0]:value[0].toString(),value[1].toString()])"
    );
    let (_, deserialize) = semantic
        .apply_deserialize(&types, &tuple, "value")
        .expect("tuple should apply its nested semantic transform");
    assert_eq!(
        deserialize,
        "([value[0]==null?value[0]:new URL(value[0]),new URL(value[1])])"
    );
}

// Named transparent structs share the newtype representation from #578.
#[test]
fn semantic_transparent_struct_transforms_use_scalar_shape() {
    let semantic = semantic_config();
    let mut types = Types::default();
    let dt = TransparentWebsite::definition(&mut types);
    let types = specta::Format::map_types(&specta_serde::Format, &types).unwrap();

    let (_, deserialize) = semantic
        .apply_deserialize(&types, &dt, "value")
        .expect("transparent struct should apply its nested semantic transform");
    assert_eq!(deserialize, "value==null?value:new URL(value)");
    let (_, serialize) = semantic
        .apply_serialize(&types, &dt, "value")
        .expect("transparent struct should apply its nested semantic transform");
    assert_eq!(serialize, "value==null?value:value.toString()");

    let types = semantic.apply_types(&types);
    let rendered = Typescript::default()
        .export(&types, specta_serde::Format)
        .unwrap();
    assert!(rendered.contains("export type TransparentWebsite = URL | null;"));
}

#[test]
fn semantic_skipped_tuple_fields_preserve_array_shape() {
    let semantic = semantic_config();
    let mut types = Types::default();
    let tuples = [
        TupleWithSkippedFirst::definition(&mut types),
        TupleWithSkippedLast::definition(&mut types),
    ];
    let types = specta::Format::map_types(&specta_serde::Format, &types).unwrap();

    for dt in tuples {
        let (_, serialize) = semantic
            .apply_serialize(&types, &dt, "value")
            .expect("single live tuple field should retain its array transform");
        assert_eq!(serialize, "([value[0]==null?value[0]:value[0].toString()])");
        let (_, deserialize) = semantic
            .apply_deserialize(&types, &dt, "value")
            .expect("single live tuple field should retain its array transform");
        assert_eq!(deserialize, "([value[0]==null?value[0]:new URL(value[0])])");
    }

    let types = semantic.apply_types(&types);
    let rendered = Typescript::default()
        .export(&types, specta_serde::Format)
        .unwrap();
    assert!(rendered.contains("export type TupleWithSkippedFirst = [URL | null];"));
    assert!(rendered.contains("export type TupleWithSkippedLast = [URL | null];"));
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
fn semantic_intersection_deduplicates_identical_runtime_transforms() {
    let semantic = semantic_config();
    let mut types = Types::default();
    let dt = SemanticPayload::definition(&mut types);
    let intersection = DataType::Intersection(vec![dt.clone(), dt]);

    let (_, runtime) = semantic
        .apply_deserialize(&types, &intersection, "payload")
        .expect("semantic transform should apply");

    assert_eq!(runtime.matches("site:new URL(payload.site)").count(), 1);
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
    assert_eq!(semantic.apply_serialize(&types, &dt, "payload"), None);
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

#[test]
fn semantic_lossless_floats_do_not_emit_identity_runtime_transforms() {
    let semantic = Configuration::default().enable_lossless_floats();
    let mut types = Types::default();

    for inline in [
        f64::definition(&mut types),
        Vec::<f64>::definition(&mut types),
        Option::<f64>::definition(&mut types),
    ] {
        for transform in [
            semantic.apply_serialize(&types, &inline, "payload"),
            semantic.apply_deserialize(&types, &inline, "payload"),
        ] {
            let (remapped, runtime) = transform.unwrap_or_else(|| {
                panic!("inline floats must retain their type remap: {inline:?}")
            });
            assert!(remapped.is_some());
            assert_eq!(runtime, "payload");
        }
    }

    let registered = IdentityFloatPayload::definition(&mut types);
    for transform in [
        semantic.apply_serialize(&types, &registered, "payload"),
        semantic.apply_deserialize(&types, &registered, "payload"),
    ] {
        assert_eq!(transform, None);
    }
}

#[test]
fn semantic_inline_identity_bigint_retains_its_type_remap() {
    let semantic = Configuration::empty().enable_lossless_bigints();
    let mut types = Types::default();
    let dt = u64::definition(&mut types);

    let (remapped, runtime) = semantic
        .apply_serialize(&types, &dt, "payload")
        .expect("inline bigints must retain their type remap");
    assert!(remapped.is_some());
    assert_eq!(runtime, "payload");
}

#[test]
fn semantic_registered_identity_rule_does_not_emit_a_runtime_transform() {
    let semantic = identity_semantic_config();
    let mut types = Types::default();
    let dt = Website::definition(&mut types);

    assert_eq!(semantic.apply_serialize(&types, &dt, "payload"), None);
    assert_eq!(semantic.apply_deserialize(&types, &dt, "payload"), None);
}

#[test]
fn semantic_definitionless_identity_rule_retains_its_type_remap() {
    let semantic = identity_semantic_config();
    let mut types = Types::default();
    let dt = Website::definition(&mut types);
    let types = types.map(|mut ndt| {
        ndt.ty = None;
        ndt
    });

    for transform in [
        semantic.apply_serialize(&types, &dt, "payload"),
        semantic.apply_deserialize(&types, &dt, "payload"),
    ] {
        let (remapped, runtime) =
            transform.expect("definitionless rules must retain their type remap");
        assert!(remapped.is_some());
        assert_eq!(runtime, "payload");
    }
}

#[test]
fn semantic_registered_generic_retains_identity_use_site_remaps() {
    let semantic = Configuration::empty().enable_lossless_floats();
    let mut types = Types::default();
    let dt = Wrapper::<f64>::definition(&mut types);

    for transform in [
        semantic.apply_serialize(&types, &dt, "payload"),
        semantic.apply_deserialize(&types, &dt, "payload"),
    ] {
        let (remapped, runtime) =
            transform.expect("registered generic arguments must retain their type remap");
        assert!(remapped.is_some());
        assert_eq!(runtime, "payload");
    }
}

#[test]
fn semantic_identity_descendants_are_pruned_from_real_runtime_transforms() {
    let semantic = semantic_config().enable_lossless_floats();
    let mut types = Types::default();
    let dt = MixedIdentityPayload::definition(&mut types);

    let (_, serialize) = semantic
        .apply_serialize(&types, &dt, "payload")
        .expect("Website serialization should require a runtime transform");
    assert_eq!(serialize, "({...payload,site:payload.site.toString()})");

    let (_, deserialize) = semantic
        .apply_deserialize(&types, &dt, "payload")
        .expect("Website deserialization should require a runtime transform");
    assert_eq!(deserialize, "({...payload,site:new URL(payload.site)})");
}

/// Map keys are JSON object property names: semantic transforms rewrite the
/// *value* only (the key stays the runtime property name), and lossless
/// built-in remaps must not rewrite the key type either.
///
/// Regression coverage for https://github.com/specta-rs/specta/issues/594.
#[test]
fn semantic_map_keys_are_not_remapped_or_transformed() {
    let semantic = semantic_config();
    let mut types = Types::default();
    let dt = SemanticMapPayload::definition(&mut types);

    // Custom rules transform map values, not keys.
    let (_, serialize) = semantic
        .apply_serialize(&types, &dt, "payload")
        .expect("Website map values should require a runtime transform");
    assert_eq!(
        serialize,
        "({...payload,sites:Object.fromEntries(Object.entries(payload.sites).map(([k,v])=>[k,v.toString()]))})"
    );
    let (_, deserialize) = semantic
        .apply_deserialize(&types, &dt, "payload")
        .expect("Website map values should require a runtime transform");
    assert_eq!(
        deserialize,
        "({...payload,sites:Object.fromEntries(Object.entries(payload.sites).map(([k,v])=>[k,new URL(v)]))})"
    );

    // The lossless BigInt remap applies to the map value but not its key, so
    // the key stays a lexical string while the value becomes a `bigint`.
    let lossless = Configuration::empty().enable_lossless_bigints();
    let semantic_types = lossless.apply_types(&types).into_owned();
    let rendered = Typescript::default()
        .export(&semantic_types, specta_serde::Format)
        .expect("lossless map keys and values should export");

    assert!(
        rendered.contains("wide: { [key in string]: bigint }"),
        "lossless BigInt map keys must stay string keys:\n{rendered}"
    );
    assert!(
        !rendered.contains("[key in bigint]"),
        "lossless BigInt map keys must not be remapped:\n{rendered}"
    );

    let (_, deserialize) = lossless
        .apply_deserialize(&types, &dt, "payload")
        .expect("lossless BigInt map values should require a runtime transform");
    assert_eq!(
        deserialize,
        "({...payload,wide:Object.fromEntries(Object.entries(payload.wide).map(([k,v])=>[k,BigInt(v)]))})"
    );
}
