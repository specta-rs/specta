use std::{
    path::Path,
    time::{Duration, SystemTime},
};

use specta::datatype::{DataType, Reference};
use specta::{Type, Types};
use specta_typescript::{JSDoc, Layout, primitives};
use tempfile::TempDir;

use crate::fs_to_string;

mod jsdoc_export_to_files_runtime_imports_types {
    use super::*;

    pub mod three {
        use super::*;

        #[derive(Type)]
        #[specta(collect = false)]
        pub struct Three {
            pub active: bool,
        }
    }

    pub mod two {
        use super::*;

        #[derive(Type)]
        #[specta(collect = false)]
        pub struct Two {
            pub value: String,
        }
    }

    pub mod one {
        use super::*;

        #[derive(Type)]
        #[specta(collect = false)]
        pub struct One {
            pub two: super::two::Two,
            pub three: super::three::Three,
        }
    }
}

#[derive(Clone, Copy)]
enum PhaseCollection {
    Raw,
    Serde,
    SerdePhases,
}

impl PhaseCollection {
    fn name(self) -> &'static str {
        match self {
            Self::Raw => "raw",
            Self::Serde => "serde",
            Self::SerdePhases => "serde_phases",
        }
    }

    fn export_jsdoc(self, types: &Types) -> Result<String, specta_typescript::Error> {
        let _ = self;
        JSDoc::default().export(types, crate::identity_format)
    }
}

fn phase_collections(types: Types) -> [(PhaseCollection, Types); 3] {
    let serde_types = (specta_serde::format.format_types)(&types)
        .map(|types| types.into_owned())
        .expect("serde formatting should succeed for shared JSDoc fixtures");
    let serde_phases_types = (specta_serde::format_phases.format_types)(&types)
        .map(|types| types.into_owned())
        .expect("serde phase formatting should succeed for shared JSDoc fixtures");

    [
        (PhaseCollection::Raw, types.clone()),
        (PhaseCollection::Serde, serde_types),
        (PhaseCollection::SerdePhases, serde_phases_types),
    ]
}

#[test]
fn export() {
    for (mode, types) in phase_collections(crate::types().0) {
        let output = mode
            .export_jsdoc(&types)
            .unwrap_or_else(|err| format!("ERROR: {err}"));

        insta::assert_snapshot!(format!("inline-{}", mode.name()), output);
    }
}

#[test]
fn primitives_export_many() {
    let (types, dts) = crate::types();

    for (mode, types) in phase_collections(types) {
        let output = {
            let jsdoc = JSDoc::default();
            let ndts = dts
                .iter()
                .filter_map(|(_, ty)| match ty {
                    DataType::Reference(Reference::Named(r)) => r.get(&types),
                    _ => None,
                })
                .collect::<Vec<_>>();

            primitives::export(&jsdoc, &types, ndts.into_iter(), "")
                .unwrap_or_else(|err| format!("ERROR: {err}"))
        };

        insta::assert_snapshot!(format!("primitives-many-inline-{}", mode.name()), output);
    }
}

