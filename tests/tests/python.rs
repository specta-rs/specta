use std::{borrow::Cow, path::Path};

use specta::{Format, Type, Types, datatype::DataType};
use specta_python::{Layout, Python, primitives};
use tempfile::TempDir;

struct IdentityFormat;

struct MapTypeOnlyFormat;

struct RootMapTypeFormat;

impl Format for MapTypeOnlyFormat {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(
        &'_ self,
        _: &Types,
        datatype: &DataType,
    ) -> Result<Cow<'_, DataType>, specta::FormatError> {
        Ok(Cow::Owned(
            if matches!(
                datatype,
                DataType::Primitive(specta::datatype::Primitive::i64)
            ) {
                DataType::Primitive(specta::datatype::Primitive::str)
            } else {
                datatype.clone()
            },
        ))
    }
}

impl Format for RootMapTypeFormat {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(
        &'_ self,
        _: &Types,
        datatype: &DataType,
    ) -> Result<Cow<'_, DataType>, specta::FormatError> {
        Ok(Cow::Owned(if matches!(datatype, DataType::Struct(_)) {
            DataType::Primitive(specta::datatype::Primitive::bool)
        } else {
            datatype.clone()
        }))
    }
}

struct FailingMapTypeFormat;

impl Format for FailingMapTypeFormat {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(
        &'_ self,
        _: &Types,
        _: &DataType,
    ) -> Result<Cow<'_, DataType>, specta::FormatError> {
        Err(std::io::Error::other("intentional formatter failure").into())
    }
}

mod duplicate_a {
    #[derive(specta::Type)]
    #[specta(collect = false)]
    pub struct Duplicate {
        pub first: String,
    }
}

mod duplicate_b {
    #[derive(specta::Type)]
    #[specta(collect = false)]
    pub struct Duplicate {
        pub second: String,
    }
}

mod files_layout {
    #[derive(specta::Type)]
    #[specta(collect = false)]
    pub struct Root {
        pub child: nested::Child,
    }

    pub mod nested {
        #[derive(specta::Type)]
        #[specta(collect = false)]
        pub struct Child {
            pub value: String,
            pub parent: Option<Box<super::Root>>,
        }
    }
}

mod import_alias_collision {
    pub mod a {
        pub mod b {
            #[derive(specta::Type)]
            #[specta(collect = false)]
            pub struct X {
                pub nested: String,
            }
        }
    }

    pub mod a_b {
        #[derive(specta::Type)]
        #[specta(collect = false)]
        pub struct X {
            pub flat: String,
        }
    }

    pub mod consumer {
        #[derive(specta::Type)]
        #[specta(collect = false)]
        pub struct Uses {
            pub nested: super::a::b::X,
            pub flat: super::a_b::X,
        }
    }
}

mod namespace_binding_collision {
    pub mod a {
        #[derive(specta::Type)]
        #[specta(collect = false)]
        pub struct B {
            pub value: String,
        }

        pub mod b {
            #[derive(specta::Type)]
            #[specta(collect = false)]
            pub struct Child {
                pub value: String,
            }
        }
    }
}

mod generic_default_import {
    pub mod target {
        #[derive(specta::Type)]
        #[specta(collect = false)]
        pub struct Other {
            pub value: String,
        }
    }

    pub mod consumer {
        #[derive(specta::Type)]
        #[specta(collect = false)]
        pub struct Box<T = super::target::Other> {
            pub value: T,
        }
    }
}

impl Format for IdentityFormat {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(
        &'_ self,
        _: &Types,
        datatype: &DataType,
    ) -> Result<Cow<'_, DataType>, specta::FormatError> {
        Ok(Cow::Owned(datatype.clone()))
    }
}

fn phase_collections() -> Vec<(&'static str, Box<dyn Format>, Types)> {
    let (types, _) = crate::types();
    let (mut phased_types, _) = crate::types_phased();
    phased_types.extend(&types);
    vec![
        ("raw", Box::new(IdentityFormat), types.clone()),
        ("serde", Box::new(specta_serde::Format), types),
        (
            "serde_phases",
            Box::new(specta_serde::PhasesFormat),
            phased_types,
        ),
    ]
}

