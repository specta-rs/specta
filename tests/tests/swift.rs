use std::borrow::Cow;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};
use specta::{
    Format, Type, Types,
    datatype::{DataType, Enum, Field, NamedDataType, Struct},
};
use specta_swift::Swift;
use tempfile::TempDir;

struct IdentityFormat;

impl Format for IdentityFormat {
    fn map_types(&'_ self, types: &Types) -> Result<Cow<'_, Types>, specta::FormatError> {
        Ok(Cow::Owned(types.clone()))
    }

    fn map_type(
        &'_ self,
        _: &Types,
        dt: &specta::datatype::DataType,
    ) -> Result<Cow<'_, specta::datatype::DataType>, specta::FormatError> {
        Ok(Cow::Owned(dt.clone()))
    }
}

fn phase_collections(types: Types) -> Vec<(&'static str, Box<dyn Format>, Types)> {
    vec![
        ("raw", Box::new(IdentityFormat), types.clone()),
        ("serde", Box::new(specta_serde::Format), types.clone()),
        ("serde_phases", Box::new(specta_serde::PhasesFormat), types),
    ]
}

fn phase_output(types: &Types, format: impl Format + 'static) -> String {
    Swift::default().export(types, format).unwrap()
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "snake_case")]
enum JobStatus {
    Queued,
    PendingApproval,
}

#[derive(Type)]
#[specta(collect = false)]
enum RegularEnum {
    VariantOne,
    VariantTwo,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "snake_case")]
enum MixedEnum {
    Unit,
    WithData(String),
}

#[test]
fn swift_export() {
    let types = Types::default()
        .register::<JobStatus>()
        .register::<RegularEnum>()
        .register::<MixedEnum>();

    for (mode, format, types) in phase_collections(types) {
        insta::assert_snapshot!(format!("swift-export-{mode}"), phase_output(&types, format));
    }
}

/// A trailing `#[serde(default)]` tuple element is optional on deserialize
/// (serde accepts `[1]`) — the deserialize half must mark it optional
/// instead of requiring every element.
#[derive(Type, Deserialize, Serialize)]
#[specta(collect = false)]
struct SwiftTupleDefault(u8, #[serde(default)] u8);

#[test]
fn swift_tuple_default_phases() {
    let rendered = Swift::default()
        .export(
            &Types::default().register::<SwiftTupleDefault>(),
            specta_serde::PhasesFormat,
        )
        .expect("Swift should support defaulted tuple elements under PhasesFormat");

    insta::assert_snapshot!("swift-tuple-default-phases", rendered);
    assert!(
        rendered.contains("field1: UInt8?"),
        "the defaulted element must be optional in the deserialize half: {rendered}"
    );
}

/// Control: Swift skips `ty: None` marker slots, so a skip-reduced defaulted
/// tuple renders only the live element (optional on deserialize).
#[derive(Type, Deserialize, Serialize)]
#[specta(collect = false)]
struct SwiftSkipSlotTuple(#[serde(skip)] u8, #[serde(default)] u8);

#[test]
fn swift_skip_slot_tuple_phases() {
    let rendered = Swift::default()
        .export(
            &Types::default().register::<SwiftSkipSlotTuple>(),
            specta_serde::PhasesFormat,
        )
        .expect("Swift should support skip-reduced defaulted tuple structs");

    assert!(
        !rendered.contains("field0"),
        "the off-wire skipped slot must not render: {rendered}"
    );
    assert!(
        rendered.contains("field1: UInt8?"),
        "the live element is optional in the deserialize half: {rendered}"
    );
}

// Regression tests for serde-tagged enums.
// https://github.com/specta-rs/specta/issues/581
//
// `specta-serde` bakes serde's tagging into the type graph: internal and
// adjacent tagging lower the discriminant into a tag field on each variant, and
// externally tagged unit variants become a bare string. The Swift exporter must
// reproduce that wire shape -- verified against `serde_json` below -- rather
// than emitting uncompilable or mismatched `Codable`.

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "reason", content = "params", rename_all = "camelCase")]
enum AdjacentReason {
    Unavailable { retry_after: u32 },
    Internal,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "context", content = "data", rename_all = "camelCase")]
enum AdjacentError {
    System(AdjacentReason),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "reason", rename_all = "camelCase")]
