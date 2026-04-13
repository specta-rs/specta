use std::path::Path;

use specta::{
    ResolvedTypes, Type, Types,
    datatype::{DataType, NamedDataType, Primitive},
};
use specta_typescript::{Layout, Typescript};
use tempfile::TempDir;

#[derive(Type)]
struct Testing {
    a: testing::Testing,
}

#[derive(Type)]
struct Another {
    bruh: String,
}

#[derive(Type)]
struct MoreType {
    u: String,
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

#[test]
fn duplicate_typenames_layouts() {
    let types = Types::default()
        .register::<Testing>()
        .register::<Another>()
        .register::<MoreType>();
    let resolved = ResolvedTypes::from_resolved_types(types.clone());

    assert_error_contains(
        Typescript::default().export(&resolved),
        "Detected multiple types",
    );

    assert_error_contains(
        Typescript::default()
            .layout(Layout::FlatFile)
            .export(&resolved),
        "Detected multiple types",
    );

    let module_prefixed = Typescript::default()
        .layout(Layout::ModulePrefixedName)
        .export(&resolved)
        .unwrap();
    insta::assert_snapshot!("layouts-duplicate-module-prefixed", module_prefixed);

    let namespaces = Typescript::default()
        .layout(Layout::Namespaces)
        .export(&resolved)
        .unwrap();
    insta::assert_snapshot!("layouts-duplicate-namespaces", namespaces);

    assert_error_contains(
        Typescript::default()
            .layout(Layout::Files)
            .export(&resolved),
        "Unable to export layout Files",
    );

    let temp = temp_dir();
    let path = temp.path().join("duplicate-layout");
    Typescript::default()
        .layout(Layout::Files)
        .export_to(&path, &resolved)
        .unwrap();

    let output = crate::fs_to_string(&path).unwrap();
    insta::assert_snapshot!("layouts-duplicate-files", output);
}

#[test]
fn non_duplicate_typenames_layouts() {
    let types = Types::default()
        .register::<Another>()
        .register::<MoreType>();
    let resolved = ResolvedTypes::from_resolved_types(types.clone());

    let default_output = Typescript::default().export(&resolved).unwrap();
    insta::assert_snapshot!("layouts-non-duplicate-default", default_output);

    let flat = Typescript::default()
        .layout(Layout::FlatFile)
        .export(&resolved)
        .unwrap();
    insta::assert_snapshot!("layouts-non-duplicate-flat", flat);

    let module_prefixed = Typescript::default()
        .layout(Layout::ModulePrefixedName)
        .export(&resolved)
        .unwrap();
    insta::assert_snapshot!("layouts-non-duplicate-module-prefixed", module_prefixed);

    let namespaces = Typescript::default()
        .layout(Layout::Namespaces)
        .export(&resolved)
        .unwrap();
    insta::assert_snapshot!("layouts-non-duplicate-namespaces", namespaces);

    assert_error_contains(
        Typescript::default()
            .layout(Layout::Files)
            .export(&resolved),
        "Unable to export layout Files",
    );

    let temp = temp_dir();
    let path = temp.path().join("no-duplicate-layout");
    Typescript::default()
        .layout(Layout::Files)
        .export_to(&path, &resolved)
        .unwrap();

    let output = crate::fs_to_string(&path).unwrap();
    insta::assert_snapshot!("layouts-non-duplicate-files", output);
}

#[test]
fn empty_module_path_layouts() {
    let mut types = Types::default();

    let mut testing = NamedDataType::new("testing", Vec::new(), DataType::Primitive(Primitive::i8));
    testing.module_path = "".into();
    testing.register(&mut types);
    let resolved = ResolvedTypes::from_resolved_types(types.clone());

    let flat = Typescript::default()
        .layout(Layout::FlatFile)
        .export(&resolved)
        .unwrap();
    insta::assert_snapshot!("layouts-empty-module-path-flat", flat);

    let module_prefixed = Typescript::default()
        .layout(Layout::ModulePrefixedName)
        .export(&resolved)
        .unwrap();
    insta::assert_snapshot!("layouts-empty-module-path-module-prefixed", module_prefixed);

    let namespaces = Typescript::default()
        .layout(Layout::Namespaces)
        .export(&resolved)
        .unwrap();
    insta::assert_snapshot!("layouts-empty-module-path-namespaces", namespaces);

    let temp = temp_dir();
    let path = temp.path().join("empty-module-path-layout");
    Typescript::default()
        .layout(Layout::Files)
        .export_to(&path, &resolved)
        .unwrap();

    let output = crate::fs_to_string(Path::new(&path)).unwrap();
    insta::assert_snapshot!("layouts-empty-module-path-files", output);
}

fn temp_dir() -> TempDir {
    let temp_root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&temp_root).unwrap();
    TempDir::new_in(temp_root).unwrap()
}

fn assert_error_contains<T>(result: Result<T, specta_typescript::Error>, expected: &str) {
    let error = match result {
        Ok(_) => panic!("expected exporter to fail"),
        Err(error) => error,
    };
    assert!(
        error.to_string().contains(expected),
        "error '{error}' did not contain '{expected}'"
    );
}
