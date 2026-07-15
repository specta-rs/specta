#![allow(non_camel_case_types)]

use std::{borrow::Cow, collections::HashMap, path::Path, process::Command};

use serde::{Deserialize, Serialize};
use specta::{Format, Type, Types, datatype::DataType};
use specta_go::{Go, Layout, primitives};
use tempfile::TempDir;

struct IdentityFormat;

impl Format for IdentityFormat {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(
        &'_ self,
        _: &Types,
        dt: &DataType,
    ) -> Result<Cow<'_, DataType>, specta::FormatError> {
        Ok(Cow::Owned(dt.clone()))
    }
}

/// A generic API response.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct ApiResponse<T> {
    /// Stable resource identifier.
    user_id: u64,
    payload: T,
    next_url: Option<String>,
    values: HashMap<String, i32>,
    huge: i128,
}

/// Current processing state.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "snake_case")]
enum JobStatus {
    Queued,
    #[serde(rename = "needs/review")]
    PendingApproval,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct EscapedNames {
    #[serde(rename = "content/type")]
    value: String,
    #[deprecated(note = "use value instead")]
    #[serde(rename = "legacy_name")]
    legacy: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct UnicodeNames {
    å: String,
    bbb: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Id(String);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct HasId {
    id: Id,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct PrimitiveCoverage {
    i8_value: i8,
    i16_value: i16,
    i32_value: i32,
    i64_value: i64,
    i128_value: i128,
    isize_value: isize,
    u8_value: u8,
    u16_value: u16,
    u32_value: u32,
    u64_value: u64,
    u128_value: u128,
    usize_value: usize,
    f32_value: f32,
    f64_value: f64,
    bool_value: bool,
    char_value: char,
    string_value: String,
    tuple: (u8, String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct TupleStruct(u8, String);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct UnitStruct;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Metadata {
    request_id: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Flattened {
    #[serde(flatten)]
    metadata: Metadata,
    ok: bool,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
enum Event {
    Started { at: String },
    Progress(u8),
    Finished,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Node {
    value: String,
    child: Option<Box<Node>>,
    status: JobStatus,
    event: Event,
}

fn types() -> Types {
    Types::default()
        .register::<ApiResponse<Node>>()
        .register::<JobStatus>()
        .register::<EscapedNames>()
        .register::<HasId>()
        .register::<Id>()
        .register::<PrimitiveCoverage>()
        .register::<TupleStruct>()
        .register::<UnitStruct>()
        .register::<UnicodeNames>()
        .register::<Flattened>()
        .register::<Metadata>()
        .register::<Event>()
        .register::<Node>()
}

fn has_tool(name: &str) -> bool {
    Command::new(name).arg("version").output().is_ok()
}

#[test]
fn go_export_raw_and_serde() {
    insta::assert_snapshot!(
        "go-export-raw",
        Go::default().export(&types(), IdentityFormat).unwrap()
    );
    insta::assert_snapshot!(
        "go-export-serde",
        Go::default()
            .export(&types(), specta_serde::Format)
            .unwrap()
    );
    insta::assert_snapshot!(
        "go-export-serde-phases",
        Go::default()
            .export(&types(), specta_serde::PhasesFormat)
            .unwrap()
    );
}

#[test]
fn go_output_is_accepted_by_go_toolchain() {
    if !has_tool("go") || !has_tool("gofmt") {
        return;
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&root).unwrap();
    let temp = TempDir::new_in(root).unwrap();
    let output = temp.path().join("bindings.go");
    Go::default()
        .package_name("bindings")
        .export_to(&output, &types(), specta_serde::Format)
        .unwrap();

    #[derive(Clone, Copy, Hash, PartialEq, Eq)]
    struct ExternalDateTime;

    impl Type for ExternalDateTime {
        fn definition(_: &mut Types) -> DataType {
            DataType::Reference(specta::datatype::Reference::opaque(Self))
        }
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct Stamp(ExternalDateTime);

    #[derive(Type)]
    #[specta(collect = false)]
    struct NullableStamp(Option<ExternalDateTime>);

    #[derive(Type)]
    #[specta(collect = false)]
    struct BigNumber(i128);

    #[derive(Type, Serialize, Deserialize)]
    #[specta(collect = false)]
    struct OptionalCollections {
        #[serde(skip_serializing_if = "Option::is_none")]
        values: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        lookup: Option<HashMap<String, i32>>,
    }

    #[derive(Type, Serialize, Deserialize)]
    #[specta(collect = false)]
    struct LocalizedWireName {
        #[serde(rename = "名字")]
        value: String,
        #[serde(rename = "-")]
        hyphen: String,
    }

    let newtypes = temp.path().join("newtypes.go");
    Go::default()
        .package_name("bindings")
        .export_to(
            &newtypes,
            &Types::default()
                .register::<Stamp>()
                .register::<NullableStamp>()
                .register::<BigNumber>(),
            IdentityFormat,
        )
        .unwrap();
    let optional_collections = temp.path().join("optional_collections.go");
    Go::default()
        .package_name("bindings")
        .export_to(
            &optional_collections,
            &Types::default().register::<OptionalCollections>(),
            specta_serde::PhasesFormat,
        )
        .unwrap();
    let localized = temp.path().join("localized.go");
    Go::default()
        .package_name("bindings")
        .export_to(
            &localized,
            &Types::default().register::<LocalizedWireName>(),
            specta_serde::Format,
        )
        .unwrap();

    let before = std::fs::read_to_string(&output).unwrap();
    let newtypes_before = std::fs::read_to_string(&newtypes).unwrap();
    let optional_collections_before = std::fs::read_to_string(&optional_collections).unwrap();
    let localized_before = std::fs::read_to_string(&localized).unwrap();
    assert!(
        localized_before.contains("Field1 string `json:\"名字\"`"),
        "{localized_before}"
    );
    assert!(
        localized_before.contains("Field2 string `json:\"-,\"`"),
        "{localized_before}"
    );
    assert!(
        newtypes_before.contains("type Stamp = time.Time"),
        "{newtypes_before}"
    );
    assert!(
        newtypes_before.contains("type BigNumber = *big.Int"),
        "{newtypes_before}"
    );
    assert!(
        newtypes_before.contains("type NullableStamp = *time.Time"),
        "{newtypes_before}"
    );

    let gofmt = Command::new("gofmt")
        .arg("-w")
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(
        gofmt.status.success(),
        "gofmt failed: {}",
        String::from_utf8_lossy(&gofmt.stderr)
    );
    assert_eq!(before, std::fs::read_to_string(&output).unwrap());
    assert_eq!(newtypes_before, std::fs::read_to_string(&newtypes).unwrap());
    assert_eq!(
        optional_collections_before,
        std::fs::read_to_string(&optional_collections).unwrap()
    );
    assert_eq!(
        localized_before,
        std::fs::read_to_string(&localized).unwrap()
    );
    std::fs::write(
        temp.path().join("bindings_test.go"),
        r#"package bindings

import (
	"encoding/json"
	"testing"
)

func TestSpectaWireNames(t *testing.T) {
	value, err := json.Marshal(EscapedNames{Field1: "value"})
	if err != nil {
		t.Fatal(err)
	}
	if got, want := string(value), `{"content/type":"value","legacy_name":""}`; got != want {
		t.Fatalf("got %s, want %s", got, want)
	}
}

func TestMethodBackedNewtypes(t *testing.T) {
	var stamp Stamp
	if err := json.Unmarshal([]byte(`"2024-01-02T03:04:05Z"`), &stamp); err != nil {
		t.Fatal(err)
	}
	if got, err := json.Marshal(stamp); err != nil || string(got) != `"2024-01-02T03:04:05Z"` {
		t.Fatalf("stamp: %s, %v", got, err)
	}

	var nullable NullableStamp
	if err := json.Unmarshal([]byte(`"2024-01-02T03:04:05Z"`), &nullable); err != nil {
		t.Fatal(err)
	}
	if nullable == nil {
		t.Fatal("nullable stamp was not allocated")
	}
	if got, err := json.Marshal(nullable); err != nil || string(got) != `"2024-01-02T03:04:05Z"` {
		t.Fatalf("nullable stamp: %s, %v", got, err)
	}

	var number BigNumber
	if err := json.Unmarshal([]byte(`12345678901234567890`), &number); err != nil {
		t.Fatal(err)
	}
	if got := number.String(); got != "12345678901234567890" {
		t.Fatalf("big number: %s", got)
	}
}

func TestOptionalCollectionsPreserveEmptyValues(t *testing.T) {
	values := []string{}
	lookup := map[string]int32{}
	present, err := json.Marshal(OptionalCollectionsSerialize{Values: &values, Lookup: &lookup})
	if err != nil {
		t.Fatal(err)
	}
	if got, want := string(present), `{"values":[],"lookup":{}}`; got != want {
		t.Fatalf("present: got %s, want %s", got, want)
	}

	absent, err := json.Marshal(OptionalCollectionsSerialize{})
	if err != nil {
		t.Fatal(err)
	}
	if got, want := string(absent), `{}`; got != want {
		t.Fatalf("absent: got %s, want %s", got, want)
	}
}

func TestLocalizedWireName(t *testing.T) {
	encoded, err := json.Marshal(LocalizedWireName{Field1: "value", Field2: "hyphen"})
	if err != nil {
		t.Fatal(err)
	}
	if got, want := string(encoded), `{"名字":"value","-":"hyphen"}`; got != want {
		t.Fatalf("got %s, want %s", got, want)
	}

	var decoded LocalizedWireName
	if err := json.Unmarshal([]byte(`{"名字":"decoded","-":"dash"}`), &decoded); err != nil {
		t.Fatal(err)
	}
	if decoded.Field1 != "decoded" {
		t.Fatalf("decoded: %s", decoded.Field1)
	}
	if decoded.Field2 != "dash" {
		t.Fatalf("decoded hyphen: %s", decoded.Field2)
	}
}
"#,
    )
    .unwrap();
    let test = Command::new("go")
        .args(["test", "./..."])
        .env("GO111MODULE", "off")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        test.status.success(),
        "go test failed:\n{}\n{}",
        String::from_utf8_lossy(&test.stdout),
        String::from_utf8_lossy(&test.stderr)
    );
}

#[test]
fn go_files_layout_and_raw_code() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&root).unwrap();
    let temp = TempDir::new_in(root).unwrap();
    Go::default()
        .layout(Layout::Files)
        .header("//go:build !specta_ignore")
        .with_raw("const SpectaGenerated = true")
        .export_to(temp.path(), &types(), specta_serde::Format)
        .unwrap();

    let files = std::fs::read_dir(temp.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect::<Vec<_>>();
    assert!(files.iter().any(|name| name == "api_response.go"));
    assert!(files.iter().any(|name| name == "job_status.go"));
    assert!(files.iter().any(|name| name == "specta.go"));
    assert!(files.iter().all(|name| name.ends_with(".go")));
    for file in files.iter().filter(|name| name.as_str() != "specta.go") {
        assert!(
            !std::fs::read_to_string(temp.path().join(file))
                .unwrap()
                .contains("SpectaGenerated")
        );
    }

    if !has_tool("gofmt") {
        return;
    }

    let gofmt = Command::new("gofmt")
        .arg("-w")
        .arg(temp.path())
        .output()
        .unwrap();
    assert!(gofmt.status.success());
}

#[test]
fn go_primitive_helpers_cover_inline_and_reference() {
    let mut types = Types::default();
    let reference = Node::definition(&mut types);
    assert_eq!(
        primitives::inline(
            &Go::default(),
            &types,
            &DataType::Primitive(specta::datatype::Primitive::bool)
        )
        .unwrap(),
        "bool"
    );
    assert!(
        primitives::inline(&Go::default(), &types, &reference)
            .unwrap()
            .contains("Node")
    );
}

#[test]
fn go_reports_invalid_configuration_and_map_keys() {
    assert!(
        Go::default()
            .package_name("not-valid")
            .export(&Types::default(), IdentityFormat)
            .unwrap_err()
            .to_string()
            .contains("package name")
    );
    assert!(
        Go::default()
            .package_name("_")
            .export(&Types::default(), IdentityFormat)
            .unwrap_err()
            .to_string()
            .contains("package name")
    );

    #[derive(Type)]
    #[specta(collect = false)]
    struct InvalidMap {
        values: HashMap<bool, String>,
    }

    let err = Go::default()
        .export(&Types::default().register::<InvalidMap>(), IdentityFormat)
        .unwrap_err();
    assert!(err.to_string().contains("map key"), "{err}");

    #[derive(Type)]
    #[specta(collect = false)]
    struct WideKey(i128);

    #[derive(Type)]
    #[specta(collect = false)]
    struct InvalidWideMap {
        values: HashMap<i128, String>,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct InvalidNamedWideMap {
        values: HashMap<WideKey, String>,
    }

    let err = Go::default()
        .export(
            &Types::default()
                .register::<WideKey>()
                .register::<InvalidWideMap>(),
            IdentityFormat,
        )
        .unwrap_err();
    assert!(err.to_string().contains("map key"), "{err}");

    let err = Go::default()
        .export(
            &Types::default()
                .register::<WideKey>()
                .register::<InvalidNamedWideMap>(),
            IdentityFormat,
        )
        .unwrap_err();
    assert!(err.to_string().contains("map key"), "{err}");

    #[derive(Type)]
    #[specta(collect = false)]
    struct ScalarKey(String);

    #[derive(Type)]
    #[specta(collect = false)]
    struct ScalarKeyMap {
        values: HashMap<ScalarKey, String>,
    }

    let scalar_key = Go::default()
        .export(
            &Types::default()
                .register::<ScalarKey>()
                .register::<ScalarKeyMap>(),
            IdentityFormat,
        )
        .unwrap();
    assert!(
        scalar_key.contains("Values map[ScalarKey]string"),
        "{scalar_key}"
    );

    #[derive(Type, Serialize)]
    #[specta(collect = false)]
    struct InvalidTag {
        #[serde(rename = "tick`name")]
        value: String,
    }

    let err = Go::default()
        .export(
            &Types::default().register::<InvalidTag>(),
            specta_serde::Format,
        )
        .unwrap_err();
    assert!(err.to_string().contains("encoding/json"), "{err}");

    #[derive(Type, Serialize)]
    #[specta(collect = false)]
    struct InvalidNumericTag {
        #[serde(rename = "value²")]
        value: String,
    }

    let err = Go::default()
        .export(
            &Types::default().register::<InvalidNumericTag>(),
            specta_serde::Format,
        )
        .unwrap_err();
    assert!(err.to_string().contains("encoding/json"), "{err}");

    #[derive(Type, Serialize, Deserialize)]
    #[specta(collect = false)]
    struct OptionalCollections {
        #[serde(skip_serializing_if = "Option::is_none")]
        values: Option<Vec<String>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        lookup: Option<HashMap<String, i32>>,
    }

    let optional = Go::default()
        .export(
            &Types::default().register::<OptionalCollections>(),
            specta_serde::PhasesFormat,
        )
        .unwrap();
    assert!(optional.contains("Values *[]string"), "{optional}");
    assert!(optional.contains("Lookup *map[string]int32"), "{optional}");
}

#[test]
fn go_applies_map_type_recursively() {
    struct BoolToString;

    impl Format for BoolToString {
        fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
            Ok(Cow::Owned(types.clone()))
        }

        fn map_type(
            &'_ self,
            _: &Types,
            dt: &DataType,
        ) -> Result<Cow<'_, DataType>, specta::FormatError> {
            Ok(match dt {
                DataType::Primitive(specta::datatype::Primitive::bool) => {
                    Cow::Owned(DataType::Primitive(specta::datatype::Primitive::str))
                }
                _ => Cow::Owned(dt.clone()),
            })
        }
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct Nested {
        values: Vec<bool>,
    }

    let output = Go::default()
        .export(&Types::default().register::<Nested>(), BoolToString)
        .unwrap();
    assert!(output.contains("Values []string"), "{output}");
}

#[test]
fn go_rejects_uncompilable_generic_shapes() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct GenericMap<T> {
        values: HashMap<T, String>,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct GenericNewtype<T>(T);

    #[allow(non_camel_case_types)]
    #[derive(Type)]
    #[specta(collect = false)]
    struct CollidingGenerics<T, t> {
        upper: T,
        lower: t,
    }

    let map_error = Go::default()
        .export(
            &Types::default().register::<GenericMap<String>>(),
            IdentityFormat,
        )
        .unwrap_err();
    assert!(map_error.to_string().contains("map key"), "{map_error}");

    let newtype_error = Go::default()
        .export(
            &Types::default().register::<GenericNewtype<String>>(),
            IdentityFormat,
        )
        .unwrap_err();
    assert!(
        newtype_error.to_string().contains("underlying type"),
        "{newtype_error}"
    );

    let collision = Go::default()
        .export(
            &Types::default().register::<CollidingGenerics<String, String>>(),
            IdentityFormat,
        )
        .unwrap_err();
    assert!(collision.to_string().contains("duplicate"), "{collision}");
}

#[test]
fn go_files_layout_preserves_specta_named_type_with_raw_code() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Specta {
        value: String,
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&root).unwrap();
    let temp = TempDir::new_in(root).unwrap();
    Go::default()
        .layout(Layout::Files)
        .with_raw("const Runtime = true")
        .export_to(
            temp.path(),
            &Types::default().register::<Specta>(),
            IdentityFormat,
        )
        .unwrap();
    let output = std::fs::read_to_string(temp.path().join("specta.go")).unwrap();
    assert!(output.contains("type Specta struct"), "{output}");
    assert!(output.contains("const Runtime = true"), "{output}");
}

#[test]
fn go_only_pointers_recursive_required_references() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct OwnedNode {
        child: Box<OwnedNode>,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct GenericA {
        b: Box<GenericB<GenericA>>,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct GenericB<T> {
        value: T,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct SafeGenericA {
        b: Box<SafeGenericB<SafeGenericA>>,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct SafeGenericB<T> {
        values: Vec<T>,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct AliasA(Box<AliasB>);

    #[derive(Type)]
    #[specta(collect = false)]
    struct AliasB(Box<AliasA>);

    #[derive(Type)]
    #[specta(collect = false)]
    struct ErasedTuple {
        value: (i128, String),
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct ErasedTupleStruct(i128, String);

    #[derive(Type, Serialize, Deserialize)]
    #[specta(collect = false)]
    struct ChainedGenericDefault<T = String, U = T> {
        first: T,
        second: U,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct UsesChainedGenericDefault {
        value: ChainedGenericDefault<i32>,
    }

    #[derive(Type, Serialize, Deserialize)]
    #[specta(collect = false)]
    struct FlattenedChainedGenericDefault {
        #[serde(flatten)]
        value: ChainedGenericDefault<i32>,
    }

    let non_recursive = Go::default().export(&types(), IdentityFormat).unwrap();
    assert!(
        non_recursive.contains("ID ID `json:\"id\"`"),
        "{non_recursive}"
    );

    let recursive = Go::default()
        .export(&Types::default().register::<OwnedNode>(), IdentityFormat)
        .unwrap();
    assert!(recursive.contains("Child *OwnedNode"), "{recursive}");

    let generic_types = Types::default()
        .register::<GenericA>()
        .register::<GenericB<GenericA>>()
        .register::<SafeGenericA>()
        .register::<SafeGenericB<SafeGenericA>>()
        .register::<AliasA>()
        .register::<AliasB>()
        .register::<ErasedTuple>()
        .register::<ErasedTupleStruct>()
        .register::<ChainedGenericDefault<i32>>()
        .register::<UsesChainedGenericDefault>();
    let generic = Go::default()
        .export(&generic_types, IdentityFormat)
        .unwrap();
    assert!(generic.contains("B *GenericB[GenericA]"), "{generic}");

    assert!(
        generic.contains("B SafeGenericB[SafeGenericA]"),
        "{generic}"
    );
    assert!(generic.contains("type AliasA *AliasB"), "{generic}");
    assert!(generic.contains("type AliasB *AliasA"), "{generic}");
    assert!(!generic.contains("math/big"), "{generic}");
    assert!(
        generic.contains("Value ChainedGenericDefault[int32, int32]"),
        "{generic}"
    );

    let flattened = Go::default()
        .export(
            &Types::default()
                .register::<ChainedGenericDefault<i32>>()
                .register::<FlattenedChainedGenericDefault>(),
            specta_serde::Format,
        )
        .unwrap();
    assert!(flattened.contains("First  int32"), "{flattened}");
    assert!(flattened.contains("Second int32"), "{flattened}");

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&root).unwrap();
    let temp = TempDir::new_in(root).unwrap();
    Go::default()
        .export_to(
            temp.path().join("generic.go"),
            &generic_types,
            IdentityFormat,
        )
        .unwrap();
    if !has_tool("go") {
        return;
    }

    let test = Command::new("go")
        .args(["test", "./..."])
        .env("GO111MODULE", "off")
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        test.status.success(),
        "{}",
        String::from_utf8_lossy(&test.stderr)
    );
}
