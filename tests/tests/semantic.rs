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

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
enum ExternalEnum {
    Unit,
    Newtype(Website),
    Named { site: Website },
    Tuple(Website, u32),
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum InternalEnum {
    Unit,
    Named { site: Website },
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind", content = "data")]
enum AdjacentEnum {
    Unit,
    Newtype(Website),
    Named { site: Website },
    Tuple(Website, u32),
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
enum NestedEnum {
    Site(ExternalEnum),
    Count(u32),
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
enum NullableEnum {
    Some(Option<Website>),
    None,
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
enum RecursiveEnum {
    Leaf(Website),
    Next(Box<RecursiveEnum>),
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedEnum {
    Site(Website),
    Other(String),
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedDistinct {
    Site(Website),
    Count(u32),
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedWithUnit {
    Site(Website),
    Nothing,
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedObj {
    Site { site: Website },
    Count(u32),
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
struct MixedContainer {
    site: Website,
    choice: UntaggedEnum,
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
struct Moment(String);

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
enum DateEnum {
    At(Moment),
    Count(u32),
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
enum BigintEnum {
    Id(u64),
    Name(String),
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
#[specta(collect = false)]
enum RenamedEnum {
    #[serde(rename = "some-value")]
    Value(Website),
    #[serde(rename = "other-value")]
    Other(String),
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

fn date_semantic_config() -> Configuration {
    Configuration::empty().define::<Moment>(
        |_| define("Date").into(),
        Some(Transform::new(|value| format!("{value}.toISOString()"))),
        Some(Transform::new(|value| format!("new Date({value})"))),
    )
}

/// Build the format-lowered graph the way a framework would before applying
/// semantic rules: map with `specta_serde::Format` first, then transform.
fn mapped_types<T: Type>() -> (Types, DataType) {
    let mut types = Types::default();
    let dt = T::definition(&mut types);
    let types = specta::Format::map_types(&specta_serde::Format, &types)
        .expect("serde mapping should succeed")
        .into_owned();
    (types, dt)
}

fn node_available() -> bool {
    std::process::Command::new("node")
        .arg("--version")
        .output()
        .is_ok()
}

/// Execute `transform` (an expression built around `value`) on `input` in a
/// real JavaScript runtime and assert `assertion` holds for the result.
///
/// The generated expression is only meaningful as JavaScript, so this is the
/// only way to check that the runtime value actually matches the exported type.
fn assert_runtime_js(transform: &str, input: &str, assertion: &str) {
    if !node_available() {
        return;
    }

    let script = format!(
        "const value = {input};\nconst out = {transform};\nif (!({assertion})) {{ throw new Error(\"assertion failed: \" + JSON.stringify(out)); }}"
    );
    let output = std::process::Command::new("node")
        .arg("-e")
        .arg(&script)
        .output()
        .expect("node should be executable");

    assert!(
        output.status.success(),
        "node rejected transform {transform:?} for input {input:?}:\n{}\nscript:\n{script}",
        String::from_utf8_lossy(&output.stderr)
    );
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

// Regression coverage for https://github.com/specta-rs/specta/issues/595.
#[test]
fn semantic_external_enum_runtime_transforms_match_the_exported_type() {
    let semantic = semantic_config();
    let (types, dt) = mapped_types::<ExternalEnum>();

    let (_, serialize) = semantic
        .try_apply_serialize(&types, &dt, "value")
        .expect("external enum serialization should be supported")
        .expect("external enum serialization should require a transform");
    assert_eq!(
        serialize,
        "value!=null&&value.Newtype!==undefined?({...value,Newtype:value.Newtype.toString()}):value!=null&&value.Named!==undefined?({...value,Named:({...value.Named,site:value.Named.site.toString()})}):value!=null&&value.Tuple!==undefined?({...value,Tuple:([value.Tuple[0].toString(),value.Tuple[1]])}):value"
    );

    let (_, deserialize) = semantic
        .try_apply_deserialize(&types, &dt, "value")
        .expect("external enum deserialization should be supported")
        .expect("external enum deserialization should require a transform");
    assert_eq!(
        deserialize,
        "value!=null&&value.Newtype!==undefined?({...value,Newtype:new URL(value.Newtype)}):value!=null&&value.Named!==undefined?({...value,Named:({...value.Named,site:new URL(value.Named.site)})}):value!=null&&value.Tuple!==undefined?({...value,Tuple:([new URL(value.Tuple[0]),value.Tuple[1]])}):value"
    );

    // The unit variant is a bare string and stays untouched.
    assert_runtime_js(&deserialize, "\"Unit\"", "out === \"Unit\"");
    assert_runtime_js(
        &deserialize,
        "{ Newtype: \"https://example.com/\" }",
        "out.Newtype instanceof URL && out.Newtype.href === \"https://example.com/\"",
    );
    assert_runtime_js(
        &deserialize,
        "{ Named: { site: \"https://example.com/\" } }",
        "out.Named.site instanceof URL",
    );
    assert_runtime_js(
        &deserialize,
        "{ Tuple: [\"https://example.com/\", 7] }",
        "out.Tuple[0] instanceof URL && out.Tuple[1] === 7",
    );

    // Serialization is the inverse: a real URL becomes the wire string.
    assert_runtime_js(&serialize, "\"Unit\"", "out === \"Unit\"");
    assert_runtime_js(
        &serialize,
        "{ Newtype: new URL(\"https://example.com/\") }",
        "out.Newtype === \"https://example.com/\"",
    );
    assert_runtime_js(
        &serialize,
        "{ Named: { site: new URL(\"https://example.com/\") } }",
        "out.Named.site === \"https://example.com/\"",
    );
    assert_runtime_js(
        &serialize,
        "{ Tuple: [new URL(\"https://example.com/\"), 7] }",
        "out.Tuple[0] === \"https://example.com/\" && out.Tuple[1] === 7",
    );

    let rendered = Typescript::default()
        .export(&semantic.apply_types(&types), specta_serde::Format)
        .expect("external enum should export");
    assert!(rendered.contains("Newtype: URL"), "{rendered}");
    assert!(rendered.contains("site: URL"), "{rendered}");
    assert!(rendered.contains("[URL, number]"), "{rendered}");
}

#[test]
fn semantic_internal_enum_runtime_transforms_dispatch_on_the_tag() {
    let semantic = semantic_config();
    let (types, dt) = mapped_types::<InternalEnum>();

    let (_, deserialize) = semantic
        .try_apply_deserialize(&types, &dt, "value")
        .expect("internal enum deserialization should be supported")
        .expect("internal enum deserialization should require a transform");
    assert_eq!(
        deserialize,
        "value!=null&&value.kind===\"Named\"?({...value,site:new URL(value.site)}):value"
    );

    assert_runtime_js(
        &deserialize,
        "{ kind: \"Unit\" }",
        "out.kind === \"Unit\" && !(\"site\" in out)",
    );
    assert_runtime_js(
        &deserialize,
        "{ kind: \"Named\", site: \"https://example.com/\" }",
        "out.site instanceof URL && out.kind === \"Named\"",
    );

    let rendered = Typescript::default()
        .export(&semantic.apply_types(&types), specta_serde::Format)
        .expect("internal enum should export");
    assert!(rendered.contains("site: URL"), "{rendered}");
}

#[test]
fn semantic_adjacent_enum_runtime_transforms_dispatch_on_the_tag() {
    let semantic = semantic_config();
    let (types, dt) = mapped_types::<AdjacentEnum>();

    let (_, deserialize) = semantic
        .try_apply_deserialize(&types, &dt, "value")
        .expect("adjacent enum deserialization should be supported")
        .expect("adjacent enum deserialization should require a transform");
    assert_eq!(
        deserialize,
        "value!=null&&value.kind===\"Newtype\"?({...value,data:new URL(value.data)}):value!=null&&value.kind===\"Named\"?({...value,data:({...value.data,site:new URL(value.data.site)})}):value!=null&&value.kind===\"Tuple\"?({...value,data:([new URL(value.data[0]),value.data[1]])}):value"
    );

    assert_runtime_js(&deserialize, "{ kind: \"Unit\" }", "out.kind === \"Unit\"");
    assert_runtime_js(
        &deserialize,
        "{ kind: \"Newtype\", data: \"https://example.com/\" }",
        "out.data instanceof URL",
    );
    assert_runtime_js(
        &deserialize,
        "{ kind: \"Named\", data: { site: \"https://example.com/\" } }",
        "out.data.site instanceof URL",
    );
    assert_runtime_js(
        &deserialize,
        "{ kind: \"Tuple\", data: [\"https://example.com/\", 7] }",
        "out.data[0] instanceof URL && out.data[1] === 7",
    );

    let rendered = Typescript::default()
        .export(&semantic.apply_types(&types), specta_serde::Format)
        .expect("adjacent enum should export");
    assert!(rendered.contains("data: URL"), "{rendered}");
}

#[test]
fn semantic_nullable_and_nested_enum_payloads_transform() {
    let semantic = semantic_config();

    let (types, dt) = mapped_types::<NullableEnum>();
    let (_, deserialize) = semantic
        .try_apply_deserialize(&types, &dt, "value")
        .expect("nullable enum deserialization should be supported")
        .expect("nullable enum deserialization should require a transform");
    assert_eq!(
        deserialize,
        "value!=null&&value.Some!==undefined?({...value,Some:value.Some==null?value.Some:new URL(value.Some)}):value"
    );
    assert_runtime_js(&deserialize, "{ Some: null }", "out.Some === null");
    assert_runtime_js(
        &deserialize,
        "{ Some: \"https://example.com/\" }",
        "out.Some instanceof URL",
    );
    assert_runtime_js(&deserialize, "\"None\"", "out === \"None\"");

    // A nested enum is transformed with the same representation-aware rules.
    let (types, dt) = mapped_types::<NestedEnum>();
    let (_, deserialize) = semantic
        .try_apply_deserialize(&types, &dt, "value")
        .expect("nested enum deserialization should be supported")
        .expect("nested enum deserialization should require a transform");
    assert_runtime_js(
        &deserialize,
        "{ Site: { Newtype: \"https://example.com/\" } }",
        "out.Site.Newtype instanceof URL",
    );
    assert_runtime_js(&deserialize, "{ Count: 3 }", "out.Count === 3");
}

// A recursive enum cannot be expanded into one expression. Rather than emit a
// transform that only rewrites the top level while the exported type claims the
// whole structure is remapped (which node confirms leaves `Next.Leaf` a
// string), the truncation is reported as unsupported. Struct recursion is
// unchanged (out of scope).
#[test]
fn semantic_recursive_enum_is_reported_unsupported() {
    let semantic = semantic_config();
    let (types, dt) = mapped_types::<RecursiveEnum>();

    for result in [
        semantic.try_apply_serialize(&types, &dt, "value"),
        semantic.try_apply_deserialize(&types, &dt, "value"),
    ] {
        let error = result.expect_err("a recursive enum must be reported as unsupported");
        assert!(
            error.to_string().contains("the enum is recursive"),
            "{error}"
        );
        assert!(
            error.to_string().contains("RecursiveEnum"),
            "the diagnostic should name the enum: {error}"
        );
    }

    // The infallible API does not panic; it simply cannot transform the root.
    assert_eq!(
        semantic.apply_deserialize(&types, &dt, "value"),
        None,
        "a root recursive enum has no transform"
    );

    // Evidence: the partial expansion that used to be emitted is wrong - the
    // nested `Next.Leaf` stays a string even though the exported type says URL.
    assert_runtime_js(
        "value!=null&&value.Leaf!==undefined?({...value,Leaf:new URL(value.Leaf)}):value",
        "{ Next: { Leaf: \"https://example.com/\" } }",
        "typeof out.Next.Leaf === \"string\"",
    );
}

#[test]
fn semantic_untagged_enum_mixed_container_keeps_the_supported_field() {
    let semantic = semantic_config();
    let (types, dt) = mapped_types::<MixedContainer>();

    // The unsupported nested enum is reported...
    assert!(
        semantic
            .try_apply_deserialize(&types, &dt, "value")
            .is_err(),
        "the ambiguous nested enum should be reported"
    );

    // ...but the enclosing struct still transforms the field it can.
    let (_, deserialize) = semantic
        .apply_deserialize(&types, &dt, "value")
        .expect("the enclosing struct should still transform its supported field");
    assert_eq!(deserialize, "({...value,site:new URL(value.site)})");
    assert_runtime_js(
        &deserialize,
        "{ site: \"https://example.com/\", choice: \"Other\" }",
        "out.site instanceof URL && out.choice === \"Other\"",
    );
}

#[test]
fn semantic_identity_rule_inside_an_enum_emits_no_runtime_transform() {
    let semantic = identity_semantic_config();
    let (types, dt) = mapped_types::<ExternalEnum>();

    assert_eq!(
        semantic
            .try_apply_serialize(&types, &dt, "value")
            .expect("identity rule should not fail"),
        None
    );
    assert_eq!(
        semantic
            .try_apply_deserialize(&types, &dt, "value")
            .expect("identity rule should not fail"),
        None
    );
}

#[test]
fn semantic_date_enum_payloads_transform() {
    let semantic = date_semantic_config();
    let (types, dt) = mapped_types::<DateEnum>();

    let (_, deserialize) = semantic
        .try_apply_deserialize(&types, &dt, "value")
        .expect("date enum deserialization should be supported")
        .expect("date enum deserialization should require a transform");
    assert_runtime_js(
        &deserialize,
        "{ At: \"2020-01-01T00:00:00.000Z\" }",
        "out.At instanceof Date && out.At.toISOString() === \"2020-01-01T00:00:00.000Z\"",
    );

    let (_, serialize) = semantic
        .try_apply_serialize(&types, &dt, "value")
        .expect("date enum serialization should be supported")
        .expect("date enum serialization should require a transform");
    assert_runtime_js(
        &serialize,
        "{ At: new Date(\"2020-01-01T00:00:00.000Z\") }",
        "out.At === \"2020-01-01T00:00:00.000Z\"",
    );
}

#[test]
fn semantic_lossless_bigint_enum_payloads_transform() {
    let semantic = Configuration::empty().enable_lossless_bigints();
    let (types, dt) = mapped_types::<BigintEnum>();

    let (_, deserialize) = semantic
        .try_apply_deserialize(&types, &dt, "value")
        .expect("bigint enum deserialization should be supported")
        .expect("bigint enum deserialization should require a transform");
    assert_runtime_js(
        &deserialize,
        "{ Id: 123 }",
        "typeof out.Id === \"bigint\" && out.Id === 123n",
    );
    assert_runtime_js(
        &deserialize,
        "{ Name: \"specta\" }",
        "out.Name === \"specta\"",
    );
}

#[test]
fn semantic_external_enum_with_non_identifier_keys_transforms() {
    let semantic = semantic_config();
    let (types, dt) = mapped_types::<RenamedEnum>();

    let (_, deserialize) = semantic
        .try_apply_deserialize(&types, &dt, "value")
        .expect("renamed enum deserialization should be supported")
        .expect("renamed enum deserialization should require a transform");
    assert_eq!(
        deserialize,
        "value!=null&&value[\"some-value\"]!==undefined?({...value,\"some-value\":new URL(value[\"some-value\"])}):value"
    );
    assert_runtime_js(
        &deserialize,
        "{ \"some-value\": \"https://example.com/\" }",
        "out[\"some-value\"] instanceof URL",
    );
    assert_runtime_js(
        &deserialize,
        "{ \"other-value\": \"plain\" }",
        "out[\"other-value\"] === \"plain\"",
    );
}

#[test]
fn semantic_ambiguous_untagged_enum_is_rejected() {
    let semantic = semantic_config();
    let (types, dt) = mapped_types::<UntaggedEnum>();

    for result in [
        semantic.try_apply_serialize(&types, &dt, "value"),
        semantic.try_apply_deserialize(&types, &dt, "value"),
    ] {
        let err = result.expect_err("an overlapping untagged enum must be rejected");
        let message = err.to_string();
        assert!(
            message.contains("cannot be discriminated without guessing"),
            "{message}"
        );
    }
}

#[test]
fn semantic_ambiguous_untagged_enum_does_not_panic() {
    let semantic = semantic_config();
    let (types, dt) = mapped_types::<UntaggedEnum>();

    // The infallible API skips the unsupported root rather than aborting.
    assert_eq!(semantic.apply_serialize(&types, &dt, "value"), None);
    assert_eq!(semantic.apply_deserialize(&types, &dt, "value"), None);
}

#[test]
fn semantic_untagged_enums_with_distinct_shapes_transform() {
    let semantic = semantic_config();

    // `Website | u32` -> `URL | number`, disjoint by `typeof`.
    let (types, dt) = mapped_types::<UntaggedDistinct>();
    let (_, deserialize) = semantic
        .try_apply_deserialize(&types, &dt, "value")
        .expect("a shape-disjoint untagged enum should be supported")
        .expect("it should require a transform");
    assert_eq!(
        deserialize,
        "typeof value===\"number\"?value:new URL(value)"
    );
    assert_runtime_js(
        &deserialize,
        "\"https://example.com/\"",
        "out instanceof URL",
    );
    assert_runtime_js(&deserialize, "7", "out === 7");

    let (_, serialize) = semantic
        .try_apply_serialize(&types, &dt, "value")
        .expect("serialization should be supported")
        .expect("it should require a transform");
    assert_runtime_js(
        &serialize,
        "new URL(\"https://example.com/\")",
        "out === \"https://example.com/\"",
    );
    assert_runtime_js(&serialize, "7", "out === 7");

    // `Website | ()` -> `URL | null`, disjoint by `null`.
    let (types, dt) = mapped_types::<UntaggedWithUnit>();
    let (_, deserialize) = semantic
        .try_apply_deserialize(&types, &dt, "value")
        .expect("a null-disjoint untagged enum should be supported")
        .expect("it should require a transform");
    assert_eq!(deserialize, "value===null?value:new URL(value)");
    assert_runtime_js(&deserialize, "null", "out === null");
    assert_runtime_js(
        &deserialize,
        "\"https://example.com/\"",
        "out instanceof URL",
    );

    // `{ site: Website } | u32` -> `{ site: URL } | number`, object vs number.
    let (types, dt) = mapped_types::<UntaggedObj>();
    let (_, deserialize) = semantic
        .try_apply_deserialize(&types, &dt, "value")
        .expect("an object/number-disjoint untagged enum should be supported")
        .expect("it should require a transform");
    assert_runtime_js(
        &deserialize,
        "{ site: \"https://example.com/\" }",
        "out.site instanceof URL",
    );
    assert_runtime_js(&deserialize, "3", "out === 3");
}
