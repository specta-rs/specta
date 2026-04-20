use std::{borrow::Cow, iter, path::Path};

use serde::{Deserialize, Serialize};
use specta::{
    Format, Type, Types,
    datatype::{DataType, NamedDataType, Primitive, Reference},
};
use specta_typescript::Typescript;
use specta_zod::{BigIntExportBehavior, Layout, Zod, primitives};
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
    child: Option<Box<Recursive>>,
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

fn zod_raw_map_types(types: &Types) -> Result<std::borrow::Cow<'_, Types>, specta::FormatError> {
    Ok(std::borrow::Cow::Borrowed(types))
}

fn zod_raw_map_datatype<'a>(
    _types: &'a Types,
    dt: &'a DataType,
) -> Result<std::borrow::Cow<'a, DataType>, specta::FormatError> {
    Ok(std::borrow::Cow::Borrowed(dt))
}

#[allow(non_upper_case_globals)]
const zod_raw_format: Format = Format::new(zod_raw_map_types, zod_raw_map_datatype);

fn zod_identity_types<'a>(types: &'a Types) -> Result<Cow<'a, Types>, specta::FormatError> {
    Ok(Cow::Borrowed(types))
}

fn zod_map_bool_to_string<'a>(
    _: &'a Types,
    dt: &'a DataType,
) -> Result<Cow<'a, DataType>, specta::FormatError> {
    Ok(match dt {
        DataType::Primitive(Primitive::bool) => Cow::Owned(DataType::Primitive(Primitive::str)),
        _ => Cow::Borrowed(dt),
    })
}

fn zod_map_reference_to_string<'a>(
    _: &'a Types,
    dt: &'a DataType,
) -> Result<Cow<'a, DataType>, specta::FormatError> {
    Ok(match dt {
        DataType::Reference(Reference::Named(_)) => Cow::Owned(DataType::Primitive(Primitive::str)),
        _ => Cow::Borrowed(dt),
    })
}

fn zod_error_on_bool<'a>(
    _: &'a Types,
    dt: &'a DataType,
) -> Result<Cow<'a, DataType>, specta::FormatError> {
    match dt {
        DataType::Primitive(Primitive::bool) => Err(std::io::Error::other("boom").into()),
        _ => Ok(Cow::Borrowed(dt)),
    }
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
    let out = Zod::default()
        .bigint(BigIntExportBehavior::Number)
        .export(&types, zod_raw_format)
        .unwrap();

    assert!(out.contains("import { z } from \"zod\";"));
    assert!(out.contains("export const DemoSchema"));
    assert!(out.contains("export type Demo = z.infer<typeof DemoSchema>;"));
}

#[test]
fn zod_primitives_smoke() {
    let (types, dts) = crate::types();
    let zod = Zod::default().bigint(BigIntExportBehavior::Number);

    for (_, ty) in &dts {
        let rendered = primitives::inline(&zod, &types, ty).unwrap();
        assert!(!rendered.is_empty());
    }

    let ndt = dts
        .iter()
        .find_map(|(_, ty)| match ty {
            DataType::Reference(Reference::Named(r)) => r.get(&types),
            _ => None,
        })
        .unwrap();

    let rendered = primitives::export(&zod, &types, iter::once(ndt), "").unwrap();
    assert!(rendered.contains("Schema"));
}

#[test]
fn zod_bigint_export_behaviors() {
    for_bigint_types!(T -> |_| {
        assert!(inline_for::<T>(&Zod::default()).is_err());
        assert!(inline_for::<T>(&Zod::default().bigint(BigIntExportBehavior::Fail)).is_err());

        assert_eq!(
            inline_for::<T>(&Zod::default().bigint(BigIntExportBehavior::String)).unwrap(),
            "z.string()"
        );
        assert_eq!(
            inline_for::<T>(&Zod::default().bigint(BigIntExportBehavior::Number)).unwrap(),
            "z.number()"
        );
        assert_eq!(
            inline_for::<T>(&Zod::default().bigint(BigIntExportBehavior::BigInt)).unwrap(),
            "z.bigint()"
        );
    });
}

#[test]
fn zod_framework_inline_applies_datatype_mapping() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Nested {
        value: bool,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct Container {
        direct: bool,
        list: Vec<bool>,
        tuple: (bool, Option<bool>),
        #[specta(inline)]
        nested: Nested,
    }

    let mut types = Types::default();
    let dt = Container::definition(&mut types);
    let DataType::Reference(Reference::Named(reference)) = dt else {
        panic!("expected named reference");
    };
    let ty = reference.get(&types).unwrap().ty.clone();

    let rendered = Zod::default()
        .framework_runtime(move |ctx| Ok(ctx.inline(&ty)?.into()))
        .export(
            &types,
            Format::new(zod_identity_types, zod_map_bool_to_string),
        )
        .unwrap();

    assert!(rendered.contains("z.string()"), "{rendered}");
    assert!(!rendered.contains("z.boolean()"), "{rendered}");
}