#[test]
fn python_export() {
    for (mode, format, types) in phase_collections() {
        insta::assert_snapshot!(
            format!("python-export-{mode}"),
            Python::default().export(&types, format).unwrap()
        );
    }
}

#[test]
fn python_configuration() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Demo {
        value: i64,
    }

    let types = Types::default().register::<Demo>();
    let output = Python::default()
        .header("# ruff: noqa")
        .with_raw("ANSWER: int = 42")
        .export(&types, IdentityFormat)
        .unwrap();

    assert!(output.starts_with("# ruff: noqa\n# This file has been generated by Specta."));
    assert!(
        output.contains("class Demo(_specta_typing.TypedDict):\n    value: _specta_builtins.int")
    );
    assert!(output.ends_with("ANSWER: int = 42\n"));
}

#[test]
fn python_applies_datatype_formatters_with_paths() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Demo {
        value: i64,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct Defaulted<T = i64> {
        value: T,
    }

    let types = Types::default().register::<Demo>().register::<Defaulted>();
    let output = Python::default().export(&types, MapTypeOnlyFormat).unwrap();
    assert!(output.contains("value: _specta_builtins.str"));
    assert!(output.contains("class Defaulted[T = _specta_builtins.str]"));

    let output = Python::default()
        .export(&Types::default().register::<Demo>(), RootMapTypeFormat)
        .unwrap();
    assert!(output.contains("type Demo = _specta_builtins.bool"));

    let error = Python::default()
        .export(&Types::default().register::<Demo>(), FailingMapTypeFormat)
        .unwrap_err();
    assert!(error.to_string().contains("Demo"));
    assert_eq!(
        error
            .named_datatype()
            .map(|datatype| datatype.name.as_ref()),
        Some("Demo")
    );
}

#[test]
fn python_inline_and_opaque_types() {
    assert_eq!(
        primitives::inline(
            &Python::default(),
            &Types::default(),
            &<Vec<Option<i64>> as Type>::definition(&mut Types::default()),
        )
        .unwrap(),
        "_specta_builtins.list[None | _specta_builtins.int]"
    );
    assert_eq!(
        primitives::inline(
            &Python::default(),
            &Types::default(),
            &<specta_python::Any as Type>::definition(&mut Types::default()),
        )
        .unwrap(),
        "_specta_typing.Any"
    );
    assert_eq!(
        primitives::inline(
            &Python::default(),
            &Types::default(),
            &DataType::Reference(specta_python::define("datetime.datetime")),
        )
        .unwrap(),
        "datetime.datetime"
    );
    assert_eq!(
        primitives::inline(
            &Python::default(),
            &Types::default(),
            &<Option<()> as Type>::definition(&mut Types::default()),
        )
        .unwrap(),
        "None"
    );
}

