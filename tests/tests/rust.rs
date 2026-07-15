use std::{
    borrow::Cow,
    fs,
    path::PathBuf,
    process::Command,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use serde::{Deserialize, Serialize};
use specta::{
    Format, Type, Types,
    datatype::{
        DataType, Enum, Field, Generic, GenericDefinition, List, NamedDataType, Reference, Struct,
        Variant,
    },
};
use specta_rust::{Layout, Rust};

struct Identity;

impl Format for Identity {
    fn map_types(&self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(&self, _: &Types, ty: &DataType) -> Result<Cow<'_, DataType>, specta::FormatError> {
        Ok(Cow::Owned(ty.clone()))
    }
}

/// A documented account.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Account {
    /// Stable database identifier.
    id: u64,
    display_name: String,
    aliases: Vec<String>,
    flags: std::collections::HashSet<String>,
    metadata: std::collections::HashMap<String, bool>,
    coordinates: (f64, f64),
    singleton: (u8,),
    bytes: [u8; 4],
    optional: Option<String>,
    #[specta(optional)]
    maybe_count: i32,
    #[serde(rename = "type")]
    kind: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct UserId(
    /// The wrapped identifier.
    u64,
);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Marker;

#[derive(Type)]
#[specta(collect = false)]
struct Keywords {
    r#type: String,
    r#abstract: bool,
}

#[derive(Type)]
#[specta(collect = false)]
struct Recursive {
    direct: Option<Box<Recursive>>,
    indirect: Vec<Recursive>,
    fixed: [Box<Recursive>; 1],
}

#[derive(Type)]
#[specta(collect = false)]
struct MutualA {
    b: MutualB,
}

#[derive(Type)]
#[specta(collect = false)]
struct MutualB {
    a: Box<MutualA>,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericRecursiveWrapper<T>(Box<T>);

#[derive(Type)]
#[specta(collect = false)]
struct GenericRecursiveNode {
    child: GenericRecursiveWrapper<GenericRecursiveNode>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Page<T> {
    items: Vec<T>,
    next: Option<String>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum Event {
    Started,
    Progress(
        /// Completion percentage.
        u8,
        #[deprecated = "use structured progress"] String,
    ),
    Finished {
        success: bool,
        message: Option<String>,
    },
}

fn types() -> Types {
    Types::default()
        .register::<Account>()
        .register::<UserId>()
        .register::<Marker>()
        .register::<Keywords>()
        .register::<Recursive>()
        .register::<MutualA>()
        .register::<Page<Account>>()
        .register::<Event>()
}

#[test]
fn exports_all_rust_datatype_shapes() {
    let output = Rust::default()
        .derive("Debug")
        .derive("Clone")
        .attribute("#[allow(dead_code)]")
        .with_raw("pub const GENERATED: bool = true;")
        .export(&types(), Identity)
        .unwrap();

    insta::assert_snapshot!("rust-export-raw", output);
}

#[test]
fn applies_serde_format() {
    #[derive(Type, Serialize, Deserialize)]
    #[specta(collect = false)]
    #[serde(rename_all = "snake_case")]
    enum Status {
        WaitingForInput,
        Complete,
    }

    let types = Types::default().register::<Status>();
    let output = Rust::default()
        .export(&types, specta_serde::Format)
        .unwrap();
    insta::assert_snapshot!("rust-export-serde", output);
    Rust::default()
        .export(&types, specta_serde::PhasesFormat)
        .expect("symmetric enum should export under phased formatting");
}

#[test]
fn structural_wire_shapes_return_honest_errors() {
    #[derive(Type, Serialize, Deserialize)]
    #[specta(collect = false)]
    struct Inner {
        value: String,
    }

    #[derive(Type, Serialize, Deserialize)]
    #[specta(collect = false)]
    struct Flattened {
        id: u8,
        #[serde(flatten)]
        inner: Inner,
    }

    let error = Rust::default()
        .export(
            &Types::default().register::<Flattened>(),
            specta_serde::Format,
        )
        .unwrap_err();
    assert!(
        matches!(
            &error,
            specta_rust::Error::UnsupportedIntersection { .. }
                | specta_rust::Error::InvalidIdentifier { .. }
        ),
        "unexpected flatten error: {error}"
    );
}

#[test]
fn collected_inline_references_are_not_rendered_as_names() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Child {
        value: String,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct Parent {
        #[specta(inline)]
        child: Child,
    }

    let error = Rust::default()
        .export(&Types::default().register::<Parent>(), Identity)
        .unwrap_err();
    assert!(
        matches!(&error, specta_rust::Error::InvalidIdentifier { path, .. } if path.ends_with("Parent.child")),
        "unexpected inline error: {error}"
    );
}

#[test]
fn exports_module_layouts() {
    let modules = Rust::default()
        .layout(Layout::Modules)
        .export(&types(), Identity)
        .unwrap();
    insta::assert_snapshot!("rust-export-modules", modules);
    compile_source("modules", &modules);

    let prefixed = Rust::default()
        .layout(Layout::ModulePrefixedName)
        .export(&types(), Identity)
        .unwrap();
    insta::assert_snapshot!("rust-export-module-prefixed", prefixed);
    compile_source("prefixed", &prefixed);
}

#[test]
fn export_to_creates_parent_and_writes_compileable_source() {
    let root = test_output_dir("flat");
    let path = root.join("generated.rs");
    let _ = fs::remove_dir_all(&root);

    Rust::default()
        .derive("Debug")
        .export_to(&path, &types(), Identity)
        .unwrap();
    compile_file(&path, &root);
}

#[test]
fn generated_single_file_layouts_compile_when_included() {
    for layout in [
        Layout::FlatFile,
        Layout::Modules,
        Layout::ModulePrefixedName,
    ] {
        let source = Rust::default()
            .layout(layout)
            .export(&types(), Identity)
            .unwrap();
        compile_included_source(&format!("included-{layout}"), &source);
    }
}

#[test]
fn recursive_nullable_fields_keep_option_outside_box() {
    let output = Rust::default().export(&types(), Identity).unwrap();
    assert!(
        output.contains("pub direct: ::std::option::Option<::std::boxed::Box<Recursive>>,"),
        "unexpected recursive nullable field:\n{output}"
    );
}

#[test]
fn recursion_detection_substitutes_generic_arguments() {
    let types = Types::default()
        .register::<GenericRecursiveWrapper<GenericRecursiveNode>>()
        .register::<GenericRecursiveNode>();
    let output = Rust::default().export(&types, Identity).unwrap();
    assert!(
        output.contains(
            "pub child: ::std::boxed::Box<GenericRecursiveWrapper<GenericRecursiveNode>>,",
        ),
        "generic recursion was not boxed:\n{output}"
    );
    compile_source("generic-recursion", &output);
}

#[test]
#[allow(deprecated)]
fn internal_deprecated_references_compile_with_denied_warnings() {
    #[deprecated = "use New"]
    #[derive(Type)]
    #[specta(collect = false)]
    struct Old;

    #[allow(deprecated)]
    #[derive(Type)]
    #[specta(collect = false)]
    struct New {
        old: Old,
    }

    #[allow(deprecated)]
    let types = Types::default().register::<Old>().register::<New>();
    let output = Rust::default().export(&types, Identity).unwrap();
    compile_source("deprecated-reference", &output);
}

#[test]
fn files_layout_writes_module_tree() {
    let root = test_output_dir("files");
    let _ = fs::remove_dir_all(&root);
    Rust::default()
        .layout(Layout::Files)
        .export_to(&root, &types(), Identity)
        .unwrap();

    assert!(root.join("mod.rs").is_file());
    assert!(root.join("test.rs").is_file());
    assert!(root.join("test/rust.rs").is_file());
    insta::assert_snapshot!(
        "rust-export-files-root",
        fs::read_to_string(root.join("mod.rs")).unwrap()
    );
    insta::assert_snapshot!(
        "rust-export-files-module",
        fs::read_to_string(root.join("test/rust.rs")).unwrap()
    );
    compile_file(&root.join("mod.rs"), &root);
}

#[test]
fn files_layout_requires_export_to() {
    assert!(matches!(
        Rust::default()
            .layout(Layout::Files)
            .export(&types(), Identity),
        Err(specta_rust::Error::ExportRequiresExportTo(Layout::Files))
    ));
}

#[test]
fn reports_contextual_errors() {
    let mut invalid = Types::default();
    NamedDataType::new("not-valid", &mut invalid, |_, ndt| {
        ndt.module_path = "api".into();
        ndt.ty = Some(DataType::Primitive(specta::datatype::Primitive::bool));
    });
    assert!(matches!(
        Rust::default().export(&invalid, Identity),
        Err(specta_rust::Error::InvalidIdentifier { name, .. }) if name == "not-valid"
    ));

    let mut underscore = Types::default();
    NamedDataType::new("_", &mut underscore, |_, ndt| {
        ndt.ty = Some(Struct::unit().into());
    });
    assert!(matches!(
        Rust::default().export(&underscore, Identity),
        Err(specta_rust::Error::InvalidIdentifier { name, .. }) if name == "_"
    ));

    let mut opaque = Types::default();
    NamedDataType::new("Opaque", &mut opaque, |_, ndt| {
        ndt.ty = Some(
            Struct::named()
                .field("value", Field::new(Reference::opaque("custom").into()))
                .build(),
        );
    });
    assert!(matches!(
        Rust::default().export(&opaque, Identity),
        Err(specta_rust::Error::UnsupportedOpaque { path, .. }) if path.ends_with("Opaque.value")
    ));

    let mut dependency_types = Types::default();
    let dependency = NamedDataType::new("Dependency", &mut dependency_types, |_, ndt| {
        ndt.ty = Some(Struct::unit().into());
    });
    let mut dangling = Types::default();
    NamedDataType::new("Container", &mut dangling, |_, ndt| {
        ndt.ty = Some(
            Struct::named()
                .field("value", Field::new(dependency.reference(Vec::new()).into()))
                .build(),
        );
    });
    assert!(matches!(
        Rust::default().export(&dangling, Identity),
        Err(specta_rust::Error::DanglingReference { path, .. }) if path.ends_with("Container.value")
    ));

    let mut duplicate = Types::default();
    NamedDataType::new("Same", &mut duplicate, |_, ndt| {
        ndt.module_path = "one".into();
        ndt.ty = Some(DataType::Primitive(specta::datatype::Primitive::u8));
    });
    NamedDataType::new("Same", &mut duplicate, |_, ndt| {
        ndt.module_path = "two".into();
        ndt.ty = Some(DataType::Primitive(specta::datatype::Primitive::u16));
    });
    assert!(matches!(
        Rust::default().export(&duplicate, Identity),
        Err(specta_rust::Error::DuplicateTypeName { name, .. }) if name == "Same"
    ));
    Rust::default()
        .layout(Layout::Namespaces)
        .export(&duplicate, Identity)
        .unwrap();

    let mut collision = Types::default();
    NamedDataType::new("api", &mut collision, |_, ndt| {
        ndt.module_path = "".into();
        ndt.ty = Some(Struct::unit().into());
    });
    NamedDataType::new("Thing", &mut collision, |_, ndt| {
        ndt.module_path = "api".into();
        ndt.ty = Some(Struct::unit().into());
    });
    assert!(matches!(
        Rust::default().layout(Layout::Modules).export(&collision, Identity),
        Err(specta_rust::Error::ModuleTypeCollision { name, .. }) if name == "api"
    ));

    let mut unstable = Types::default();
    NamedDataType::new("Half", &mut unstable, |_, ndt| {
        ndt.ty = Some(DataType::Primitive(specta::datatype::Primitive::f16));
    });
    assert!(matches!(
        Rust::default().export(&unstable, Identity),
        Err(specta_rust::Error::UnsupportedPrimitive {
            primitive: "f16",
            ..
        })
    ));

    let mut unused = Types::default();
    NamedDataType::new("Unused", &mut unused, |_, ndt| {
        ndt.generics = vec![GenericDefinition::new("T".into(), None)].into();
        ndt.ty = Some(Struct::unit().into());
    });
    assert!(matches!(
        Rust::default().export(&unused, Identity),
        Err(specta_rust::Error::UnusedGeneric { name, .. }) if name == "T"
    ));

    let mut skipped_generic = Types::default();
    NamedDataType::new("Skipped", &mut skipped_generic, |_, ndt| {
        ndt.generics = vec![GenericDefinition::new("T".into(), None)].into();
        let mut enm = Enum::default();
        enm.variants = vec![
            (
                "Hidden".into(),
                Variant::unnamed()
                    .field(Field::new(Generic::new("T".into()).into()))
                    .skip()
                    .build(),
            ),
            ("Visible".into(), Variant::unit()),
        ];
        ndt.ty = Some(enm.into());
    });
    assert!(matches!(
        Rust::default().export(&skipped_generic, Identity),
        Err(specta_rust::Error::UnusedGeneric { name, .. }) if name == "T"
    ));
}

#[test]
fn formats_each_datatype_once() {
    #[derive(Clone)]
    struct NonIdempotent(Arc<AtomicUsize>);

    impl Format for NonIdempotent {
        fn map_types(&self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
            Ok(Cow::Owned(types.clone()))
        }

        fn map_type(
            &self,
            _: &Types,
            ty: &DataType,
        ) -> Result<Cow<'_, DataType>, specta::FormatError> {
            self.0.fetch_add(1, Ordering::Relaxed);
            Ok(Cow::Owned(match ty {
                DataType::Primitive(specta::datatype::Primitive::i8) => {
                    DataType::Primitive(specta::datatype::Primitive::i16)
                }
                DataType::Primitive(specta::datatype::Primitive::i16) => {
                    DataType::Primitive(specta::datatype::Primitive::i32)
                }
                ty => ty.clone(),
            }))
        }
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct Once {
        value: i8,
    }

    let calls = Arc::new(AtomicUsize::new(0));
    let output = Rust::default()
        .export(
            &Types::default().register::<Once>(),
            NonIdempotent(calls.clone()),
        )
        .unwrap();
    assert!(output.contains("value: i16"), "{output}");
    assert!(!output.contains("value: i32"), "{output}");
    assert_eq!(calls.load(Ordering::Relaxed), 2);
}

#[test]
fn formats_the_top_level_named_datatype() {
    struct Promote;

    impl Format for Promote {
        fn map_types(&self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
            Ok(Cow::Owned(types.clone()))
        }

        fn map_type(
            &self,
            _: &Types,
            ty: &DataType,
        ) -> Result<Cow<'_, DataType>, specta::FormatError> {
            Ok(Cow::Owned(match ty {
                DataType::Primitive(specta::datatype::Primitive::i8) => {
                    DataType::Primitive(specta::datatype::Primitive::i16)
                }
                ty => ty.clone(),
            }))
        }
    }

    let mut types = Types::default();
    NamedDataType::new("Alias", &mut types, |_, ndt| {
        ndt.module_path = "".into();
        ndt.ty = Some(DataType::Primitive(specta::datatype::Primitive::i8));
    });
    let output = Rust::default().export(&types, Promote).unwrap();
    assert!(output.contains("pub type Alias = i16;"), "{output}");
}

#[test]
fn formats_generic_defaults_and_helper_type_graphs() {
    struct RenameGraph;
    impl Format for RenameGraph {
        fn map_types(&self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
            Ok(Cow::Owned(types.clone().map(|mut ndt| {
                ndt.name = "Renamed".into();
                ndt
            })))
        }

        fn map_type(
            &self,
            _: &Types,
            ty: &DataType,
        ) -> Result<Cow<'_, DataType>, specta::FormatError> {
            Ok(Cow::Owned(match ty {
                DataType::Primitive(specta::datatype::Primitive::i8) => {
                    DataType::Primitive(specta::datatype::Primitive::i16)
                }
                ty => ty.clone(),
            }))
        }
    }

    let mut types = Types::default();
    let alias = NamedDataType::new("Alias", &mut types, |_, ndt| {
        ndt.module_path = "".into();
        ndt.generics = vec![GenericDefinition::new(
            "T".into(),
            Some(DataType::Primitive(specta::datatype::Primitive::i8)),
        )]
        .into();
        ndt.ty = Some(Generic::new("T".into()).into());
    });
    let output = Rust::default().export(&types, RenameGraph).unwrap();
    assert!(output.contains("Renamed<T = i16>"), "{output}");
    assert_eq!(
        Rust::default()
            .reference(&types, &alias.reference(Vec::new()), RenameGraph)
            .unwrap(),
        "Renamed"
    );
}

#[test]
fn opaque_renderer_and_inline_helpers() {
    let mut types = Types::default();
    let opaque = Reference::opaque("custom");
    NamedDataType::new("Opaque", &mut types, |_, ndt| {
        ndt.ty = Some(
            Struct::named()
                .field("value", Field::new(opaque.clone().into()))
                .build(),
        );
    });
    let exporter = Rust::default().opaque_type(|_| Some("std::path::PathBuf".into()));
    assert!(
        exporter
            .export(&types, Identity)
            .unwrap()
            .contains("PathBuf")
    );
    assert_eq!(
        exporter
            .inline(
                &types,
                &DataType::Nullable(Box::new(DataType::Primitive(
                    specta::datatype::Primitive::u8,
                ))),
                Identity,
            )
            .unwrap(),
        "::std::option::Option<u8>"
    );
    assert_eq!(
        exporter.reference(&types, &opaque, Identity).unwrap(),
        "std::path::PathBuf"
    );
}

#[test]
fn inline_tuple_structs_preserve_optional_fields() {
    let mut field = Field::new(DataType::Primitive(specta::datatype::Primitive::u8));
    field.optional = true;
    let ty = Struct::unnamed().field(field).build();
    assert_eq!(
        Rust::default()
            .inline(&Types::default(), &ty, Identity)
            .unwrap(),
        "(::std::option::Option<u8>,)"
    );
}

#[test]
fn generic_recursive_inline_references_fail_contextually() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Node<T>(T, #[specta(inline)] std::vec::Vec<Node<T>>);

    #[derive(Type)]
    #[specta(collect = false)]
    struct Container {
        #[specta(inline)]
        node: Node<u8>,
    }

    let error = Rust::default()
        .export(&Types::default().register::<Container>(), Identity)
        .unwrap_err();
    assert!(
        matches!(error, specta_rust::Error::RecursiveInline { .. }),
        "unexpected recursive-inline error: {error}"
    );
}

#[test]
fn custom_header_keeps_generated_marker() {
    let output = Rust::default()
        .header("#![allow(clippy::all)]")
        .with_raw("pub const CUSTOM: bool = true;")
        .export(&Types::default(), Identity)
        .unwrap();
    assert!(output.starts_with("// This file has been generated by Specta."));
    assert!(output.contains("#![allow(clippy::all)]"));
    assert!(output.ends_with("pub const CUSTOM: bool = true;\n"));
}

#[test]
fn exports_named_alias_with_generic_default() {
    let mut types = Types::default();
    NamedDataType::new("Items", &mut types, |_, ndt| {
        ndt.module_path = "".into();
        ndt.generics = vec![GenericDefinition::new(
            "T".into(),
            Some(DataType::Primitive(specta::datatype::Primitive::u8)),
        )]
        .into();
        ndt.ty = Some(List::new(Generic::new("T".into()).into()).into());
    });
    let output = Rust::default().export(&types, Identity).unwrap();
    assert!(
        output.contains("pub type Items<T = u8> = ::std::vec::Vec<T>;"),
        "{output}"
    );
    compile_source("generic-alias", &output);
}

#[test]
fn generated_nonstandard_names_compile_with_denied_warnings() {
    let mut types = Types::default();
    NamedDataType::new("TypeName", &mut types, |_, ndt| {
        ndt.module_path = "API".into();
        ndt.ty = Some(
            Struct::named()
                .field(
                    "FieldName",
                    Field::new(DataType::Primitive(specta::datatype::Primitive::bool)),
                )
                .build(),
        );
    });
    let output = Rust::default()
        .layout(Layout::Modules)
        .export(&types, Identity)
        .unwrap();
    compile_source("nonstandard-names", &output);
}

#[test]
fn generated_standard_library_wrappers_cannot_be_shadowed() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Option;
    #[derive(Type)]
    #[specta(collect = false)]
    struct Vec;
    #[derive(Type)]
    #[specta(collect = false)]
    struct Box;
    #[derive(Type)]
    #[specta(collect = false)]
    struct String;
    #[derive(Type)]
    #[specta(collect = false)]
    struct UsesStd {
        nullable: std::option::Option<bool>,
        sequence: std::vec::Vec<u8>,
        text: std::string::String,
    }
    #[derive(Type)]
    #[specta(collect = false)]
    struct RecursiveUsesStd {
        next: std::option::Option<std::boxed::Box<RecursiveUsesStd>>,
    }

    let types = Types::default()
        .register::<Option>()
        .register::<Vec>()
        .register::<Box>()
        .register::<String>()
        .register::<UsesStd>()
        .register::<RecursiveUsesStd>();
    let output = Rust::default().export(&types, Identity).unwrap();
    assert!(output.contains("::std::option::Option<bool>"), "{output}");
    assert!(output.contains("::std::vec::Vec<u8>"), "{output}");
    assert!(output.contains("::std::string::String"), "{output}");
    assert!(
        output.contains("::std::boxed::Box<RecursiveUsesStd>"),
        "{output}"
    );
    compile_source("standard-library-shadowing", &output);
}

#[test]
fn files_layout_removes_only_stale_generated_files() {
    let root = test_output_dir("stale");
    let _ = fs::remove_dir_all(&root);
    let exporter = Rust::default().layout(Layout::Files);
    exporter.export_to(&root, &types(), Identity).unwrap();
    let stale = root.join("test/rust.rs");
    assert!(stale.exists());
    fs::write(root.join("keep.rs"), "pub struct UserOwned;\n").unwrap();
    fs::create_dir_all(root.join("user-empty")).unwrap();

    exporter
        .export_to(&root, &Types::default(), Identity)
        .unwrap();
    assert!(!stale.exists());
    assert!(root.join("keep.rs").exists());
    assert!(root.join("user-empty").exists());
    assert!(root.join("mod.rs").exists());
}

#[cfg(unix)]
#[test]
fn files_layout_does_not_follow_symlinks() {
    use std::os::unix::fs::symlink;

    let root = test_output_dir("symlink-root");
    let outside = test_output_dir("symlink-outside");
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&outside);
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let outside_file = outside.join("generated.rs");
    fs::write(
        &outside_file,
        "// This file has been generated by Specta. Do not edit this file manually.\n",
    )
    .unwrap();
    symlink(&outside, root.join("linked")).unwrap();

    Rust::default()
        .layout(Layout::Files)
        .export_to(&root, &Types::default(), Identity)
        .unwrap();
    assert!(outside_file.exists());
}

