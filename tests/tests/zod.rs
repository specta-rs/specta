use std::{borrow::Cow, collections::HashMap, iter, path::Path};

use serde::{Deserialize, Serialize};
use specta::{
    Type, Types,
    datatype::{DataType, NamedDataType, Primitive, Reference},
};
use specta_typescript::Typescript;
use specta_util::Remapper;
use specta_zod::{Any, Layout, Never, Unknown, Zod, define, primitives};
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

    /// This documentation mentions test$zod$testing.Testing verbatim.
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
}

#[derive(Type)]
struct UsesKeywordModule {
    value: r#type::KeywordModule,
}

fn inline_for<T: Type>(zod: &Zod) -> Result<String, specta_zod::Error> {
    let mut types = Types::default();
    let dt = T::definition(&mut types);
    primitives::inline(zod, &types, &dt)
}

fn temp_root() -> std::path::PathBuf {
    let temp_root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&temp_root).unwrap();
    temp_root
}

#[test]
fn zod_export_smoke() {
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
    let out = Zod::default().export(&types, specta_serde::Format).unwrap();

    assert!(out.contains("import { z } from \"zod\";"));
    assert!(out.contains("export const DemoSchema"));
    assert!(out.contains("export type Demo ="));
    assert!(out.contains("DemoSchema: z.ZodType<Demo>"));
}

#[test]
fn zod_primitives_smoke() {
    let (types, dts) = crate::types();
    let zod = Zod::default();

    for (_, ty) in &dts {
        let rendered = primitives::inline(&zod, &types, ty).unwrap();
        assert!(!rendered.is_empty());
    }

    let ndt = dts
        .iter()
        .find_map(|(_, ty)| match ty {
            DataType::Reference(Reference::Named(r)) => types.get(r),
            _ => None,
        })
        .unwrap();

    let rendered = primitives::export(&zod, &types, iter::once(ndt), "").unwrap();
    assert!(rendered.contains("Schema"));
}

#[test]
fn zod_bigint_forbidden_by_default() {
    for_bigint_types!(T -> |_| {
        assert!(
            inline_for::<T>(&Zod::default()).is_err(),
            "bigint-style primitives must be forbidden by default"
        );
    });
}

