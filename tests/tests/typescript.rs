use std::{
    borrow::Cow,
    collections::HashMap,
    iter,
    path::Path,
    time::{Duration, SystemTime},
};

use specta::{
    Format, Type, Types,
    datatype::{DataType, Primitive, Reference, Tuple},
};
use specta_typescript::{Exporter, Layout, Typescript, branded, primitives};
use tempfile::TempDir;

use crate::fs_to_string;

branded!(struct BoolBrand(bool) as "BoolBrand");
branded!(struct RefBrand(BrandedReferenceInner) as "RefBrand");

fn identity_types(types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
    Ok(Cow::Borrowed(types))
}

fn identity_datatype<'a>(
    _: &'a Types,
    dt: &'a DataType,
) -> Result<Cow<'a, DataType>, specta::FormatError> {
    Ok(Cow::Borrowed(dt))
}

fn map_bool_to_string<'a>(
    _: &'a Types,
    dt: &'a DataType,
) -> Result<Cow<'a, DataType>, specta::FormatError> {
    Ok(match dt {
        DataType::Primitive(Primitive::bool) => Cow::Owned(DataType::Primitive(Primitive::str)),
        _ => Cow::Borrowed(dt),
    })
}

fn map_bool_to_null_tuple<'a>(
    _: &'a Types,
    dt: &'a DataType,
) -> Result<Cow<'a, DataType>, specta::FormatError> {
    Ok(match dt {
        DataType::Primitive(Primitive::bool) => Cow::Owned(DataType::Tuple(Tuple::new(Vec::new()))),
        _ => Cow::Borrowed(dt),
    })
}

fn map_reference_to_string<'a>(
    _: &'a Types,
    dt: &'a DataType,
) -> Result<Cow<'a, DataType>, specta::FormatError> {
    Ok(match dt {
        DataType::Reference(_) => Cow::Owned(DataType::Primitive(Primitive::str)),
        _ => Cow::Borrowed(dt),
    })
}

fn error_on_bool<'a>(
    _: &'a Types,
    dt: &'a DataType,
) -> Result<Cow<'a, DataType>, specta::FormatError> {
    match dt {
        DataType::Primitive(Primitive::bool) => Err("boom".into()),
        _ => Ok(Cow::Borrowed(dt)),
    }
}

const IDENTITY_FORMAT: Format = Format::new(identity_types, identity_datatype);

#[derive(Type)]
#[specta(collect = false)]
struct BrandedReferenceInner {
    value: bool,
}

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
    );
    let _ = <HashMap<specta_typescript::Any, ()> as Type>::definition(&mut types);

    (types, dts)
}

type PhaseCollection = (&'static str, Format, Vec<(&'static str, DataType)>, Types);

fn phase_collections() -> [PhaseCollection; 3] {
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

    [
        ("raw", IDENTITY_FORMAT, dts.clone(), types.clone()),
        ("serde", specta_serde::format, dts, types),
        (
            "serde_phases",
            specta_serde::format_phases,
            phased_dts,
            phased_types,
        ),
    ]
}

