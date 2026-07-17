use std::{borrow::Cow, collections::HashMap, iter, path::Path};

use serde::{Deserialize, Serialize};
use specta::{
    Format, Type, Types,
    datatype::{DataType, NamedDataType, Primitive, Reference},
};
use specta_typescript::Typescript;
use specta_util::Remapper;
use specta_valibot::{Any, Layout, Never, Unknown, Valibot, define, primitives};
use tempfile::TempDir;

macro_rules! for_bigint_types {
    (T -> $s:expr) => {{
        for_bigint_types!(usize, isize, i64, u64, i128, u128; $s);
    }};
    ($($i:ty),+; $s:expr) => {{
        $({
            type T = $i;
            $s(stringify!($i));
        })*
    }};
}

#[derive(Type)]
#[specta(collect = false)]
struct StructWithBigInt {
    a: i128,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ProtoField {
    #[serde(rename = "__proto__")]
    prototype: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericMap<K = bool> {
    values: HashMap<K, String>,
}

#[derive(Type)]
#[specta(collect = false)]
struct ChainedDefaultMap<T = bool, U = T> {
    marker: Option<T>,
    values: HashMap<U, String>,
}

#[derive(Type)]
#[specta(collect = false)]
struct StructWithStructWithBigInt {
    #[specta(inline)]
    abc: StructWithBigInt,
}

#[derive(Type)]
#[specta(collect = false)]
struct StructWithOptionWithStructWithBigInt {
    #[specta(inline)]
    optional_field: Option<StructWithBigInt>,
}

#[derive(Type)]
#[specta(collect = false)]
enum EnumWithInlineStructWithBigInt {
    #[specta(inline)]
    B { a: i128 },
}

#[derive(Type)]
struct Recursive {
    children: Vec<Recursive>,
}

#[derive(Type)]
struct Testing {
    a: testing::Testing,
}

#[derive(Type)]
struct Another {
    bruh: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct EmptyStruct {}

#[derive(Type)]
#[specta(collect = false)]
enum EmptyNamedVariant {
    A {},
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum SerdeTaggedEnum {
    Unit,
    StringValue(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedMatchingField {
    Variant {
        #[serde(rename = "Variant")]
        value: String,
    },
    Empty {},
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum InvalidInternallyTaggedEnum {
    A(String),
}

mod testing {
    use super::*;

    /// This documentation mentions test$valibot$testing.Testing verbatim.
    #[derive(Type)]
    pub struct Testing {
        b: testing2::Testing,
    }

    pub mod testing2 {
        use super::*;

        #[derive(Type)]
        pub struct Testing {
            c: String,
        }
    }
}

mod r#type {
    use super::*;

    #[derive(Type)]
    pub struct KeywordModule {
        value: String,
    }

    #[allow(clippy::module_inception)]
    pub mod r#type {
        use super::*;

        #[derive(Type)]
        pub struct NestedKeywordModule {
            value: String,
        }
    }
}

#[derive(Type)]
struct UsesKeywordModule {
    value: r#type::KeywordModule,
    nested: r#type::r#type::NestedKeywordModule,
}

fn inline_for<T: Type>(valibot: &Valibot) -> Result<String, specta_valibot::Error> {
    let mut types = Types::default();
    let dt = T::definition(&mut types);
    primitives::inline(valibot, &types, &dt)
}

fn temp_root() -> std::path::PathBuf {
    let temp_root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&temp_root).unwrap();
    temp_root
}

#[test]
fn valibot_export_smoke() {
    #[derive(Type)]
    struct Inner {
        value: String,
    }

    #[derive(Type)]
    struct Demo {
        inner: Inner,
        count: i32,
        maybe: Option<String>,
    }

    let types = Types::default().register::<Demo>();
    let out = Valibot::default()
        .export(&types, specta_serde::Format)
        .unwrap();

    assert!(out.contains("import * as v from \"valibot\";"));
    assert!(out.contains("export const DemoSchema"));
    assert!(out.contains("export type Demo ="));
    assert!(out.contains("DemoSchema: v.GenericSchema<Demo>"));
}

#[test]
fn valibot_primitives_smoke() {
    let (types, dts) = crate::types();
    let valibot = Valibot::default();

    for (_, ty) in &dts {
        let rendered = primitives::inline(&valibot, &types, ty).unwrap();
        assert!(!rendered.is_empty());
    }

    let ndt = dts
        .iter()
        .find_map(|(_, ty)| match ty {
            DataType::Reference(Reference::Named(r)) => types.get(r),
            _ => None,
        })
        .unwrap();

    let rendered = primitives::export(&valibot, &types, iter::once(ndt), "").unwrap();
    assert!(rendered.contains("Schema"));
}

#[test]
fn valibot_bigint_forbidden_by_default() {
    for_bigint_types!(T -> |_| {
        assert!(
            inline_for::<T>(&Valibot::default()).is_err(),
            "bigint-style primitives must be forbidden by default"
        );
    });
}

#[test]
fn valibot_wrappers_serde_roundtrip() {
    for value in [
        serde_json::to_value(serde_json::from_str::<Any<String>>(r#""value""#).unwrap()).unwrap(),
        serde_json::to_value(serde_json::from_str::<Unknown<String>>(r#""value""#).unwrap())
            .unwrap(),
        serde_json::to_value(serde_json::from_str::<Never<String>>(r#""value""#).unwrap()).unwrap(),
    ] {
        assert_eq!(value, serde_json::json!("value"));
    }
}

#[test]
fn valibot_bigint_override_via_define() {
    // `specta_valibot::define` is the escape hatch for the forbidden bigint
    // primitives: remap them to a Valibot schema of your choosing, mirroring the
    // Typescript exporter's `define`.
    let remapper = Remapper::new().rule(Primitive::i64.into(), define("v.bigint()").into());
    let dt = remapper.remap_dt(Primitive::i64.into());

    assert_eq!(
        primitives::inline(&Valibot::default(), &Types::default(), &dt).unwrap(),
        "v.bigint()"
    );
}

#[test]
fn valibot_high_level_export_supports_valibot_opaque_types() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct OpaqueTypes {
        any: Any<String>,
        unknown: Unknown<String>,
        never: Never<String>,
    }

    let wrappers = export_for::<OpaqueTypes>().unwrap();
    assert!(wrappers.contains("any: any"));
    assert!(wrappers.contains("unknown: unknown"));
    assert!(wrappers.contains("never: never"));
    assert!(wrappers.contains("any: v.any()"));
    assert!(wrappers.contains("unknown: v.unknown()"));
    assert!(wrappers.contains("never: v.never()"));

    let types = Remapper::new()
        .rule(Primitive::i128.into(), define("v.bigint()").into())
        .remap_types(Types::default().register::<StructWithBigInt>());
    let defined = Valibot::default()
        .export(&types, specta_serde::Format)
        .unwrap();
    assert!(defined.contains("a: unknown"));
    assert!(defined.contains("a: v.bigint()"));
}

#[test]
fn valibot_define_map_key_uses_a_valid_typescript_property_key() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct DefinedMapKey {
        values: HashMap<i64, String>,
        named: HashMap<DefinedKey, String>,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct DefinedKey(i64);

    let types = Remapper::new()
        .rule(Primitive::i64.into(), define("v.string()").into())
        .remap_types(Types::default().register::<DefinedMapKey>());
    let out = Valibot::default()
        .export(&types, specta_serde::Format)
        .unwrap();

    assert!(
        out.contains("values: Partial<{ [key in string]: string }>"),
        "unexpected export: {out}"
    );
    assert!(out.contains(
        r"values: $spectaRecord(v.string(), v.pipe(v.string(), v.check((value) => !/[\uD800-\uDFFF]/u.test(value))))"
    ));
    assert!(out.contains("named: Partial<{ [key in string]: string }>"));
    assert!(out.contains(
        r"named: $spectaRecord(v.string(), v.pipe(v.string(), v.check((value) => !/[\uD800-\uDFFF]/u.test(value))))"
    ));
    assert!(!out.contains("[key in unknown]"));
}

#[test]
fn valibot_emits_proto_as_a_computed_object_key() {
    let out = export_for::<ProtoField>().unwrap();
    assert!(out.contains(r#"["__proto__"]: v.pipe(v.string()"#), "{out}");
    assert!(!out.contains("\n  __proto__:"), "{out}");
}

#[test]
fn valibot_generic_maps_receive_serialized_key_schemas() {
    #[derive(Type)]
    #[specta(collect = false)]
    enum FiniteKey {
        First,
        Second,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct UsesGenericMaps {
        booleans: GenericMap<bool>,
        integers: GenericMap<i32>,
        finite: GenericMap<FiniteKey>,
        chained: ChainedDefaultMap,
    }

    let out = export_for::<UsesGenericMaps>().unwrap();
    assert!(
        out.contains(
            "export function GenericMapSchema(): v.GenericSchema<GenericMap, GenericMap>;"
        ),
        "{out}"
    );
    assert!(
        out.contains("export function GenericMapSchema<K extends v.GenericSchema, $key$K extends v.GenericSchema<string, string>>(K: K, $key$K: $key$K): v.GenericSchema<GenericMap<v.InferInput<K>>, GenericMap<v.InferOutput<K>>>;"),
        "{out}"
    );
    assert!(
        out.contains("GenericMapSchema(v.boolean(), v.picklist([\"true\", \"false\"]))"),
        "{out}"
    );
    assert!(
        out.contains("GenericMapSchema(v.pipe(v.number(), v.integer(), v.minValue(-2147483648), v.maxValue(2147483647)), v.pipe(v.string(), v.regex(/^-?\\d+$/)"),
        "{out}"
    );
    assert!(
        out.contains("GenericMapSchema(v.lazy(() => FiniteKeySchema), v.union([v.literal(\"First\"), v.literal(\"Second\")]))"),
        "{out}"
    );
    assert!(
        out.contains("$key$U: v.GenericSchema<string, string> = $key$T"),
        "{out}"
    );
}

#[test]
fn valibot_rejects_invalid_concrete_and_default_generic_map_keys() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct InvalidConcreteKey {
        value: GenericMap<Vec<String>>,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct InvalidDefaultMap<K = Vec<String>> {
        values: HashMap<K, String>,
    }

    for err in [
        export_for::<InvalidConcreteKey>().unwrap_err(),
        export_for::<InvalidDefaultMap>().unwrap_err(),
    ] {
        let err = err.to_string();
        assert!(err.contains("Invalid map key"), "{err}");
        assert!(
            err.contains("collection, map, and nullable keys are not supported"),
            "{err}"
        );
    }
}

#[test]
fn valibot_rejects_non_json_map_keys() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct NamedKey {
        value: String,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct RecursiveKey(Box<RecursiveKey>);

    fn assert_invalid<K: Type>(reason: &str) {
        let mut types = Types::default();
        let dt = HashMap::<K, String>::definition(&mut types);
        let err = primitives::inline(&Valibot::default(), &types, &dt)
            .unwrap_err()
            .to_string();
        assert!(err.contains("Invalid map key"), "{err}");
        assert!(err.contains(reason), "{err}");
    }

    assert_invalid::<(String, String)>("tuple keys are not supported");
    assert_invalid::<Option<String>>("nullable keys are not supported");
    assert_invalid::<Vec<String>>("collection, map, and nullable keys are not supported");
    assert_invalid::<HashMap<String, String>>(
        "collection, map, and nullable keys are not supported",
    );
    assert_invalid::<NamedKey>("struct keys must serialize as a newtype struct");
    assert_invalid::<RecursiveKey>("recursive map key reference cycle detected");
    assert_invalid::<Any>("opaque references cannot be validated");
}

#[test]
fn valibot_rejects_tagged_enum_payload_map_keys_like_serde_json() {
    #[derive(Type, Serialize, Deserialize, Eq, Hash, PartialEq)]
    #[specta(collect = false)]
    enum TaggedPayloadKey {
        Unit,
        Newtype(u8),
        Tuple(u8, u8),
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct TaggedPayloadMap {
        values: HashMap<TaggedPayloadKey, String>,
    }

    for key in [TaggedPayloadKey::Newtype(1), TaggedPayloadKey::Tuple(1, 2)] {
        let runtime = serde_json::to_string(&HashMap::from([(key, "value")])).unwrap_err();
        assert!(runtime.to_string().contains("key must be a string"));
    }

    let high_level_err = export_for::<TaggedPayloadMap>().unwrap_err().to_string();
    assert!(
        high_level_err.contains("Invalid map key"),
        "{high_level_err}"
    );

    let mut types = Types::default();
    let dt = HashMap::<TaggedPayloadKey, String>::definition(&mut types);
    let err = primitives::inline(&Valibot::default(), &types, &dt)
        .unwrap_err()
        .to_string();
    assert!(
        err.contains("tagged newtype or tuple serialization"),
        "{err}"
    );
}

#[test]
fn valibot_accepts_variant_untagged_scalar_map_keys() {
    #[derive(Type, Serialize, Deserialize, Eq, Hash, PartialEq)]
    #[specta(collect = false)]
    enum MixedKey {
        Unit,
        #[serde(untagged)]
        Number(u8),
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct MixedMap {
        values: HashMap<MixedKey, String>,
    }

    let runtime = serde_json::to_string(&HashMap::from([
        (MixedKey::Unit, "unit"),
        (MixedKey::Number(42), "number"),
    ]))
    .unwrap();
    assert!(runtime.contains("\"Unit\""));
    assert!(runtime.contains("\"42\""));

    let rendered = export_for::<MixedMap>().unwrap();
    assert!(rendered.contains("v.literal(\"Unit\")"), "{rendered}");
    assert!(rendered.contains("Number(value) <= 255"), "{rendered}");
}

#[test]
fn valibot_rejects_tagged_unit_object_map_keys_when_tag_matches_variant() {
    #[derive(Type, Serialize, Deserialize, Eq, Hash, PartialEq)]
    #[specta(collect = false)]
    #[serde(tag = "kind")]
    enum MatchingTagKey {
        #[serde(rename = "kind")]
        Kind,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct MatchingTagMap {
        values: HashMap<MatchingTagKey, String>,
    }

    let runtime =
        serde_json::to_string(&HashMap::from([(MatchingTagKey::Kind, "value")])).unwrap_err();
    assert!(runtime.to_string().contains("key must be a string"));

    let err = export_for::<MatchingTagMap>().unwrap_err().to_string();
    assert!(err.contains("Invalid map key"), "{err}");
}

#[test]
fn valibot_bigint_errors_propagate_from_nested_types() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct MapWithBigIntKey {
        values: HashMap<usize, String>,
    }

    for err in [
        export_for::<StructWithBigInt>(),
        export_for::<StructWithStructWithBigInt>(),
        export_for::<StructWithOptionWithStructWithBigInt>(),
        export_for::<EnumWithInlineStructWithBigInt>(),
        export_for::<MapWithBigIntKey>(),
    ] {
        let err = err.expect_err("bigint export should be rejected by default");
        assert!(
            err.to_string()
                .contains("forbids exporting BigInt-style types"),
            "unexpected error: {err}"
        );
    }

    assert!(inline_for::<HashMap<usize, String>>(&Valibot::default()).is_err());
    assert!(inline_for::<HashMap<isize, String>>(&Valibot::default()).is_err());
}

#[test]
fn valibot_layout_duplicate_typenames() {
    let types = Types::default().register::<Testing>().register::<Another>();
    let err = Valibot::default()
        .export(&types, specta_serde::Format)
        .unwrap_err();
    assert!(err.to_string().contains("Detected multiple types"));

    let module_prefixed = Valibot::default()
        .layout(Layout::ModulePrefixedName)
        .export(&types, specta_serde::Format)
        .unwrap();
    assert!(module_prefixed.contains("TestingSchema"));
    assert!(module_prefixed.contains("testing2"));
}

#[test]
fn valibot_module_prefixed_duplicate_checks_use_rendered_names() {
    fn add_type(types: &mut Types, name: &'static str, module_path: &'static str) {
        NamedDataType::new(name, types, |_, ndt| {
            ndt.module_path = module_path.into();
            ndt.ty = Some(Primitive::str.into());
        });
    }

    let mut distinct = Types::default();
    add_type(&mut distinct, "Bar", "");
    add_type(&mut distinct, "Bar", "foo");
    let out = Valibot::default()
        .layout(Layout::ModulePrefixedName)
        .export(&distinct, specta_serde::Format)
        .unwrap();
    assert!(out.contains("export type Bar = string"));
    assert!(out.contains("export type foo_Bar = string"));

    let mut references = Types::default();
    let root = NamedDataType::new("RootReference", &mut references, |_, ndt| {
        ndt.module_path = "".into();
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("UsesRoot", &mut references, |_, ndt| {
        ndt.module_path = "".into();
        ndt.ty = Some(DataType::Reference(root.reference(vec![])));
    });
    let references = Valibot::default()
        .layout(Layout::ModulePrefixedName)
        .export(&references, specta_serde::Format)
        .unwrap();
    assert!(references.contains("v.lazy(() => RootReferenceSchema)"));
    assert!(!references.contains("_RootReferenceSchema"));

    let mut colliding = Types::default();
    add_type(&mut colliding, "foo_Bar", "");
    add_type(&mut colliding, "Bar", "foo");
    let err = Valibot::default()
        .layout(Layout::ModulePrefixedName)
        .export(&colliding, specta_serde::Format)
        .unwrap_err();
    assert!(err.to_string().contains("Detected multiple types"));
}

#[test]
fn valibot_layout_files_export_to() {
    let types = Types::default().register::<Testing>().register::<Another>();
    let temp = temp_dir();
    let path = temp.path().join("valibot-layout-files");

    Valibot::default()
        .layout(Layout::Files)
        .export_to(&path, &types, specta_serde::Format)
        .unwrap();

    let output = crate::fs_to_string(Path::new(&path)).unwrap();
    assert!(output.contains(".ts"));
    assert!(output.contains("import * as v from \"valibot\";"));
    assert!(output.contains("test$valibot$testing.Testing verbatim"));
}

#[test]
fn valibot_layout_namespaces() {
    let types = Types::default().register::<Testing>().register::<Another>();
    let rendered = Valibot::default()
        .layout(Layout::Namespaces)
        .export(&types, specta_serde::Format)
        .unwrap();

    insta::assert_snapshot!("valibot-layout-namespaces", rendered);
    assert!(rendered.contains("namespace $s$"));
    assert!(rendered.contains("export namespace testing"));
    assert!(rendered.contains("testing.TestingSchema)"));
    assert!(rendered.contains("export import test = $s$.test;"));
}

#[test]
fn valibot_layout_namespaces_reexports_root_schema() {
    let mut types = Types::default();
    NamedDataType::new("Root", &mut types, |_, ndt| {
        ndt.module_path = "".into();
        ndt.ty = Some(Primitive::str.into());
    });

    let rendered = Valibot::default()
        .layout(Layout::Namespaces)
        .export(&types, specta_serde::Format)
        .unwrap();
    assert!(rendered.contains("export import Root = $s$.Root;"));
    assert!(rendered.contains("export import RootSchema = $s$.RootSchema;"));
}

#[test]
fn valibot_layout_sanitises_reserved_module_identifiers() {
    let types = Types::default().register::<UsesKeywordModule>();
    let rendered = Valibot::default()
        .layout(Layout::Namespaces)
        .export(&types, specta_serde::Format)
        .unwrap();

    assert!(rendered.contains("export namespace $type"));
    assert!(rendered.contains(".$type.KeywordModule"));
    assert!(rendered.contains(".$type.$type.NestedKeywordModule"));
    assert!(!rendered.contains(".$type.type.NestedKeywordModule"));
    assert!(!rendered.contains("namespace type"));
}

#[test]
fn valibot_layout_files_sanitises_the_valibot_prelude_binding() {
    fn read_typescript_files(path: &Path, output: &mut String) {
        for entry in std::fs::read_dir(path).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                read_typescript_files(&path, output);
            } else if path.extension().is_some_and(|extension| extension == "ts") {
                output.push_str(&std::fs::read_to_string(path).unwrap());
            }
        }
    }

    let temp = temp_dir();
    let path = temp.path().join("valibot-prelude-binding");
    let mut types = Types::default();
    let v_type = NamedDataType::new("PreludeCollision", &mut types, |_, ndt| {
        ndt.module_path = "v".into();
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("UsesV", &mut types, |_, ndt| {
        ndt.module_path = "other".into();
        ndt.ty = Some(DataType::Reference(v_type.reference(vec![])));
    });
    Valibot::default()
        .layout(Layout::Files)
        .export_to(&path, &types, specta_serde::Format)
        .unwrap();

    let mut rendered = String::new();
    read_typescript_files(&path, &mut rendered);
    assert!(rendered.contains("import * as $v from"));
    assert!(!rendered.contains("import * as v from \"../v\""));
    assert!(rendered.contains("$v.PreludeCollisionSchema"));
}

#[test]
fn valibot_layout_files_qualifies_root_types_from_modules() {
    let temp = temp_dir();
    let path = temp.path().join("valibot-root-reference");
    let mut types = Types::default();
    let root = NamedDataType::new("RootReference", &mut types, |_, ndt| {
        ndt.module_path = "".into();
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("UsesRoot", &mut types, |_, ndt| {
        ndt.module_path = "other".into();
        ndt.ty = Some(DataType::Reference(root.reference(vec![])));
    });

    Valibot::default()
        .layout(Layout::Files)
        .export_to(&path, &types, specta_serde::Format)
        .unwrap();

    let module = std::fs::read_to_string(path.join("other.ts")).unwrap();
    assert!(module.contains("import * as $root from \"./index\";"));
    assert!(module.contains("export type UsesRoot = $root.RootReference;"));
    assert!(module.contains("$root.RootReferenceSchema"));
}

#[test]
fn valibot_layout_files_preserves_a_top_level_index_module() {
    let temp = temp_dir();
    let path = temp.path().join("valibot-index-module");
    let mut types = Types::default();
    let index_type = NamedDataType::new("IndexType", &mut types, |_, ndt| {
        ndt.module_path = "index".into();
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("UsesIndex", &mut types, |_, ndt| {
        ndt.module_path = "other".into();
        ndt.ty = Some(DataType::Reference(index_type.reference(vec![])));
    });
    NamedDataType::new("ChildUsesIndex", &mut types, |_, ndt| {
        ndt.module_path = "index::child".into();
        ndt.ty = Some(DataType::Reference(index_type.reference(vec![])));
    });

    Valibot::default()
        .layout(Layout::Files)
        .with_raw("export const runtime = true;")
        .export_to(&path, &types, specta_serde::Format)
        .unwrap();

    let root = std::fs::read_to_string(path.join("index.ts")).unwrap();
    let index = std::fs::read_to_string(path.join("$index.ts")).unwrap();
    let other = std::fs::read_to_string(path.join("other.ts")).unwrap();
    let child = std::fs::read_to_string(path.join("$index/child.ts")).unwrap();
    assert!(root.contains("export const runtime = true;"), "{root}");
    assert!(index.contains("export type IndexType = string"), "{index}");
    assert!(other.contains("from \"./$index\""), "{other}");
    assert!(child.contains("from \"../$index\""), "{child}");
}

#[test]
fn valibot_layout_files_imports_parent_module_files() {
    let temp = temp_dir();
    let path = temp.path().join("valibot-parent-reference");
    let mut types = Types::default();
    let parent = NamedDataType::new("ParentReference", &mut types, |_, ndt| {
        ndt.module_path = "parent".into();
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("UsesParent", &mut types, |_, ndt| {
        ndt.module_path = "parent::child".into();
        ndt.ty = Some(DataType::Reference(parent.reference(vec![])));
    });

    Valibot::default()
        .layout(Layout::Files)
        .export_to(&path, &types, specta_serde::Format)
        .unwrap();

    let child = std::fs::read_to_string(path.join("parent/child.ts")).unwrap();
    assert!(child.contains("import * as parent from \"../parent\";"));
    assert!(!child.contains(" from \".\";"));
    assert!(child.contains("parent.ParentReferenceSchema"));
}

#[test]
fn valibot_layout_files_empty_export_allows_a_missing_directory() {
    let temp = temp_dir();
    let path = temp.path().join("missing-empty-export");
    assert!(!path.exists());

    Valibot::default()
        .layout(Layout::Files)
        .export_to(&path, &Types::default(), specta_serde::Format)
        .unwrap();

    assert!(!path.exists());
}

#[test]
fn valibot_layout_namespaces_rejects_module_type_collisions() {
    let mut types = Types::default().register::<Testing>();
    NamedDataType::new("test", &mut types, |_, ndt| {
        ndt.module_path = "".into();
        ndt.ty = Some(Primitive::str.into());
    });

    let err = Valibot::default()
        .layout(Layout::Namespaces)
        .export(&types, specta_serde::Format)
        .unwrap_err();
    assert!(err.to_string().contains("namespace exports"));
}

#[test]
fn valibot_layout_namespaces_rejects_nested_schema_module_collisions() {
    let mut types = Types::default();
    NamedDataType::new("Foo", &mut types, |_, ndt| {
        ndt.module_path = "outer".into();
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("Nested", &mut types, |_, ndt| {
        ndt.module_path = "outer::FooSchema".into();
        ndt.ty = Some(Primitive::str.into());
    });

    let err = Valibot::default()
        .layout(Layout::Namespaces)
        .export(&types, specta_serde::Format)
        .unwrap_err();
    assert!(err.to_string().contains("namespace exports"), "{err}");
}

#[test]
fn valibot_with_raw_exports_to_files_index() {
    let temp = temp_dir();
    let path = temp.path().join("valibot-layout-files-raw");
    Valibot::default()
        .layout(Layout::Files)
        .with_raw("export const first = 1;")
        .with_raw("export const second = 2;")
        .export_to(&path, &Types::default(), specta_serde::Format)
        .unwrap();

    let index = std::fs::read_to_string(path.join("index.ts")).unwrap();
    assert!(index.contains("export const first = 1;\nexport const second = 2;"));
}

#[test]
fn valibot_uses_serde_transformed_resolved_types() {
    let types = Types::default().register::<SerdeTaggedEnum>();
    let serde_out = Valibot::default()
        .export(&types, specta_serde::Format)
        .unwrap();

    assert!(serde_out.contains("type: v.literal(\"unit\")"));
    assert!(serde_out.contains("type: v.literal(\"string_value\")"));
    assert!(serde_out.contains(
        r"data: v.pipe(v.string(), v.check((value) => !/[\uD800-\uDFFF]/u.test(value)))"
    ));
}

#[test]
fn valibot_empty_named_shapes_are_strict() {
    let empty_struct = export_for::<EmptyStruct>().unwrap();
    assert!(empty_struct.contains("$spectaObject({})"));

    let empty_variant = export_for::<EmptyNamedVariant>().unwrap();
    assert!(empty_variant.contains("$spectaObject({"));
    assert!(empty_variant.contains("}, true)"));
}

#[test]
fn valibot_untagged_matching_field_name_is_not_strict() {
    for rendered in [
        export_for::<UntaggedMatchingField>().unwrap(),
        Valibot::default()
            .export(
                &Types::default().register::<UntaggedMatchingField>(),
                specta_serde::PhasesFormat,
            )
            .unwrap(),
    ] {
        assert!(rendered.contains("$spectaObject({"));
        assert!(rendered.contains("$spectaObject({})"));
        assert!(!rendered.contains("}, true)"));
    }

    for value in [
        serde_json::json!({ "Variant": "value", "extra": true }),
        serde_json::json!({ "extra": true }),
    ] {
        assert!(serde_json::from_value::<UntaggedMatchingField>(value).is_ok());
    }
}

#[test]
fn valibot_layout_files_preserves_unrelated_typescript_files() {
    let types = Types::default().register::<Testing>().register::<Another>();
    let temp = TempDir::new_in(temp_root()).unwrap();
    let path = temp.path().join("valibot-layout-files-preserve");
    std::fs::create_dir_all(&path).unwrap();

    let keep_path = path.join("keep.ts");
    std::fs::write(&keep_path, "export const keep = true;\n").unwrap();

    Valibot::default()
        .layout(Layout::Files)
        .export_to(&path, &types, specta_serde::Format)
        .unwrap();

    assert!(keep_path.exists());
    assert!(
        std::fs::read_to_string(&keep_path)
            .unwrap()
            .contains("export const keep = true;")
    );
}

#[test]
fn typescript_layout_files_preserves_unrelated_typescript_files() {
    let types = Types::default().register::<Testing>().register::<Another>();
    let temp = TempDir::new_in(temp_root()).unwrap();
    let path = temp.path().join("typescript-layout-files-preserve");
    std::fs::create_dir_all(&path).unwrap();

    let keep_path = path.join("keep.ts");
    std::fs::write(&keep_path, "export const keep = true;\n").unwrap();

    Typescript::default()
        .layout(specta_typescript::Layout::Files)
        .export_to(&path, &types, specta_serde::Format)
        .unwrap();

    assert!(keep_path.exists());
    assert!(
        std::fs::read_to_string(&keep_path)
            .unwrap()
            .contains("export const keep = true;")
    );
}

#[test]
fn valibot_recursive_types_use_lazy() {
    let types = Types::default().register::<Recursive>();
    let out = Valibot::default()
        .export(&types, specta_serde::PhasesFormat)
        .unwrap();
    assert!(out.contains("v.lazy(() => RecursiveSchema)"));
}

#[test]
fn valibot_recursive_named_references_in_intersections_remain_lazy() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct InlineRecursive {
        #[specta(inline)]
        child: Option<Box<InlineRecursive>>,
    }

    fn find_recursive_reference(ty: &DataType) -> Option<DataType> {
        match ty {
            DataType::Reference(Reference::Named(reference)) => match &reference.inner {
                specta::datatype::NamedReferenceType::Recursive(_) => Some(ty.clone()),
                specta::datatype::NamedReferenceType::Inline { dt, .. } => {
                    find_recursive_reference(dt)
                }
                specta::datatype::NamedReferenceType::Reference { .. } => None,
            },
            DataType::Struct(strct) => match &strct.fields {
                specta::datatype::Fields::Named(fields) => fields
                    .fields
                    .iter()
                    .find_map(|(_, field)| field.ty.as_ref().and_then(find_recursive_reference)),
                specta::datatype::Fields::Unnamed(fields) => fields
                    .fields
                    .iter()
                    .find_map(|field| field.ty.as_ref().and_then(find_recursive_reference)),
                specta::datatype::Fields::Unit => None,
            },
            DataType::List(list) => find_recursive_reference(&list.ty),
            DataType::Nullable(inner) => find_recursive_reference(inner),
            _ => None,
        }
    }

    let mut types = Types::default();
    let root = InlineRecursive::definition(&mut types);
    let DataType::Reference(Reference::Named(root_reference)) = root else {
        panic!("recursive type definition should be a named reference");
    };
    let root_ty = types
        .get(&root_reference)
        .and_then(|ndt| ndt.ty.as_ref())
        .expect("recursive type should have a registered definition");
    let recursive = find_recursive_reference(root_ty)
        .expect("inline recursive type should contain a recursive named reference");

    let tag = specta::datatype::Struct::named()
        .field("kind", specta::datatype::Field::new(Primitive::str.into()))
        .build();
    let intersection = DataType::Intersection(vec![tag, recursive]);
    let rendered = primitives::inline(&Valibot::default(), &types, &intersection).unwrap();
    assert!(rendered.contains("v.lazy"), "{rendered}");
}

#[test]
fn valibot_inline_recursive_generics_keep_schema_arguments() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct GenericNode<T> {
        value: T,
        #[specta(inline)]
        children: Vec<GenericNode<T>>,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct ConcreteHolder {
        #[specta(inline)]
        node: GenericNode<i32>,
    }

    let mut types = Types::default();
    let holder = ConcreteHolder::definition(&mut types);
    let ty = types
        .into_unsorted_iter()
        .find(|ndt| ndt.name == "GenericNode")
        .and_then(|ndt| ndt.ty.as_ref())
        .expect("generic node definition should be registered");
    let rendered = primitives::inline(&Valibot::default(), &types, ty).unwrap();
    assert!(
        rendered.contains("v.lazy(() => GenericNodeSchema(T))"),
        "{rendered}"
    );

    let DataType::Reference(Reference::Named(holder)) = holder else {
        panic!("holder definition should be a named reference");
    };
    let ty = types
        .get(&holder)
        .and_then(|ndt| ndt.ty.as_ref())
        .expect("holder definition should be registered");
    let rendered = primitives::inline(&Valibot::default(), &types, ty).unwrap();
    assert!(
        rendered
            .contains("v.lazy(() => GenericNodeSchema(v.pipe(v.number(), v.integer(), v.minValue(-2147483648), v.maxValue(2147483647))))"),
        "{rendered}"
    );
    assert!(!rendered.contains("GenericNodeSchema(T)"), "{rendered}");
}

#[test]
fn valibot_map_key_validation_ignores_skipped_variants() {
    #[derive(Type, Serialize, Deserialize, Eq, Hash, PartialEq)]
    #[specta(collect = false)]
    enum SkippedKey {
        Visible,
        #[specta(skip)]
        #[serde(skip)]
        Hidden(String),
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct SkippedKeyMap {
        values: HashMap<SkippedKey, String>,
    }

    let rendered = export_for::<SkippedKeyMap>().unwrap();
    assert!(rendered.contains("v.literal(\"Visible\")"), "{rendered}");
    assert!(!rendered.contains("Hidden"), "{rendered}");
}

#[test]
fn valibot_reserved_type_name_errors() {
    let mut types = Types::default();
    NamedDataType::new("class", &mut types, |_, ndt| {
        ndt.ty = Some(DataType::Primitive(Primitive::i8));
    });
    let err = Valibot::default()
        .export(&types, specta_serde::Format)
        .unwrap_err();
    assert!(err.to_string().contains("reserved keyword"));

    let mut types = Types::default();
    NamedDataType::new("GenericV", &mut types, |_, ndt| {
        let generic = specta::datatype::GenericDefinition::new("v".into(), None);
        ndt.generics = Cow::Owned(vec![generic.clone()]);
        ndt.ty = Some(DataType::Generic(generic.reference()));
    });
    let err = Valibot::default()
        .export(&types, specta_serde::Format)
        .unwrap_err();
    assert!(err.to_string().contains("<generic v>"));
    assert!(err.to_string().contains("reserved keyword"));
}

#[test]
fn valibot_layout_files_errors_on_export() {
    let types = Types::default();
    let err = Valibot::default()
        .layout(Layout::Files)
        .export(&types, specta_serde::Format)
        .unwrap_err();
    assert!(err.to_string().contains("Unable to export layout Files"));
}

#[test]
fn valibot_integers_use_integer_pipeline() {
    // Valibot validates integer and range constraints through pipeline actions.
    #[derive(Type)]
    #[specta(collect = false)]
    struct Ints {
        signed: i32,
        unsigned: u8,
        floating: f64,
    }

    let out = export_for::<Ints>().unwrap();
    assert!(out.contains(
        "signed: v.pipe(v.number(), v.integer(), v.minValue(-2147483648), v.maxValue(2147483647))"
    ));
    assert!(
        out.contains("unsigned: v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(255))")
    );
    assert!(out.contains("floating: v.nullable(v.pipe(v.number(), v.finite()))"));
}

#[test]
fn valibot_narrow_floats_use_representable_ranges() {
    let mut types = Types::default();
    NamedDataType::new("Half", &mut types, |_, ndt| {
        ndt.ty = Some(DataType::Primitive(Primitive::f16));
    });
    NamedDataType::new("Single", &mut types, |_, ndt| {
        ndt.ty = Some(DataType::Primitive(Primitive::f32));
    });

    let out = Valibot::default()
        .export(&types, specta_serde::Format)
        .unwrap();
    assert!(out.contains(
        "export const HalfSchema: v.GenericSchema<Half> = v.nullable(v.pipe(v.number(), v.finite(), v.minValue(-65504), v.maxValue(65504)))"
    ), "{out}");
    assert!(out.contains(
        "export const SingleSchema: v.GenericSchema<Single> = v.nullable(v.pipe(v.number(), v.finite(), v.minValue(-3.4028235e38), v.maxValue(3.4028235e38)))"
    ), "{out}");
}

#[derive(Type)]
#[specta(collect = false)]
struct ValibotIntegerKey(i32);

#[derive(Type)]
#[specta(collect = false)]
enum ValibotFiniteKey {
    First,
    Second,
}

#[derive(Type)]
#[specta(collect = false)]
struct ValibotWireTypes {
    character: char,
    integer_keys: HashMap<i32, String>,
    boolean_keys: HashMap<bool, String>,
    newtype_keys: HashMap<ValibotIntegerKey, String>,
    enum_keys: HashMap<ValibotFiniteKey, String>,
}

#[test]
fn valibot_char_and_json_map_key_wire_schemas() {
    let rendered = export_for::<ValibotWireTypes>().unwrap();
    assert!(rendered.contains("[...value].length === 1"));
    assert!(rendered.contains(
        r"$spectaRecord(v.pipe(v.string(), v.regex(/^-?\d+$/), v.check((value) => Number(value) >= -2147483648 && Number(value) <= 2147483647)), v.pipe(v.string(), v.check((value) => !/[\uD800-\uDFFF]/u.test(value))))"
    ));
    assert!(rendered.contains(
        r#"$spectaRecord(v.picklist(["true", "false"]), v.pipe(v.string(), v.check((value) => !/[\uD800-\uDFFF]/u.test(value))))"#
    ));
    assert!(rendered.contains(
        r#"$spectaRecord(v.union([v.literal("First"), v.literal("Second")]), v.pipe(v.string(), v.check((value) => !/[\uD800-\uDFFF]/u.test(value))))"#
    ));
}

#[test]
fn valibot_generic_schema_uses_generic_schema() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct GenericWrapper<T> {
        value: T,
    }

    let types = Types::default().register::<GenericWrapper<String>>();
    let out = Valibot::default()
        .export(&types, specta_serde::Format)
        .unwrap();
    assert!(out.contains("extends v.GenericSchema"));
}

fn temp_dir() -> TempDir {
    TempDir::new_in(temp_root()).unwrap()
}

fn export_for<T: Type>() -> Result<String, specta_valibot::Error> {
    let types = Types::default().register::<T>();
    Valibot::default().export(&types, specta_serde::Format)
}

/// A trailing `#[serde(default)]` tuple element is optional on deserialize
/// (serde accepts `[1]`): the deserialize half must render
/// a union of exact-length strict tuples.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ValibotTupleDefault(u8, #[serde(default)] u8);

#[test]
fn valibot_tuple_default_phases() {
    let rendered = Valibot::default()
        .export(
            &Types::default().register::<ValibotTupleDefault>(),
            specta_serde::PhasesFormat,
        )
        .expect("Valibot should support defaulted tuple elements under PhasesFormat");

    insta::assert_snapshot!("valibot-tuple-default-phases", rendered);
    assert!(
        rendered.contains("ValibotTupleDefault_DeserializeSchema")
            && rendered.contains("= v.union([v.strictTuple(["),
        "the deserialize half must accept exact tuple prefixes: {rendered}"
    );
}

/// Control: valibot's tuple arm filters live fields already, so a skip-reduced
/// defaulted tuple sizes over live elements only.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ValibotSkipSlotTuple(#[serde(skip)] u8, #[serde(default)] u8);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ValibotTupleVariantDefault {
    Value(u8, #[serde(default)] u8),
}

#[test]
fn valibot_skip_slot_tuple_phases() {
    let rendered = Valibot::default()
        .export(
            &Types::default().register::<ValibotSkipSlotTuple>(),
            specta_serde::PhasesFormat,
        )
        .expect("Valibot should support skip-reduced defaulted tuple structs");

    assert!(
        rendered.contains("ValibotSkipSlotTuple_DeserializeSchema: v.GenericSchema<ValibotSkipSlotTuple_Deserialize> = v.union([v.strictTuple([]), v.strictTuple([v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(255))])])"),
        "only the live element is sized, and it is optional on deserialize: {rendered}"
    );
    assert!(
        rendered.contains("ValibotSkipSlotTuple_SerializeSchema: v.GenericSchema<ValibotSkipSlotTuple_Serialize> = v.strictTuple([v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(255))])"),
        "serialize keeps the single live element required: {rendered}"
    );
}

#[test]
fn valibot_tuple_variant_default_phases() {
    let rendered = Valibot::default()
        .export(
            &Types::default().register::<ValibotTupleVariantDefault>(),
            specta_serde::PhasesFormat,
        )
        .unwrap();

    assert!(
        rendered.contains("Value: v.union([v.strictTuple(["),
        "the trailing defaulted variant field must use exact tuple prefixes: {rendered}"
    );
}

/// Top-level documentation containing a terminator: */ still remains valid.
#[deprecated(note = "Use the replacement instead")]
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ValibotCoverage<T = String> {
    /// Field documentation.
    #[deprecated(note = "Use `renamed` instead")]
    #[serde(rename = "quote\"slash\\line\nseparator")]
    renamed: String,
    generic: T,
    optional: Option<bool>,
    tuple: (i32, String),
    list: Vec<f64>,
    map: HashMap<String, u16>,
}

#[derive(Type)]
#[specta(collect = false)]
struct ValibotGenericDefaults<T = String, U = T> {
    first: T,
    second: U,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ValibotFlattenInner {
    inner: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ValibotFlattenOuter {
    outer: bool,
    #[serde(flatten)]
    inner: ValibotFlattenInner,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ValibotOptionalFlatten {
    id: String,
    #[serde(flatten)]
    inner: Option<ValibotFlattenInner>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ValibotEnumCoverage {
    /// Unit documentation.
    Unit,
    Newtype(String),
    Tuple(i32, bool),
    Named {
        /// Nested field documentation.
        value: String,
    },
}

#[derive(Type)]
#[specta(collect = false)]
struct AForwardReference {
    later: ZLaterReference,
}

#[derive(Type)]
#[specta(collect = false)]
struct ZLaterReference {
    value: String,
}

#[test]
#[allow(deprecated)]
fn valibot_datatype_generics_comments_and_escaping() {
    let types = Types::default()
        .register::<ValibotCoverage>()
        .register::<ValibotGenericDefaults>()
        .register::<ValibotEnumCoverage>()
        .register::<ValibotFlattenOuter>()
        .register::<ValibotOptionalFlatten>()
        .register::<ValibotWireTypes>()
        .register::<AForwardReference>();
    let rendered = Valibot::default()
        .export(&types, specta_serde::Format)
        .unwrap();

    insta::assert_snapshot!("valibot-parity-coverage", rendered);
    assert!(rendered.contains("export type ValibotCoverage<T = string>"));
    assert!(rendered.contains("@deprecated Use the replacement instead"));
    assert!(rendered.contains("Field documentation."));
    assert!(rendered.contains(r#""quote\"slash\\line\nseparator""#));
    assert!(rendered.contains("generic: T"));
    assert!(rendered.contains(
        "export function ValibotCoverageSchema<T extends v.GenericSchema>(T: T): v.GenericSchema<ValibotCoverage<v.InferInput<T>>, ValibotCoverage<v.InferOutput<T>>>;"
    ));
    assert!(rendered.contains(
        "export function ValibotGenericDefaultsSchema<T extends v.GenericSchema, U extends v.GenericSchema>(T: T, U: U): v.GenericSchema<ValibotGenericDefaults<v.InferInput<T>, v.InferInput<U>>, ValibotGenericDefaults<v.InferOutput<T>, v.InferOutput<U>>>;"
    ));
    assert!(rendered.contains("v.array(v.nullable(v.pipe(v.number(), v.finite())))"));
    assert!(rendered.contains(
        r"$spectaRecord(v.pipe(v.string(), v.check((value) => !/[\uD800-\uDFFF]/u.test(value))), v.pipe(v.number(), v.integer(), v.minValue(0), v.maxValue(65535)))"
    ));
    assert!(rendered.contains("v.lazy(() => ZLaterReferenceSchema)"));
    assert!(rendered.contains("ValibotFlattenOuterSchema"));
    assert!(rendered.contains("$spectaIntersect(["));
    assert!(rendered.contains("ValibotOptionalFlattenSchema"));
}

#[test]
fn valibot_header_raw_runtime_and_manual_rendering() {
    let types = Types::default().register::<Another>();
    let rendered = Valibot::default()
        .header("// custom header")
        .with_raw("")
        .with_raw("export const raw = true;")
        .framework_runtime(|mut exporter| {
            let types = exporter.render_types()?;
            Ok(Cow::Owned(format!("export const runtime = true;\n{types}")))
        })
        .export(&types, specta_serde::Format)
        .unwrap();

    assert!(rendered.starts_with("// custom header\nimport * as v from \"valibot\";"));
    assert!(rendered.contains("export const runtime = true;"));
    assert!(rendered.contains("AnotherSchema"));
    assert!(rendered.contains("export const raw = true;"));
    assert!(
        rendered.find("export const runtime = true;").unwrap()
            < rendered.find("export const raw = true;").unwrap()
    );
    assert_eq!(rendered.matches("export const AnotherSchema").count(), 1);
}

#[test]
fn valibot_framework_export_formats_generic_named_datatypes() {
    let types = Types::default().register::<ValibotGenericDefaults>();
    let rendered = Valibot::default()
        .framework_runtime(|exporter| {
            let generic_defaults = exporter
                .types
                .into_unsorted_iter()
                .filter(|ndt| ndt.name == "ValibotGenericDefaults");
            Ok(Cow::Owned(exporter.export(generic_defaults, "")?))
        })
        .export(&types, specta_serde::Format)
        .unwrap();

    assert!(rendered.contains("export type ValibotGenericDefaults<T = string, U = T>"));
    assert!(rendered.contains("first: T"));
}

#[test]
fn valibot_type_alias_cleanup_preserves_jsdoc_markdown_breaks() {
    #[doc = "First line  \nSecond line"]
    #[derive(Type)]
    #[specta(collect = false)]
    struct Documented {
        value: String,
    }

    let rendered = Valibot::default()
        .export(
            &Types::default().register::<Documented>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(rendered.contains("First line  \n"), "{rendered}");
}

#[test]
fn valibot_framework_inline_formats_inline_named_reference_children() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Child {
        value: String,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct Parent {
        #[specta(inline)]
        child: Child,
    }

    struct StringsAsBooleans;

    impl Format for StringsAsBooleans {
        fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
            Ok(Cow::Owned(types.clone()))
        }

        fn map_type(
            &'_ self,
            _: &Types,
            dt: &DataType,
        ) -> Result<Cow<'_, DataType>, specta::FormatError> {
            Ok(Cow::Owned(
                if matches!(dt, DataType::Primitive(Primitive::str)) {
                    DataType::Primitive(Primitive::bool)
                } else {
                    dt.clone()
                },
            ))
        }
    }

    let mut types = Types::default();
    let parent = Parent::definition(&mut types);
    let DataType::Reference(Reference::Named(parent)) = parent else {
        panic!("expected a named parent reference");
    };
    let parent = types.get(&parent).unwrap().ty.as_ref().unwrap();
    let DataType::Struct(parent) = parent else {
        panic!("expected a parent struct");
    };
    let specta::datatype::Fields::Named(fields) = &parent.fields else {
        panic!("expected named parent fields");
    };
    let inline_child = fields.fields[0].1.ty.clone().unwrap();

    let rendered = Valibot::default()
        .framework_runtime(move |exporter| {
            Ok(Cow::Owned(format!(
                "export const mapped = {};",
                exporter.inline(&inline_child)?
            )))
        })
        .export(&types, StringsAsBooleans)
        .unwrap();

    assert!(
        rendered.contains("export const mapped = $spectaObject({\n\tvalue: v.boolean(),\n});"),
        "{rendered}"
    );
}

#[test]
fn valibot_files_framework_runtime_deduplicates_body_imports() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct FrameworkRoot {
        child: testing::Testing,
    }

    let mut types = Types::default().register::<FrameworkRoot>();
    types.iter_mut(|ndt| {
        if ndt.name == "FrameworkRoot" {
            ndt.module_path = Cow::Borrowed("");
        }
    });
    let DataType::Reference(reference) = testing::Testing::definition(&mut types) else {
        panic!("expected a module reference");
    };
    let temp = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&temp).unwrap();
    let temp = TempDir::new_in(temp).unwrap();

    Valibot::default()
        .layout(Layout::Files)
        .framework_runtime(move |mut exporter| {
            let reference = exporter.reference(&reference)?;
            let types = exporter.render_types()?;
            Ok(Cow::Owned(format!(
                "{types}\nexport const runtime = {reference};"
            )))
        })
        .export_to(temp.path(), &types, specta_serde::Format)
        .unwrap();

    let index = std::fs::read_to_string(temp.path().join("index.ts")).unwrap();
    assert_eq!(
        index
            .matches("import * as test$valibot$testing from \"./test/valibot/testing\";")
            .count(),
        1,
        "{index}"
    );
}

#[test]
fn valibot_nested_errors_include_the_field_path() {
    let err = export_for::<StructWithBigInt>().unwrap_err().to_string();
    assert!(
        err.contains("StructWithBigInt.a"),
        "unexpected error: {err}"
    );
}
