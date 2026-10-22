use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use specta::{
    Format, Type, Types,
    datatype::{DataType, Enum},
};
use specta_swift::Swift;

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
/// into the variant's object, which the generated wrapper struct cannot express.
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

    assert!(err.to_string().contains("flattens field"), "{err}");
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