#[test]
fn zod_wrappers_serde_roundtrip() {
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
fn zod_bigint_override_via_define() {
    // `specta_zod::define` is the escape hatch for the forbidden bigint
    // primitives: remap them to a Zod schema of your choosing, mirroring the
    // Typescript exporter's `define`.
    let remapper = Remapper::new().rule(Primitive::i64.into(), define("z.bigint()").into());
    let dt = remapper.remap_dt(Primitive::i64.into());

    assert_eq!(
        primitives::inline(&Zod::default(), &Types::default(), &dt).unwrap(),
        "z.bigint()"
    );
}

#[test]
fn zod_high_level_export_supports_zod_opaque_types() {
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
    assert!(wrappers.contains("any: z.any()"));
    assert!(wrappers.contains("unknown: z.unknown()"));
    assert!(wrappers.contains("never: z.never()"));

    let types = Remapper::new()
        .rule(Primitive::i128.into(), define("z.bigint()").into())
        .remap_types(Types::default().register::<StructWithBigInt>());
    let defined = Zod::default().export(&types, specta_serde::Format).unwrap();
    assert!(defined.contains("a: unknown"));
    assert!(defined.contains("a: z.bigint()"));
}

#[test]
fn zod_define_map_key_uses_a_valid_typescript_property_key() {
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
        .rule(Primitive::i64.into(), define("z.string()").into())
        .remap_types(Types::default().register::<DefinedMapKey>());
    let out = Zod::default().export(&types, specta_serde::Format).unwrap();

    assert!(
        out.contains("values: Partial<{ [key in string]: string }>"),
        "unexpected export: {out}"
    );
    assert!(out.contains("values: z.record(z.string(), z.string())"));
    assert!(out.contains("named: Partial<{ [key in string]: string }>"));
    assert!(out.contains("named: z.record(z.string(), z.string())"));
    assert!(!out.contains("[key in unknown]"));
}

#[test]
fn zod_bigint_errors_propagate_from_nested_types() {
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

    assert!(inline_for::<HashMap<usize, String>>(&Zod::default()).is_err());
    assert!(inline_for::<HashMap<isize, String>>(&Zod::default()).is_err());
}

#[test]
fn zod_layout_duplicate_typenames() {
    let types = Types::default().register::<Testing>().register::<Another>();
    let err = Zod::default()
        .export(&types, specta_serde::Format)
        .unwrap_err();
    assert!(err.to_string().contains("Detected multiple types"));

    let module_prefixed = Zod::default()
        .layout(Layout::ModulePrefixedName)
        .export(&types, specta_serde::Format)
        .unwrap();
    assert!(module_prefixed.contains("TestingSchema"));
    assert!(module_prefixed.contains("testing2"));
}

#[test]
fn zod_module_prefixed_duplicate_checks_use_rendered_names() {
    fn add_type(types: &mut Types, name: &'static str, module_path: &'static str) {
        NamedDataType::new(name, types, |_, ndt| {
            ndt.module_path = module_path.into();
            ndt.ty = Some(Primitive::str.into());
        });
    }

    let mut distinct = Types::default();
    add_type(&mut distinct, "foo_Bar", "");
    add_type(&mut distinct, "Bar", "foo");
    let out = Zod::default()
        .layout(Layout::ModulePrefixedName)
        .export(&distinct, specta_serde::Format)
        .unwrap();
    assert!(out.contains("export type _foo_Bar = string"));
    assert!(out.contains("export type foo_Bar = string"));

    let mut colliding = Types::default();
    add_type(&mut colliding, "_foo_Bar", "");
    add_type(&mut colliding, "foo_Bar", "_");
    let err = Zod::default()
        .layout(Layout::ModulePrefixedName)
        .export(&colliding, specta_serde::Format)
        .unwrap_err();
    assert!(err.to_string().contains("Detected multiple types"));
}

#[test]
fn zod_layout_files_export_to() {
    let types = Types::default().register::<Testing>().register::<Another>();
    let temp = temp_dir();
    let path = temp.path().join("zod-layout-files");

    Zod::default()
        .layout(Layout::Files)
        .export_to(&path, &types, specta_serde::Format)
        .unwrap();

    let output = crate::fs_to_string(Path::new(&path)).unwrap();
    assert!(output.contains(".ts"));
    assert!(output.contains("import { z } from \"zod\";"));
    assert!(output.contains("test$zod$testing.Testing verbatim"));
}

#[test]
fn zod_layout_namespaces() {
    let types = Types::default().register::<Testing>().register::<Another>();
    let rendered = Zod::default()
        .layout(Layout::Namespaces)
        .export(&types, specta_serde::Format)
        .unwrap();

    insta::assert_snapshot!("zod-layout-namespaces", rendered);
    assert!(rendered.contains("namespace $s$"));
    assert!(rendered.contains("export namespace testing"));
    assert!(rendered.contains("testing.TestingSchema)"));
    assert!(rendered.contains("export import test = $s$.test;"));
}

#[test]
fn zod_layout_namespaces_reexports_root_schema() {
    let mut types = Types::default();
    NamedDataType::new("Root", &mut types, |_, ndt| {
        ndt.module_path = "".into();
        ndt.ty = Some(Primitive::str.into());
    });

    let rendered = Zod::default()
        .layout(Layout::Namespaces)
        .export(&types, specta_serde::Format)
        .unwrap();
    assert!(rendered.contains("export import Root = $s$.Root;"));
    assert!(rendered.contains("export import RootSchema = $s$.RootSchema;"));
}

#[test]
fn zod_layout_sanitises_reserved_module_identifiers() {
    let types = Types::default().register::<UsesKeywordModule>();
    let rendered = Zod::default()
        .layout(Layout::Namespaces)
        .export(&types, specta_serde::Format)
        .unwrap();

    assert!(rendered.contains("export namespace $type"));
    assert!(rendered.contains(".$type.KeywordModule"));
    assert!(!rendered.contains("namespace type"));
}

#[test]
fn zod_layout_files_sanitises_the_zod_prelude_binding() {
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
    let path = temp.path().join("zod-prelude-binding");
    let mut types = Types::default();
    let z_type = NamedDataType::new("PreludeCollision", &mut types, |_, ndt| {
        ndt.module_path = "z".into();
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("UsesZ", &mut types, |_, ndt| {
        ndt.module_path = "other".into();
        ndt.ty = Some(DataType::Reference(z_type.reference(vec![])));
    });
    Zod::default()
        .layout(Layout::Files)
        .export_to(&path, &types, specta_serde::Format)
        .unwrap();

    let mut rendered = String::new();
    read_typescript_files(&path, &mut rendered);
    assert!(rendered.contains("import * as $z from"));
    assert!(!rendered.contains("import * as z from"));
    assert!(rendered.contains("$z.PreludeCollisionSchema"));
}

#[test]
fn zod_layout_files_qualifies_root_types_from_modules() {
    let temp = temp_dir();
    let path = temp.path().join("zod-root-reference");
    let mut types = Types::default();
    let root = NamedDataType::new("RootReference", &mut types, |_, ndt| {
        ndt.module_path = "".into();
        ndt.ty = Some(Primitive::str.into());
    });
    NamedDataType::new("UsesRoot", &mut types, |_, ndt| {
        ndt.module_path = "other".into();
        ndt.ty = Some(DataType::Reference(root.reference(vec![])));
    });

    Zod::default()
        .layout(Layout::Files)
        .export_to(&path, &types, specta_serde::Format)
        .unwrap();

    let module = std::fs::read_to_string(path.join("other.ts")).unwrap();
    assert!(module.contains("import * as $root from \"./index\";"));
    assert!(module.contains("export type UsesRoot = $root.RootReference;"));
    assert!(module.contains("$root.RootReferenceSchema"));
}

#[test]
fn zod_layout_namespaces_rejects_module_type_collisions() {
    let mut types = Types::default().register::<Testing>();
    NamedDataType::new("test", &mut types, |_, ndt| {
        ndt.module_path = "".into();
        ndt.ty = Some(Primitive::str.into());
    });

    let err = Zod::default()
        .layout(Layout::Namespaces)
        .export(&types, specta_serde::Format)
        .unwrap_err();
    assert!(err.to_string().contains("namespace exports"));
}

#[test]
fn zod_with_raw_exports_to_files_index() {
    let temp = temp_dir();
    let path = temp.path().join("zod-layout-files-raw");
    Zod::default()
        .layout(Layout::Files)
        .with_raw("export const first = 1;")
        .with_raw("export const second = 2;")
        .export_to(&path, &Types::default(), specta_serde::Format)
        .unwrap();

    let index = std::fs::read_to_string(path.join("index.ts")).unwrap();
    assert!(index.contains("export const first = 1;\nexport const second = 2;"));
}

#[test]
fn zod_uses_serde_transformed_resolved_types() {
    let types = Types::default().register::<SerdeTaggedEnum>();
    let serde_out = Zod::default().export(&types, specta_serde::Format).unwrap();

    assert!(serde_out.contains("type: z.literal(\"unit\")"));
    assert!(serde_out.contains("type: z.literal(\"string_value\")"));
    assert!(serde_out.contains("data: z.string()"));
}

#[test]
fn zod_empty_named_shapes_are_strict() {
    let empty_struct = export_for::<EmptyStruct>().unwrap();
    assert!(empty_struct.contains("z.object({})"));

    let empty_variant = export_for::<EmptyNamedVariant>().unwrap();
    assert!(empty_variant.contains("z.strictObject({"));
}

#[test]
fn zod_untagged_matching_field_name_is_not_strict() {
    for rendered in [
        export_for::<UntaggedMatchingField>().unwrap(),
        Zod::default()
            .export(
                &Types::default().register::<UntaggedMatchingField>(),
                specta_serde::PhasesFormat,
            )
            .unwrap(),
    ] {
        assert!(rendered.contains("z.object({"));
        assert!(rendered.contains("z.object({})"));
        assert!(!rendered.contains("z.strictObject({"));
    }

    for value in [
        serde_json::json!({ "Variant": "value", "extra": true }),
        serde_json::json!({ "extra": true }),
    ] {
        assert!(serde_json::from_value::<UntaggedMatchingField>(value).is_ok());
    }
}

#[test]
fn zod_layout_files_preserves_unrelated_typescript_files() {
    let types = Types::default().register::<Testing>().register::<Another>();
    let temp = TempDir::new_in(temp_root()).unwrap();
    let path = temp.path().join("zod-layout-files-preserve");
    std::fs::create_dir_all(&path).unwrap();

    let keep_path = path.join("keep.ts");
    std::fs::write(&keep_path, "export const keep = true;\n").unwrap();

    Zod::default()
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
fn zod_recursive_types_use_lazy() {
    let types = Types::default().register::<Recursive>();
    let out = Zod::default()
        .export(&types, specta_serde::PhasesFormat)
        .unwrap();
    assert!(out.contains("z.lazy(() => RecursiveSchema)"));
}

#[test]
fn zod_reserved_type_name_errors() {
    let mut types = Types::default();
    NamedDataType::new("class", &mut types, |_, ndt| {
        ndt.ty = Some(DataType::Primitive(Primitive::i8));
    });
    let err = Zod::default()
        .export(&types, specta_serde::Format)
        .unwrap_err();
    assert!(err.to_string().contains("reserved keyword"));

    let mut types = Types::default();
    NamedDataType::new("GenericZ", &mut types, |_, ndt| {
        let generic = specta::datatype::GenericDefinition::new("z".into(), None);
        ndt.generics = Cow::Owned(vec![generic.clone()]);
        ndt.ty = Some(DataType::Generic(generic.reference()));
    });
    let err = Zod::default()
        .export(&types, specta_serde::Format)
        .unwrap_err();
    assert!(err.to_string().contains("<generic z>"));
    assert!(err.to_string().contains("reserved keyword"));
}

#[test]
fn zod_layout_files_errors_on_export() {
    let types = Types::default();
    let err = Zod::default()
        .layout(Layout::Files)
        .export(&types, specta_serde::Format)
        .unwrap_err();
    assert!(err.to_string().contains("Unable to export layout Files"));
}

#[test]
fn zod_integers_use_z_int() {
    // Zod 4: integers validate with `z.int()`; plain `z.number()` accepts floats.
    #[derive(Type)]
    #[specta(collect = false)]
    struct Ints {
        signed: i32,
        unsigned: u8,
        floating: f64,
    }

    let out = export_for::<Ints>().unwrap();
    assert!(out.contains("signed: z.int()"));
    assert!(out.contains("signed: z.int().min(-2147483648).max(2147483647)"));
    assert!(out.contains("unsigned: z.int()"));
    assert!(out.contains("unsigned: z.int().min(0).max(255)"));
    assert!(out.contains("floating: z.number().nullable()"));
}

#[derive(Type)]
#[specta(collect = false)]
struct ZodIntegerKey(i32);

#[derive(Type)]
#[specta(collect = false)]
enum ZodFiniteKey {
    First,
    Second,
}

#[derive(Type)]
#[specta(collect = false)]
struct ZodWireTypes {
    character: char,
    integer_keys: HashMap<i32, String>,
    boolean_keys: HashMap<bool, String>,
    newtype_keys: HashMap<ZodIntegerKey, String>,
    enum_keys: HashMap<ZodFiniteKey, String>,
}

#[test]
fn zod_char_and_json_map_key_wire_schemas() {
    let rendered = export_for::<ZodWireTypes>().unwrap();
    assert!(rendered.contains("[...value].length === 1"));
    assert!(rendered.contains(
        r"z.record(z.string().regex(/^-?\d+$/).refine((value) => Number(value) >= -2147483648 && Number(value) <= 2147483647), z.string())"
    ));
    assert!(rendered.contains(r#"z.partialRecord(z.enum(["true", "false"]), z.string())"#));
    assert!(rendered.contains(
        r#"z.partialRecord(z.union([z.literal("First"), z.literal("Second")]), z.string())"#
    ));
}

#[test]
fn zod_generic_schema_uses_zod_type() {
    // Zod 4 eliminated `z.ZodTypeAny`; the generic constraint must be `z.ZodType`.
    #[derive(Type)]
    #[specta(collect = false)]
    struct GenericWrapper<T> {
        value: T,
    }

    let types = Types::default().register::<GenericWrapper<String>>();
    let out = Zod::default().export(&types, specta_serde::Format).unwrap();
    assert!(out.contains("extends z.ZodType"));
    assert!(!out.contains("ZodTypeAny"));
}

fn temp_dir() -> TempDir {
    TempDir::new_in(temp_root()).unwrap()
}

fn export_for<T: Type>() -> Result<String, specta_zod::Error> {
    let types = Types::default().register::<T>();
    Zod::default().export(&types, specta_serde::Format)
}

/// A trailing `#[serde(default)]` tuple element is optional on deserialize
/// (serde accepts `[1]`): the deserialize half must render
/// `z.tuple([..., z.number().optional()])`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ZodTupleDefault(u8, #[serde(default)] u8);

#[test]
fn zod_tuple_default_phases() {
    let rendered = Zod::default()
        .export(
            &Types::default().register::<ZodTupleDefault>(),
            specta_serde::PhasesFormat,
        )
        .expect("Zod should support defaulted tuple elements under PhasesFormat");

    insta::assert_snapshot!("zod-tuple-default-phases", rendered);
    assert!(
        rendered.contains("z.int().min(0).max(255).optional()"),
        "the defaulted element must be `.optional()` in the deserialize half: {rendered}"
    );
}

/// Control: zod's tuple arm filters live fields already, so a skip-reduced
/// defaulted tuple sizes over live elements only.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ZodSkipSlotTuple(#[serde(skip)] u8, #[serde(default)] u8);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ZodTupleVariantDefault {
    Value(u8, #[serde(default)] u8),
}

#[test]
fn zod_skip_slot_tuple_phases() {
    let rendered = Zod::default()
        .export(
            &Types::default().register::<ZodSkipSlotTuple>(),
            specta_serde::PhasesFormat,
        )
        .expect("Zod should support skip-reduced defaulted tuple structs");

    assert!(
        rendered.contains("ZodSkipSlotTuple_DeserializeSchema: z.ZodType<ZodSkipSlotTuple_Deserialize> = z.tuple([z.int().min(0).max(255).optional()])"),
        "only the live element is sized, and it is optional on deserialize: {rendered}"
    );
    assert!(
        rendered.contains("ZodSkipSlotTuple_SerializeSchema: z.ZodType<ZodSkipSlotTuple_Serialize> = z.tuple([z.int().min(0).max(255)])"),
        "serialize keeps the single live element required: {rendered}"
    );
}

#[test]
fn zod_tuple_variant_default_phases() {
    let rendered = Zod::default()
        .export(
            &Types::default().register::<ZodTupleVariantDefault>(),
            specta_serde::PhasesFormat,
        )
        .unwrap();

    assert!(
        rendered.contains("z.int().min(0).max(255).optional()"),
        "the trailing defaulted variant field must be optional: {rendered}"
    );
}

/// Top-level documentation containing a terminator: */ still remains valid.
#[deprecated(note = "Use the replacement instead")]
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ZodCoverage<T = String> {
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
struct ZodGenericDefaults<T = String, U = T> {
    first: T,
    second: U,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ZodFlattenInner {
    inner: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ZodFlattenOuter {
    outer: bool,
    #[serde(flatten)]
    inner: ZodFlattenInner,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ZodOptionalFlatten {
    id: String,
    #[serde(flatten)]
    inner: Option<ZodFlattenInner>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ZodEnumCoverage {
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
fn zod_datatype_generics_comments_and_escaping() {
    let types = Types::default()
        .register::<ZodCoverage>()
        .register::<ZodGenericDefaults>()
        .register::<ZodEnumCoverage>()
        .register::<ZodFlattenOuter>()
        .register::<ZodOptionalFlatten>()
        .register::<ZodWireTypes>()
        .register::<AForwardReference>();
    let rendered = Zod::default().export(&types, specta_serde::Format).unwrap();

    insta::assert_snapshot!("zod-parity-coverage", rendered);
    assert!(rendered.contains("export type ZodCoverage<T = string>"));
    assert!(rendered.contains("@deprecated Use the replacement instead"));
    assert!(rendered.contains("Field documentation."));
    assert!(rendered.contains(r#""quote\"slash\\line\nseparator""#));
    assert!(rendered.contains("generic: T"));
    assert!(rendered.contains(
        "export function ZodCoverageSchema<T extends z.ZodType>(T: T): z.ZodType<ZodCoverage<z.output<T>>>;"
    ));
    assert!(rendered.contains(
        "export function ZodGenericDefaultsSchema<T extends z.ZodType, U extends z.ZodType>(T: T, U: U): z.ZodType<ZodGenericDefaults<z.output<T>, z.output<U>>>;"
    ));
    assert!(rendered.contains("z.array(z.number().nullable())"));
    assert!(rendered.contains("z.record(z.string(), z.int().min(0).max(65535))"));
    assert!(rendered.contains("z.lazy(() => ZLaterReferenceSchema)"));
    assert!(rendered.contains("ZodFlattenOuterSchema"));
    assert!(rendered.contains(".and("));
    assert!(rendered.contains("ZodOptionalFlattenSchema"));
}

#[test]
fn zod_header_raw_runtime_and_manual_rendering() {
    let types = Types::default().register::<Another>();
    let rendered = Zod::default()
        .header("// custom header")
        .with_raw("")
        .with_raw("export const raw = true;")
        .framework_runtime(|mut exporter| {
            let types = exporter.render_types()?;
            Ok(Cow::Owned(format!("export const runtime = true;\n{types}")))
        })
        .export(&types, specta_serde::Format)
        .unwrap();

    assert!(rendered.starts_with("// custom header\nimport { z } from \"zod\";"));
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
fn zod_nested_errors_include_the_field_path() {
    let err = export_for::<StructWithBigInt>().unwrap_err().to_string();
    assert!(
        err.contains("StructWithBigInt.a"),
        "unexpected error: {err}"
    );
}
