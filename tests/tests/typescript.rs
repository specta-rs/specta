use std::path::Path;

use specta::{
    Type, TypeCollection,
    datatype::{DataType, Reference},
};
use specta_serde::SerdeMode;
use specta_typescript::{
    Any, BigIntExportBehavior, Layout, Never, Typescript, Unknown, primitives,
};
use tempfile::TempDir;

use crate::fs_to_string;

pub fn types() -> (TypeCollection, Vec<(&'static str, DataType)>) {
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
    #[derive(Type)]
    pub enum TestCollectionRegister {}
    types = types
        .register::<TestCollectionRegister>()
        .register::<TestCollectionRegister>();

    (types, dts)
}

#[test]
fn typescript_export() {
    for mode in [
        SerdeMode::Both,
        SerdeMode::Serialize,
        SerdeMode::Deserialize,
    ] {
        insta::assert_snapshot!(
            format!("ts-export-{}", mode.to_string().to_lowercase()),
            Typescript::default()
                .with_serde(mode)
                .bigint(BigIntExportBehavior::Number)
                .export(&types().0)
                .unwrap()
        );
    }
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
        for mode in [
            SerdeMode::Both,
            SerdeMode::Serialize,
            SerdeMode::Deserialize,
        ] {
            let name = format!(
                "ts-export-to-{}-{}",
                layout.to_string().to_lowercase(),
                mode.to_string().to_lowercase()
            );
            let path = temp.path().join(&name);

            Typescript::default()
                .with_serde(mode)
                .bigint(BigIntExportBehavior::Number)
                .layout(layout)
                .export_to(&path, &types().0)
                .unwrap();

            insta::assert_snapshot!(name, fs_to_string(&path).unwrap());
        }
    }

    temp.close().unwrap();

    // TODO: Assert layouts error out with `export` method
    // TODO: Assert it errors if given the path to a file
}

#[test]
fn primitives_typescript_framework_utils() {
    // TODO
}

#[test]
fn primitives_export() {
    for mode in [
        SerdeMode::Both,
        SerdeMode::Serialize,
        SerdeMode::Deserialize,
    ] {
        let ts = Typescript::default()
            .with_serde(mode)
            .bigint(BigIntExportBehavior::Number);
        let (types, dts) = crate::types();
        insta::assert_snapshot!(
            format!("export-{}", mode.to_string().to_lowercase()),
            dts.iter()
                .filter_map(|(s, ty)| match ty {
                    DataType::Reference(Reference::Named(r)) =>
                        r.get(&types).cloned().map(|ty| (s, ty)),
                    _ => None,
                })
                .map(|(s, ty)| primitives::export(&ts, &types, &ty).map(|ty| format!("{s}: {ty}")))
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .join("\n")
        );
    }
}

#[test]
fn primitives_reference() {
    for mode in [
        SerdeMode::Both,
        SerdeMode::Serialize,
        SerdeMode::Deserialize,
    ] {
        let ts = Typescript::default()
            .with_serde(mode)
            .bigint(BigIntExportBehavior::Number);
        let (types, dts) = crate::types();
        insta::assert_snapshot!(
            format!("reference-{}", mode.to_string().to_lowercase()),
            dts.iter()
                .filter_map(|(s, ty)| match ty {
                    DataType::Reference(r) => Some((s, r)),
                    _ => None,
                })
                .map(|(s, ty)| primitives::reference(&ts, &types, ty).map(|ty| format!("{s}: {ty}")))
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .join("\n")
        );
    }
}

#[test]
fn primitives_inline() {
    for mode in [
        SerdeMode::Both,
        SerdeMode::Serialize,
        SerdeMode::Deserialize,
    ] {
        let ts = Typescript::default()
            .with_serde(mode)
            .bigint(BigIntExportBehavior::Number);
        let (types, dts) = crate::types();
        insta::assert_snapshot!(
            format!("inline-{}", mode.to_string().to_lowercase()),
            dts.iter()
                .map(|(s, ty)| primitives::inline(&ts, &types, ty).map(|ty| format!("{s}: {ty}")))
                .collect::<Result<Vec<_>, _>>()
                .unwrap()
                .join("\n")
        );
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

        let mut types = TypeCollection::default();
        let ndt = match r#enum::definition(&mut types) {
            DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
            _ => panic!("Failed to get reference"),
        };

        insta::assert_snapshot!(primitives::export(&Typescript::default(), &types, ndt).unwrap_err().to_string(), @r#"Attempted to export Type but was unable to due to name  conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = "new name")]`"#);
    }

    {
        #[derive(Type)]
        #[specta(collect = false)]
        #[allow(non_camel_case_types)]
        pub struct r#enum(String);

        let mut types = TypeCollection::default();
        let ndt = match r#enum::definition(&mut types) {
            DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
            _ => panic!("Failed to get reference"),
        };

        insta::assert_snapshot!(primitives::export(&Typescript::default(), &types, ndt).unwrap_err().to_string(), @r#"Attempted to export Type but was unable to due to name  conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = "new name")]`"#);
    }

    {
        // Typescript reserved type name
        #[derive(Type)]
        #[specta(collect = false)]
        #[allow(non_camel_case_types)]
        pub enum r#enum {
            A(String),
        }

        let mut types = TypeCollection::default();
        let ndt = match r#enum::definition(&mut types) {
            DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
            _ => panic!("Failed to get reference"),
        };

        insta::assert_snapshot!(primitives::export(&Typescript::default(), &types, ndt).unwrap_err().to_string(), @r#"Attempted to export Type but was unable to due to name  conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = "new name")]`"#);
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
//             .export(&TypeCollection::default().register::<Demo>())
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