#[test]
fn jsdoc_export_bigint_errors() {
    fn assert_bigint_error<T: Type>(failures: &mut Vec<String>, name: &str) {
        let jsdoc = JSDoc::default();
        let mut types = Types::default();
        let dt = T::definition(&mut types);

        match primitives::inline(&jsdoc, &types, &dt) {
            Ok(ty) => failures.push(format!(
                "{name} [inline]: expected BigInt error, but export succeeded with '{ty}'"
            )),
            Err(err)
                if err
                    .to_string()
                    .contains("forbids exporting BigInt-style types") => {}
            Err(err) => failures.push(format!("{name} [inline]: unexpected error '{err}'")),
        }

        if types.is_empty() {
            return;
        }

        match jsdoc.export(&types, crate::identity_format) {
            Ok(output) => failures.push(format!(
                "{name} [export]: expected BigInt error, but export succeeded with '{output}'"
            )),
            Err(err)
                if err
                    .to_string()
                    .contains("forbids exporting BigInt-style types") => {}
            Err(err) => failures.push(format!("{name} [export]: unexpected error '{err}'")),
        }
    }

    fn assert_inline_bigint_error<T: Type>(failures: &mut Vec<String>, name: &str) {
        let jsdoc = JSDoc::default();
        let mut types = Types::default();
        let dt = T::definition(&mut types);

        match primitives::inline(&jsdoc, &types, &dt) {
            Ok(ty) => failures.push(format!(
                "{name} [inline]: expected BigInt error, but export succeeded with '{ty}'"
            )),
            Err(err)
                if err
                    .to_string()
                    .contains("forbids exporting BigInt-style types") => {}
            Err(err) => failures.push(format!("{name} [inline]: unexpected error '{err}'")),
        }
    }

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
    struct StructWithSystemTime {
        // https://github.com/specta-rs/specta/issues/77
        #[specta(inline)]
        value: SystemTime,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct StructWithDuration {
        // https://github.com/specta-rs/specta/issues/77
        #[specta(inline)]
        value: Duration,
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
    struct StructWithStructWithStructWithBigInt {
        #[specta(inline)]
        field1: StructWithStructWithBigInt,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct StructWithOptionWithStructWithBigInt {
        #[specta(inline)]
        optional_field: Option<StructWithBigInt>,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    enum EnumWithStructWithStructWithBigInt {
        #[specta(inline)]
        A(StructWithStructWithBigInt),
    }

    #[derive(Type)]
    #[specta(collect = false)]
    enum EnumWithInlineStructWithBigInt {
        #[specta(inline)]
        B { a: i128 },
    }

    let mut failures = Vec::new();

    for_bigint_types!(T -> |name| {
        assert_bigint_error::<T>(&mut failures, name);
    });

    for (name, assert) in [
        (
            "StructWithSystemTime",
            assert_bigint_error::<StructWithSystemTime> as fn(&mut Vec<String>, &str),
        ),
        (
            "StructWithDuration",
            assert_bigint_error::<StructWithDuration> as fn(&mut Vec<String>, &str),
        ),
        (
            "StructWithBigInt",
            assert_bigint_error::<StructWithBigInt> as fn(&mut Vec<String>, &str),
        ),
        (
            "StructWithStructWithBigInt",
            assert_bigint_error::<StructWithStructWithBigInt> as fn(&mut Vec<String>, &str),
        ),
        (
            "StructWithStructWithStructWithBigInt",
            assert_bigint_error::<StructWithStructWithStructWithBigInt>
                as fn(&mut Vec<String>, &str),
        ),
        (
            "StructWithOptionWithStructWithBigInt",
            assert_bigint_error::<StructWithOptionWithStructWithBigInt>
                as fn(&mut Vec<String>, &str),
        ),
        (
            "EnumWithStructWithStructWithBigInt",
            assert_bigint_error::<EnumWithStructWithStructWithBigInt> as fn(&mut Vec<String>, &str),
        ),
        (
            "EnumWithInlineStructWithBigInt",
            assert_bigint_error::<EnumWithInlineStructWithBigInt> as fn(&mut Vec<String>, &str),
        ),
    ] {
        assert(&mut failures, name);
    }

    assert_inline_bigint_error::<SystemTime>(&mut failures, "SystemTime");
    assert_inline_bigint_error::<Duration>(&mut failures, "Duration");

    assert!(
        failures.is_empty(),
        "Unexpected JSDoc BigInt export behavior:\n{}",
        failures.join("\n")
    );
}

#[test]
fn jsdoc_export_to_files_uses_jsdoc_import_typedefs() {
    let temp = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&temp).unwrap();
    let temp = TempDir::new_in(temp).unwrap();

    let path = temp.path().join("jsdoc-export-to-files-both");
    let types = Types::default()
        .register::<jsdoc_export_to_files_runtime_imports_types::one::One>()
        .register::<jsdoc_export_to_files_runtime_imports_types::two::Two>()
        .register::<jsdoc_export_to_files_runtime_imports_types::three::Three>();

    JSDoc::default()
        .layout(Layout::Files)
        .export_to(&path, &types, crate::identity_format)
        .unwrap();

    let output = fs_to_string(&path).unwrap();
    insta::assert_snapshot!("jsdoc-export-to-files-both", output);

    temp.close().unwrap();
}

// TODO: Confirm different layouts
// TODO: Unit test JSDoc and other languages

// TODO: Ensure this is feature matching with the Typescript testing
