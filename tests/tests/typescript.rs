use std::{iter, path::Path};

use specta::{
    ResolvedTypes, Type, Types,
    datatype::{DataType, Reference},
};
use specta_typescript::{BigIntExportBehavior, Layout, Typescript, primitives};
use tempfile::TempDir;

use crate::fs_to_string;

pub fn types() -> (Types, Vec<(&'static str, DataType)>) {
    let (mut types, dts) = crate::types();

    // Test ts-specific types
    // types = types
    //     .register::<Any>()
    //     .register::<Any<String>>()
    //     .register::<Unknown>()
    //     .register::<Unknown<String>>()
    //     .register::<Never>()
    //     .register::<Never<String>>();

    // dts.push(value);

    // Test that the types don't get duplicated in the type map.
    {
        #[derive(Type)]
        #[specta(collect = false)]
        pub enum TestCollectionRegister {}
        types = types
            .register::<TestCollectionRegister>()
            .register::<TestCollectionRegister>();
    }

    (types, dts)
}

fn phase_collections(
    types: Types,
) -> [(&'static str, Result<ResolvedTypes, specta_serde::Error>); 3] {
    [
        ("raw", Ok(ResolvedTypes::from_resolved_types(types.clone()))),
        ("serde", specta_serde::apply(types.clone())),
        ("serde_phases", specta_serde::apply_phases(types)),
    ]
}

#[test]
fn typescript_export() {
    for (mode, types) in phase_collections(types().0) {
        insta::assert_snapshot!(
            format!("ts-export-{mode}"),
            match types {
                Ok(types) => Typescript::default()
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
        let types = Types::default().register::<T>();
        for (mode, output) in [
            (
                "serde",
                specta_serde::apply(types.clone()).map(|types| {
                    Typescript::default()
                        .bigint(BigIntExportBehavior::Number)
                        .export(&types)
                }),
            ),
            (
                "serde_phases",
                specta_serde::apply_phases(types.clone()).map(|types| {
                    Typescript::default()
                        .bigint(BigIntExportBehavior::Number)
                        .export(&types)
                }),
            ),
        ] {
            match output {
                Err(err) if !err.to_string().contains(expected_error) => failures.push(format!(
                    "{name} ({mode}): expected error containing '{expected_error}', got '{err}'"
                )),
                Err(_) => {}
                Ok(Err(err)) => failures.push(format!(
                    "{name} ({mode}): expected serde error but export failed with '{err}'"
                )),
                Ok(Ok(output)) => failures.push(format!(
                    "{name} ({mode}): expected serde error, got output: {output}"
                )),
            }
        }
    }

    #[derive(Type, serde::Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(tag = "type")]
    enum InternallyTaggedB {
        A(String),
    }

    #[derive(Type, serde::Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(tag = "type")]
    enum InternallyTaggedC {
        A(Vec<String>),
    }

    #[derive(Type, serde::Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(tag = "type")]
    enum InternallyTaggedG {
        A(InternallyTaggedGInner),
    }

    #[derive(Type, serde::Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(untagged)]
    enum InternallyTaggedGInner {
        A(String),
    }

    #[derive(Type, serde::Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(tag = "type")]
    enum InternallyTaggedI {
        A(InternallyTaggedIInner),
    }

    #[derive(Type, serde::Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(transparent)]
    struct InternallyTaggedIInner(String);

    #[derive(Type, serde::Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(tag = "a")]
    enum TaggedEnumOfEmptyTupleStruct {
        A(EmptyTupleStruct),
        B(EmptyTupleStruct),
    }

    #[derive(Type, serde::Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    struct EmptyTupleStruct();

    #[derive(Type, serde::Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    enum SkipOnlyVariantExternallyTagged {
        #[specta(skip)]
        A(String),
    }

    #[derive(Type, serde::Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(tag = "t")]
    enum SkipOnlyVariantInternallyTagged {
        #[specta(skip)]
        A(String),
    }

    #[derive(Type, serde::Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(tag = "t", content = "c")]
    enum SkipOnlyVariantAdjacentlyTagged {
        #[specta(skip)]
        A(String),
    }

    #[derive(Type, serde::Serialize, serde::Deserialize)]
    #[specta(collect = false)]
    #[serde(untagged)]
    enum SkipOnlyVariantUntagged {
        #[specta(skip)]
        A(String),
    }

    let mut failures = Vec::new();

    assert_serde_error::<InternallyTaggedB>(
        &mut failures,
        "InternallyTaggedB",
        "Invalid internally tagged enum",
    );
    assert_serde_error::<InternallyTaggedC>(
        &mut failures,
        "InternallyTaggedC",
        "Invalid internally tagged enum",
    );
    assert_serde_error::<InternallyTaggedG>(
        &mut failures,
        "InternallyTaggedG",
        "Invalid internally tagged enum",
    );
    assert_serde_error::<InternallyTaggedI>(
        &mut failures,
        "InternallyTaggedI",
        "Invalid internally tagged enum",
    );

    assert_serde_error::<TaggedEnumOfEmptyTupleStruct>(
        &mut failures,
        "TaggedEnumOfEmptyTupleStruct",
        "Invalid internally tagged enum",
    );
    assert_serde_error::<SkipOnlyVariantExternallyTagged>(
        &mut failures,
        "SkipOnlyVariantExternallyTagged",
        "Invalid usage of #[serde(skip)]",
    );
    assert_serde_error::<SkipOnlyVariantInternallyTagged>(
        &mut failures,
        "SkipOnlyVariantInternallyTagged",
        "Invalid usage of #[serde(skip)]",
    );
    assert_serde_error::<SkipOnlyVariantAdjacentlyTagged>(
        &mut failures,
        "SkipOnlyVariantAdjacentlyTagged",
        "Invalid usage of #[serde(skip)]",
    );
    assert_serde_error::<SkipOnlyVariantUntagged>(
        &mut failures,
        "SkipOnlyVariantUntagged",
        "Invalid usage of #[serde(skip)]",
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
        for (mode, types) in phase_collections(types().0) {
            let name = format!("ts-export-to-{}-{mode}", layout.to_string().to_lowercase());
            let output = match types {
                Ok(types) => {
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
    let (types, dts) = crate::types();
    for (mode, types) in phase_collections(types) {
        let output = match types {
            Ok(types) => {
                let ts = Typescript::default().bigint(BigIntExportBehavior::Number);
                dts.iter()
                    .filter_map(|(s, ty)| match ty {
                        DataType::Reference(Reference::Named(r)) => {
                            r.get(types.as_types()).cloned().map(|ty| (s, ty))
                        }
                        _ => None,
                    })
                    .map(|(s, ty)| {
                        primitives::export(&ts, &types, iter::once(&ty), "")
                            .map(|ty| format!("{s}: {ty}"))
                    })
                    .collect::<Result<Vec<_>, _>>()
                    .unwrap()
                    .join("\n")
            }
            Err(err) => format!("ERROR: {err}"),
        };

        insta::assert_snapshot!(format!("export-{mode}"), output);
    }
}

#[test]
fn primitives_export_many() {
    let (types, dts) = crate::types();
    for (mode, types) in phase_collections(types) {
        let output = match types {
            Ok(types) => {
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
fn primitives_reference() {
    let (types, dts) = crate::types();
    for (mode, types) in phase_collections(types) {
        let output = match types {
            Ok(types) => {
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
    let (types, dts) = crate::types();
    for (mode, types) in phase_collections(types) {
        let output = match types {
            Ok(types) => {
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