#[cfg(unix)]
#[test]
fn files_layout_refuses_symlinked_generated_paths() {
    use std::os::unix::fs::symlink;

    let root = test_output_dir("symlink-generated-root");
    let outside = test_output_dir("symlink-generated-outside");
    let _ = fs::remove_dir_all(&root);
    let _ = fs::remove_dir_all(&outside);
    fs::create_dir_all(&root).unwrap();
    fs::create_dir_all(&outside).unwrap();
    let outside_file = outside.join("target.rs");
    fs::write(&outside_file, "user owned\n").unwrap();
    symlink(&outside_file, root.join("mod.rs")).unwrap();

    let error = Rust::default()
        .layout(Layout::Files)
        .export_to(&root, &Types::default(), Identity)
        .unwrap_err();
    assert!(matches!(error, specta_rust::Error::Io { .. }));
    assert_eq!(fs::read_to_string(outside_file).unwrap(), "user owned\n");
}

fn test_output_dir(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target/specta-rust-tests")
        .join(name)
}

fn compile_source(name: &str, source: &str) {
    let root = test_output_dir(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let path = root.join("generated.rs");
    fs::write(&path, source).unwrap();
    compile_file(&path, &root);
}

fn compile_included_source(name: &str, source: &str) {
    let root = test_output_dir(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("generated.rs"), source).unwrap();
    let path = root.join("lib.rs");
    fs::write(
        &path,
        "pub mod bindings {\n    include!(\"generated.rs\");\n}\n",
    )
    .unwrap();
    compile_file(&path, &root);
}

fn compile_file(path: &std::path::Path, output_dir: &std::path::Path) {
    let output = Command::new("rustc")
        .args(["--edition=2024", "--crate-type=lib", "-Dwarnings"])
        .arg(path)
        .arg("--out-dir")
        .arg(output_dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "generated Rust failed to compile:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