#[test]
fn python_flatten_uses_omitted_generic_defaults() {
    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    struct Inner<T = String> {
        value: T,
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    struct Outer {
        outer: bool,
        #[serde(flatten)]
        inner: Inner,
    }

    let output = Python::default()
        .export(&Types::default().register::<Outer>(), specta_serde::Format)
        .unwrap();
    assert!(
        output.contains("\"value\": _specta_builtins.str"),
        "{output}"
    );
}

#[test]
fn python_rejects_debug_constant_name() {
    #[allow(non_camel_case_types)]
    #[derive(Type)]
    #[specta(collect = false)]
    struct __debug__ {
        value: String,
    }

    let error = Python::default()
        .export(&Types::default().register::<__debug__>(), IdentityFormat)
        .unwrap_err();
    assert!(error.to_string().contains("__debug__"));
}

#[test]
fn python_preserves_dunder_wire_keys() {
    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    struct Dunder {
        #[serde(rename = "__private")]
        private: String,
    }

    #[derive(Type, serde::Serialize)]
    #[specta(collect = false)]
    struct Normalized {
        #[serde(rename = "K")]
        kelvin: String,
    }

    let output = Python::default()
        .export(
            &Types::default()
                .register::<Dunder>()
                .register::<Normalized>(),
            specta_serde::Format,
        )
        .unwrap();
    assert!(output.contains(
        "type Dunder = _specta_typing.TypedDict(\"test__python__Dunder\", {\"__private\": _specta_builtins.str})"
    ));
    assert!(!output.contains("class Dunder("));
    assert!(output.contains(
        "type Normalized = _specta_typing.TypedDict(\"test__python__Normalized\", {\"K\": _specta_builtins.str})"
    ));
    assert!(!output.contains("class Normalized("));
}

#[test]
fn python_export_to_file_and_files_layout() {
    let types = Types::default().register::<files_layout::Root>();
    let temp = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&temp).unwrap();
    let temp = TempDir::new_in(temp).unwrap();

    let file = temp.path().join("bindings.py");
    Python::default()
        .export_to(&file, &types, IdentityFormat)
        .unwrap();
    assert!(
        std::fs::read_to_string(file)
            .unwrap()
            .contains("class Root")
    );

    let package = temp.path().join("bindings");
    Python::default()
        .layout(Layout::Files)
        .export_to(&package, &types, IdentityFormat)
        .unwrap();
    let unrelated = package.join("unrelated/empty");
    std::fs::create_dir_all(&unrelated).unwrap();
    Python::default()
        .layout(Layout::Files)
        .export_to(&package, &types, IdentityFormat)
        .unwrap();
    assert!(unrelated.exists());
    assert!(package.join("__init__.py").exists());
    assert!(package.join("test/__init__.py").exists());
    assert!(package.join("test/python/__init__.py").exists());
    assert!(
        package
            .join("test/python/files_layout/__init__.py")
            .exists()
    );
    assert!(
        package
            .join("test/python/files_layout/nested/__init__.py")
            .exists()
    );
    let root_module =
        std::fs::read_to_string(package.join("test/python/files_layout/__init__.py")).unwrap();
    assert!(root_module.contains(
        "from .nested import Child as _specta_import_4_test_6_python_12_files_layout_6_nested_Child"
    ));
    assert!(
        root_module
            .contains("child: _specta_import_4_test_6_python_12_files_layout_6_nested_Child")
    );
    assert!(
        root_module.rfind("from .nested import Child as ").unwrap()
            > root_module.find("class Root(").unwrap()
    );
    let child_module =
        std::fs::read_to_string(package.join("test/python/files_layout/nested/__init__.py"))
            .unwrap();
    assert!(
        child_module
            .contains("from .. import Root as _specta_import_4_test_6_python_12_files_layout_Root")
    );
    assert!(
        child_module.contains("parent: None | _specta_import_4_test_6_python_12_files_layout_Root")
    );
    assert!(
        child_module.rfind("from .. import Root as ").unwrap()
            > child_module.find("class Child(").unwrap()
    );

    let error = Python::default()
        .layout(Layout::Files)
        .export(&types, IdentityFormat)
        .unwrap_err();
    assert!(error.to_string().contains("Python::export"));
}

#[test]
fn python_files_layout_uses_unambiguous_import_aliases() {
    let types = Types::default().register::<import_alias_collision::consumer::Uses>();
    let temp = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&temp).unwrap();
    let temp = TempDir::new_in(temp).unwrap();
    let package = temp.path().join("bindings");
    Python::default()
        .layout(Layout::Files)
        .export_to(&package, &types, IdentityFormat)
        .unwrap();

    let consumer = std::fs::read_to_string(
        package.join("test/python/import_alias_collision/consumer/__init__.py"),
    )
    .unwrap();
    let aliases = consumer
        .lines()
        .map(str::trim_start)
        .filter(|line| line.starts_with("from ") && line.ends_with("_X"))
        .collect::<Vec<_>>();
    assert_eq!(aliases.len(), 4);
    assert_ne!(
        aliases[0].split(" as ").nth(1),
        aliases[1].split(" as ").nth(1)
    );
}

#[test]
fn python_files_layout_imports_generic_defaults() {
    let types = Types::default().register::<generic_default_import::consumer::Box>();
    let temp = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&temp).unwrap();
    let temp = TempDir::new_in(temp).unwrap();
    let package = temp.path().join("bindings");
    Python::default()
        .layout(Layout::Files)
        .export_to(&package, &types, IdentityFormat)
        .unwrap();

    let consumer = std::fs::read_to_string(
        package.join("test/python/generic_default_import/consumer/__init__.py"),
    )
    .unwrap();
    assert!(consumer.contains("from ..target import Other as "));
    assert!(consumer.contains("class Box[T = _specta_import_"));
    assert!(
        consumer.rfind("from ..target import Other as ").unwrap()
            > consumer.find("class Box[").unwrap()
    );
}

