use std::path::Path;

use specta::datatype::{DataType, Reference};
use specta::{ResolvedTypes, Type, Types};
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
fn export() {
    for (mode, types) in phase_collections(crate::sanitize_typescript_bigints_in_types(
        crate::types().0,
    )) {
        let output = match types {
            Ok(types) => JSDoc::default().export(&types).unwrap(),
            Err(err) => format!("ERROR: {err}"),
        };

        insta::assert_snapshot!(format!("inline-{mode}"), output);
    }
}

#[test]
fn primitives_export_many() {
    let (types, dts) = crate::types();
    let dts = crate::sanitize_typescript_bigints_in_dts(dts);

    for (mode, types) in phase_collections(crate::sanitize_typescript_bigints_in_types(types)) {
        let output = match types {
            Ok(types) => {
                let jsdoc = JSDoc::default();
                let ndts = dts
                    .iter()
                    .filter_map(|(_, ty)| match ty {
                        DataType::Reference(Reference::Named(r)) => r.get(types.as_types()),
                        _ => None,
                    })
                    .collect::<Vec<_>>();

                primitives::export(&jsdoc, &types, ndts.into_iter(), "").unwrap()
            }
            Err(err) => format!("ERROR: {err}"),
        };

        insta::assert_snapshot!(format!("primitives-many-inline-{mode}"), output);
    }
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
        .export_to(&path, &ResolvedTypes::from_resolved_types(types))
        .unwrap();

    let output = fs_to_string(&path).unwrap();
    insta::assert_snapshot!("jsdoc-export-to-files-both", output);

    temp.close().unwrap();
}

// TODO: BigInt checks
// TODO: Confirm different layouts
// TODO: Unit test JSDoc and other languages

// TODO: Ensure this is feature matching with the Typescript testing
