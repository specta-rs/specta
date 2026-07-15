use std::{borrow::Cow, path::PathBuf};

use serde::{Deserialize, Serialize};
use specta::{Format, Type, Types};
use specta_csharp::{CSharp, Error, Layout, Visibility};

struct IdentityFormat;

impl Format for IdentityFormat {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(
        &'_ self,
        _types: &Types,
        datatype: &specta::datatype::DataType,
    ) -> Result<Cow<'_, specta::datatype::DataType>, specta::FormatError> {
        Ok(Cow::Owned(datatype.clone()))
    }
}

struct RejectingFormat;

impl Format for RejectingFormat {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(
        &'_ self,
        _types: &Types,
        _datatype: &specta::datatype::DataType,
    ) -> Result<Cow<'_, specta::datatype::DataType>, specta::FormatError> {
        Err(std::io::Error::other("rejected for test").into())
    }
}

/// A documented generic response.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Response<T> {
    /// The response payload.
    value: T,
    #[serde(rename = "request-id")]
    request_id: String,
    #[serde(default)]
    warning: Option<String>,
    counts: std::collections::HashMap<String, u64>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "snake_case")]
enum Status {
    InProgress,
    Complete,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum Event<T> {
    Started,
    Progress(u8, T),
    Failed { message: String, retryable: bool },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct KeywordFields {
    r#class: bool,
    #[serde(rename = "kebab-case")]
    kebab: i32,
}

#[derive(Type)]
#[specta(collect = false)]
struct ContainingNameCollision {
    containing_name_collision: bool,
    clone: bool,
    equality_contract: bool,
    print_members: bool,
}

#[derive(Type)]
#[specta(collect = false)]
struct InlineInner {
    value: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct InlineFields {
    #[specta(inline)]
    inner: InlineInner,
    tuple: (u8, String),
    #[specta(inline)]
    status: Status,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericType<T> {
    value: T,
}

#[allow(non_camel_case_types)]
#[derive(Type)]
#[specta(collect = false)]
enum VariantCollisions {
    FooBar,
    Foo_Bar,
}

#[derive(Type)]
#[specta(collect = false)]
struct InlineNode {
    #[specta(inline)]
    next: Option<Box<InlineNode>>,
}

#[derive(Type)]
#[specta(collect = false)]
struct Foo;

mod foo {
    #[derive(specta::Type)]
    #[specta(collect = false)]
    pub struct Bar;
}

fn types() -> Types {
    Types::default()
        .register::<Response<Status>>()
        .register::<Status>()
        .register::<Event<String>>()
        .register::<KeywordFields>()
}

#[test]
fn export_across_format_phases() {
    let types = types();
    let cases: [(&str, Box<dyn Format>); 3] = [
        ("raw", Box::new(IdentityFormat)),
        ("serde", Box::new(specta_serde::Format)),
        ("serde-phases", Box::new(specta_serde::PhasesFormat)),
    ];

    for (name, format) in cases {
        let output = CSharp::default()
            .namespace("Example.Bindings")
            .header("// custom header")
            .with_raw("internal static class BindingMarker { }")
            .export(&types, format)
            .unwrap();
        insta::assert_snapshot!(format!("csharp-export-{name}"), output);
    }
}

#[test]
fn configuration_and_namespaces() {
    let output = CSharp::new()
        .namespace("Company.Api")
        .layout(Layout::Namespaces)
        .visibility(Visibility::Internal)
        .indent("  ")
        .with_raw("internal static class BindingMarker { }")
        .export(&types(), specta_serde::Format)
        .unwrap();

    insta::assert_snapshot!("csharp-namespaces", output);
}

#[test]
fn declaration_collisions_are_disambiguated() {
    let types = Types::default()
        .register::<ContainingNameCollision>()
        .register::<VariantCollisions>();
    let output = CSharp::new().export(&types, IdentityFormat).unwrap();

    assert!(output.contains("bool ContainingNameCollision2"));
    assert!(output.contains("bool Clone2"));
    assert!(output.contains("bool EqualityContract2"));
    assert!(output.contains("bool PrintMembers2"));
    assert!(output.contains("FooBar,"));
    assert!(output.contains("FooBar2,"));
}

#[test]
fn inline_fields_preserve_their_wrapper_properties() {
    let output = CSharp::new()
        .export(&Types::default().register::<InlineFields>(), IdentityFormat)
        .unwrap();

    assert!(output.contains("record InnerValue"));
    assert!(output.contains("InnerValue Inner"));
    assert!(output.contains("(byte, string) Tuple"));
    assert!(output.contains("enum StatusValue"));
    assert!(output.contains("StatusValue Status"));
}

#[test]
fn duplicate_generic_parameters_are_rejected() {
    let mut types = Types::default().register::<GenericType<String>>();
    types.iter_mut(|datatype| {
        if datatype.name == "GenericType" {
            datatype.generics = vec![
                specta::datatype::GenericDefinition::new("T".into(), None),
                specta::datatype::GenericDefinition::new("T".into(), None),
            ]
            .into();
        }
    });

    assert!(matches!(
        CSharp::new().export(&types, IdentityFormat),
        Err(Error::InvalidName { .. })
    ));
}

#[test]
fn invalid_namespace_and_module_paths_are_rejected_before_writes() {
    let types = Types::default().register::<Status>();
    assert!(matches!(
        CSharp::new()
            .namespace("bad-name")
            .export(&types, IdentityFormat),
        Err(Error::InvalidName { .. })
    ));

    let mut types = types;
    types.iter_mut(|datatype| datatype.module_path = "../../escape".into());
    let root = workspace_scratch("path-traversal");
    let _ = std::fs::remove_dir_all(&root);
    assert!(matches!(
        CSharp::new()
            .layout(Layout::Files)
            .export_to(&root, &types, IdentityFormat),
        Err(Error::InvalidName { .. })
    ));
    assert!(!root.exists());
}

#[test]
fn namespace_and_type_declaration_collisions_are_rejected() {
    let types = Types::default().register::<Foo>().register::<foo::Bar>();
    assert!(matches!(
        CSharp::new()
            .layout(Layout::Namespaces)
            .export(&types, IdentityFormat),
        Err(Error::DuplicateTypeName { .. })
    ));
}

#[test]
fn datatype_format_errors_include_the_named_type_path() {
    let error = CSharp::new()
        .export(
            &Types::default().register::<KeywordFields>(),
            RejectingFormat,
        )
        .unwrap_err();
    let message = error.to_string();

    assert!(message.contains("datatype formatter failed at test::csharp::KeywordFields"));
    assert!(message.contains("rejected for test"));
}

#[test]
fn files_are_rendered_before_the_output_directory_is_created() {
    let (types, _) = crate::types();
    let root = workspace_scratch("atomic-render-error");
    let _ = std::fs::remove_dir_all(&root);
    assert!(matches!(
        CSharp::new()
            .layout(Layout::Files)
            .export_to(&root, &types, IdentityFormat),
        Err(Error::UnsupportedType { .. })
    ));
    assert!(!root.exists());
}

#[test]
fn recursive_inline_structures_are_rejected() {
    let error = CSharp::new()
        .export(&Types::default().register::<InlineNode>(), IdentityFormat)
        .unwrap_err();

    assert!(
        matches!(
            error,
            Error::RecursiveInline { .. } | Error::UnsupportedType { .. }
        ),
        "unexpected error: {error:?}"
    );
}

#[test]
fn module_prefixed_names_are_flat() {
    let output = CSharp::new()
        .layout(Layout::ModulePrefixedName)
        .export(&types(), IdentityFormat)
        .unwrap();

    assert!(output.contains("record Test_Csharp_Response<T>"));
    assert!(output.contains("Test_Csharp_Status"));
}

#[test]
fn unsupported_anonymous_types_return_an_error() {
    let (types, _) = crate::types();
    let error = CSharp::new()
        .layout(Layout::ModulePrefixedName)
        .export(&types, IdentityFormat)
        .unwrap_err();

    assert!(matches!(error, Error::UnsupportedType { .. }));
}

#[test]
fn low_level_datatype_api_covers_composites() {
    use specta::datatype::{DataType, List, Map, Primitive};

    let datatype = DataType::Map(Map::new(
        Primitive::str.into(),
        DataType::Nullable(Box::new(DataType::List(List::new(Primitive::i32.into())))),
    ));
    let output =
        specta_csharp::primitives::datatype(&CSharp::new(), &Types::default(), &datatype).unwrap();

    assert_eq!(
        output,
        "global::System.Collections.Generic.IReadOnlyDictionary<string, global::System.Collections.Generic.IReadOnlyList<int>?>"
    );
}

#[test]
fn opaque_types_require_an_explicit_or_builtin_mapping() {
    #[derive(Hash, PartialEq, Eq)]
    struct CustomOpaque;

    let datatype = specta::datatype::Reference::opaque(CustomOpaque).into();
    assert!(matches!(
        specta_csharp::primitives::datatype(&CSharp::new(), &Types::default(), &datatype),
        Err(Error::UnsupportedOpaque { .. })
    ));
    assert_eq!(
        specta_csharp::primitives::datatype(
            &CSharp::new().opaque_type(std::any::type_name::<CustomOpaque>(), "CustomType"),
            &Types::default(),
            &datatype,
        )
        .unwrap(),
        "CustomType"
    );
}

#[test]
fn files_layout_writes_and_cleans_generated_files() {
    let root = workspace_scratch("files");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        root.join("stale.cs"),
        "// This file has been generated by Specta. Do not edit this file manually.",
    )
    .unwrap();
    std::fs::write(
        root.join("keep.cs"),
        "// user owned\n// generated by Specta should not establish ownership",
    )
    .unwrap();

    let exporter = CSharp::new()
        .namespace("Example")
        .layout(Layout::Files)
        .with_raw("internal static class BindingMarker { }");
    assert!(matches!(
        exporter.export(&types(), IdentityFormat),
        Err(Error::ExportRequiresExportTo(Layout::Files))
    ));
    exporter.export_to(&root, &types(), IdentityFormat).unwrap();

    assert!(!root.join("stale.cs").exists());
    assert!(root.join("keep.cs").exists());
    let files = crate::fs_to_string(&root).unwrap();
    insta::assert_snapshot!("csharp-files", files);
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn files_cleanup_does_not_follow_directory_symlinks() {
    use std::os::unix::fs::symlink;

    let root = workspace_scratch("symlink-root");
    let outside = workspace_scratch("symlink-outside");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(&outside).unwrap();
    let outside_file = outside.join("outside.cs");
    std::fs::write(
        &outside_file,
        "// This file has been generated by Specta. Do not edit this file manually.\n",
    )
    .unwrap();
    symlink(&outside, root.join("linked")).unwrap();

    CSharp::new()
        .layout(Layout::Files)
        .export_to(
            &root,
            &Types::default().register::<Status>(),
            IdentityFormat,
        )
        .unwrap();

    assert!(outside_file.exists());
    std::fs::remove_dir_all(root).unwrap();
    std::fs::remove_dir_all(outside).unwrap();
}

#[test]
fn generated_csharp_compiles_when_dotnet_sdk_is_available() {
    let Ok(version) = std::process::Command::new("dotnet")
        .arg("--version")
        .output()
    else {
        return;
    };
    if !version.status.success() {
        return;
    }

    let root = workspace_scratch("dotnet-build");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let types = Types::default()
        .register::<InlineFields>()
        .register::<ContainingNameCollision>()
        .register::<VariantCollisions>();
    let bindings = CSharp::new().export(&types, IdentityFormat).unwrap();
    std::fs::write(root.join("Bindings.cs"), bindings).unwrap();
    std::fs::write(
        root.join("Bindings.csproj"),
        r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <LangVersion>latest</LangVersion>
    <Nullable>enable</Nullable>
  </PropertyGroup>
</Project>
"#,
    )
    .unwrap();

    let output = std::process::Command::new("dotnet")
        .args(["build", "--nologo", "--verbosity", "quiet"])
        .current_dir(&root)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "dotnet build failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    std::fs::remove_dir_all(root).unwrap();
}

fn workspace_scratch(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("specta-csharp-tests")
        .join(name)
}