enum InternalReason {
    CredentialsInvalid,
    QuotaExhausted { retry_after: u32 },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase")]
enum ExternalReason {
    CredentialsInvalid,
    QuotaExhausted { retry_after: u32 },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase")]
enum ExternalError {
    Auth(ExternalReason),
    Validation(Vec<String>),
}

/// The wire format the generated `Codable` has to match, straight from serde.
#[test]
fn swift_serde_tagged_enum_ground_truth() {
    use serde_json::json;

    assert_eq!(
        serde_json::to_value(AdjacentReason::Unavailable { retry_after: 30 }).unwrap(),
        json!({ "reason": "unavailable", "params": { "retry_after": 30 } })
    );
    assert_eq!(
        serde_json::to_value(AdjacentReason::Internal).unwrap(),
        json!({ "reason": "internal" })
    );
    assert_eq!(
        serde_json::to_value(AdjacentError::System(AdjacentReason::Unavailable {
            retry_after: 30
        }))
        .unwrap(),
        json!({
            "context": "system",
            "data": { "reason": "unavailable", "params": { "retry_after": 30 } }
        })
    );
    assert_eq!(
        serde_json::to_value(InternalReason::CredentialsInvalid).unwrap(),
        json!({ "reason": "credentialsInvalid" })
    );
    assert_eq!(
        serde_json::to_value(InternalReason::QuotaExhausted { retry_after: 30 }).unwrap(),
        json!({ "reason": "quotaExhausted", "retry_after": 30 })
    );
    assert_eq!(
        serde_json::to_value(ExternalReason::CredentialsInvalid).unwrap(),
        json!("credentialsInvalid")
    );
    assert_eq!(
        serde_json::to_value(ExternalReason::QuotaExhausted { retry_after: 30 }).unwrap(),
        json!({ "quotaExhausted": { "retry_after": 30 } })
    );
    assert_eq!(
        serde_json::to_value(ExternalError::Auth(ExternalReason::CredentialsInvalid)).unwrap(),
        json!({ "auth": "credentialsInvalid" })
    );
    assert_eq!(
        serde_json::to_value(ExternalError::Validation(vec!["a".to_string()])).unwrap(),
        json!({ "validation": ["a"] })
    );
}

#[test]
fn swift_serde_tagged_enum_export() {
    let types = Types::default()
        .register::<AdjacentReason>()
        .register::<AdjacentError>()
        .register::<InternalReason>()
        .register::<ExternalReason>()
        .register::<ExternalError>();

    insta::assert_snapshot!(
        "swift-serde-tagged-enums",
        Swift::default()
            .export(&types, specta_serde::Format)
            .unwrap()
    );
}

/// A `#[serde(flatten)]`ed payload inside an internally tagged variant is an
/// intersection, which Swift's `Codable` cannot express. The exporter must say
/// so instead of emitting invalid Swift.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "context", rename_all = "camelCase")]
enum InternalError {
    Auth(InternalReason),
    Validation { errors: Vec<String> },
}

#[test]
fn swift_serde_flattened_internal_payload_reports_error() {
    let err = Swift::default()
        .export(
            &Types::default().register::<InternalError>(),
            specta_serde::Format,
        )
        .expect_err("a flattened internally tagged payload cannot be expressed in Swift");

    assert!(
        err.to_string().contains("Intersection"),
        "the error should name the unsupported shape: {err}"
    );
}

/// An adjacently tagged empty-struct variant still nests its (empty) content.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c", rename_all = "camelCase")]
enum AdjacentEmpty {
    Unit,
    EmptyStruct {},
}

#[test]
fn swift_adjacent_empty_struct_variant() {
    assert_eq!(
        serde_json::to_value(AdjacentEmpty::EmptyStruct {}).unwrap(),
        serde_json::json!({ "t": "emptyStruct", "c": {} })
    );

    let rendered = Swift::default()
        .export(
            &Types::default().register::<AdjacentEmpty>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(
        rendered.contains("case emptyStruct(AdjacentEmptyEmptystructData)"),
        "{rendered}"
    );
    assert!(
        rendered.contains("public struct AdjacentEmptyEmptystructData: Codable {"),
        "{rendered}"
    );
}

/// The tag key may equal the variant's own serialized name; the tag must still
/// be recovered (a naive "the field is not named after the variant" heuristic
/// gets this wrong).
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type", rename_all = "camelCase")]
enum TagIsVariantName {
    Type { x: u32 },
    Other { y: u32 },
}

/// Payload fields that collide with Swift keywords, and optional/defaulted
/// payload fields.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum TaggedFieldEdgeCases {
    Keywords {
        default: bool,
        private: u32,
    },
    Optional {
        opt: Option<u32>,
        #[serde(default)]
        defaulted: u32,
    },
}

#[test]
fn swift_serde_tagged_enum_edge_cases() {
    assert_eq!(
        serde_json::to_value(TagIsVariantName::Type { x: 1 }).unwrap(),
        serde_json::json!({ "type": "type", "x": 1 })
    );
    assert_eq!(
        serde_json::to_value(TaggedFieldEdgeCases::Keywords {
            default: true,
            private: 3
        })
        .unwrap(),
        serde_json::json!({ "t": "Keywords", "default": true, "private": 3 })
    );

    let types = Types::default()
        .register::<TagIsVariantName>()
        .register::<TaggedFieldEdgeCases>();
    insta::assert_snapshot!(
        "swift-serde-tagged-enum-edge-cases",
        Swift::default()
            .export(&types, specta_serde::Format)
            .unwrap()
    );
}

/// Variant-level `#[serde(untagged)]` mixes tagged and untagged variants, which
/// no single Swift `Codable` can express.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum MixedTagging {
    A {
        x: u32,
    },
    #[serde(untagged)]
    B {
        y: u32,
    },
}

#[test]
fn swift_mixed_tagging_reports_error() {
    let err = Swift::default()
        .export(
            &Types::default().register::<MixedTagging>(),
            specta_serde::Format,
        )
        .expect_err("mixed tagging cannot be expressed in Swift");

    assert!(
        err.to_string().contains("mixes tagged and untagged"),
        "{err}"
    );
}

/// Swift tuples are not `Codable`, so a tuple payload must be rejected rather
/// than emitted as uncompilable Swift.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjacentTuple {
    Pair(u32, u32),
}

#[test]
fn swift_tagged_tuple_payload_reports_error() {
    let err = Swift::default()
        .export(
            &Types::default().register::<AdjacentTuple>(),
            specta_serde::Format,
        )
        .expect_err("a tuple payload cannot be expressed in Swift");

    assert!(err.to_string().contains("cannot encode"), "{err}");
}