#[test]
fn python_layouts() {
    mod nested {
        #[derive(specta::Type)]
        #[specta(collect = false)]
        pub struct Demo {
            pub value: String,
        }
    }

    let types = Types::default().register::<nested::Demo>();
    let prefixed = Python::default()
        .layout(Layout::ModulePrefixedName)
        .export(&types, IdentityFormat)
        .unwrap();
    assert!(prefixed.contains("class test_python_nested_Demo(_specta_typing.TypedDict):"));

    let namespaced = Python::default()
        .layout(Layout::Namespaces)
        .export(&types, IdentityFormat)
        .unwrap();
    assert!(namespaced.contains("class test:\n    class python:\n        class nested:"));
    assert!(namespaced.contains("            class Demo(_specta_typing.TypedDict):"));
}

#[test]
fn python_reports_name_collisions() {
    let types = Types::default()
        .register::<duplicate_a::Duplicate>()
        .register::<duplicate_b::Duplicate>();
    let error = Python::default()
        .export(&types, IdentityFormat)
        .unwrap_err();

    assert!(error.to_string().contains("duplicate exported Python name"));
    assert_eq!(
        error
            .named_datatype()
            .map(|datatype| datatype.name.as_ref()),
        Some("Duplicate")
    );

    Python::default()
        .layout(Layout::ModulePrefixedName)
        .export(&types, IdentityFormat)
        .unwrap();

    let mut types = Types::default()
        .register::<namespace_binding_collision::a::B>()
        .register::<namespace_binding_collision::a::b::Child>();
    types.iter_mut(|datatype| {
        if datatype.name == "B" {
            datatype.name = "b".into();
        }
    });
    let error = Python::default()
        .layout(Layout::Namespaces)
        .export(&types, IdentityFormat)
        .unwrap_err();
    assert!(error.to_string().contains("duplicate exported Python name"));

    let temp = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&temp).unwrap();
    let temp = TempDir::new_in(temp).unwrap();
    let error = Python::default()
        .layout(Layout::Files)
        .export_to(temp.path().join("bindings"), &types, IdentityFormat)
        .unwrap_err();
    assert!(error.to_string().contains("duplicate exported Python name"));
}

#[test]
fn python_reports_unsupported_opaque_references() {
    #[derive(PartialEq, Eq, Hash)]
    struct Unsupported;

    let error = primitives::inline(
        &Python::default(),
        &Types::default(),
        &DataType::Reference(specta::datatype::Reference::opaque(Unsupported)),
    )
    .unwrap_err();

    assert!(error.to_string().contains("unsupported opaque reference"));
}

#[test]
fn python_qualifies_builtins_and_preserves_generic_defaults() {
    #[allow(non_camel_case_types)]
    #[derive(Type)]
    #[specta(collect = false)]
    struct int {
        value: i32,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct Defaulted<T = String> {
        value: T,
    }

    let types = Types::default().register::<int>().register::<Defaulted>();
    let output = Python::default().export(&types, IdentityFormat).unwrap();
    assert!(output.contains("class int(_specta_typing.TypedDict):"));
    assert!(output.contains("value: _specta_builtins.int"));
    assert!(
        output.contains("class Defaulted[T = _specta_builtins.str](_specta_typing.TypedDict):")
    );
}

#[cfg(unix)]
#[test]
fn python_files_layout_refuses_symlink_traversal() {
    use std::os::unix::fs::symlink;

    let types = Types::default().register::<files_layout::Root>();
    let temp = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&temp).unwrap();
    let temp = TempDir::new_in(temp).unwrap();
    let outside = TempDir::new_in(Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp")).unwrap();
    let package = temp.path().join("bindings");
    std::fs::create_dir_all(&package).unwrap();
    symlink(outside.path(), package.join("test")).unwrap();

    let error = Python::default()
        .layout(Layout::Files)
        .export_to(&package, &types, IdentityFormat)
        .unwrap_err();
    assert!(error.to_string().contains("refusing to traverse symlink"));
    assert!(!outside.path().join("python").exists());
}
