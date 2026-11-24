use std::{
    borrow::Cow,
    collections::HashMap,
    path::Path,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use serde::{Deserialize, Serialize};
use specta::{
    Format, FormatError, Type, Types,
    datatype::{DataType, Fields, Primitive},
};
use specta_effect::{Effect, Layout, Never, Unknown, define, primitives};
use tempfile::TempDir;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Generic<T> {
    value: T,
    values: Vec<T>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Defaults<T = String, U = T> {
    first: T,
    second: U,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct GenericMap<K = bool, V = String>
where
    K: Eq + std::hash::Hash,
{
    values: HashMap<K, V>,
}

#[derive(Type)]
#[specta(collect = false)]
struct ChainedMap<K = bool, U = K>
where
    K: Eq + std::hash::Hash,
    U: Eq + std::hash::Hash,
{
    marker: Option<K>,
    values: HashMap<U, String>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum Message<T> {
    Hello(T),
    Ping,
    Goodbye { reason: Option<String> },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct OptionalFields {
    #[serde(default)]
    nickname: Option<String>,
    required: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Complex {
    generic: Generic<i32>,
    defaulted: Defaults,
    messages: Vec<Message<String>>,
    booleans: HashMap<bool, Vec<i32>>,
    pair: (i32, Option<String>),
    optional: OptionalFields,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Node {
    value: i32,
    children: Vec<Node>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ProtoField {
    #[serde(rename = "__proto__")]
    value: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Flattened {
    id: u32,
    #[serde(flatten)]
    extra: HashMap<String, String>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Aliased {
    #[serde(alias = "old_value")]
    value: String,
    other: i32,
}

#[derive(Type)]
#[specta(collect = false)]
struct DefaultedKey<T = bool>(T);

#[derive(Type)]
#[specta(collect = false)]
struct DefaultedKeyMap {
    values: HashMap<DefaultedKey, String>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct OptionalFlattenInner {
    a: i32,
    b: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct OptionalFlatten {
    id: u32,
    #[serde(flatten)]
    inner: Option<OptionalFlattenInner>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct OptionalMapFlatten {
    id: u32,
    #[serde(flatten)]
    inner: Option<HashMap<String, String>>,
}

#[derive(Type)]
#[specta(collect = false)]
struct MutualA {
    b: Option<Box<MutualB>>,
}

#[derive(Type)]
#[specta(collect = false)]
struct MutualB {
    a: Option<Box<MutualA>>,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericWrapper<T> {
    value: T,
}

#[derive(Type)]
#[specta(collect = false)]
struct CycleA {
    b: Option<Vec<GenericWrapper<CycleB>>>,
}

#[derive(Type)]
#[specta(collect = false)]
struct CycleB {
    a: Option<Vec<GenericWrapper<CycleA>>>,
}

#[derive(Type)]
#[specta(collect = false)]
struct DefaultCycleA<T = DefaultCycleB> {
    value: Option<Box<T>>,
}

#[derive(Type)]
#[specta(collect = false)]
struct DefaultCycleB {
    a: DefaultCycleA,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedGenericKey<T> {
    Value(T),
}

#[derive(Type)]
#[specta(collect = false)]
struct UntaggedGenericKeyMap<T> {
    values: HashMap<UntaggedGenericKey<T>, String>,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericMapDefault<T = GenericMap<String, i32>> {
    value: T,
}

#[derive(Type)]
#[specta(collect = false)]
struct DefaultMapHolder<T = bool, U = GenericMap<T, String>>
where
    T: Eq + std::hash::Hash,
{
    marker: T,
    value: U,
}

#[test]
fn effect_exports_complex_generic_types() {
    let types = Types::default().register::<Complex>();
    let output = Effect::default()
        .export(&types, specta_serde::Format)
        .unwrap();

    assert!(output.contains("import { Schema } from \"effect\";"));
    assert!(output.contains("export const GenericSchema = <T extends Schema.Constraint>"));
    assert!(output.contains("values: Schema.Array(T)"));
    assert!(output.contains("export const DefaultsSchema"));
    assert!(output.contains("U extends Schema.Constraint = T"));
    assert!(output.contains("generic: Schema.suspend(() => GenericSchema(Schema.Int))"));
    assert!(output.contains("Schema.Union(["));
    assert!(output.contains("Schema.NullOr(Schema.String.pipe"));
    assert!(output.contains("Schema.optionalKey(Schema.NullOr("));
    assert!(output.contains("Schema.Record("));
    assert!(!output.contains("export type "));
    assert!(!output.contains("/**"));

    insta::assert_snapshot!("effect-complex", output);
}

#[test]
fn effect_generic_maps_receive_key_schemas() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct UsesGenericMaps {
        booleans: GenericMap<bool>,
        integers: GenericMap<i32>,
        defaulted: GenericMap,
        chained: ChainedMap,
    }

    let output = Effect::default()
        .export(
            &Types::default().register::<UsesGenericMaps>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(output.contains("$key$K extends Schema.Record.Key"));
    assert!(output.contains("GenericMapSchema(Schema.Boolean, Schema.String"));
    assert!(
        output.contains("Schema.makeFilter((key: string) => [\"true\", \"false\"].includes(key)")
    );
    assert!(output.contains("GenericMapSchema(Schema.Int, Schema.Int,"));
    assert!(output.contains("GenericMapSchema(Schema.Boolean, Schema.String.pipe"));
    assert!(output.contains("$key$U extends Schema.Record.Key = $key$K"));
}

#[test]
fn effect_suspends_recursive_references() {
    let output = Effect::default()
        .export(&Types::default().register::<Node>(), specta_serde::Format)
        .unwrap();

    assert!(
        output.contains("Schema.Array(Schema.suspend((): Schema.Codec<unknown> => NodeSchema))")
    );
}

#[test]
fn effect_handles_proto_and_flattened_maps() {
    let types = Types::default()
        .register::<ProtoField>()
        .register::<Flattened>();
    let output = Effect::default()
        .export(&types, specta_serde::Format)
        .unwrap();

    assert!(output.contains(r#"["__proto__"]: Schema.String"#));
    assert!(output.contains("Schema.StructWithRest("));
    assert!(output.contains("Schema.makeFilter((key: PropertyKey)"));
    assert!(output.contains("[\"id\"].includes(String(key))"));
}

#[test]
fn effect_wrappers_and_define_render_schema_expressions() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Wrappers {
        any: specta_effect::Any<String>,
        unknown: Unknown<String>,
        never: Never<String>,
        defined: i64,
    }

    let types = specta_util::Remapper::new()
        .rule(
            specta::datatype::Primitive::i64.into(),
            define("Schema.BigInt").into(),
        )
        .remap_types(Types::default().register::<Wrappers>());
    let output = Effect::default()
        .export(&types, specta_serde::Format)
        .unwrap();

    assert!(output.contains("any: Schema.Any"));
    assert!(output.contains("unknown: Schema.Unknown"));
    assert!(output.contains("never: Schema.Never"));
    assert!(output.contains("defined: Schema.BigInt"));
}

#[test]
fn effect_primitives_preserve_named_references() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Inner {
        value: String,
    }

    let mut registered = Types::default();
    let dt = Inner::definition(&mut registered);
    let specta::datatype::DataType::Reference(reference) = dt else {
        panic!("expected a named reference");
    };
    let rendered = primitives::reference(&Effect::default(), &registered, &reference).unwrap();
    assert!(rendered.contains("InnerSchema"));

    let optional = Option::<Inner>::definition(&mut registered);
    let rendered = primitives::inline(&Effect::default(), &registered, &optional).unwrap();
    assert!(rendered.contains("Schema.NullOr(Schema.suspend(() => InnerSchema))"));
}

#[test]
fn effect_files_layout_emits_imports_and_schemas_only() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Root {
        child: child::Child,
    }

    mod child {
        use super::*;

        #[derive(Type)]
        #[specta(collect = false)]
        pub struct Child {
            value: String,
        }
    }

    let temp = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&temp).unwrap();
    let temp = TempDir::new_in(temp).unwrap();

    let types = Types::default().register::<Root>();
    Effect::default()
        .layout(Layout::Files)
        .export_to(temp.path(), &types, specta_serde::Format)
        .unwrap();

    let root = std::fs::read_to_string(temp.path().join("test/effect.ts")).unwrap();
    let child = std::fs::read_to_string(temp.path().join("test/effect/child.ts")).unwrap();
    assert!(root.contains("import * as test$effect$child"));
    assert!(root.contains("test$effect$child.ChildSchema"));
    assert!(child.contains("export const ChildSchema"));
    assert!(!root.contains("export type "));
    assert!(!child.contains("export type "));
}

#[test]
fn effect_handles_aliases_defaulted_keys_and_optional_flatten() {
    let types = Types::default()
        .register::<Aliased>()
        .register::<DefaultedKeyMap>()
        .register::<OptionalFlatten>()
        .register::<OptionalMapFlatten>();
    let output = Effect::default()
        .export(&types, specta_serde::Format)
        .unwrap();

    assert!(output.contains("Schema.Union(["));
    assert!(output.contains("old_value: Schema.optionalKey(Schema.Never)"));
    assert!(output.contains("value: Schema.optionalKey(Schema.Never)"));
    assert!(output.contains(r#"["true", "false"].includes(key)"#));
    assert!(output.contains("a: Schema.optionalKey(Schema.Never)"));
    assert!(output.contains("b: Schema.optionalKey(Schema.Never)"));
    assert!(output.contains(
        "Schema.Record(Schema.String.pipe(Schema.check(Schema.makeFilter((key: PropertyKey)"
    ));
    assert!(output.contains("Schema.Never)"));
}

#[test]
fn effect_reports_the_alias_combination_limit() {
    #[derive(Type, Serialize, Deserialize)]
    #[specta(collect = false)]
    struct ManyAliases {
        #[serde(alias = "old_a")]
        a: i32,
        #[serde(alias = "old_b")]
        b: i32,
        #[serde(alias = "old_c")]
        c: i32,
        #[serde(alias = "old_d")]
        d: i32,
        #[serde(alias = "old_e")]
        e: i32,
        #[serde(alias = "old_f")]
        f: i32,
        #[serde(alias = "old_g")]
        g: i32,
        #[serde(alias = "old_h")]
        h: i32,
        #[serde(alias = "old_i")]
        i: i32,
    }

    let error = Effect::default()
        .export(
            &Types::default().register::<ManyAliases>(),
            specta_serde::Format,
        )
        .unwrap_err()
        .to_string();

    assert!(error.contains("256-combination limit"), "{error}");
}

#[test]
fn effect_handles_mutual_recursion_and_unresolved_generic_keys() {
    let types = Types::default()
        .register::<MutualA>()
        .register::<UntaggedGenericKeyMap<String>>();
    let output = Effect::default()
        .export(&types, specta_serde::Format)
        .unwrap();

    assert_eq!(
        output
            .matches("Schema.suspend((): Schema.Codec<unknown> =>")
            .count(),
        2
    );
    assert!(output.contains("Schema.Record(Schema.String, Schema.String.pipe"));
}

#[test]
fn effect_handles_recursion_through_generics_and_defaults() {
    let output = Effect::default()
        .export(
            &Types::default()
                .register::<CycleA>()
                .register::<DefaultCycleA>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(output.contains("Schema.suspend((): Schema.Codec<unknown> => CycleBSchema)"));
    assert!(output.contains("Schema.suspend((): Schema.Codec<unknown> => CycleASchema)"));
    assert!(output.contains("Schema.suspend((): Schema.Codec<unknown> => DefaultCycleBSchema)"));
    assert!(output.contains("Schema.suspend((): Schema.Codec<unknown> => DefaultCycleASchema("));
}

#[test]
fn effect_generic_defaults_instantiate_schema_factories() {
    let output = Effect::default()
        .export(
            &Types::default()
                .register::<GenericMapDefault>()
                .register::<DefaultMapHolder>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(output.contains("ReturnType<typeof GenericMapSchema<"));
    assert!(output.contains("GenericMapSchema(Schema.String.pipe"));
    assert!(output.contains("$key$T extends Schema.Record.Key"));
    assert!(output.contains("GenericMapSchema(T, $key$t, Schema.String.pipe"));
}

#[test]
fn effect_rejects_wide_numbers_even_when_collapsed_to_finite() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct NumberWrapper {
        value: serde_json::Number,
    }

    let error = Effect::default()
        .export(
            &Types::default().register::<NumberWrapper>(),
            specta_serde::Format,
        )
        .unwrap_err()
        .to_string();

    assert!(error.contains("BigInt-style"), "{error}");
}

#[test]
fn effect_does_not_reapply_a_format_to_its_owned_result() {
    struct OwnedStructFormat(Arc<AtomicUsize>);

    impl Format for OwnedStructFormat {
        fn map_types<'a>(&'a self, types: &'a Types) -> Result<Cow<'a, Types>, FormatError> {
            Ok(Cow::Borrowed(types))
        }

        fn map_type<'a>(
            &'a self,
            _: &'a Types,
            dt: &'a DataType,
        ) -> Result<Cow<'a, DataType>, FormatError> {
            let DataType::Struct(strct) = dt else {
                return Ok(Cow::Borrowed(dt));
            };
            self.0.fetch_add(1, Ordering::SeqCst);
            let mut strct = strct.clone();
            if let Fields::Named(fields) = &mut strct.fields {
                for (_, field) in &mut fields.fields {
                    if field.ty.as_ref() == Some(&DataType::Primitive(Primitive::u32)) {
                        field.ty = Some(DataType::Primitive(Primitive::str));
                    }
                }
            }
            Ok(Cow::Owned(DataType::Struct(strct)))
        }
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct Item {
        value: u32,
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let output = Effect::default()
        .export(
            &Types::default().register::<Item>(),
            OwnedStructFormat(calls.clone()),
        )
        .unwrap();

    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(output.contains("value: Schema.String.pipe"));
}

#[test]
fn effect_framework_runtime_can_render_types() {
    let exported = Effect::default()
        .framework_runtime(|mut ctx| {
            Ok(Cow::Owned(format!(
                "export const runtime = {};",
                ctx.inline(&specta::datatype::Primitive::str.into())?
            )))
        })
        .export(&Types::default(), specta_serde::Format)
        .unwrap();

    assert!(exported.contains("export const runtime = Schema.String.pipe"));
}
