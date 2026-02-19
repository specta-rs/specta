use std::path::Path;

use specta::datatype::{DataType, Reference};
use specta::{Type, TypeCollection};
use specta_serde::SerdeMode;
use specta_typescript::{BigIntExportBehavior, JSDoc, Layout, primitives};
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

#[test]
fn export() {
    for mode in [
        SerdeMode::Both,
        SerdeMode::Serialize,
        SerdeMode::Deserialize,
    ] {
        insta::assert_snapshot!(
            format!("inline-{}", mode.to_string().to_lowercase()),
            JSDoc::default()
                .with_serde(mode)
                .bigint(BigIntExportBehavior::Number)
                .export(&crate::types().0)
                .unwrap()
        );
    }
}

#[test]
fn primitives_export_many() {
    for mode in [
        SerdeMode::Both,
        SerdeMode::Serialize,
        SerdeMode::Deserialize,
    ] {
        let jsdoc = JSDoc::default()
            .with_serde(mode)
            .bigint(BigIntExportBehavior::Number);
        let (types, dts) = crate::types();
        let ndts = dts
            .iter()
            .filter_map(|(_, ty)| match ty {
                DataType::Reference(Reference::Named(r)) => r.get(&types),
                _ => None,
            })
            .collect::<Vec<_>>();

        insta::assert_snapshot!(
            format!("primitives-many-inline-{}", mode.to_string().to_lowercase()),
            primitives::export(&jsdoc, &types, ndts.into_iter(), "").unwrap()
        );
    }
}

#[test]
fn jsdoc_export_to_files_uses_jsdoc_import_typedefs() {
    let temp = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&temp).unwrap();
    let temp = TempDir::new_in(temp).unwrap();

    let path = temp.path().join("jsdoc-export-to-files-both");
    let types = TypeCollection::default()
        .register::<jsdoc_export_to_files_runtime_imports_types::one::One>()
        .register::<jsdoc_export_to_files_runtime_imports_types::two::Two>()
        .register::<jsdoc_export_to_files_runtime_imports_types::three::Three>();

    JSDoc::default()
        .layout(Layout::Files)
        .export_to(&path, &types)
        .unwrap();

    let output = fs_to_string(&path).unwrap();
    assert!(!output.contains("import type"));
    assert!(!output.contains("import * as"));
    assert!(output.contains("@typedef {import(\""));
    insta::assert_snapshot!("jsdoc-export-to-files-both", output);

    temp.close().unwrap();
}

// TODO: BigInt checks
// TODO: Confirm different layouts
// TODO: Unit test JSDoc and other languages

// TODO: Ensure this is feature matching with the Typescript testing
