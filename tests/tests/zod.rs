use std::{iter, path::Path};

use serde::{Deserialize, Serialize};
use specta::{
    Type, Types,
    datatype::{DataType, NamedDataType, Primitive, Reference},
};
use specta_typescript::Typescript;
use specta_util::Remapper;
use specta_zod::{Layout, Zod, define, primitives};
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

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum InvalidInternallyTaggedEnum {
    A(String),
}

mod testing {
    use super::*;

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
    assert!(out.contains("export type Demo = z.infer<typeof DemoSchema>;"));
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
fn zod_bigint_errors_propagate_from_nested_types() {
    for err in [
        export_for::<StructWithBigInt>(),
        export_for::<StructWithStructWithBigInt>(),
        export_for::<StructWithOptionWithStructWithBigInt>(),
        export_for::<EnumWithInlineStructWithBigInt>(),
    ] {
        let err = err.expect_err("bigint export should be rejected by default");
        assert!(
            err.to_string()
                .contains("forbids exporting BigInt-style types"),
            "unexpected error: {err}"
        );
    }
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
    assert!(empty_struct.contains("z.strictObject({})"));

    let empty_variant = export_for::<EmptyNamedVariant>().unwrap();
    assert!(empty_variant.contains("z.strictObject({})"));
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
    assert!(out.contains("unsigned: z.int()"));
    assert!(out.contains("floating: z.number()"));
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
        rendered.contains("z.tuple([z.int(), z.int().optional()])"),
        "the defaulted element must be `.optional()` in the deserialize half: {rendered}"
    );
}

/// Control: zod's tuple arm filters live fields already, so a skip-reduced
/// defaulted tuple sizes over live elements only.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ZodSkipSlotTuple(#[serde(skip)] u8, #[serde(default)] u8);

#[test]
fn zod_skip_slot_tuple_phases() {
    let rendered = Zod::default()
        .export(
            &Types::default().register::<ZodSkipSlotTuple>(),
            specta_serde::PhasesFormat,
        )
        .expect("Zod should support skip-reduced defaulted tuple structs");

    assert!(
        rendered.contains("ZodSkipSlotTuple_DeserializeSchema = z.tuple([z.int().optional()])"),
        "only the live element is sized, and it is optional on deserialize: {rendered}"
    );
    assert!(
        rendered.contains("ZodSkipSlotTuple_SerializeSchema = z.tuple([z.int()])"),
        "serialize keeps the single live element required: {rendered}"
    );
}
