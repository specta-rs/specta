use std::{iter, path::Path};

use specta::{
    ResolvedTypes, Type, Types,
    datatype::{DataType, NamedDataType, Primitive, Reference},
};
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
    primitives::inline(zod, &ResolvedTypes::from_resolved_types(types), &dt)
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
    let resolved = ResolvedTypes::from_resolved_types(types);

    let out = Zod::default()
        .bigint(BigIntExportBehavior::Number)
        .export(&resolved)
        .unwrap();

    assert!(out.contains("import { z } from \"zod\";"));
    assert!(out.contains("export const DemoSchema"));
    assert!(out.contains("export type Demo = z.infer<typeof DemoSchema>;"));
}

#[test]
fn zod_primitives_smoke() {
    let (types, dts) = crate::types();
    let resolved = ResolvedTypes::from_resolved_types(types);

    let zod = Zod::default().bigint(BigIntExportBehavior::Number);

    for (_, ty) in &dts {
        let rendered = primitives::inline(&zod, &resolved, ty).unwrap();
        assert!(!rendered.is_empty());
    }

    let ndt = dts
        .iter()
        .find_map(|(_, ty)| match ty {
            DataType::Reference(Reference::Named(r)) => r.get(resolved.as_types()),
            _ => None,
        })
        .unwrap();

    let rendered = primitives::export(&zod, &resolved, iter::once(ndt), "").unwrap();
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
    let resolved = ResolvedTypes::from_resolved_types(types.clone());

    let err = Zod::default().export(&resolved).unwrap_err();
    assert!(err.to_string().contains("Detected multiple types"));

    let module_prefixed = Zod::default()
        .layout(Layout::ModulePrefixedName)
        .export(&resolved)
        .unwrap();
    assert!(module_prefixed.contains("TestingSchema"));
    assert!(module_prefixed.contains("testing2"));
}

#[test]
fn zod_layout_files_export_to() {
    let types = Types::default().register::<Testing>().register::<Another>();
    let resolved = ResolvedTypes::from_resolved_types(types.clone());

    let temp = temp_dir();
    let path = temp.path().join("zod-layout-files");

    Zod::default()
        .layout(Layout::Files)
        .export_to(&path, &resolved)
        .unwrap();

    let output = crate::fs_to_string(Path::new(&path)).unwrap();
    assert!(output.contains(".ts"));
    assert!(output.contains("import { z } from \"zod\";"));
}

#[test]
fn zod_recursive_types_use_lazy() {
    let types = Types::default().register::<Recursive>();
    let resolved = ResolvedTypes::from_resolved_types(types);

    let out = Zod::default().export(&resolved).unwrap();
    assert!(out.contains("z.lazy(() => RecursiveSchema)"));
}

#[test]
fn zod_reserved_type_name_errors() {
    let mut types = Types::default();
    NamedDataType::new("class", Vec::new(), DataType::Primitive(Primitive::i8))
        .register(&mut types);
    let resolved = ResolvedTypes::from_resolved_types(types);

    let err = Zod::default().export(&resolved).unwrap_err();
    assert!(err.to_string().contains("reserved keyword"));
}

#[test]
fn zod_layout_files_errors_on_export() {
    let types = Types::default();
    let resolved = ResolvedTypes::from_resolved_types(types);

    let err = Zod::default()
        .layout(Layout::Files)
        .export(&resolved)
        .unwrap_err();
    assert!(err.to_string().contains("Unable to export layout Files"));
}

fn temp_dir() -> TempDir {
    let temp_root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&temp_root).unwrap();
    TempDir::new_in(temp_root).unwrap()
}

fn export_for<T: Type>() -> Result<String, specta_zod::Error> {
    let types = Types::default().register::<T>();
    Zod::default().export(&ResolvedTypes::from_resolved_types(types))
}
