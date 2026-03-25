use std::{collections::HashMap, iter, path::Path};

use specta::{
    ResolvedTypes, Type, Types,
    datatype::{DataType, Reference},
};
use specta_typescript::{BigIntExportBehavior, Layout, Typescript, primitives};
use tempfile::TempDir;

use crate::fs_to_string;

fn typescript_types() -> (Types, Vec<(&'static str, DataType)>) {
    crate::types!(
        specta_typescript::Any,
        specta_typescript::Any<String>,
        specta_typescript::Unknown,
        specta_typescript::Unknown<String>,
        specta_typescript::Never,
        specta_typescript::Never<String>,
        HashMap<specta_typescript::Any, ()>,
    )
}

fn phase_collections() -> [(
    &'static str,
    Result<(Vec<(&'static str, DataType)>, ResolvedTypes), specta_serde::Error>,
); 3] {
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
        (
            "raw",
            Ok((
                dts.clone(),
                ResolvedTypes::from_resolved_types(types.clone()),
            )),
        ),
        (
            "serde",
            specta_serde::apply(types).map(|types| (dts, types)),
        ),
        (
            "serde_phases",
            specta_serde::apply_phases(phased_types).map(|types| (phased_dts, types)),
        ),
    ]
}

#[test]
fn typescript_export() {
    for (mode, types) in phase_collections() {
        insta::assert_snapshot!(
            format!("ts-export-{mode}"),
            match types {
                Ok((_, types)) => Typescript::default()
                    .bigint(BigIntExportBehavior::Number)
                    .export(&types)
                    .unwrap(),
                Err(err) => format!("ERROR: {err}"),
            }
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

        for mode in ["serde", "serde_phases"] {
            let types = match mode {
                "serde" => specta_serde::apply(types.clone()),
                _ => specta_serde::apply_phases(types.clone()),
            };

            let types = match types {
                Ok(types) => types,
                Err(err) => {
                    assert_expected_error(failures, name, mode, "apply", expected_error, err);
                    continue;
                }
            };

            if let Err(err) = specta_serde::validate(&dt, &types) {
                assert_expected_error(failures, name, mode, "validate", expected_error, err);
                continue;
            }

            match Typescript::default()
                .bigint(BigIntExportBehavior::Number)
                .export(&types)
            {
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
        for (mode, types) in phase_collections() {
            let name = format!("ts-export-to-{}-{mode}", layout.to_string().to_lowercase());
            let output = match types {
                Ok((_, types)) => {
                    let path = temp.path().join(&name);
                    Typescript::default()
                        .bigint(BigIntExportBehavior::Number)
                        .layout(layout)
                        .export_to(&path, &types)
                        .unwrap();
                    fs_to_string(&path).unwrap()
                }
                Err(err) => format!("ERROR: {err}"),
            };

            insta::assert_snapshot!(name, output);
        }
    }

    temp.close().unwrap();

    // TODO: Assert layouts error out with `export` method
    // TODO: Assert it errors if given the path to a file
}

#[test]
fn primitives_export() {
    for (mode, result) in phase_collections() {
        let output = result.map_or_else(
            |err| format!("ERROR: {err}"),
            |(dts, types)| {
                let ts = Typescript::default().bigint(BigIntExportBehavior::Number);
                dts.iter()
                    .filter_map(|(name, ty)| match ty {
                        DataType::Reference(Reference::Named(reference)) => {
                            reference.get(types.as_types()).map(|ty| (name, ty))
                        }
                        _ => None,
                    })
                    .map(|(name, ty)| {
                        primitives::export(&ts, &types, iter::once(ty), "")
                            .map(|ty| format!("{name}: {ty}"))
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .map(|exports| exports.join("\n"))
                    .unwrap_or_else(|err| format!("ERROR: {err}"))
            },
        );

        insta::assert_snapshot!(format!("export-{mode}"), output);
    }
}

#[test]
fn primitives_export_many() {
    for (mode, types) in phase_collections() {
        let output = match types {
            Ok((dts, types)) => {
                let ts = Typescript::default().bigint(BigIntExportBehavior::Number);
                let ndts = dts
                    .iter()
                    .filter_map(|(_, ty)| match ty {
                        DataType::Reference(Reference::Named(r)) => r.get(types.as_types()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                primitives::export(&ts, &types, ndts.into_iter(), "").unwrap()
            }
            Err(err) => format!("ERROR: {err}"),
        };

        insta::assert_snapshot!(format!("export-many-{mode}"), output);
    }
}

#[test]
fn primitives_export_allows_generic_hashmap_definition() {
    for (mode, types) in phase_collections() {
        let output = match types {
            Ok((dts, types)) => {
                let ts = Typescript::default().bigint(BigIntExportBehavior::Number);
                let hash_map = dts
                    .iter()
                    .find_map(|(_, ty)| match ty {
                        DataType::Reference(Reference::Named(r)) => r
                            .get(types.as_types())
                            .filter(|ndt| ndt.name() == "HashMap"),
                        _ => None,
                    })
                    .expect("HashMap should be registered in shared test fixtures");

                primitives::export(&ts, &types, iter::once(hash_map), "").unwrap()
            }
            Err(err) => format!("ERROR: {err}"),
        };

        assert!(
            !output.starts_with("ERROR:"),
            "unexpected error while exporting generic HashMap in {mode}: {output}"
        );
        assert!(output.contains("export type HashMap<K, V> = { [key in K]: V };"));
    }
}

#[test]
fn primitives_reference() {
    for (mode, types) in phase_collections() {
        let output = match types {
            Ok((dts, types)) => {
                let ts = Typescript::default().bigint(BigIntExportBehavior::Number);
                dts.iter()
                    .filter_map(|(s, ty)| match ty {
                        DataType::Reference(r) => Some((s, r)),
                        _ => None,
                    })
                    .map(|(s, ty)| {
                        primitives::reference(&ts, &types, ty).map(|ty| format!("{s}: {ty}"))
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap()
                    .join("\n")
            }
            Err(err) => format!("ERROR: {err}"),
        };

        insta::assert_snapshot!(format!("reference-{mode}"), output);
    }
}

#[test]
fn primitives_inline() {
    for (mode, types) in phase_collections() {
        let output = match types {
            Ok((dts, types)) => {
                let ts = Typescript::default().bigint(BigIntExportBehavior::Number);
                dts.iter()
                    .map(|(s, ty)| {
                        primitives::inline(&ts, &types, ty).map(|ty| format!("{s}: {ty}"))
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap()
                    .join("\n")
            }
            Err(err) => format!("ERROR: {err}"),
        };

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
            DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
            _ => panic!("Failed to get reference"),
        };
        let resolved = ResolvedTypes::from_resolved_types(types.clone());

        insta::assert_snapshot!(primitives::export(&Typescript::default(), &resolved, iter::once(ndt), "").unwrap_err().to_string(), @r#"Attempted to export  but was unable to due to name "enum" conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = "new name")]`"#);
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
        let resolved = ResolvedTypes::from_resolved_types(types.clone());

        insta::assert_snapshot!(primitives::export(&Typescript::default(), &resolved, iter::once(ndt), "").unwrap_err().to_string(), @r#"Attempted to export  but was unable to due to name "enum" conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = "new name")]`"#);
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
        let resolved = ResolvedTypes::from_resolved_types(types.clone());

        insta::assert_snapshot!(primitives::export(&Typescript::default(), &resolved, iter::once(ndt), "").unwrap_err().to_string(), @r#"Attempted to export  but was unable to due to name "enum" conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = "new name")]`"#);
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
