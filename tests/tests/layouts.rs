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
    assert!(module_prefixed.contains("Another"));
    assert!(module_prefixed.contains("MoreType"));
    assert!(module_prefixed.contains("testing2"));

    let namespaces = Typescript::default()
        .layout(Layout::Namespaces)
        .export(&resolved)
        .unwrap();
    assert!(namespaces.contains("export namespace"));
    assert!(namespaces.contains("testing2"));

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
    assert!(output.contains(".ts"));
    assert!(output.contains("testing.ts"));
    assert!(output.contains("testing2.ts"));
}

#[test]
fn non_duplicate_typenames_layouts() {
    let types = Types::default()
        .register::<Another>()
        .register::<MoreType>();
    let resolved = ResolvedTypes::from_resolved_types(types.clone());

    let default_output = Typescript::default().export(&resolved).unwrap();
    assert!(default_output.contains("export type Another"));
    assert!(default_output.contains("export type MoreType"));

    let flat = Typescript::default()
        .layout(Layout::FlatFile)
        .export(&resolved)
        .unwrap();
    assert!(flat.contains("export type Another"));
    assert!(flat.contains("export type MoreType"));

    let module_prefixed = Typescript::default()
        .layout(Layout::ModulePrefixedName)
        .export(&resolved)
        .unwrap();
    assert!(module_prefixed.contains("Another"));
    assert!(module_prefixed.contains("MoreType"));

    let namespaces = Typescript::default()
        .layout(Layout::Namespaces)
        .export(&resolved)
        .unwrap();
    assert!(namespaces.contains("export namespace"));
    assert!(namespaces.contains("Another"));
    assert!(namespaces.contains("MoreType"));

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
    assert!(output.contains("layouts.ts"));
    assert!(output.contains("export type Another"));
    assert!(output.contains("export type MoreType"));
}

#[test]
fn empty_module_path_layouts() {
    let mut types = Types::default();

    let mut testing = NamedDataType::new("testing", Vec::new(), DataType::Primitive(Primitive::i8));
    testing.set_module_path("".into());
    testing.register(&mut types);
    let resolved = ResolvedTypes::from_resolved_types(types.clone());

    let flat = Typescript::default()
        .layout(Layout::FlatFile)
        .export(&resolved)
        .unwrap();
    assert!(flat.contains("export type testing = number"));

    let module_prefixed = Typescript::default()
        .layout(Layout::ModulePrefixedName)
        .export(&resolved)
        .unwrap();
    assert!(module_prefixed.contains("testing"));

    let namespaces = Typescript::default()
        .layout(Layout::Namespaces)
        .export(&resolved)
        .unwrap();
    assert!(namespaces.contains("export namespace"));
    assert!(namespaces.contains("testing = number"));

    let temp = temp_dir();
    let path = temp.path().join("empty-module-path-layout");
    Typescript::default()
        .layout(Layout::Files)
        .export_to(&path, &resolved)
        .unwrap();

    let output = crate::fs_to_string(Path::new(&path)).unwrap();
    assert!(output.contains(".ts"));
    assert!(output.contains("export type testing = number"));
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
