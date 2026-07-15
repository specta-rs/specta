use std::{borrow::Cow, collections::BTreeMap};

use serde::{Deserialize, Serialize};
use specta::{Format, Type, Types, datatype::DataType};
use specta_kotlin::{Error, IndentStyle, Kotlin, Layout, NamingConvention, Serialization};
use tempfile::TempDir;

struct IdentityFormat;

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

/// Account information sent to the client.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Account<T> {
    /// Stable account identifier.
    id: u64,
    display_name: String,
    metadata: BTreeMap<String, Option<T>>,
    #[specta(optional)]
    nickname: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "snake_case")]
enum Event<T> {
    Started,
    Message(String),
    Progress { current: u32, total: u32 },
    Generic(T),
}

#[derive(Type)]
#[specta(collect = false)]
struct TupleRecord(u8, String, bool);

#[derive(Type)]
#[specta(collect = false)]
struct NewType(String);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct KotlinxRecord {
    name: String,
    maybe: Option<String>,
}

#[derive(Type)]
#[specta(inline, collect = false)]
struct InlineUnit;

#[derive(Type)]
#[specta(inline, collect = false)]
enum InlineUnitEnum {
    First,
    Second,
}

#[derive(Type)]
#[specta(collect = false)]
struct UsesInlineUnit {
    unit: InlineUnit,
}

#[derive(Type)]
#[specta(collect = false)]
struct UsesInlineUnitEnum {
    unit_enum: InlineUnitEnum,
}

#[derive(Type)]
#[specta(collect = false)]
struct InnerPayload {
    value: String,
}

#[derive(Type)]
#[specta(collect = false)]
enum SelfNamedRawVariant {
    Foo { foo: InnerPayload },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum KeywordEnum {
    #[serde(rename = "when")]
    When,
}

#[derive(Type)]
#[specta(collect = false)]
struct UsesEverything {
    account: Account<String>,
    event: Event<u8>,
    tuple: (String, i32),
    nested: Vec<Option<Account<bool>>>,
    keyword: KeywordEnum,
}

fn raw_types() -> Types {
    Types::default()
        .register::<Account<String>>()
        .register::<Event<String>>()
        .register::<TupleRecord>()
        .register::<NewType>()
        .register::<KeywordEnum>()
        .register::<UsesEverything>()
}

#[test]
fn kotlin_export_raw() {
    insta::assert_snapshot!(
        "kotlin-export-raw",
        Kotlin::default()
            .export(&raw_types(), IdentityFormat)
            .expect("Kotlin export should succeed")
    );
}

#[test]
fn kotlin_raw_enum_keeps_self_named_field() {
    let output = Kotlin::default()
        .export(
            &Types::default().register::<SelfNamedRawVariant>(),
            IdentityFormat,
        )
        .expect("raw enum should export");
    assert!(output.contains("val foo: InnerPayload"));
}

#[test]
fn kotlin_rejects_non_exportable_named_references() {
    let mut types = Types::default();
    let hidden = specta::datatype::NamedDataType::new("Hidden", &mut types, |_, _| {});
    specta::datatype::NamedDataType::new("UsesHidden", &mut types, |_, datatype| {
        datatype.ty = Some(
            specta::datatype::Struct::named()
                .field(
                    "value",
                    specta::datatype::Field::new(DataType::Reference(hidden.reference(vec![]))),
                )
                .build(),
        );
    });

    let error = Kotlin::default()
        .export(&types, IdentityFormat)
        .expect_err("references without an exported declaration must be rejected");
    assert!(
        error
            .to_string()
            .contains("does not have an exportable definition")
    );
}

#[test]
fn kotlin_export_serde() {
    insta::assert_snapshot!(
        "kotlin-export-serde",
        Kotlin::default()
            .package("dev.specta.generated")
            .export(&raw_types(), specta_serde::Format)
            .expect("Serde-formatted Kotlin export should succeed")
    );
}

#[test]
fn kotlin_configuration() {
    let output = Kotlin::new()
        .header("// custom")
        .without_package()
        .indent(IndentStyle::Tabs)
        .naming(NamingConvention::SnakeCase)
        .serialization(Serialization::None)
        .mutable_properties(true)
        .with_raw("public const val generated = true")
        .export(
            &Types::default().register::<UsesEverything>(),
            IdentityFormat,
        )
        .expect("configured Kotlin export should succeed");

    assert!(output.starts_with("// custom"));
    assert!(output.contains("public data class uses_everything"));
    assert!(output.contains("public var account:"));
    assert!(!output.contains("@Serializable"));
    assert!(output.contains("public const val generated = true"));
}

#[test]
fn kotlinx_is_opt_in_and_rejects_incompatible_wire_shapes() {
    let supported = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<KotlinxRecord>(),
            specta_serde::Format,
        )
        .expect("plain records have compatible Kotlinx declarations");
    assert!(supported.contains("@Serializable"));
    assert!(supported.contains("@EncodeDefault"));
    assert!(supported.contains("val maybe: String? = null"));