/// A `#[serde(flatten)]`ed field inside a tagged variant's payload is merged
/// into the variant's object by `specta-serde`, which the generated wrapper
/// struct cannot express. The merged payload surfaces as an intersection,
/// which the Swift exporter rejects.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenInner {
    value: u32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjacentFlatten {
    V {
        #[serde(flatten)]
        inner: FlattenInner,
        extra: u32,
    },
}

#[test]
fn swift_tagged_flattened_payload_reports_error() {
    let err = Swift::default()
        .export(
            &Types::default().register::<AdjacentFlatten>(),
            specta_serde::Format,
        )
        .expect_err("a flattened payload cannot be expressed in Swift");

    assert!(
        err.to_string()
            .contains("Intersection types are not supported"),
        "{err}"
    );
}

/// `#[serde(default)] Option<T>` is optional on the wire, so its generated
/// property must not gain a second `?`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct DefaultOptionStruct {
    #[serde(default)]
    x: Option<u32>,
    y: u32,
}

#[test]
fn swift_default_option_struct_is_not_double_optional() {
    let rendered = Swift::default()
        .export(
            &Types::default().register::<DefaultOptionStruct>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(rendered.contains("public let x: UInt32?"), "{rendered}");
    assert!(!rendered.contains("UInt32??"), "{rendered}");
}

/// An externally tagged tuple variant is a JSON array under the variant key, so
/// it must round-trip through an unkeyed container rather than a tuple type.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase")]
enum ExternalTuple {
    Pair(u32, String),
    None,
}

#[test]
fn swift_external_tuple_variant() {
    assert_eq!(
        serde_json::to_value(ExternalTuple::Pair(1, "a".to_string())).unwrap(),
        serde_json::json!({ "pair": [1, "a"] })
    );

    let rendered = Swift::default()
        .export(
            &Types::default().register::<ExternalTuple>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(rendered.contains("case pair(UInt32, String)"), "{rendered}");
    assert!(
        rendered.contains("nestedUnkeyedContainer(forKey: .pair)"),
        "{rendered}"
    );
    assert!(
        rendered.contains("let value0 = try nested.decode(UInt32.self)"),
        "{rendered}"
    );
    assert!(
        rendered.contains("let value1 = try nested.decode(String.self)"),
        "{rendered}"
    );
}

/// A tuple variant reduced to a single live field by `#[serde(skip)]` is still
/// a JSON array, so it must use the unkeyed container too.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase")]
enum ExternalSkipTuple {
    T(u32, #[serde(skip)] String),
}

#[test]
fn swift_external_skip_reduced_tuple_variant() {
    assert_eq!(
        serde_json::to_value(ExternalSkipTuple::T(1, "x".to_string())).unwrap(),
        serde_json::json!({ "t": [1] })
    );

    let rendered = Swift::default()
        .export(
            &Types::default().register::<ExternalSkipTuple>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(
        rendered.contains("nestedUnkeyedContainer(forKey: .t)"),
        "{rendered}"
    );
}

/// A zero-arg tuple variant is an empty JSON array, which renders as `Void` and
/// so cannot be a `Codable` payload.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c", rename_all = "camelCase")]
enum AdjacentZeroTuple {
    A(),
    B,
}

#[test]
fn swift_tagged_empty_tuple_payload_reports_error() {
    let err = Swift::default()
        .export(
            &Types::default().register::<AdjacentZeroTuple>(),
            specta_serde::Format,
        )
        .expect_err("an empty tuple payload cannot be expressed in Swift");

    assert!(err.to_string().contains("cannot encode"), "{err}");
}

/// An externally tagged tuple with a `#[serde(default)]` element lowers to an
/// unnamed struct, which Swift's `Codable` cannot carry, and an externally
/// tagged empty payload renders as `Void`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase")]
enum ExternalDefaultTuple {
    T(u32, #[serde(default)] u32),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename_all = "camelCase")]
enum ExternalEmptyTuple {
    A(),
}

#[test]
fn swift_external_unsupported_payloads_report_errors() {
    for err in [
        Swift::default()
            .export(
                &Types::default().register::<ExternalDefaultTuple>(),
                specta_serde::Format,
            )
            .expect_err("a defaulted tuple element cannot be expressed in Swift"),
        Swift::default()
            .export(
                &Types::default().register::<ExternalEmptyTuple>(),
                specta_serde::Format,
            )
            .expect_err("an empty payload cannot be expressed in Swift"),
    ] {
        assert!(err.to_string().contains("cannot encode"), "{err}");
    }
}

/// `precedencegroup` is a Swift keyword that must be escaped.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum KeywordPrecedence {
    V { precedencegroup: u32 },
}

#[test]
fn swift_precedencegroup_keyword_is_escaped() {
    let rendered = Swift::default()
        .export(
            &Types::default().register::<KeywordPrecedence>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(
        rendered.contains("public let `precedencegroup`: UInt32"),
        "{rendered}"
    );
}

/// A tagged enum's variants are string literals only because the tag is one, so
/// it must not advertise a `String` raw type whose values differ from the wire.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", rename_all = "SCREAMING_SNAKE_CASE")]
enum TaggedUnitEnum {
    AlphaOne,
    BetaTwo,
}

#[test]
fn swift_tagged_unit_enum_has_no_raw_type() {
    assert_eq!(
        serde_json::to_value(TaggedUnitEnum::AlphaOne).unwrap(),
        serde_json::json!({ "t": "ALPHA_ONE" })
    );

    let rendered = Swift::default()
        .export(
            &Types::default().register::<TaggedUnitEnum>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(
        rendered.contains("public enum TaggedUnitEnum {"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("public enum TaggedUnitEnum: String"),
        "{rendered}"
    );
}

/// The datatype `ty!([])` builds: a list whose element type is uninhabited, which Specta
/// represents as an enum with no variants. `std::convert::Infallible` is the same shape.
struct Uninhabited;

impl Type for Uninhabited {
    fn definition(_: &mut Types) -> DataType {
        DataType::Enum(Enum::default())
    }
}

#[derive(Type)]
#[specta(collect = false)]
struct HoldsUninhabited {
    field: Vec<Uninhabited>,
}

/// Swift names an uninhabited type `Never`. Without it the inline enum path returned an
/// empty string and spliced that into the surrounding type.
#[test]
fn swift_renders_an_uninhabited_element_type_as_never() {
    let output = Swift::default()
        .export(
            &Types::default().register::<HoldsUninhabited>(),
            IdentityFormat,
        )
        .expect("an uninhabited element type should export");
    assert!(output.contains("[Never]"), "{output}");
}

// Regression tests for newtype and tuple structs.
// https://github.com/specta-rs/specta/issues/596
//
// serde only key-encodes structs with named fields: a newtype struct writes its
// bare inner value (`"hello"`), and a tuple struct writes a JSON array
// (`["hello",5]`). The generated Swift used to rely on the synthesised keyed
// `Codable`, which encoded `{"value":"hello"}` / `{"field0":...}` and decoded
// neither direction. These tests compile and *run* the generated Swift against
// fixtures produced by serde itself, because a snapshot alone missed the defect.

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftNewtype(String);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftNewtypeNested(SwiftNewtype);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftNewtypeOpt(Option<u32>);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftPair(String, u32);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftTriple(u8, u8, u8);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftOptPair(u8, Option<u8>);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftSkipMiddle(u8, #[serde(skip)] u8, u8);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftSkipReduced(#[serde(skip)] u8, #[serde(default)] u8);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftTrailingDefault(u8, #[serde(default)] u8);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftSkipIfTuple(
    u8,
    #[serde(skip_serializing_if = "Option::is_none")] Option<u8>,
);

/// A run of two trailing `#[serde(default)]` elements: only a nil *suffix* may
/// be omitted, and a nil interior element cannot be represented at all.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftTwoTrailing(u8, #[serde(default)] u8, #[serde(default)] u8);

/// Container-level `#[serde(default)]` makes every element omittable, with the
/// same trailing-suffix rule as per-field defaults.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
#[serde(default)]
struct SwiftContainerDefault(u8, u8);

/// A defaulted `Option<T>` element is written as `null` (serde does not omit a
/// defaulted element without `skip_serializing_if`).
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftTrailOpt(u8, #[serde(default)] Option<u8>);

/// `#[serde(default)]` on a newtype's only field is ignored by serde (it still
/// writes the bare value and rejects `null`), so the property must be
/// non-optional.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftNewtypeDefault(#[serde(default)] u8);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftEmptyTuple();

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftUnitStruct;

/// A tuple-valued element cannot be a Swift `Codable` property, and a
/// non-trailing omittable element is a shape serde itself cannot round-trip.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftNestedTuple((u8, u8));

/// The inner type is a tuple struct, so `#[specta(inline)]` would splice its
/// body into a type position.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftInlinePair(String, u32);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftOuterInline(#[specta(inline)] SwiftInlinePair);

/// A hand-built newtype whose only declared field is skipped. `specta-serde`
/// never produces this (a skipped newtype field keeps `ty: Some(..)`), so this
/// covers the defensive "no live field" error.
struct HandBuiltSkippedNewtype;

impl Type for HandBuiltSkippedNewtype {
    fn definition(types: &mut Types) -> DataType {
        let ndt = NamedDataType::new("HandBuiltSkippedNewtype", types, |_, ndt| {
            ndt.ty = Some(Struct::unnamed().field(Field::default()).build());
        });
        DataType::Reference(ndt.reference(vec![]))
    }
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftMidOmittable(
    u8,
    #[serde(skip_serializing_if = "Option::is_none")] Option<u8>,
    u8,
);

fn swift_wire_types() -> Types {
    Types::default()
        .register::<SwiftNewtype>()
        .register::<SwiftNewtypeNested>()
        .register::<SwiftNewtypeOpt>()
        .register::<SwiftNewtypeDefault>()
        .register::<SwiftPair>()
        .register::<SwiftTriple>()
        .register::<SwiftOptPair>()
        .register::<SwiftSkipMiddle>()
        .register::<SwiftSkipReduced>()
        .register::<SwiftTrailingDefault>()
        .register::<SwiftTwoTrailing>()
        .register::<SwiftContainerDefault>()
        .register::<SwiftTrailOpt>()
        .register::<SwiftEmptyTuple>()
        .register::<SwiftUnitStruct>()
}

/// Whether the host provides `swiftc`; the generated-code regression tests are
/// skipped without it, like the Go toolchain tests.
fn has_swift_toolchain() -> bool {
    Command::new("swiftc")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

/// Render a string as a Swift string literal.
fn swift_literal(value: &str) -> String {
    let mut out = String::from("\"");
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            _ => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Compile `harness` alongside the exported Swift and run it. The harness must
/// print `SWIFT-OK` once every check has passed; a failed `precondition` aborts
/// the process and fails the test.
fn run_swift(exported: &str, harness: &str) {
    if !has_swift_toolchain() {
        return;
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".temp");
    std::fs::create_dir_all(&root).unwrap();
    let temp = TempDir::new_in(root).unwrap();

    let bindings = temp.path().join("Bindings.swift");
    std::fs::write(&bindings, exported).unwrap();
    let main = temp.path().join("main.swift");
    std::fs::write(&main, harness).unwrap();
    let binary = temp.path().join("harness");

    let compile = Command::new("swiftc")
        .arg("-o")
        .arg(&binary)
        .arg(&bindings)
        .arg(&main)
        .output()
        .expect("swiftc should be runnable");
    assert!(
        compile.status.success(),
        "swiftc rejected the generated Swift:\n{}\n{}",
        String::from_utf8_lossy(&compile.stdout),
        String::from_utf8_lossy(&compile.stderr)
    );

    let run = Command::new(&binary)
        .output()
        .expect("compiled harness runs");
    assert!(
        run.status.success(),
        "the generated Swift failed at runtime:\n{}\n{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        String::from_utf8_lossy(&run.stdout).contains("SWIFT-OK"),
        "the harness did not reach its success marker:\n{}",
        String::from_utf8_lossy(&run.stdout)
    );
}

const SWIFT_ROUND_TRIP_HARNESS: &str = r#"
import Foundation

func expect(_ actual: String, _ expected: String, _ label: String) {
    precondition(actual == expected, "\(label): got \(actual), want \(expected)")
}

func check(_ condition: Bool, _ label: String) {
    precondition(condition, label)
}

func encode<T: Encodable>(_ value: T) -> String {
    String(data: try! JSONEncoder().encode(value), encoding: .utf8)!
}

func decode<T: Decodable>(_ json: String, _: T.Type) -> T {
    try! JSONDecoder().decode(T.self, from: json.data(using: .utf8)!)
}

func encodeThrowing<T: Encodable>(_ value: T) throws -> String {
    String(data: try JSONEncoder().encode(value), encoding: .utf8)!
}

func throwsError(_ body: () throws -> Void) -> Bool {
    do {
        try body()
        return false
    } catch {
        return true
    }
}

// Newtype: serde writes the bare inner value, not `{"value": ...}`.
expect(encode(SwiftNewtype(value: "hello")), newtypeJSON, "newtype encode")
expect(decode(newtypeJSON, SwiftNewtype.self).value, "hello", "newtype decode")

// A nested newtype composes its inner single-value container.
expect(
    encode(SwiftNewtypeNested(value: SwiftNewtype(value: "x"))),
    nestedNewtypeJSON,
    "nested newtype encode"
)
expect(decode(nestedNewtypeJSON, SwiftNewtypeNested.self).value.value, "x", "nested newtype decode")

// An `Option<T>` newtype writes `null` as its bare value.
expect(encode(SwiftNewtypeOpt(value: nil)), newtypeOptNullJSON, "optional newtype encode null")
check(decode(newtypeOptNullJSON, SwiftNewtypeOpt.self).value == nil, "optional newtype decode null")
expect(encode(SwiftNewtypeOpt(value: 5)), newtypeOptJSON, "optional newtype encode value")
check(decode(newtypeOptJSON, SwiftNewtypeOpt.self).value == 5, "optional newtype decode value")

// `#[serde(default)]` is ignored on a newtype's only field: serde writes the
// bare value and rejects `null`.
expect(encode(SwiftNewtypeDefault(value: 7)), newtypeDefaultJSON, "newtype default encode")
check(decode(newtypeDefaultJSON, SwiftNewtypeDefault.self).value == 7, "newtype default decode")

// Tuple struct: serde writes a JSON array.
expect(encode(SwiftPair(field0: "hello", field1: 5)), pairJSON, "tuple encode")
let pair = decode(pairJSON, SwiftPair.self)
expect(pair.field0, "hello", "tuple decode field0")
expect(String(pair.field1), "5", "tuple decode field1")

expect(encode(SwiftTriple(field0: 1, field1: 2, field2: 3)), tripleJSON, "triple encode")
check(decode(tripleJSON, SwiftTriple.self).field2 == 3, "triple decode")

// An `Option<T>` tuple element is still present, as `null`.
expect(encode(SwiftOptPair(field0: 1, field1: nil)), optPairNullJSON, "optional tuple element encode")
check(decode(optPairNullJSON, SwiftOptPair.self).field1 == nil, "optional tuple element decode")
expect(encode(SwiftOptPair(field0: 1, field1: 2)), optPairSomeJSON, "optional tuple element encode some")

// A skipped middle slot compacts out of the sequence.
expect(encode(SwiftSkipMiddle(field0: 1, field1: 3)), skipMiddleJSON, "skip-reduced tuple encode")
check(decode(skipMiddleJSON, SwiftSkipMiddle.self).field1 == 3, "skip-reduced tuple decode")

// A declared-2 tuple reduced to one live field by a skip stays an array.
expect(encode(SwiftSkipReduced(field1: 2)), skipReducedJSON, "skip-reduced one-element tuple encode")
check(decode(skipReducedJSON, SwiftSkipReduced.self).field1 == 2, "skip-reduced one-element tuple decode")
check(decode("[]", SwiftSkipReduced.self).field1 == nil, "skip-reduced one-element tuple decode empty")

// A trailing `#[serde(default)]` element is decoded by container exhaustion.
expect(encode(SwiftTrailingDefault(field0: 1, field1: 2)), trailingDefaultJSON, "trailing default encode")
expect(encode(SwiftTrailingDefault(field0: 1, field1: nil)), trailingDefaultTruncatedJSON, "trailing default encode nil")
check(decode(trailingDefaultTruncatedJSON, SwiftTrailingDefault.self).field1 == nil, "trailing default decode short")
check(decode(trailingDefaultJSON, SwiftTrailingDefault.self).field1 == 2, "trailing default decode full")

// A run of two trailing defaults: only a nil *suffix* is omitted.
expect(encode(SwiftTwoTrailing(field0: 1, field1: 2, field2: 3)), twoTrailingJSON, "two trailing encode full")
expect(encode(SwiftTwoTrailing(field0: 1, field1: 2, field2: nil)), twoTrailingShortJSON, "two trailing encode one nil")
expect(encode(SwiftTwoTrailing(field0: 1, field1: nil, field2: nil)), twoTrailingShorterJSON, "two trailing encode two nil")
check(decode(twoTrailingShortJSON, SwiftTwoTrailing.self).field1 == 2, "two trailing decode one nil")
// A nil interior element cannot be represented in a serde sequence, so the
// encoder must reject it instead of silently shifting the following value.
check(
    throwsError { _ = try encodeThrowing(SwiftTwoTrailing(field0: 1, field1: nil, field2: 2)) },
    "two trailing interior nil must throw"
)

// Container-level `#[serde(default)]` follows the same rule.
expect(encode(SwiftContainerDefault(field0: 1, field1: 2)), containerDefaultJSON, "container default encode full")
expect(encode(SwiftContainerDefault(field0: 1, field1: nil)), containerDefaultShortJSON, "container default encode one nil")
check(
    throwsError { _ = try encodeThrowing(SwiftContainerDefault(field0: nil, field1: 2)) },
    "container default interior nil must throw"
)

// A defaulted `Option<T>` element is written as `null`, not omitted.
expect(encode(SwiftTrailOpt(field0: 1, field1: nil)), trailOptNullJSON, "default option encodes null")
expect(encode(SwiftTrailOpt(field0: 1, field1: 2)), trailOptSomeJSON, "default option encodes value")
check(decode(trailOptNullJSON, SwiftTrailOpt.self).field1 == nil, "default option decode null")

// An empty tuple struct is an empty array; a unit struct is `null`.
expect(encode(SwiftEmptyTuple()), emptyTupleJSON, "empty tuple encode")
_ = decode(emptyTupleJSON, SwiftEmptyTuple.self)
expect(encode(SwiftUnitStruct()), unitStructJSON, "unit struct encode")
_ = decode(unitStructJSON, SwiftUnitStruct.self)

print("SWIFT-OK")
"#;

fn swift_fixtures() -> String {
    format!(
        "let newtypeJSON = {}\n\
         let nestedNewtypeJSON = {}\n\
         let newtypeOptNullJSON = {}\n\
         let newtypeOptJSON = {}\n\
         let newtypeDefaultJSON = {}\n\
         let pairJSON = {}\n\
         let tripleJSON = {}\n\
         let optPairNullJSON = {}\n\
         let optPairSomeJSON = {}\n\
         let skipMiddleJSON = {}\n\
         let skipReducedJSON = {}\n\
         let skipIfOmittedJSON = {}\n\
         let skipIfPresentJSON = {}\n\
         let skipIfNullJSON = {}\n\
         let trailingDefaultJSON = {}\n\
         let trailingDefaultTruncatedJSON = {}\n\
         let twoTrailingJSON = {}\n\
         let twoTrailingShortJSON = {}\n\
         let twoTrailingShorterJSON = {}\n\
         let containerDefaultJSON = {}\n\
         let containerDefaultShortJSON = {}\n\
         let trailOptNullJSON = {}\n\
         let trailOptSomeJSON = {}\n\
         let emptyTupleJSON = {}\n\
         let unitStructJSON = {}\n",
        swift_literal(&serde_json::to_string(&SwiftNewtype("hello".into())).unwrap()),
        swift_literal(
            &serde_json::to_string(&SwiftNewtypeNested(SwiftNewtype("x".into()))).unwrap()
        ),
        swift_literal(&serde_json::to_string(&SwiftNewtypeOpt(None)).unwrap()),
        swift_literal(&serde_json::to_string(&SwiftNewtypeOpt(Some(5))).unwrap()),
        swift_literal(&serde_json::to_string(&SwiftNewtypeDefault(7)).unwrap()),
        swift_literal(&serde_json::to_string(&SwiftPair("hello".into(), 5)).unwrap()),
        swift_literal(&serde_json::to_string(&SwiftTriple(1, 2, 3)).unwrap()),
        swift_literal(&serde_json::to_string(&SwiftOptPair(1, None)).unwrap()),
        swift_literal(&serde_json::to_string(&SwiftOptPair(1, Some(2))).unwrap()),
        swift_literal(&serde_json::to_string(&SwiftSkipMiddle(1, 2, 3)).unwrap()),
        swift_literal(&serde_json::to_string(&SwiftSkipReduced(1, 2)).unwrap()),
        swift_literal(&serde_json::to_string(&SwiftSkipIfTuple(1, None)).unwrap()),
        swift_literal(&serde_json::to_string(&SwiftSkipIfTuple(1, Some(2))).unwrap()),
        // `skip_serializing_if` makes `[1]` the serialize output, but serde's
        // deserialize half still requires the slot and accepts an explicit
        // `null` for it.
        swift_literal("[1,null]"),
        swift_literal(&serde_json::to_string(&SwiftTrailingDefault(1, 2)).unwrap()),
        // serde always *writes* both elements; the truncated form is a valid
        // deserialize-only input (asserted in `swift_struct_wire_ground_truth`).
        swift_literal("[1]"),
        swift_literal(&serde_json::to_string(&SwiftTwoTrailing(1, 2, 3)).unwrap()),
        // A nil suffix is omitted by the generated codec; serde reads the
        // shorter sequence as `(1, 2, 0)` / `(1, 0, 0)`.
        swift_literal("[1,2]"),
        swift_literal("[1]"),
        swift_literal(&serde_json::to_string(&SwiftContainerDefault(1, 2)).unwrap()),
        swift_literal("[1]"),
        swift_literal(&serde_json::to_string(&SwiftTrailOpt(1, None)).unwrap()),
        swift_literal(&serde_json::to_string(&SwiftTrailOpt(1, Some(2))).unwrap()),
        swift_literal(&serde_json::to_string(&SwiftEmptyTuple()).unwrap()),
        swift_literal(&serde_json::to_string(&SwiftUnitStruct).unwrap()),
    )
}

/// The serde wire formats these Swift fixtures have to match. Asserted here so a
/// future serde change cannot silently invalidate the harness's expectations.
#[test]
fn swift_struct_wire_ground_truth() {
    assert_eq!(
        serde_json::to_string(&SwiftNewtype("hello".into())).unwrap(),
        r#""hello""#
    );
    assert_eq!(
        serde_json::to_string(&SwiftPair("hello".into(), 5)).unwrap(),
        r#"["hello",5]"#
    );
    assert_eq!(
        serde_json::to_string(&SwiftSkipReduced(1, 2)).unwrap(),
        r#"[2]"#
    );
    assert_eq!(serde_json::to_string(&SwiftEmptyTuple()).unwrap(), r#"[]"#);
    assert_eq!(serde_json::to_string(&SwiftUnitStruct).unwrap(), "null");
    assert!(serde_json::from_str::<SwiftSkipReduced>("[]").is_ok());
    assert!(serde_json::from_str::<SwiftTrailingDefault>("[1]").is_ok());
    // serde ignores `#[serde(default)]` on a newtype's only field.
    assert_eq!(serde_json::to_string(&SwiftNewtypeDefault(7)).unwrap(), "7");
    assert!(serde_json::from_str::<SwiftNewtypeDefault>("null").is_err());
    // serde writes a defaulted `Option<T>` element as `null`.
    assert_eq!(
        serde_json::to_string(&SwiftTrailOpt(1, None)).unwrap(),
        r#"[1,null]"#
    );
    // A nil suffix omits trailing defaults; serde reads the shorter sequence as
    // the defaulted value.
    assert_eq!(
        serde_json::to_string(&SwiftTwoTrailing(1, 2, 3)).unwrap(),
        r#"[1,2,3]"#
    );
    assert_eq!(
        serde_json::from_str::<SwiftTwoTrailing>("[1,2]").unwrap().2,
        0
    );
    assert_eq!(
        serde_json::from_str::<SwiftContainerDefault>("[1]")
            .unwrap()
            .1,
        0
    );
}

#[test]
fn swift_newtype_and_tuple_structs_round_trip() {
    let exported = Swift::default()
        .export(&swift_wire_types(), specta_serde::Format)
        .expect("newtype and tuple structs should export");

    assert!(
        exported.contains("public struct SwiftNewtype: Codable {"),
        "{exported}"
    );
    assert!(
        exported.contains("let container = try decoder.singleValueContainer()"),
        "a newtype must decode from a single-value container:\n{exported}"
    );
    assert!(
        exported.contains("var container = encoder.unkeyedContainer()"),
        "a tuple struct must encode through an unkeyed container:\n{exported}"
    );
    assert!(
        exported.contains("public let value: UInt8\n"),
        "`#[serde(default)]` on a newtype's only field must not make it optional:\n{exported}"
    );
    assert!(
        !exported.contains("public let value: UInt8?"),
        "the newtype default must not produce a double-optional property:\n{exported}"
    );
    let harness = format!("{}\n{SWIFT_ROUND_TRIP_HARNESS}", swift_fixtures());
    run_swift(&exported, &harness);
}

/// `PhasesFormat` splits a struct whose serialize and deserialize shapes differ
/// into `*Serialize`/`*Deserialize` types. Both halves are wire types in their
/// own right, so both must carry the correct sequence codec.
#[test]
fn swift_phase_split_structs_round_trip() {
    let types = Types::default()
        .register::<SwiftSkipReduced>()
        .register::<SwiftTrailingDefault>()
        .register::<SwiftTwoTrailing>()
        .register::<SwiftSkipIfTuple>()
        .register::<SwiftTrailOpt>()
        .register::<SwiftNewtypeOpt>();
    let exported = Swift::default()
        .export(&types, specta_serde::PhasesFormat)
        .expect("phase-split tuple structs should export");

    for name in [
        "SwiftSkipReducedSerialize",
        "SwiftSkipReducedDeserialize",
        "SwiftTrailingDefaultSerialize",
        "SwiftTrailingDefaultDeserialize",
        "SwiftTwoTrailingSerialize",
        "SwiftTwoTrailingDeserialize",
        "SwiftSkipIfTupleSerialize",
        "SwiftSkipIfTupleDeserialize",
        "SwiftTrailOptSerialize",
        "SwiftTrailOptDeserialize",
        "SwiftNewtypeOptSerialize",
        "SwiftNewtypeOptDeserialize",
    ] {
        assert!(
            exported.contains(&format!("public struct {name}: Codable {{")),
            "expected the split phase type `{name}`:\n{exported}"
        );
    }

    let harness = format!(
        r#"{}
import Foundation

func expect(_ actual: String, _ expected: String, _ label: String) {{
    precondition(actual == expected, "\(label): got \(actual), want \(expected)")
}}

func check(_ condition: Bool, _ label: String) {{
    precondition(condition, label)
}}

func encode<T: Encodable>(_ value: T) -> String {{
    String(data: try! JSONEncoder().encode(value), encoding: .utf8)!
}}

func decode<T: Decodable>(_ json: String, _: T.Type) -> T {{
    try! JSONDecoder().decode(T.self, from: json.data(using: .utf8)!)
}}

// The serialize half writes every element; the deserialize half tolerates the
// truncated trailing default.
expect(
    encode(SwiftTrailingDefaultSerialize(field0: 1, field1: 2)),
    trailingDefaultJSON,
    "phase trailing default serialize"
)
check(
    decode(trailingDefaultTruncatedJSON, SwiftTrailingDefaultDeserialize.self).field1 == nil,
    "phase trailing default deserialize short"
)
check(
    decode(trailingDefaultJSON, SwiftTrailingDefaultDeserialize.self).field1 == 2,
    "phase trailing default deserialize full"
)

// Skip-reduced: the serialize half requires the live element, the deserialize
// half accepts the empty array.
expect(encode(SwiftSkipReducedSerialize(field1: 2)), skipReducedJSON, "phase skip-reduced serialize")
check(decode("[]", SwiftSkipReducedDeserialize.self).field1 == nil, "phase skip-reduced deserialize empty")
check(decode(skipReducedJSON, SwiftSkipReducedDeserialize.self).field1 == 2, "phase skip-reduced deserialize")

// `skip_serializing_if`: the serialize half omits the absent element, while the
// deserialize half still requires the slot (and accepts `null`).
expect(encode(SwiftSkipIfTupleSerialize(field0: 1, field1: nil)), skipIfOmittedJSON, "phase skip-if serialize omitted")
expect(encode(SwiftSkipIfTupleSerialize(field0: 1, field1: 2)), skipIfPresentJSON, "phase skip-if serialize present")
check(decode(skipIfPresentJSON, SwiftSkipIfTupleDeserialize.self).field1 == 2, "phase skip-if deserialize present")
check(decode(skipIfNullJSON, SwiftSkipIfTupleDeserialize.self).field1 == nil, "phase skip-if deserialize null")

// A defaulted `Option<T>` is written as `null` by both halves (serde does not
// omit it without `skip_serializing_if`).
expect(encode(SwiftTrailOptSerialize(field0: 1, field1: nil)), trailOptNullJSON, "phase default option serialize null")
check(decode(trailOptNullJSON, SwiftTrailOptDeserialize.self).field1 == nil, "phase default option deserialize null")

// A run of trailing defaults: the serialize half writes every element, the
// deserialize half accepts the truncated suffix.
expect(encode(SwiftTwoTrailingSerialize(field0: 1, field1: 2, field2: 3)), twoTrailingJSON, "phase two trailing serialize")
check(decode(twoTrailingShortJSON, SwiftTwoTrailingDeserialize.self).field2 == nil, "phase two trailing deserialize short")

// An optional newtype stays a bare value in both halves.
expect(encode(SwiftNewtypeOptSerialize(value: nil)), newtypeOptNullJSON, "phase optional newtype serialize null")
check(decode(newtypeOptNullJSON, SwiftNewtypeOptDeserialize.self).value == nil, "phase optional newtype deserialize null")

print("SWIFT-OK")
"#,
        swift_fixtures()
    );

    run_swift(&exported, &harness);
}

/// Shapes with no faithful Swift codec must fail the export rather than silently
/// emitting a keyed `Codable` that disagrees with serde.
#[test]
fn swift_unrepresentable_struct_shapes_report_errors() {
    let nested_tuple = Swift::default()
        .export(
            &Types::default().register::<SwiftNestedTuple>(),
            specta_serde::Format,
        )
        .expect_err("a tuple-valued element cannot be expressed in Swift");
    assert!(
        nested_tuple.to_string().contains("cannot"),
        "{nested_tuple}"
    );

    // `specta-serde` already rejects a non-final `skip_serializing_if` on an
    // unnamed field, because omitting an earlier element shifts every following
    // value; the exporter surfaces that diagnostic rather than emitting a codec.
    let mid_omittable = Swift::default()
        .export(
            &Types::default().register::<SwiftMidOmittable>(),
            specta_serde::Format,
        )
        .expect_err("a non-trailing omittable element cannot be expressed in Swift");
    assert!(
        mid_omittable.to_string().contains("final live field"),
        "{mid_omittable}"
    );

    // An inlined tuple struct would splice its body into a type position.
    let inline_tuple = Swift::default()
        .export(
            &Types::default().register::<SwiftOuterInline>(),
            specta_serde::Format,
        )
        .expect_err("an inlined tuple struct cannot be expressed in Swift");
    assert!(
        inline_tuple.to_string().contains("cannot"),
        "{inline_tuple}"
    );

    // Defensive: a newtype with no live field cannot be a `Codable` value.
    // `specta-serde` never produces this shape (a skipped newtype field keeps
    // `ty: Some(..)`), so the raw graph is used to reach the guard.
    let skipped_newtype = Swift::default()
        .export(
            &Types::default().register::<HandBuiltSkippedNewtype>(),
            IdentityFormat,
        )
        .expect_err("a newtype with no live field cannot be expressed in Swift");
    assert!(
        skipped_newtype.to_string().contains("no live field"),
        "{skipped_newtype}"
    );
}