#[test]
fn zod_framework_reference_and_export_apply_datatype_mapping() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Demo {
        value: bool,
    }

    let mut types = Types::default();
    let reference = Demo::definition(&mut types);
    let DataType::Reference(Reference::Named(reference)) = reference else {
        panic!("expected named reference");
    };
    let ndt = reference.get(&types).unwrap().clone();

    let reference_rendered = Zod::default()
        .framework_runtime({
            let reference = reference.clone();
            move |ctx| Ok(ctx.reference(&Reference::Named(reference.clone()))?.into())
        })
        .export(
            &types,
            Format::new(zod_identity_types, zod_map_reference_to_string),
        )
        .unwrap();
    assert!(
        reference_rendered.contains("z.string()"),
        "{reference_rendered}"
    );

    let export_rendered = Zod::default()
        .framework_runtime(move |ctx| Ok(ctx.export(iter::once(&ndt), "")?.into()))
        .export(
            &types,
            Format::new(zod_identity_types, zod_map_bool_to_string),
        )
        .unwrap();
    assert!(export_rendered.contains("z.string()"), "{export_rendered}");
}

#[test]
fn zod_framework_datatype_mapping_errors_bubble_out() {
    let types = Types::default();
    let err = Zod::default()
        .framework_runtime(|ctx| Ok(ctx.inline(&DataType::Primitive(Primitive::bool))?.into()))
        .export(&types, Format::new(zod_identity_types, zod_error_on_bool))
        .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Framework error: datatype formatter failed: boom"
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
            err.to_string().contains("forbids exporting BigInt types"),
            "unexpected error: {err}"
        );
    }
}

#[test]
fn zod_layout_duplicate_typenames() {
    let types = Types::default().register::<Testing>().register::<Another>();
    let err = Zod::default().export(&types, zod_raw_format).unwrap_err();
    assert!(err.to_string().contains("Detected multiple types"));

    let module_prefixed = Zod::default()
        .layout(Layout::ModulePrefixedName)
        .export(&types, zod_raw_format)
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
        .export_to(&path, &types, zod_raw_format)
        .unwrap();

    let output = crate::fs_to_string(Path::new(&path)).unwrap();
    assert!(output.contains(".ts"));
    assert!(output.contains("import { z } from \"zod\";"));
}

#[test]
fn zod_uses_serde_transformed_resolved_types() {
    let types = Types::default().register::<SerdeTaggedEnum>();

    let raw_out = Zod::default()
        .export(
            &Types::default().register::<SerdeTaggedEnum>(),
            zod_raw_format,
        )
        .unwrap();
    let serde_out = Zod::default().export(&types, specta_serde::format).unwrap();

    assert_ne!(raw_out, serde_out);
    assert!(serde_out.contains("type: z.literal(\"unit\")"));
    assert!(serde_out.contains("type: z.literal(\"string_value\")"));
    assert!(serde_out.contains("data: z.string()"));
}

#[test]
fn zod_rejects_invalid_serde_shapes_via_transformation() {
    let types = Types::default().register::<InvalidInternallyTaggedEnum>();
    let format = specta_serde::format;
    let err = (format.format_types)(&types).unwrap_err();

    assert!(err.to_string().contains("Invalid internally tagged enum"));
}

#[test]
fn zod_empty_named_shapes_are_strict() {
    let empty_struct = export_for::<EmptyStruct>().unwrap();
    assert!(empty_struct.contains("z.object({}).strict()"));

    let empty_variant = export_for::<EmptyNamedVariant>().unwrap();
    assert!(empty_variant.contains("z.object({}).strict()"));
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
        .export_to(&path, &types, zod_raw_format)
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
        .export_to(&path, &types, crate::identity_format)
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
    let out = Zod::default().export(&types, zod_raw_format).unwrap();
    assert!(out.contains("z.lazy(() => RecursiveSchema)"));
}

#[test]
fn zod_reserved_type_name_errors() {
    let mut types = Types::default();
    NamedDataType::new("class", Vec::new(), DataType::Primitive(Primitive::i8))
        .register(&mut types);
    let err = Zod::default().export(&types, zod_raw_format).unwrap_err();
    assert!(err.to_string().contains("reserved keyword"));
}

#[test]
fn zod_layout_files_errors_on_export() {
    let types = Types::default();
    let err = Zod::default()
        .layout(Layout::Files)
        .export(&types, zod_raw_format)
        .unwrap_err();
    assert!(err.to_string().contains("Unable to export layout Files"));
}

fn temp_dir() -> TempDir {
    TempDir::new_in(temp_root()).unwrap()
}

fn export_for<T: Type>() -> Result<String, specta_zod::Error> {
    let types = Types::default().register::<T>();
    Zod::default().export(&types, zod_raw_format)
}