fn phase_output(
    format: Format,
    dts: &[(&'static str, DataType)],
    types: &Types,
    f: impl FnOnce(&[(&'static str, DataType)], &Types) -> Result<String, String>,
) -> String {
    let types = match (format.format_types)(types) {
        Ok(types) => types.into_owned(),
        Err(err) => return format!("ERROR: {err}"),
    };
    let dts = match dts
        .iter()
        .map(|(name, dt)| (format.format_dt)(&types, dt).map(|dt| (*name, dt.into_owned())))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(dts) => dts,
        Err(err) => return format!("ERROR: {err}"),
    };

    f(&dts, &types).unwrap_or_else(|err| format!("ERROR: {err}"))
}

#[test]
fn typescript_export() {
    for (mode, format, _, types) in phase_collections() {
        insta::assert_snapshot!(
            format!("ts-export-{mode}"),
            Typescript::default()
                .export(&types, format)
                .unwrap_or_else(|err| format!("ERROR: {err}"))
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

        let mut registered_types = Types::default();
        let dt = T::definition(&mut registered_types);

        for (mode, format) in [
            ("serde", specta_serde::format),
            ("serde_phases", specta_serde::format_phases),
        ] {
            let types = (format.format_types)(&registered_types).map(|types| types.into_owned());

            let types = match types {
                Ok(types) => types,
                Err(err) => {
                    assert_expected_error(failures, name, mode, "apply", expected_error, err);
                    continue;
                }
            };

            let validate = (format.format_dt)(&types, &dt);

            if let Err(err) = validate {
                assert_expected_error(failures, name, mode, "validate", expected_error, err);
                continue;
            }

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

    let mut failures = Vec::new();

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

        match ts.export(&types, specta_serde::format) {
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
            let output = (|| {
                let path = temp.path().join(&name);
                Typescript::default()
                    .layout(layout)
                    .export_to(&path, &types, format)
                    .map_err(|err| err.to_string())?;
                fs_to_string(&path).map_err(|err| err.to_string())
            })()
            .unwrap_or_else(|err| format!("ERROR: {err}"));

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
        let output = phase_output(format, &dts, &types, |dts, types| {
            let ts = Typescript::default();

            dts.iter()
                .filter_map(|(name, ty)| match ty {
                    DataType::Reference(Reference::Named(reference)) => {
                        reference.get(types).map(|ty| (name, ty))
                    }
                    _ => None,
                })
                .map(|(name, ty)| {
                    primitives::export(&ts, types, iter::once(ty), "")
                        .map(|ty| format!("{name}: {ty}"))
                })
                .collect::<Result<Vec<_>, _>>()
                .map(|exports| exports.join("\n"))
                .map_err(|err| err.to_string())
        });

        insta::assert_snapshot!(format!("export-{mode}"), output);
    }
}

#[test]
fn primitives_export_many() {
    for (mode, format, dts, types) in phase_collections() {
        let output = phase_output(format, &dts, &types, |dts, types| {
            let ts = Typescript::default();
            let ndts = dts
                .iter()
                .filter_map(|(_, ty)| match ty {
                    DataType::Reference(Reference::Named(r)) => r.get(types),
                    _ => None,
                })
                .collect::<Vec<_>>();

            primitives::export(&ts, types, ndts.into_iter(), "").map_err(|err| err.to_string())
        });

        insta::assert_snapshot!(format!("export-many-{mode}"), output);
    }
}

#[test]
fn primitives_export_allows_generic_hashmap_definition() {
    for (mode, format, dts, types) in phase_collections() {
        let output = phase_output(format, &dts, &types, |dts, types| {
            let ts = Typescript::default();
            let hash_map = dts
                .iter()
                .find_map(|(_, ty)| match ty {
                    DataType::Reference(Reference::Named(r)) => {
                        r.get(types).filter(|ndt| ndt.name == "HashMap")
                    }
                    _ => None,
                })
                .expect("HashMap should be registered in shared test fixtures");

            primitives::export(&ts, types, iter::once(hash_map), "").map_err(|err| err.to_string())
        });

        assert!(
            !output.starts_with("ERROR:"),
            "unexpected error while exporting generic HashMap in {}: {output}",
            mode
        );
        assert!(output.contains("export type HashMap<K, V> = { [key in K]: V };"));
    }
}

#[test]
fn primitives_reference() {
    for (mode, format, dts, types) in phase_collections() {
        let output = phase_output(format, &dts, &types, |dts, types| {
            let ts = Typescript::default();
            dts.iter()
                .filter_map(|(s, ty)| match ty {
                    DataType::Reference(r) => Some((s, r)),
                    _ => None,
                })
                .map(|(s, ty)| primitives::reference(&ts, types, ty).map(|ty| format!("{s}: {ty}")))
                .collect::<Result<Vec<_>, _>>()
                .map(|exports| exports.join("\n"))
                .map_err(|err| err.to_string())
        });

        insta::assert_snapshot!(format!("reference-{mode}"), output);
    }
}

#[test]
fn primitives_inline() {
    for (mode, format, dts, types) in phase_collections() {
        let output = phase_output(format, &dts, &types, |dts, types| {
            let ts = Typescript::default();
            dts.iter()
                .map(|(s, ty)| primitives::inline(&ts, types, ty).map(|ty| format!("{s}: {ty}")))
                .collect::<Result<Vec<_>, _>>()
                .map(|exports| exports.join("\n"))
                .map_err(|err| err.to_string())
        });

        insta::assert_snapshot!(format!("inline-{mode}"), output);
    }
}

#[test]
fn primitives_format_datatype_hook_is_recursive() {
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
    let rendered = Exporter::from(Typescript::default())
        .framework_runtime(move |ctx| Ok(ctx.inline(&dt)?.into()))
        .export(&types, Format::new(identity_types, map_bool_to_string))
        .unwrap();

    assert!(rendered.contains("direct: string"), "{rendered}");
    assert!(rendered.contains("list: string[]"), "{rendered}");
    assert!(rendered.contains("[string, string | null]"), "{rendered}");
    assert!(rendered.contains("value: string"), "{rendered}");
}

#[test]
fn primitives_format_datatype_hook_can_return_owned_types() {
    let types = Types::default();
    let rendered = Exporter::from(Typescript::default())
        .framework_runtime(|ctx| Ok(ctx.inline(&DataType::Primitive(Primitive::bool))?.into()))
        .export(&types, Format::new(identity_types, map_bool_to_null_tuple))
        .unwrap();

    assert!(rendered.contains("\n\nnull\n"), "{rendered}");
}

#[test]
fn primitives_reference_format_datatype_hook_can_replace_reference() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Demo {
        value: bool,
    }

    let mut types = Types::default();
    let dt = Demo::definition(&mut types);
    let DataType::Reference(reference) = dt else {
        panic!("expected named reference");
    };

    let rendered = Exporter::from(Typescript::default())
        .framework_runtime(move |ctx| Ok(ctx.reference(&reference)?.into()))
        .export(&types, Format::new(identity_types, map_reference_to_string))
        .unwrap();

    assert!(rendered.contains("\n\nstring\n"), "{rendered}");
}

#[test]
fn primitives_export_format_datatype_hook_updates_named_bodies() {
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
    let rendered = Exporter::from(Typescript::default())
        .framework_runtime(move |ctx| Ok(ctx.export(iter::once(&ndt), "")?.into()))
        .export(&types, Format::new(identity_types, map_bool_to_string))
        .unwrap();

    assert!(rendered.contains("value: string"), "{rendered}");
}

#[test]
fn primitives_format_datatype_hook_errors_bubble_out() {
    let types = Types::default();
    let err = Exporter::from(Typescript::default())
        .framework_runtime(|ctx| Ok(ctx.inline(&DataType::Primitive(Primitive::bool))?.into()))
        .export(&types, Format::new(identity_types, error_on_bool))
        .unwrap_err();

    assert_eq!(
        err.to_string(),
        "Format error: datatype formatter failed: boom"
    );
}

#[test]
fn branded_type_exporter_inline_applies_datatype_mapping() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Demo {
        value: BoolBrand,
    }

    let types = Types::default().register::<Demo>();
    let rendered = Typescript::default()
        .branded_type_impl(|ctx, branded| Ok(ctx.inline(branded.ty())?.into()))
        .export(&types, Format::new(identity_types, map_bool_to_string))
        .unwrap();

    assert!(rendered.contains("value: string"), "{rendered}");
}

#[test]
fn branded_type_exporter_reference_applies_datatype_mapping() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Demo {
        value: RefBrand,
    }

    let types = Types::default().register::<Demo>();
    let rendered = Typescript::default()
        .branded_type_impl(|ctx, branded| match branded.ty() {
            DataType::Reference(reference) => Ok(ctx.reference(reference)?.into()),
            dt => Ok(ctx.inline(dt)?.into()),
        })
        .export(&types, Format::new(identity_types, map_reference_to_string))
        .unwrap();

    assert!(rendered.contains("value: string"), "{rendered}");
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
            DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
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
            DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
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
            DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
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
