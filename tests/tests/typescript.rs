use std::{
    borrow::Cow,
    collections::HashMap,
    iter,
    path::Path,
    time::{Duration, SystemTime},
};

use specta::{
    Format, Type, Types,
    datatype::{DataType, Reference},
};
use specta_typescript::{ErrorTraceFrame, Layout, Typescript, primitives};
use tempfile::TempDir;

use crate::fs_to_string;

const BIGINT_DOCS_URL: &str =
    "https://docs.rs/specta-typescript/latest/specta_typescript/struct.Error.html#bigint-forbidden";

fn typescript_types() -> (Types, Vec<(&'static str, DataType)>) {
    let mut types = Types::default();
    let mut dts = Vec::new();

    register!(types, dts;
        specta_typescript::Any,
        specta_typescript::Any<String>,
        specta_typescript::Unknown,
        specta_typescript::Unknown<String>,
        specta_typescript::Never,
        specta_typescript::Never<String>,
        specta_typescript::Number,
        specta_typescript::Number<i128>,
        specta_typescript::BigInt,
        specta_typescript::BigInt<i128>,
    );
    let _ = <HashMap<specta_typescript::Any, ()> as Type>::definition(&mut types);

    (types, dts)
}

pub type PhaseCollection = (
    &'static str,
    Box<dyn Format>,
    Vec<(&'static str, DataType)>,
    Types,
);

struct IdentityFormat;

impl Format for IdentityFormat {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(
        &'_ self,
        _: &Types,
        dt: &DataType,
    ) -> Result<Cow<'_, DataType>, specta::FormatError> {
        Ok(Cow::Owned(dt.clone()))
    }
}

pub fn phase_collections() -> Vec<PhaseCollection> {
    let (types, dts) = {
        let (mut types, mut dts) = crate::types();
        let (types2, dts2) = typescript_types();
        types.extend(&types2);
        dts.extend(dts2);
        (types, dts)
    };
    let (phased_types, phased_dts) = {
        let (mut types2, mut dts2) = crate::types_phased();
        types2.extend(&types);
        dts2.extend(dts.iter().cloned());
        (types2, dts2)
    };

    vec![
        ("raw", Box::new(IdentityFormat), dts.clone(), types.clone()),
        ("serde", Box::new(specta_serde::Format), dts, types),
        (
            "serde_phases",
            Box::new(specta_serde::PhasesFormat),
            phased_dts,
            phased_types,
        ),
    ]
}

#[test]
fn typescript_export() {
    for (mode, format, _, types) in phase_collections() {
        insta::assert_snapshot!(
            format!("ts-export-{mode}"),
            Typescript::default().export(&types, format).unwrap()
        );
    }
}

#[test]
fn typescript_export_serde_errors() {
    fn assert_serde_error<T: Type>(failures: &mut Vec<String>, name: &str, expected_error: &str) {
        fn assert_expected_error(
            failures: &mut Vec<String>,
            name: &str,
            mode: &str,
            stage: &str,
            expected_error: &str,
            err: impl std::fmt::Display,
        ) {
            let err = err.to_string();
            if !err.contains(expected_error) {
                failures.push(format!(
                    "{name} ({mode}) [{stage}]: expected error containing '{expected_error}', got '{err}'"
                ));
            }
        }

        let mut types = Types::default();
        let dt = T::definition(&mut types);

        for (mode, format) in [
            ("serde", Box::new(specta_serde::Format) as Box<dyn Format>),
            ("serde_phases", Box::new(specta_serde::PhasesFormat)),
        ] {
            match Typescript::default().export(&types, format) {
                Ok(_) => failures.push(format!(
                    "{name} ({mode}) [export]: expected error containing '{expected_error}', but export succeeded"
                )),
                Err(err) => {
                    assert_expected_error(failures, name, mode, "export", expected_error, err)
                }
            }
        }
    }

    fn assert_serde_export_ok<T: Type>(failures: &mut Vec<String>, name: &str) {
        let mut types = Types::default();
        let dt = T::definition(&mut types);

        for (mode, format) in [
            ("serde", Box::new(specta_serde::Format) as Box<dyn Format>),
            ("serde_phases", Box::new(specta_serde::PhasesFormat)),
        ] {
            if let Err(err) = format.map_type(&types, &dt) {
                failures.push(format!(
                    "{name} ({mode}) [map_type]: expected export to succeed, got '{err}'"
                ));
            }
        }
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    #[serde(tag = "type")]
    enum InternallyTaggedB {
        A(String),
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    #[serde(tag = "type")]
    enum InternallyTaggedC {
        A(Vec<String>),
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    #[serde(tag = "type")]
    enum InternallyTaggedG {
        A(InternallyTaggedGInner),
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    #[serde(untagged)]
    enum InternallyTaggedGInner {
        A(String),
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    #[serde(tag = "type")]
    enum InternallyTaggedI {
        A(InternallyTaggedIInner),
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    #[serde(transparent)]
    struct InternallyTaggedIInner(String);

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    #[serde(tag = "a")]
    enum TaggedEnumOfEmptyTupleStruct {
        A(EmptyTupleStruct),
        B(EmptyTupleStruct),
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    struct EmptyTupleStruct();

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    enum SkipOnlyVariantExternallyTagged {
        #[serde(skip)]
        A(String),
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    #[serde(tag = "t")]
    enum SkipOnlyVariantInternallyTagged {
        #[serde(skip)]
        A(String),
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    #[serde(tag = "t", content = "c")]
    enum SkipOnlyVariantAdjacentlyTagged {
        #[serde(skip)]
        A(String),
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    #[serde(untagged)]
    enum SkipOnlyVariantUntagged {
        #[serde(skip)]
        A(String),
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    struct RegularStruct {
        a: String,
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    enum Variants {
        A(String),
        B(i32),
        C(u8),
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    #[serde(transparent)]
    struct MaybeValidKey<T>(T);

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    #[serde(transparent)]
    struct InvalidMaybeValidKey(HashMap<MaybeValidKey<()>, ()>);

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    #[serde(transparent)]
    struct InvalidMaybeValidKeyNested(HashMap<MaybeValidKey<MaybeValidKey<()>>, ()>);

    #[derive(Type)]
    #[specta(transparent, collect = false)]
    struct RecursiveMapKeyTrick(RecursiveMapKey);

    #[derive(Type)]
    #[specta(collect = false)]
    struct RecursiveMapKey {
        demo: HashMap<RecursiveMapKeyTrick, String>,
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    #[serde(tag = "type")]
    enum InternallyTaggedBoxedStruct {
        // Regression test for https://github.com/specta-rs/specta/issues/482
        // `Box<T>` is transparent to serde, so this must validate like `T`.
        A(Box<InternallyTaggedBoxedStructPayload>),
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    struct InternallyTaggedBoxedStructPayload {
        message: String,
    }

    let mut failures = Vec::new();

    assert_serde_export_ok::<InternallyTaggedBoxedStruct>(
        &mut failures,
        "InternallyTaggedBoxedStruct",
    );

    // Serde Error: "cannot serialize tagged newtype variant InternallyTaggedB::A containing a string"
    assert_serde_error::<InternallyTaggedB>(
        &mut failures,
        "InternallyTaggedB",
        "Invalid internally tagged enum",
    );
    // Serde Error: "cannot serialize tagged newtype variant InternallyTaggedC::A containing a sequence"
    assert_serde_error::<InternallyTaggedC>(
        &mut failures,
        "InternallyTaggedC",
        "Invalid internally tagged enum",
    );
    // Serde Error: "cannot serialize tagged newtype variant InternallyTaggedG::A containing a string"
    assert_serde_error::<InternallyTaggedG>(
        &mut failures,
        "InternallyTaggedG",
        "Invalid internally tagged enum",
    );
    // Serde Error: "cannot serialize tagged newtype variant InternallyTaggedI::A containing a string"
    assert_serde_error::<InternallyTaggedI>(
        &mut failures,
        "InternallyTaggedI",
        "Invalid internally tagged enum",
    );

    // Serde Error: "cannot serialize tagged newtype variant TaggedEnumOfEmptyTupleStruct::A containing a tuple struct"
    assert_serde_error::<TaggedEnumOfEmptyTupleStruct>(
        &mut failures,
        "TaggedEnumOfEmptyTupleStruct",
        "Invalid internally tagged enum",
    );
    // Serde Error: "the enum variant SkipOnlyVariantExternallyTagged::A cannot be serialized"
    assert_serde_error::<SkipOnlyVariantExternallyTagged>(
        &mut failures,
        "SkipOnlyVariantExternallyTagged",
        "Invalid usage of #[serde(skip)]",
    );
    // Serde Error: "the enum variant SkipOnlyVariantInternallyTagged::A cannot be serialized"
    assert_serde_error::<SkipOnlyVariantInternallyTagged>(
        &mut failures,
        "SkipOnlyVariantInternallyTagged",
        "Invalid usage of #[serde(skip)]",
    );
    // Serde Error: "the enum variant SkipOnlyVariantAdjacentlyTagged::A cannot be serialized"
    assert_serde_error::<SkipOnlyVariantAdjacentlyTagged>(
        &mut failures,
        "SkipOnlyVariantAdjacentlyTagged",
        "Invalid usage of #[serde(skip)]",
    );
    // Serde Error: "the enum variant SkipOnlyVariantUntagged::A cannot be serialized"
    assert_serde_error::<SkipOnlyVariantUntagged>(
        &mut failures,
        "SkipOnlyVariantUntagged",
        "Invalid usage of #[serde(skip)]",
    );

    // These need to be named data types so they are exported by `Typescript::export`
    {
        #[derive(Type)]
        #[specta(collect = false)]
        pub struct A(HashMap<(), ()>);

        assert_serde_error::<A>(
            &mut failures,
            "A(HashMap<() /* `null` */, ()>)",
            "tuple keys are not supported by serde_json map key serialization",
        );
    }
    {
        #[derive(Type)]
        #[specta(collect = false)]
        pub struct B(HashMap<RegularStruct, ()>);

        assert_serde_error::<B>(
            &mut failures,
            "B(HashMap<RegularStruct, ()>)",
            "struct keys must serialize as a newtype struct to be valid serde_json map keys",
        );
    }
    {
        #[derive(Type)]
        #[specta(collect = false)]
        pub struct C(HashMap<Variants, ()>);

        assert_serde_error::<C>(
            &mut failures,
            "C(HashMap<Variants, ()>)",
            "enum key variant 'A' serializes as a struct variant, which serde_json rejects",
        );
    }
    assert_serde_error::<InvalidMaybeValidKey>(
        &mut failures,
        "InvalidMaybeValidKey",
        "tuple keys are not supported by serde_json map key serialization",
    );
    assert_serde_error::<InvalidMaybeValidKeyNested>(
        &mut failures,
        "InvalidMaybeValidKeyNested",
        "tuple keys are not supported by serde_json map key serialization",
    );

    assert_serde_error::<RecursiveMapKey>(
        &mut failures,
        "RecursiveMapKey",
        "struct keys must serialize as a newtype struct to be valid serde_json map keys",
    );

    assert!(
        failures.is_empty(),
        "Unexpected TypeScript serde export behavior:\n{}",
        failures.join("\n")
    );
}

#[test]
fn typescript_export_bigint_errors() {
    fn assert_bigint_error<T: Type>(failures: &mut Vec<String>, name: &str) {
        let ts = Typescript::default();
        let mut types = Types::default();
        let dt = T::definition(&mut types);

        match primitives::inline(&ts, &types, &dt) {
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

        match ts.export(&types, specta_serde::Format) {
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
        let ts = Typescript::default();
        let mut types = Types::default();
        let dt = T::definition(&mut types);

        match primitives::inline(&ts, &types, &dt) {
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
        "Unexpected TypeScript BigInt export behavior:\n{}",
        failures.join("\n")
    );
}

#[test]
fn typescript_errors_include_named_datatype_and_inline_trace() {
    let regular_outer_line = line!() + 1;
    #[derive(Type)]
    #[specta(collect = false)]
    struct RegularOuter {
        value: i128,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct InlineInner {
        value: i128,
    }

    let inline_outer_line = line!() + 1;
    #[derive(Type)]
    #[specta(collect = false)]
    struct InlineOuter {
        #[specta(inline)]
        inner: InlineInner,
    }

    let ts = Typescript::default();
    let mut types = Types::default();
    let dt = RegularOuter::definition(&mut types);
    let ndt = match dt {
        DataType::Reference(Reference::Named(r)) => types.get(&r).unwrap().to_owned(),
        _ => panic!("expected named reference"),
    };
    let err = primitives::export(&ts, &types, [ndt].iter(), "").unwrap_err();
    assert_eq!(
        err.named_datatype().map(|dt| dt.name.as_ref()),
        Some("RegularOuter")
    );
    assert!(err.trace().is_empty());
    assert_eq!(
        err.to_string(),
        format!(
            "Attempted to export \"test::typescript::RegularOuter.value\" but Specta forbids exporting BigInt-style types (usize, isize, i64, u64, i128, u128) to avoid precision loss. See {BIGINT_DOCS_URL} for a full explanation.\nRust type: test::typescript::RegularOuter at {}:{}:{}",
            file!(),
            regular_outer_line,
            14
        )
    );

    let mut types = Types::default();
    let dt = InlineOuter::definition(&mut types);
    let ndt = match dt {
        DataType::Reference(Reference::Named(r)) => types.get(&r).unwrap().to_owned(),
        _ => panic!("expected named reference"),
    };
    let err = primitives::export(&ts, &types, [ndt].iter(), "").unwrap_err();
    assert_eq!(
        err.named_datatype().map(|dt| dt.name.as_ref()),
        Some("InlineOuter")
    );
    assert_eq!(err.trace().len(), 1);

    match &err.trace()[0] {
        ErrorTraceFrame::Inlined {
            named_datatype,
            path,
        } => {
            assert_eq!(
                named_datatype.as_deref().map(|dt| dt.name.as_ref()),
                Some("InlineInner")
            );
            assert_eq!(path, "test::typescript::InlineOuter.inner");
        }
        _ => panic!("expected inline trace frame"),
    }
    assert_eq!(
        err.to_string(),
        format!(
            "Attempted to export \"test::typescript::InlineOuter.inner.value\" but Specta forbids exporting BigInt-style types (usize, isize, i64, u64, i128, u128) to avoid precision loss. See {BIGINT_DOCS_URL} for a full explanation.\nRust type: test::typescript::InlineOuter at {}:{}:{}\nWhile inlining:\n  test::typescript::InlineOuter.inner -> test::typescript::InlineInner",
            file!(),
            inline_outer_line,
            14
        )
    );
}

#[test]
fn typescript_errors_include_enum_variant_paths() {
    let named_variant_enum_line = line!() + 1;
    #[derive(Type)]
    #[specta(collect = false)]
    enum EnumWithNamedVariantBigInt {
        Variant { value: i128 },
    }

    let tuple_variant_enum_line = line!() + 1;
    #[derive(Type)]
    #[specta(collect = false)]
    enum EnumWithTupleVariantBigInt {
        Variant(i128),
    }

    let ts = Typescript::default();

    let types = Types::default().register::<EnumWithNamedVariantBigInt>();
    let err = ts.export(&types, specta_serde::Format).unwrap_err();
    assert_eq!(
        err.to_string(),
        format!(
            "Attempted to export \"test::typescript::EnumWithNamedVariantBigInt.Variant.value\" but Specta forbids exporting BigInt-style types (usize, isize, i64, u64, i128, u128) to avoid precision loss. See {BIGINT_DOCS_URL} for a full explanation.\nRust type: test::typescript::EnumWithNamedVariantBigInt at {}:{}:{}",
            file!(),
            named_variant_enum_line,
            14
        )
    );

    let types = Types::default().register::<EnumWithTupleVariantBigInt>();
    let err = ts.export(&types, specta_serde::Format).unwrap_err();
    assert_eq!(
        err.to_string(),
        format!(
            "Attempted to export \"test::typescript::EnumWithTupleVariantBigInt.Variant.0\" but Specta forbids exporting BigInt-style types (usize, isize, i64, u64, i128, u128) to avoid precision loss. See {BIGINT_DOCS_URL} for a full explanation.\nRust type: test::typescript::EnumWithTupleVariantBigInt at {}:{}:{}",
            file!(),
            tuple_variant_enum_line,
            14
        )
    );
}

#[test]
fn typescript_errors_include_recursive_inline_error() {
    let mut dt = DataType::Primitive(specta::datatype::Primitive::str);
    for _ in 0..25 {
        dt = DataType::Nullable(Box::new(dt));
    }

    let err = primitives::inline(&Typescript::default(), &Types::default(), &dt).unwrap_err();

    assert_eq!(
        err.to_string(),
        "Attempted to export  but was unable to due to name \"Type recursion limit exceeded during inline expansion\" containing an invalid character. Try renaming it or using `#[specta(rename = \"new name\")]`"
    );
}

#[test]
fn typescript_export_to() {
    let temp = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&temp).unwrap();
    let temp = TempDir::new_in(temp).unwrap();

    for layout in [
        Layout::Files,
        Layout::FlatFile,
        Layout::ModulePrefixedName,
        Layout::Namespaces,
    ] {
        for (mode, format, _, types) in phase_collections() {
            let name = format!(
                "ts-export-to-{}-{}",
                layout.to_string().to_lowercase(),
                mode
            );
            let output = {
                let path = temp.path().join(&name);
                Typescript::default()
                    .layout(layout)
                    .export_to(&path, &types, format)
                    .unwrap();
                fs_to_string(&path).map_err(|err| err.to_string())
            }
            .unwrap();

            insta::assert_snapshot!(name, output);
        }
    }

    temp.close().unwrap();

    // TODO: Assert layouts error out with `export` method
    // TODO: Assert it errors if given the path to a file
}

#[test]
fn primitives_export() {
    for (mode, format, dts, types) in phase_collections() {
        let types = format.map_types(&types).unwrap().into_owned();

        let output = dts
            .iter()
            .filter_map(|(name, dt)| {
                let mut ndt = match dt {
                    DataType::Reference(Reference::Named(r)) => types.get(r).unwrap().to_owned(),
                    _ => return None,
                };

                if let Some(ty) = &mut ndt.ty {
                    *ty = format.map_type(&types, ty).unwrap().into_owned();
                }

                Some(
                    primitives::export(&Typescript::default(), &types, [ndt].iter(), "")
                        .map(|ty| format!("{name}: {ty}")),
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|exports| exports.join("\n"))
            .unwrap();

        insta::assert_snapshot!(format!("export-{mode}"), output);
    }
}

#[test]
fn primitives_export_many() {
    for (mode, format, dts, types) in phase_collections() {
        let types = format.map_types(&types).unwrap().into_owned();

        let output = primitives::export(
            &Typescript::default(),
            &types,
            dts.iter()
                .filter_map(|(_, ty)| match ty {
                    DataType::Reference(Reference::Named(r)) => types.get(r).cloned(),
                    _ => None,
                })
                .map(|mut ndt| {
                    if let Some(ty) = &mut ndt.ty {
                        *ty = format.map_type(&types, ty).unwrap().into_owned();
                    }
                    ndt
                })
                .collect::<Vec<_>>()
                .iter(),
            "",
        )
        .unwrap();

        insta::assert_snapshot!(format!("export-many-{mode}"), output);
    }
}

#[test]
fn primitives_reference() {
    for (mode, format, dts, types) in phase_collections() {
        let types = format.map_types(&types).unwrap().into_owned();

        let output = dts
            .iter()
            .filter_map(|(name, dt)| {
                let dt = format.map_type(&types, dt).unwrap().into_owned();

                let reference = match dt {
                    DataType::Reference(reference) => reference.clone(),
                    _ => return None,
                };

                Some(
                    primitives::reference(&Typescript::default(), &types, &reference)
                        .map(|ty| format!("{name}: {ty}")),
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|exports| exports.join("\n"))
            .unwrap();

        insta::assert_snapshot!(format!("reference-{mode}"), output);
    }
}

#[test]
fn primitives_inline() {
    for (mode, format, dts, types) in phase_collections() {
        let types = format.map_types(&types).unwrap().into_owned();

        let output = dts
            .iter()
            .map(|(name, dt)| {
                let dt = format.map_type(&types, dt).unwrap().into_owned();

                primitives::inline(&Typescript::default(), &types, &dt)
                    .map(|ty| format!("{name}: {ty}"))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(|exports| exports.join("\n"))
            .unwrap();

        insta::assert_snapshot!(format!("inline-{mode}"), output);
    }
}

#[test]
fn reserved_names() {
    {
        #[derive(Type)]
        #[specta(collect = false)]
        #[allow(non_camel_case_types)]
        pub struct r#enum {
            a: String,
        }

        let mut types = Types::default();
        let ndt = match r#enum::definition(&mut types) {
            DataType::Reference(Reference::Named(r)) => types.get(&r).unwrap(),
            _ => panic!("Failed to get reference"),
        };
        insta::assert_snapshot!(primitives::export(&Typescript::default(), &types, iter::once(ndt), "").unwrap_err().to_string(), @r#"Attempted to export  but was unable to due to name "enum" conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = "new name")]`"#);
    }

    {
        #[derive(Type)]
        #[specta(collect = false)]
        #[allow(non_camel_case_types)]
        pub struct r#enum(String);

        let mut types = Types::default();
        let ndt = match r#enum::definition(&mut types) {
            DataType::Reference(Reference::Named(r)) => types.get(&r).unwrap(),
            _ => panic!("Failed to get reference"),
        };
        insta::assert_snapshot!(primitives::export(&Typescript::default(), &types, iter::once(ndt), "").unwrap_err().to_string(), @r#"Attempted to export  but was unable to due to name "enum" conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = "new name")]`"#);
    }

    {
        // Typescript reserved type name
        #[derive(Type)]
        #[specta(collect = false)]
        #[allow(non_camel_case_types)]
        pub enum r#enum {
            A(String),
        }

        let mut types = Types::default();
        let ndt = match r#enum::definition(&mut types) {
            DataType::Reference(Reference::Named(r)) => types.get(&r).unwrap(),
            _ => panic!("Failed to get reference"),
        };
        insta::assert_snapshot!(primitives::export(&Typescript::default(), &types, iter::once(ndt), "").unwrap_err().to_string(), @r#"Attempted to export  but was unable to due to name "enum" conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = "new name")]`"#);
    }
}

// #[test]
// fn duplicate_ty_name() {
//     mod one {
//         use super::*;

//         #[derive(Type)]
//         #[specta(collect = false)]
//         pub struct One {
//             pub a: String,
//         }
//     }

//     #[derive(Type)]
//     #[specta(collect = false)]
//     pub struct One {
//         pub one: one::One,
//     }

//     assert!(
//         Typescript::default()
//             .export(&Types::default().register::<Demo>())
//             .is_err_and(|err| err
//                 .to_string()
//                 .starts_with("Detected multiple types with the same name:"))
//     );
// }

// TODO
//
// Break out testing of `specta_typescript` types from all languages (just jsdoc & typescript)
// Make a `typescript` folder for extra testing on the Typescript exporter
//
// Testing different combos of feature flags + external impls. Can we come up with a proper multi-binary system for this???
//
// BigInt checks
//
// Test frameworks API's. Eg. prelude and runtime for each layout.
// Test framework references and code replacing
// Test `Any`, etc for this and JSDoc
//
// TODO: For core:
// Testing Specta macros in many basic cases.
// Test `borrow`, `skip` and other Specta attributes
// Testing all Serde features in the AST layer???
// Test that the macro attribute lowering system works.
//
// Tests for framework primitives (prelude, runtime, runtime imports, etc)
// Tauri `Channel` tests
