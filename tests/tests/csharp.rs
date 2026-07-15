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

struct RootRejectingFormat;

impl Format for RootRejectingFormat {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(
        &'_ self,
        _types: &Types,
        datatype: &specta::datatype::DataType,
    ) -> Result<Cow<'_, specta::datatype::DataType>, specta::FormatError> {
        if matches!(datatype, specta::datatype::DataType::Struct(_)) {
            return Err(std::io::Error::other("root rejected for test").into());
        }
        Ok(Cow::Owned(datatype.clone()))
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
struct DefaultedNonNullableFields {
    #[serde(default)]
    count: u32,
    #[serde(default)]
    label: String,
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

#[derive(Type)]
#[specta(collect = false)]
enum GenericVariantCollision<T> {
    T(T),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct KeywordFields {
    r#class: bool,
    #[serde(rename = "kebab-case")]
    kebab: i32,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct CSharpStringEscapes {
    #[serde(rename = "control\u{0085}line\u{2028}paragraph\u{2029}end")]
    value: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct ContainingNameCollision {
    containing_name_collision: bool,
    clone: bool,
    equality_contract: bool,
    print_members: bool,
    finalize: bool,
    get_type: bool,
    memberwise_clone: bool,
    reference_equals: bool,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct SkippedWireDependencyOwner {
    #[serde(skip)]
    hidden: WireNewtype,
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
    #[specta(inline)]
    optional: Option<InlineInner>,
    #[specta(inline)]
    children: Vec<InlineInner>,
}

#[derive(Type)]
#[specta(collect = false)]
struct FooValue {
    value: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct InlineFoo {
    value: bool,
}

#[derive(Type)]
#[specta(collect = false)]
struct InlineNameShadowing {
    #[specta(inline)]
    foo: InlineFoo,
    top_level: FooValue,
}

#[derive(Type)]
#[specta(collect = false)]
enum InlineEnumWithSkippedPayload {
    Visible,
    #[specta(skip)]
    Hidden(String),
}

#[derive(Type)]
#[specta(collect = false)]
enum InlineEnumWithSkippedRecursivePayload {
    Visible,
    #[specta(skip)]
    Hidden(#[specta(inline)] InlineNode),
}

#[derive(Type)]
#[specta(collect = false)]
struct InlineSkippedRecursiveEnumOwner {
    #[specta(inline)]
    state: InlineEnumWithSkippedRecursivePayload,
}

#[derive(Type)]
#[specta(collect = false)]
struct InlineSkippedEnumOwner {
    #[specta(inline)]
    state: InlineEnumWithSkippedPayload,
}

#[derive(Type)]
#[specta(collect = false, inline)]
struct AlwaysInlineA {
    value: String,
}

#[derive(Type)]
#[specta(collect = false, inline)]
struct AlwaysInlineB {
    value: u32,
}

#[derive(Type)]
#[specta(collect = false)]
struct InlinePair<A, B> {
    first: A,
    second: B,
}

#[derive(Type)]
#[specta(collect = false)]
struct ObjectNewtype(AlwaysInlineA);

#[derive(Type)]
#[specta(collect = false)]
struct GenericObjectNewtype<T>(T);

#[derive(Type)]
#[specta(collect = false)]
struct MultiInlineFields {
    tuple: (AlwaysInlineA, AlwaysInlineB),
    map: std::collections::HashMap<AlwaysInlineA, AlwaysInlineB>,
    pair: InlinePair<AlwaysInlineA, AlwaysInlineB>,
    mixed: (AlwaysInlineA, WireNewtype),
    hidden: ObjectNewtype,
    hidden_generic: GenericObjectNewtype<AlwaysInlineB>,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericType<T> {
    value: T,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericMemberCollision<T> {
    t: T,
}

#[derive(Type)]
#[specta(collect = false)]
struct ShadowedUser {
    value: bool,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericNameShadow<ShadowedUser> {
    generic: ShadowedUser,
    top_level: self::ShadowedUser,
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
enum RecordVariantCollisions {
    Clone(String),
    Equals { value: u32 },
    ToString(bool),
}

#[derive(Type)]
#[specta(collect = false)]
struct InlineNode {
    #[specta(inline)]
    next: Option<Box<InlineNode>>,
}

#[derive(Hash, PartialEq, Eq)]
struct UnsupportedOpaque;

impl Type for UnsupportedOpaque {
    fn definition(_types: &mut Types) -> specta::datatype::DataType {
        specta::datatype::Reference::opaque(UnsupportedOpaque).into()
    }
}

#[derive(Type)]
#[specta(collect = false)]
struct UnsupportedExport {
    value: UnsupportedOpaque,
}

#[derive(Default, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct WireUnit;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct WireNewtype(String);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct WireTuple(String, u32);

#[derive(Default, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct WireGeneric<T>(Option<T>);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct NonObjectWireShapes {
    raw_unit: (),
    unit: WireUnit,
    id: WireNewtype,
    tuple: WireTuple,
    generic: WireGeneric<u8>,
    nested_generic: WireGeneric<Option<u8>>,
    #[serde(default)]
    optional_unit: WireUnit,
    #[serde(default)]
    optional_generic: WireGeneric<u8>,
    optional_non_object: Option<WireGeneric<u8>>,
    optional_unit_reference: Option<WireUnit>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct RecursiveWireNode(Box<RecursiveWireNode>);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct RecursiveWireOwner {
    node: RecursiveWireNode,
}

#[derive(Type)]
#[specta(collect = false)]
struct RecursiveWireInlineOwner {
    mixed: (AlwaysInlineA, RecursiveWireNode),
}

#[derive(Type)]
#[specta(collect = false)]
enum UnionTypeShadowing {
    Foo(Foo),
}

#[derive(Type)]
#[specta(collect = false)]
struct Foo {
    value: bool,
}

#[derive(Type)]
#[specta(collect = false)]
struct FileCaseA {
    value: bool,
}

#[derive(Type)]
#[specta(collect = false)]
struct FileCaseB {
    value: bool,
}

mod foo {
    #[derive(specta::Type)]
    #[specta(collect = false)]
    pub struct Bar {
        value: bool,
    }
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
        .register::<VariantCollisions>()
        .register::<RecordVariantCollisions>();
    let output = CSharp::new().export(&types, IdentityFormat).unwrap();

    assert!(output.contains("bool ContainingNameCollision2"));
    assert!(output.contains("bool Finalize2"));
    assert!(output.contains("bool GetType2"));
    assert!(output.contains("bool MemberwiseClone2"));
    assert!(output.contains("bool ReferenceEquals2"));
    assert!(output.contains("JsonPropertyName(\"get_type\")"));
    assert!(output.contains("bool Clone2"));
    assert!(output.contains("bool EqualityContract2"));
    assert!(output.contains("bool PrintMembers2"));
    assert!(output.contains("FooBar,"));
    assert!(output.contains("FooBar2,"));
    assert!(output.contains("record Clone2"));
    assert!(output.contains("record Equals2"));
    assert!(output.contains("record ToString2"));
}

#[test]
fn inline_fields_preserve_their_wrapper_properties() {
    let types = Types::default()
        .register::<InlineFields>()
        .register::<MultiInlineFields>();
    let output = CSharp::new().export(&types, IdentityFormat).unwrap();

    assert!(output.contains("record InnerValue"));
    assert!(output.contains("InnerValue Inner"));
    assert!(output.contains("(byte, string) Tuple"));
    assert!(output.contains("enum StatusValue"));
    assert!(output.contains("StatusValue Status"));
    assert!(output.contains("record OptionalValue"));
    assert!(output.contains("OptionalValue? Optional"));
    assert!(output.contains("record ChildrenValue"));
    assert!(output.contains("IReadOnlyList<ChildrenValue> Children"));
    assert!(output.contains("(TupleValue, TupleValue2) Tuple"));
    assert!(output.contains("IReadOnlyDictionary<MapValue, MapValue2> Map"));
    assert!(output.contains("InlinePair<PairValue, PairValue2> Pair"));
    assert!(output.contains("(MixedValue, string) Mixed"));
    assert!(output.contains("record HiddenValue"));
    assert!(output.contains("HiddenValue Hidden"));
    assert!(output.contains("record HiddenGenericValue"));
    assert!(output.contains("HiddenGenericValue HiddenGeneric"));
}

#[test]
fn flat_references_are_not_shadowed_by_inline_helpers() {
    let output = CSharp::new()
        .export(
            &Types::default().register::<InlineNameShadowing>(),
            IdentityFormat,
        )
        .unwrap();

    assert!(output.contains("record FooValue2"));
    assert!(output.contains("FooValue TopLevel"));
}

#[test]
fn skipped_payload_variants_do_not_change_inline_enum_kind() {
    let output = CSharp::new()
        .export(
            &Types::default().register::<InlineSkippedEnumOwner>(),
            IdentityFormat,
        )
        .unwrap();

    assert!(output.contains("enum StateValue"));
    assert!(output.contains("Visible,"));
    assert!(!output.contains("Hidden"));
    assert!(!output.contains("abstract record StateValue"));
}

#[test]
fn skipped_inline_enum_variants_do_not_trigger_recursion_errors() {
    let output = CSharp::new()
        .export(
            &Types::default().register::<InlineSkippedRecursiveEnumOwner>(),
            IdentityFormat,
        )
        .unwrap();

    assert!(output.contains("Visible,"));
    assert!(!output.contains("Hidden"));
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
fn flat_generic_parameters_cannot_shadow_top_level_types() {
    let types = Types::default().register::<GenericNameShadow<String>>();
    assert!(matches!(
        CSharp::new().export(&types, IdentityFormat),
        Err(Error::DuplicateTypeName { .. })
    ));
}

#[test]
fn generic_parameters_do_not_collide_with_record_members() {
    let output = CSharp::new()
        .export(
            &Types::default().register::<GenericMemberCollision<String>>(),
            IdentityFormat,
        )
        .unwrap();

    assert!(output.contains("record GenericMemberCollision<T>"));
    assert!(output.contains("T T2 { get; init; }"));
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
fn files_layout_rejects_case_insensitive_path_collisions() {
    let mut types = Types::default()
        .register::<FileCaseA>()
        .register::<FileCaseB>();
    types.iter_mut(|ndt| {
        ndt.name = if ndt.name == "FileCaseA" {
            "Url".into()
        } else {
            "URL".into()
        };
    });

    let root = workspace_scratch("case-colliding-files");
    let _ = std::fs::remove_dir_all(&root);
    assert!(matches!(
        CSharp::new()
            .layout(Layout::Files)
            .export_to(&root, &types, IdentityFormat),
        Err(Error::DuplicateTypeName { .. })
    ));
    assert!(!root.exists());
}

#[test]
fn files_layout_handles_case_only_renames() {
    let mut types = Types::default().register::<FileCaseA>();
    types.iter_mut(|ndt| ndt.name = "Url".into());
    let root = workspace_scratch("case-only-file-rename");
    let _ = std::fs::remove_dir_all(&root);
    CSharp::new()
        .layout(Layout::Files)
        .export_to(&root, &types, IdentityFormat)
        .unwrap();

    types.iter_mut(|ndt| ndt.name = "URL".into());
    CSharp::new()
        .layout(Layout::Files)
        .export_to(&root, &types, IdentityFormat)
        .unwrap();

    let generated_files = std::fs::read_dir(root.join("Test/Csharp"))
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "cs"))
        .count();
    assert_eq!(generated_files, 1);
    assert!(root.join("Test/Csharp/URL.cs").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn real_virtual_module_segments_are_preserved() {
    let mut types = Types::default().register::<Status>();
    types.iter_mut(|ndt| ndt.module_path = "foo::virtual".into());

    let output = CSharp::new()
        .layout(Layout::Namespaces)
        .export(&types, IdentityFormat)
        .unwrap();
    assert!(output.contains("namespace Specta.Generated.Foo.Virtual"));

    types.iter_mut(|ndt| ndt.module_path = "virtual".into());
    let output = CSharp::new()
        .layout(Layout::Namespaces)
        .export(&types, IdentityFormat)
        .unwrap();
    assert!(output.contains("namespace Specta.Generated.Virtual"));
}

#[test]
fn generic_parameters_do_not_collide_with_union_variants() {
    let output = CSharp::new()
        .export(
            &Types::default().register::<GenericVariantCollision<String>>(),
            IdentityFormat,
        )
        .unwrap();

    assert!(output.contains("abstract record GenericVariantCollision<T>"));
    assert!(output.contains("record T2 : GenericVariantCollision<T>"));
}

#[test]
fn csharp_string_literals_escape_all_line_separators() {
    let output = CSharp::new()
        .export(
            &Types::default().register::<CSharpStringEscapes>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(output.contains(r#"control\u0085line\u2028paragraph\u2029end"#));
    assert!(!output.contains(['\u{0085}', '\u{2028}', '\u{2029}']));
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
fn datatype_format_is_applied_to_the_named_root() {
    let error = CSharp::new()
        .export(
            &Types::default().register::<KeywordFields>(),
            RootRejectingFormat,
        )
        .unwrap_err();
    let message = error.to_string();

    assert!(message.contains("datatype formatter failed at test::csharp::KeywordFields"));
    assert!(message.contains("root rejected for test"));
}

#[test]
fn files_are_rendered_before_the_output_directory_is_created() {
    let types = Types::default().register::<UnsupportedExport>();
    let root = workspace_scratch("atomic-render-error");
    let _ = std::fs::remove_dir_all(&root);
    assert!(matches!(
        CSharp::new()
            .layout(Layout::Files)
            .export_to(&root, &types, IdentityFormat),
        Err(Error::UnsupportedOpaque { .. })
    ));
    assert!(!root.exists());
}

#[test]
fn recursive_inline_structures_fall_back_to_named_references() {
    let output = CSharp::new()
        .export(&Types::default().register::<InlineNode>(), IdentityFormat)
        .unwrap();

    assert!(output.contains("record NextValue"));
    assert!(output.contains("InlineNode? Next"));
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
fn anonymous_structural_types_are_supported() {
    let (types, _) = crate::types();
    let output = CSharp::new()
        .layout(Layout::ModulePrefixedName)
        .export(&types, IdentityFormat)
        .unwrap();

    assert!(output.contains("record Test_Types_BoxInline"));
    assert!(output.contains("record CValue"));
    assert!(output.contains("CValue C"));
}

#[test]
fn serde_non_object_structs_render_as_their_wire_shapes() {
    let output = CSharp::new()
        .export(
            &Types::default().register::<NonObjectWireShapes>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(output.contains("object? Unit"));
    assert!(output.contains("object? RawUnit"));
    assert!(output.contains("string Id"));
    assert!(output.contains("(string, uint) Tuple"));
    assert!(output.contains("byte? Generic"));
    assert!(output.contains("byte? NestedGeneric"));
    assert!(output.contains("object? OptionalUnit"));
    assert!(output.contains("byte? OptionalGeneric"));
    assert!(output.contains("byte? OptionalNonObject"));
    assert!(output.contains("object? OptionalUnitReference"));
    assert!(!output.contains("??"));
    assert!(!output.contains("record WireNewtype"));
    assert!(!output.contains("record WireTuple"));

    let root = workspace_scratch("non-object-files");
    let _ = std::fs::remove_dir_all(&root);
    CSharp::new()
        .layout(Layout::Files)
        .export_to(
            &root,
            &Types::default().register::<NonObjectWireShapes>(),
            specta_serde::Format,
        )
        .unwrap();
    assert!(root.join("Test/Csharp/NonObjectWireShapes.cs").exists());
    assert!(!root.join("Test/Csharp/WireUnit.cs").exists());
    assert!(!root.join("Test/Csharp/WireNewtype.cs").exists());
    assert!(!root.join("Test/Csharp/WireTuple.cs").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn directly_registered_non_object_roots_are_rejected() {
    let types = Types::default().register::<WireNewtype>();
    assert!(matches!(
        CSharp::new().export(&types, specta_serde::Format),
        Err(Error::UnsupportedRoot { .. })
    ));

    let root = workspace_scratch("non-object-root-files");
    let _ = std::fs::remove_dir_all(&root);
    assert!(matches!(
        CSharp::new()
            .layout(Layout::Files)
            .export_to(&root, &types, specta_serde::Format),
        Err(Error::UnsupportedRoot { .. })
    ));
    assert!(!root.exists());
}

#[test]
fn skipped_non_object_dependencies_are_not_treated_as_roots() {
    let output = CSharp::new()
        .export(
            &Types::default().register::<SkippedWireDependencyOwner>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(output.contains("record SkippedWireDependencyOwner"));
    assert!(!output.contains("WireNewtype"));
}

#[test]
fn serde_unit_enums_keep_their_string_wire_shape() {
    let output = CSharp::new()
        .export(&Types::default().register::<Status>(), specta_serde::Format)
        .unwrap();

    assert!(output.contains("enum Status"));
    assert!(output.contains("InProgress,"));
    assert!(output.contains("Complete,"));
    assert!(output.contains("JsonConverter(typeof(__SpectaStatusJsonConverter))"));
    assert!(output.contains("\"in_progress\" => Status.InProgress"));
    assert!(output.contains("Status.Complete => \"complete\""));
    assert!(!output.contains("abstract record Status"));
    assert!(!output.contains("Item1"));
}

#[test]
fn references_to_non_emitted_named_types_are_rejected() {
    let mut types = Types::default().register::<InlineNameShadowing>();
    types.iter_mut(|ndt| {
        if ndt.name == "FooValue" {
            ndt.ty = None;
        }
    });

    assert!(matches!(
        CSharp::new().export(&types, IdentityFormat),
        Err(Error::HiddenReference { .. })
    ));
}

#[test]
fn inline_overrides_reject_references_to_non_emitted_named_types() {
    let mut types = Types::default().register::<MultiInlineFields>();
    types.iter_mut(|ndt| {
        if ndt.name == "WireNewtype" {
            ndt.ty = None;
        }
    });

    assert!(matches!(
        CSharp::new().export(&types, IdentityFormat),
        Err(Error::HiddenReference { .. })
    ));
}

#[test]
fn serde_defaults_do_not_make_non_nullable_fields_nullable() {
    let output = CSharp::new()
        .export(
            &Types::default().register::<DefaultedNonNullableFields>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(output.contains("public uint Count { get; init; } = default!;"));
    assert!(output.contains("public string Label { get; init; } = default!;"));
    assert!(!output.contains("uint? Count"));
    assert!(!output.contains("string? Label"));
}

#[test]
fn recursive_non_object_structs_return_an_error() {
    assert!(matches!(
        CSharp::new().export(
            &Types::default().register::<RecursiveWireOwner>(),
            specta_serde::Format,
        ),
        Err(Error::RecursiveInline { .. })
    ));
}

#[test]
fn recursive_non_object_structs_inside_inline_overrides_return_an_error() {
    assert!(matches!(
        CSharp::new().export(
            &Types::default().register::<RecursiveWireInlineOwner>(),
            IdentityFormat,
        ),
        Err(Error::RecursiveInline { .. })
    ));
}

#[test]
fn flat_union_variants_do_not_shadow_top_level_types() {
    let output = CSharp::new()
        .export(
            &Types::default().register::<UnionTypeShadowing>(),
            IdentityFormat,
        )
        .unwrap();

    assert!(output.contains("record Foo2"));
    assert!(output.contains("Foo Item1"));
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

    for (datatype, expected) in [
        (
            specta::datatype::Reference::opaque(String::new()).into(),
            "string",
        ),
        (
            specta::datatype::Reference::opaque(std::time::Duration::ZERO).into(),
            "global::System.TimeSpan",
        ),
        (
            specta::datatype::Reference::opaque(std::time::SystemTime::UNIX_EPOCH).into(),
            "global::System.DateTimeOffset",
        ),
    ] {
        assert_eq!(
            specta_csharp::primitives::datatype(&CSharp::new(), &Types::default(), &datatype,)
                .unwrap(),
            expected
        );
    }
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

#[cfg(unix)]
#[test]
fn files_layout_rejects_symlinked_expected_paths() {
    use std::os::unix::fs::symlink;

    let root = workspace_scratch("symlink-expected-root");
    let outside = workspace_scratch("symlink-expected-outside");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&outside);
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir_all(outside.join("Csharp")).unwrap();
    let outside_file = outside.join("Csharp/Status.cs");
    std::fs::write(&outside_file, "user owned").unwrap();
    symlink(&outside, root.join("Test")).unwrap();

    assert!(matches!(
        CSharp::new().layout(Layout::Files).export_to(
            &root,
            &Types::default().register::<Status>(),
            IdentityFormat,
        ),
        Err(Error::Io { .. })
    ));
    assert_eq!(
        std::fs::read_to_string(&outside_file).unwrap(),
        "user owned"
    );
    std::fs::remove_dir_all(root).unwrap();

    let root = workspace_scratch("symlink-expected-file-root");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("Test/Csharp")).unwrap();
    symlink(&outside_file, root.join("Test/Csharp/Status.cs")).unwrap();
    assert!(matches!(
        CSharp::new().layout(Layout::Files).export_to(
            &root,
            &Types::default().register::<Status>(),
            IdentityFormat,
        ),
        Err(Error::Io { .. })
    ));
    assert_eq!(
        std::fs::read_to_string(&outside_file).unwrap(),
        "user owned"
    );
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
        .register::<MultiInlineFields>()
        .register::<ContainingNameCollision>()
        .register::<VariantCollisions>()
        .register::<RecordVariantCollisions>()
        .register::<GenericMemberCollision<String>>()
        .register::<GenericVariantCollision<String>>()
        .register::<DefaultedNonNullableFields>();
    let types = types.register::<NonObjectWireShapes>();
    let bindings = CSharp::new().export(&types, IdentityFormat).unwrap();
    std::fs::write(root.join("Bindings.cs"), bindings).unwrap();
    std::fs::write(
        root.join("Bindings.csproj"),
        r#"<Project Sdk="Microsoft.NET.Sdk">
  <PropertyGroup>
    <TargetFramework>net8.0</TargetFramework>
    <LangVersion>latest</LangVersion>
    <Nullable>enable</Nullable>
    <TreatWarningsAsErrors>true</TreatWarningsAsErrors>
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