    let error = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<Event<String>>(),
            specta_serde::Format,
        )
        .expect_err("payload enums require a representation-aware custom serializer");
    assert!(
        error
            .to_string()
            .contains("cannot preserve every Serde enum representation")
    );

    let mutable_newtype = Kotlin::default()
        .mutable_properties(true)
        .export(&Types::default().register::<NewType>(), IdentityFormat)
        .expect("mutable newtype should export as a regular data class");
    assert!(mutable_newtype.contains("data class NewType"));
    assert!(mutable_newtype.contains("public var field0: String"));
    assert!(!mutable_newtype.contains("@JvmInline"));

    let error = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .mutable_properties(true)
        .export(&Types::default().register::<NewType>(), IdentityFormat)
        .expect_err("mutable Kotlinx newtypes cannot preserve scalar encoding");
    assert!(error.to_string().contains("mutable Kotlinx newtypes"));

    let error = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<UsesInlineUnit>(),
            IdentityFormat,
        )
        .expect_err("inline unit structs must not bypass Kotlinx wire-shape validation");
    assert!(error.to_string().contains("unit-struct null encoding"));

    let error = Kotlin::default()
        .serialization(Serialization::Kotlinx)
        .export(
            &Types::default().register::<UsesInlineUnitEnum>(),
            IdentityFormat,
        )
        .expect_err("inline unit enums must not bypass Kotlinx wire-shape validation");
    assert!(error.to_string().contains("structural union"));
}

#[test]
fn kotlin_files_layout() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&root).expect("temporary workspace directory should be creatable");
    let temp = TempDir::new_in(root).expect("temporary export directory should be creatable");
    let exporter = Kotlin::default()
        .layout(Layout::Files)
        .with_raw("public const val generated = true");

    assert!(matches!(
        exporter.export(&raw_types(), IdentityFormat),
        Err(Error::ExportRequiresExportTo(Layout::Files))
    ));
    exporter
        .export_to(temp.path(), &raw_types(), IdentityFormat)
        .expect("file layout should export");

    assert!(temp.path().join("Account.kt").is_file());
    assert!(temp.path().join("Event.kt").is_file());
    assert!(temp.path().join("Specta.kt").is_file());
    let account = std::fs::read_to_string(temp.path().join("Account.kt"))
        .expect("generated Kotlin file should be readable");
    assert!(account.contains("data class Account<T>"));
    assert!(!account.contains("data class Event<T>"));

    std::fs::write(temp.path().join("UserOwned.kt"), "// user owned")
        .expect("unrelated Kotlin file should be creatable");
    exporter
        .export_to(
            temp.path(),
            &Types::default().register::<Account<String>>(),
            IdentityFormat,
        )
        .expect("subsequent file export should clean stale generated files");
    assert!(!temp.path().join("Event.kt").exists());
    assert!(temp.path().join("UserOwned.kt").exists());
}

#[test]
fn kotlin_module_prefixed_layout_updates_declarations_and_references() {
    let output = Kotlin::default()
        .layout(Layout::ModulePrefixedName)
        .export(
            &Types::default()
                .register::<Account<String>>()
                .register::<UsesEverything>(),
            IdentityFormat,
        )
        .expect("module-prefixed layout should export");

    assert!(output.contains("data class TestKotlinAccount<T>"));
    assert!(output.contains("val account: TestKotlinAccount<String>"));
}

#[test]
fn kotlin_export_to_creates_parent_directories() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&root).expect("temporary workspace directory should be creatable");
    let temp = TempDir::new_in(root).expect("temporary export directory should be creatable");
    let output = temp.path().join("nested/generated/Bindings.kt");

    Kotlin::default()
        .export_to(&output, &raw_types(), IdentityFormat)
        .expect("flat export should create parent directories");
    assert!(output.is_file());
}

#[test]
fn kotlin_rejects_unsafe_filenames_and_member_collisions() {
    let mut unsafe_types = Types::default();
    specta::datatype::NamedDataType::new("../../escape", &mut unsafe_types, |_, datatype| {
        datatype.ty = Some(DataType::Struct(specta::datatype::Struct::unit()));
    });
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&root).expect("temporary workspace directory should be creatable");
    let temp = TempDir::new_in(root).expect("temporary export directory should be creatable");
    assert!(matches!(
        Kotlin::default().layout(Layout::Files).export_to(
            temp.path(),
            &unsafe_types,
            IdentityFormat
        ),
        Err(Error::InvalidIdentifier { .. })
    ));

    let mut collision_types = Types::default();
    specta::datatype::NamedDataType::new("Collision", &mut collision_types, |_, datatype| {
        datatype.ty = Some(
            specta::datatype::Struct::named()
                .field(
                    "foo_bar",
                    specta::datatype::Field::new(DataType::Primitive(
                        specta::datatype::Primitive::bool,
                    )),
                )
                .field(
                    "fooBar",
                    specta::datatype::Field::new(DataType::Primitive(
                        specta::datatype::Primitive::bool,
                    )),
                )
                .build(),
        );
    });
    assert!(matches!(
        Kotlin::default()
            .naming(NamingConvention::CamelCase)
            .export(&collision_types, IdentityFormat),
        Err(Error::DuplicateIdentifier { .. })
    ));
}

#[test]
fn kotlin_exports_shared_type_corpus() {
    let (mut types, _) = crate::types();
    // `crate::types()` adds a synthetic `Primitives` record whose stringify-based field names
    // intentionally contain duplicates. It is useful to TypeScript's structural snapshots but
    // cannot be a Kotlin constructor, so exercise the actual collected named corpus here.
    types.iter_mut(|datatype| {
        if datatype.name == "Primitives" {
            datatype.ty = None;
        }
    });
    let exporter = Kotlin::default().serialization(Serialization::None);
    exporter
        .export(&types, IdentityFormat)
        .expect("the raw shared datatype corpus should export to Kotlin");
    exporter
        .export(&types, specta_serde::Format)
        .expect("the Serde shared datatype corpus should export to Kotlin");

    let (phased_types, _) = crate::types_phased();
    exporter
        .export(&phased_types, specta_serde::PhasesFormat)
        .expect("the phased Serde shared datatype corpus should export to Kotlin");
}
