use std::borrow::Cow;
use std::path::Path;
use std::process::Command;

use serde::{Deserialize, Serialize};
use specta::{
    Format, Type, Types,
    datatype::{DataType, Enum, Field, NamedDataType, Struct},
};
use specta_swift::{GenericStyle, Swift};
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

/// The same tagged enums with `unknown_variants(true)`, so the generated
/// catch-all is reviewable in a snapshot as well as exercised at runtime.
#[test]
fn swift_serde_tagged_enums_unknown_export() {
    let types = Types::default()
        .register::<AdjacentReason>()
        .register::<AdjacentError>()
        .register::<InternalReason>()
        .register::<ExternalReason>()
        .register::<ExternalError>();

    insta::assert_snapshot!(
        "swift-serde-tagged-enums-unknown",
        Swift::default()
            .unknown_variants(true)
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
        rendered.contains("public struct AdjacentEmptyEmptystructData: Codable, Sendable {"),
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
    // serde writes a nil `Option` payload field as `null`, not omitted, so the
    // generated `Codable` must too
    // (<https://github.com/specta-rs/specta/issues/616>).
    assert_eq!(
        serde_json::to_value(TaggedFieldEdgeCases::Optional {
            opt: None,
            defaulted: 0
        })
        .unwrap(),
        serde_json::json!({ "t": "Optional", "opt": null, "defaulted": 0 })
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
        rendered.contains("public enum TaggedUnitEnum: Sendable {"),
        "{rendered}"
    );
    assert!(
        !rendered.contains("public enum TaggedUnitEnum: String"),
        "{rendered}"
    );
}

// Regression tests for forward-compatible tagged enums.
// https://github.com/specta-rs/specta/issues/617
//
// A generated tagged enum is closed: an `init(from:)` that meets a tag it does
// not know throws, and the app loses the whole value. `unknown_variants(true)`
// adds a catch-all case that keeps the tag and its payload as JSON, so a build
// that predates a variant can still decode it (and re-encode what it read).
// These tests compile and *run* the generated Swift against unknown payloads.

#[test]
fn swift_unknown_variants_round_trip() {
    let types = Types::default()
        .register::<AdjacentReason>()
        .register::<AdjacentError>()
        .register::<InternalReason>()
        .register::<ExternalReason>()
        .register::<ExternalError>();
    let exported = Swift::default()
        .unknown_variants(true)
        .export(&types, specta_serde::Format)
        .expect("tagged enums should export with unknown variants");

    assert!(
        exported.contains("case unknown(tag: String, content: JSONValue?)"),
        "{exported}"
    );
    assert!(
        exported.contains("public enum JSONValue: Codable, Equatable, Sendable {"),
        "{exported}"
    );

    let fixtures = format!(
        "let unknownAdjacentReasonJSON = {}\n\
         let unknownAdjacentUnitJSON = {}\n\
         let unknownNestedJSON = {}\n\
         let unknownInternalJSON = {}\n\
         let unknownExternalStringJSON = {}\n\
         let unknownExternalObjectJSON = {}\n\
         let unknownExternalNestedJSON = {}\n",
        swift_literal(r#"{"params":{"retry_after":45},"reason":"future"}"#),
        swift_literal(r#"{"reason":"future"}"#),
        swift_literal(
            r#"{"context":"system","data":{"params":{"retry_after":45},"reason":"future"}}"#
        ),
        swift_literal(r#"{"reason":"future","retry_after":45}"#),
        swift_literal(r#""future""#),
        swift_literal(r#"{"future":{"retry_after":45}}"#),
        swift_literal(r#"{"auth":"future"}"#),
    );

    let harness = format!(
        r#"import Foundation

{}
func check(_ condition: Bool, _ label: String) {{
    precondition(condition, label)
}}

func decode<T: Decodable>(_ json: String, _: T.Type) -> T {{
    try! JSONDecoder().decode(T.self, from: json.data(using: .utf8)!)
}}

func encode<T: Encodable>(_ value: T) -> String {{
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    return String(data: try! encoder.encode(value), encoding: .utf8)!
}}

// Adjacent tagging: an unknown tag keeps its `content` value.
let unknownReason = decode(unknownAdjacentReasonJSON, AdjacentReason.self)
guard case .unknown(let tag, let content) = unknownReason else {{
    preconditionFailure("expected an unknown reason")
}}
check(tag == "future", "unknown tag")
check(content == .object(["retry_after": .number(45)]), "unknown content")
check(encode(unknownReason) == unknownAdjacentReasonJSON, "unknown reason round-trip")

// A unit-like unknown variant has no content.
let unitReason = decode(unknownAdjacentUnitJSON, AdjacentReason.self)
guard case .unknown(let unitTag, let unitContent) = unitReason else {{
    preconditionFailure("expected an unknown unit reason")
}}
check(unitTag == "future" && unitContent == nil, "unit unknown")
check(encode(unitReason) == unknownAdjacentUnitJSON, "unit unknown round-trip")

// An unknown inner variant stays inside its known outer case.
let nested = decode(unknownNestedJSON, AdjacentError.self)
guard case .system(let outer) = nested, case .unknown(let nestedTag, let nestedContent) = outer.data else {{
    preconditionFailure("expected a nested unknown")
}}
check(nestedTag == "future", "nested tag")
check(nestedContent == .object(["retry_after": .number(45)]), "nested content")

// Internal tagging: the payload is whatever fields remain beside the tag.
let internalReason = decode(unknownInternalJSON, InternalReason.self)
guard case .unknown(let internalTag, let internalContent) = internalReason else {{
    preconditionFailure("expected an unknown internal reason")
}}
check(internalTag == "future", "internal tag")
check(internalContent == .object(["retry_after": .number(45)]), "internal content")
check(encode(internalReason) == unknownInternalJSON, "internal round-trip")

// External tagging: a bare string is an unknown unit-like variant...
let externalString = decode(unknownExternalStringJSON, ExternalReason.self)
guard case .unknown(let stringTag, let stringContent) = externalString else {{
    preconditionFailure("expected an unknown external string reason")
}}
check(stringTag == "future" && stringContent == nil, "external string unknown")
check(encode(externalString) == unknownExternalStringJSON, "external string round-trip")

// ...and a single-key object keeps that key's value.
let externalObject = decode(unknownExternalObjectJSON, ExternalReason.self)
guard case .unknown(let objectTag, let objectContent) = externalObject else {{
    preconditionFailure("expected an unknown external object reason")
}}
check(objectTag == "future", "external object tag")
check(objectContent == .object(["retry_after": .number(45)]), "external object content")
check(encode(externalObject) == unknownExternalObjectJSON, "external object round-trip")

// A known external variant can itself hold an unknown payload.
let externalNested = decode(unknownExternalNestedJSON, ExternalError.self)
guard case .auth(let auth) = externalNested, case .unknown(_, _) = auth else {{
    preconditionFailure("expected a nested external unknown")
}}

print("SWIFT-OK")
"#,
        fixtures
    );

    run_swift(&exported, &harness);
}

/// Without the option the generated `Codable` is still closed, which is what
/// makes `unknown_variants` an opt-in rather than a silent wire change.
#[test]
fn swift_unknown_variants_default_still_throws() {
    let exported = Swift::default()
        .export(
            &Types::default().register::<AdjacentReason>(),
            specta_serde::Format,
        )
        .unwrap();
    assert!(!exported.contains("case unknown"), "{exported}");

    let harness = format!(
        r#"import Foundation

func check(_ condition: Bool, _ label: String) {{
    precondition(condition, label)
}}

func throwsError(_ body: () throws -> Void) -> Bool {{
    do {{
        try body()
        return false
    }} catch {{
        return true
    }}
}}

let unknown = {}.data(using: .utf8)!
check(
    throwsError {{ _ = try JSONDecoder().decode(AdjacentReason.self, from: unknown) }},
    "an unknown tag must throw without `unknown_variants`"
)

print("SWIFT-OK")
"#,
        swift_literal(r#"{"reason":"future","params":{"retry_after":45}}"#),
    );

    run_swift(&exported, &harness);
}

/// A variant already named `Unknown` must not collide with the generated
/// catch-all, which it would if both rendered as `case unknown`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c", rename_all = "camelCase")]
enum UnknownCollision {
    Unknown { x: u32 },
    Other { y: u32 },
}

#[test]
fn swift_unknown_variant_case_avoids_collision() {
    let rendered = Swift::default()
        .unknown_variants(true)
        .export(
            &Types::default().register::<UnknownCollision>(),
            specta_serde::Format,
        )
        .unwrap();

    assert!(
        rendered.contains("case unknown(UnknownCollisionUnknownData)"),
        "the declared variant keeps its case name: {rendered}"
    );
    assert!(
        rendered.contains("case unknownVariant(tag: String, content: JSONValue?)"),
        "the catch-all avoids the collision: {rendered}"
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

// Regression tests for named structs.
// https://github.com/specta-rs/specta/issues/610
//
// The Swift exporter used to leave a named struct's `Codable` to the compiler,
// which encodes optionals with `encodeIfPresent` and therefore omits every nil.
// Serde writes a plain `Option<T>` and a `#[serde(default)] Option<T>` as
// `null`, and omits the key only for `#[serde(skip_serializing_if = ...)]`.
// These tests compile and *run* the generated Swift against fixtures produced by
// serde itself.

/// A plain `Option<T>` field: serde writes `null` and reads a missing key as
/// `None`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftNamedPlain {
    x: Option<u8>,
    y: u8,
}

/// A `#[serde(default)] Option<T>` field: serde still writes `null`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftNamedDefaultOption {
    #[serde(default)]
    x: Option<u8>,
    y: u8,
}

/// A `#[serde(skip_serializing_if = "Option::is_none")]` field: serde omits the
/// key when it is nil, and reads a missing key as `None`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftNamedSkipIf {
    #[serde(skip_serializing_if = "Option::is_none")]
    x: Option<u8>,
    y: u8,
}

/// A required non-optional field: serde always writes it and requires the key on
/// deserialize.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftNamedRequired {
    x: u8,
    y: String,
}

/// Nested optionals: `None` and `Some(None)` both write `null`; only
/// `Some(Some(v))` writes `v`, and `null` reads back as `None`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftNamedNestedOptional {
    x: Option<Option<u8>>,
    y: u8,
}

/// A `#[serde(skip_serializing_if = "Vec::is_empty")]` non-`Option` field: serde
/// omits it when serializing but still *requires* the key when deserializing.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftNamedSkipVec {
    #[serde(skip_serializing_if = "Vec::is_empty")]
    x: Vec<u8>,
    y: u8,
}

/// A container-level `#[serde(default)]`: serde tolerates an absent key for
/// every field.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
#[serde(default)]
struct SwiftNamedContainerDefault {
    x: u8,
    y: u8,
}

/// An empty named struct: serde writes and reads `{}`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftNamedEmpty {}

/// A field-level `#[serde(default)]` on a non-`Option` field: serde tolerates
/// an absent key (the branch is distinct from the container-level default).
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftNamedFieldDefault {
    #[serde(default)]
    x: u8,
    y: u8,
}

/// A `#[specta(type = ...)]` override changes the exported datatype but not
/// serde's behaviour: `#[specta(type = u8)] x: Option<u8>` is still an
/// `Option<u8>` to serde, so it writes `null` and tolerates an absent key.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct SwiftNamedOverrideNullable {
    #[specta(type = u8)]
    x: Option<u8>,
    y: u8,
}

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
        exported.contains("public struct SwiftNewtype: Codable, Sendable {"),
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

/// A generic type used to be emitted with unconstrained parameters and its
/// variant wrapper structs referred to `T` without declaring it, so nothing in
/// the generic path compiled. Generic parameters now carry the `Codable` bound
/// their declaration needs, and the wrappers repeat them.
#[derive(Type, Serialize, Deserialize)]
struct SwiftGenericBox<T> {
    value: T,
    label: String,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(tag = "type")]
enum SwiftGenericTagged<T> {
    Data { value: T },
    Unit,
}

#[derive(Type, Serialize, Deserialize)]
enum SwiftGenericExternal<T> {
    Wrapped(T),
    Named { value: T },
    Nothing,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(tag = "t", content = "c")]
enum SwiftGenericAdjacent<T> {
    Data { value: T },
    Unit,
}

#[derive(Type, Serialize, Deserialize)]
struct SwiftGenericNewtype<T>(T);

#[derive(Type, Serialize, Deserialize)]
struct SwiftGenericTuple<T>(T, String);

#[test]
fn swift_generic_types_round_trip() {
    let types = Types::default()
        .register::<SwiftGenericBox<String>>()
        .register::<SwiftGenericTagged<u32>>()
        .register::<SwiftGenericExternal<String>>()
        .register::<SwiftGenericAdjacent<u32>>()
        .register::<SwiftGenericNewtype<u32>>()
        .register::<SwiftGenericTuple<u32>>();
    let exported = Swift::default()
        .export(&types, specta_serde::Format)
        .expect("generic types should export");

    assert!(
        exported.contains("public struct SwiftGenericBox<T: Codable>: Codable {"),
        "a generic struct needs a `Codable` bound on its parameter:\n{exported}"
    );
    assert!(
        exported.contains("public enum SwiftGenericTagged<T: Codable> {"),
        "{exported}"
    );
    assert!(
        exported.contains("public struct SwiftGenericTaggedDataData<T: Codable>: Codable {"),
        "a variant wrapper struct must declare the enum's parameters:\n{exported}"
    );
    assert!(
        exported.contains("case data(SwiftGenericTaggedDataData<T>)"),
        "a case payload must pass the parameters through:\n{exported}"
    );
    assert!(
        exported.contains("case data(SwiftGenericAdjacentDataData<T>)"),
        "an adjacent content payload passes them through too:\n{exported}"
    );
    assert!(
        exported.contains("extension SwiftGenericBox: Sendable where T: Sendable {}"),
        "the conformance stays conditional so a non-Sendable payload still decodes:\n{exported}"
    );

    let harness = r#"
import Foundation

let decoder = JSONDecoder()
let encoder = JSONEncoder()

let box = try decoder.decode(
    SwiftGenericBox<String>.self,
    from: Data("{\"value\":\"hi\",\"label\":\"box\"}".utf8)
)
precondition(box.value == "hi" && box.label == "box")
let roundTripped = try decoder.decode(
    SwiftGenericBox<String>.self,
    from: try encoder.encode(box)
)
precondition(roundTripped.value == "hi")

let tagged = try decoder.decode(
    SwiftGenericTagged<UInt32>.self,
    from: Data("{\"type\":\"Data\",\"value\":7}".utf8)
)
guard case .data(let payload) = tagged else { preconditionFailure("tagged payload") }
precondition(payload.value == 7)

let external = try decoder.decode(
    SwiftGenericExternal<String>.self,
    from: Data("{\"Named\":{\"value\":\"named\"}}".utf8)
)
guard case .named(let named) = external else { preconditionFailure("named payload") }
precondition(named.value == "named")

let wrapped = try decoder.decode(
    SwiftGenericExternal<String>.self,
    from: Data("{\"Wrapped\":\"wrapped\"}".utf8)
)
guard case .wrapped(let value) = wrapped else { preconditionFailure("wrapped payload") }
precondition(value == "wrapped")

let adjacent = try decoder.decode(
    SwiftGenericAdjacent<UInt32>.self,
    from: Data("{\"t\":\"Data\",\"c\":{\"value\":9}}".utf8)
)
guard case .data(let adjacentPayload) = adjacent else {
    preconditionFailure("adjacent payload")
}
precondition(adjacentPayload.value == 9)

let newtype = try decoder.decode(
    SwiftGenericNewtype<UInt32>.self,
    from: Data("5".utf8)
)
precondition(newtype.value == 5)

let tuple = try decoder.decode(
    SwiftGenericTuple<UInt32>.self,
    from: Data("[6,\"six\"]".utf8)
)
precondition(tuple.field0 == 6 && tuple.field1 == "six")

print("SWIFT-OK")
"#;
    run_swift(&exported, harness);
}

/// The `where`-style generic bound and the configured protocols produce
/// declarations that compile and behave, including through `JSONValue`.
#[test]
fn swift_typealias_generics_and_protocols_round_trip() {
    let types = Types::default()
        .register::<SwiftGenericBox<String>>()
        .register::<SwiftGenericTagged<u32>>();
    let exported = Swift::default()
        .generics(GenericStyle::Typealias)
        .add_protocol("Equatable")
        .add_protocol("Hashable")
        .unknown_variants(true)
        .export(&types, specta_serde::Format)
        .expect("generic types should export");

    assert!(
        exported.contains(
            "public struct SwiftGenericBox<T>: Codable, Equatable, Hashable \
             where T: Codable & Equatable & Hashable {"
        ),
        "{exported}"
    );
    assert!(
        exported.contains(
            "public enum SwiftGenericTagged<T>: Equatable, Hashable \
             where T: Codable & Equatable & Hashable {"
        ),
        "{exported}"
    );
    assert!(
        exported.contains("public enum JSONValue: Codable, Equatable, Sendable, Hashable {"),
        "the helper keeps `Equatable` once and gains the configured protocol:\n{exported}"
    );

    let harness = r#"
import Foundation

let decoder = JSONDecoder()

let first = try decoder.decode(
    SwiftGenericBox<String>.self,
    from: Data("{\"value\":\"hi\",\"label\":\"box\"}".utf8)
)
let second = try decoder.decode(
    SwiftGenericBox<String>.self,
    from: Data("{\"value\":\"hi\",\"label\":\"box\"}".utf8)
)
precondition(first == second)
precondition(first.hashValue == second.hashValue)

let unknown = try decoder.decode(
    SwiftGenericTagged<UInt32>.self,
    from: Data("{\"type\":\"Future\",\"value\":1}".utf8)
)
guard case .unknown(let tag, _) = unknown else { preconditionFailure("unknown variant") }
precondition(tag == "Future")

print("SWIFT-OK")
"#;
    run_swift(&exported, harness);
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
            exported.contains(&format!("public struct {name}: Codable, Sendable {{")),
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
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    return String(data: try! encoder.encode(value), encoding: .utf8)!
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

/// serde's JSON for `value` with object keys sorted, so it can be compared
/// byte-for-byte with Swift's `JSONEncoder` under `.sortedKeys`. Foundation does
/// not otherwise define key order, and `serde_json`'s default map is a
/// `BTreeMap`, so this is the canonical form both sides agree on.
fn swift_sorted_json(value: &impl Serialize) -> String {
    serde_json::to_string(&serde_json::to_value(value).unwrap()).unwrap()
}

/// The serde wire fixtures the named-struct harnesses assert against, so a
/// future serde change cannot silently invalidate their expectations.
fn swift_named_fixtures() -> String {
    format!(
        "let namedPlainNullJSON = {}\n\
         let namedPlainSomeJSON = {}\n\
         let namedPlainMissingJSON = {}\n\
         let namedDefaultNullJSON = {}\n\
         let namedDefaultMissingJSON = {}\n\
         let namedSkipOmittedJSON = {}\n\
         let namedSkipPresentJSON = {}\n\
         let namedSkipNullJSON = {}\n\
         let namedRequiredJSON = {}\n\
         let namedNestedNullJSON = {}\n\
         let namedNestedSomeJSON = {}\n",
        swift_literal(&swift_sorted_json(&SwiftNamedPlain { x: None, y: 1 })),
        swift_literal(&swift_sorted_json(&SwiftNamedPlain { x: Some(2), y: 1 })),
        swift_literal(r#"{"y":1}"#),
        swift_literal(&swift_sorted_json(&SwiftNamedDefaultOption {
            x: None,
            y: 1
        })),
        swift_literal(r#"{"y":1}"#),
        swift_literal(&swift_sorted_json(&SwiftNamedSkipIf { x: None, y: 1 })),
        swift_literal(&swift_sorted_json(&SwiftNamedSkipIf { x: Some(2), y: 1 })),
        swift_literal(r#"{"x":null,"y":1}"#),
        swift_literal(&swift_sorted_json(&SwiftNamedRequired {
            x: 1,
            y: "a".into(),
        })),
        swift_literal(&swift_sorted_json(&SwiftNamedNestedOptional {
            x: None,
            y: 1
        })),
        swift_literal(&swift_sorted_json(&SwiftNamedNestedOptional {
            x: Some(Some(2)),
            y: 1,
        })),
    )
}

/// The serde wire formats the named-struct harnesses have to match, asserted
/// here so a serde change cannot silently invalidate them.
#[test]
fn swift_named_option_wire_ground_truth() {
    // A plain `Option<T>` field is written as `null`, not omitted, and a
    // missing key still reads as `None`.
    assert_eq!(
        serde_json::to_string(&SwiftNamedPlain { x: None, y: 1 }).unwrap(),
        r#"{"x":null,"y":1}"#
    );
    assert_eq!(
        serde_json::to_string(&SwiftNamedPlain { x: Some(2), y: 1 }).unwrap(),
        r#"{"x":2,"y":1}"#
    );
    assert_eq!(
        serde_json::from_str::<SwiftNamedPlain>(r#"{"y":1}"#)
            .unwrap()
            .x,
        None
    );

    // `#[serde(default)]` does not change the serialize shape.
    assert_eq!(
        serde_json::to_string(&SwiftNamedDefaultOption { x: None, y: 1 }).unwrap(),
        r#"{"x":null,"y":1}"#
    );
    assert_eq!(
        serde_json::from_str::<SwiftNamedDefaultOption>(r#"{"y":1}"#)
            .unwrap()
            .x,
        None
    );

    // Only `skip_serializing_if` omits the key.
    assert_eq!(
        serde_json::to_string(&SwiftNamedSkipIf { x: None, y: 1 }).unwrap(),
        r#"{"y":1}"#
    );
    assert_eq!(
        serde_json::to_string(&SwiftNamedSkipIf { x: Some(2), y: 1 }).unwrap(),
        r#"{"x":2,"y":1}"#
    );
    assert_eq!(
        serde_json::from_str::<SwiftNamedSkipIf>(r#"{"y":1}"#)
            .unwrap()
            .x,
        None
    );

    // A required non-optional field is always written and always required.
    assert_eq!(
        serde_json::to_string(&SwiftNamedRequired {
            x: 1,
            y: "a".into()
        })
        .unwrap(),
        r#"{"x":1,"y":"a"}"#
    );
    assert!(serde_json::from_str::<SwiftNamedRequired>(r#"{"y":"a"}"#).is_err());

    // Nested optionals: `None` and `Some(None)` both write `null`; `null` reads
    // back as `None`.
    assert_eq!(
        serde_json::to_string(&SwiftNamedNestedOptional { x: None, y: 1 }).unwrap(),
        r#"{"x":null,"y":1}"#
    );
    assert_eq!(
        serde_json::to_string(&SwiftNamedNestedOptional {
            x: Some(None),
            y: 1,
        })
        .unwrap(),
        r#"{"x":null,"y":1}"#
    );
    assert_eq!(
        serde_json::to_string(&SwiftNamedNestedOptional {
            x: Some(Some(2)),
            y: 1,
        })
        .unwrap(),
        r#"{"x":2,"y":1}"#
    );
    assert!(
        serde_json::from_str::<SwiftNamedNestedOptional>(r#"{"x":null,"y":1}"#)
            .unwrap()
            .x
            .is_none()
    );
}

fn swift_named_types() -> Types {
    Types::default()
        .register::<SwiftNamedPlain>()
        .register::<SwiftNamedDefaultOption>()
        .register::<SwiftNamedSkipIf>()
        .register::<SwiftNamedRequired>()
        .register::<SwiftNamedNestedOptional>()
}

/// Named structs must write a nil `Option<T>` as `null` (serde does), and omit
/// the key only for `skip_serializing_if`. The synthesised `Codable` used
/// `encodeIfPresent` and omitted every nil.
#[test]
fn swift_named_struct_optionals_round_trip() {
    let exported = Swift::default()
        .export(&swift_named_types(), specta_serde::Format)
        .expect("named structs should export");

    assert!(
        exported.contains("try container.encode(x, forKey: .x)"),
        "a plain optional must be written unconditionally:\n{exported}"
    );
    assert!(
        exported.contains("public init(from decoder: Decoder) throws {"),
        "a hand-written codec must replace the synthesised one:\n{exported}"
    );
    assert!(
        !exported.contains("encodeIfPresent"),
        "the hand-written codec must not use `encodeIfPresent`:\n{exported}"
    );

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
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    return String(data: try! encoder.encode(value), encoding: .utf8)!
}}

func decode<T: Decodable>(_ json: String, _: T.Type) -> T {{
    try! JSONDecoder().decode(T.self, from: json.data(using: .utf8)!)
}}

// A plain `Option<T>` field is written as `null` (serde does not omit it).
expect(encode(SwiftNamedPlain(x: nil, y: 1)), namedPlainNullJSON, "plain nil encodes null")
expect(encode(SwiftNamedPlain(x: 2, y: 1)), namedPlainSomeJSON, "plain some encodes value")
check(decode(namedPlainNullJSON, SwiftNamedPlain.self).x == nil, "plain decode null")
check(decode(namedPlainMissingJSON, SwiftNamedPlain.self).x == nil, "plain decode missing")
check(decode(namedPlainSomeJSON, SwiftNamedPlain.self).x == 2, "plain decode some")

// `#[serde(default)] Option<T>` is still written as `null`.
expect(encode(SwiftNamedDefaultOption(x: nil, y: 1)), namedDefaultNullJSON, "default nil encodes null")
check(decode(namedDefaultMissingJSON, SwiftNamedDefaultOption.self).x == nil, "default decode missing")

// `skip_serializing_if` omits the key when nil, and still reads a missing key.
expect(encode(SwiftNamedSkipIf(x: nil, y: 1)), namedSkipOmittedJSON, "skip-if nil omits key")
expect(encode(SwiftNamedSkipIf(x: 2, y: 1)), namedSkipPresentJSON, "skip-if some writes key")
check(decode(namedSkipOmittedJSON, SwiftNamedSkipIf.self).x == nil, "skip-if decode omitted")
check(decode(namedSkipNullJSON, SwiftNamedSkipIf.self).x == nil, "skip-if decode null")

// A required non-optional field is always present.
expect(encode(SwiftNamedRequired(x: 1, y: "a")), namedRequiredJSON, "required encodes")
let required = decode(namedRequiredJSON, SwiftNamedRequired.self)
expect(required.y, "a", "required decode")

// Nested optionals: nil and `.some(nil)` both write `null`; only
// `.some(.some(v))` writes the value.
expect(encode(SwiftNamedNestedOptional(x: nil, y: 1)), namedNestedNullJSON, "nested nil writes null")
expect(encode(SwiftNamedNestedOptional(x: .some(nil), y: 1)), namedNestedNullJSON, "nested some-nil writes null")
expect(encode(SwiftNamedNestedOptional(x: .some(.some(2)), y: 1)), namedNestedSomeJSON, "nested some-some writes value")
check(decode(namedNestedNullJSON, SwiftNamedNestedOptional.self).x == nil, "nested decode null")
check(decode(namedNestedSomeJSON, SwiftNamedNestedOptional.self).x! == 2, "nested decode some-some")

print("SWIFT-OK")
"#,
        swift_named_fixtures()
    );

    run_swift(&exported, &harness);
}

/// `PhasesFormat` splits a named struct whose serialize and deserialize shapes
/// differ into `*Serialize`/`*Deserialize` types. The serialize half writes
/// `null` for a plain/defaulted `Option<T>` and omits only a `skip_serializing_if`
/// field; the deserialize half tolerates the absence serde tolerates.
#[test]
fn swift_named_struct_phase_split_round_trip() {
    let exported = Swift::default()
        .export(&swift_named_types(), specta_serde::PhasesFormat)
        .expect("phase-split named structs should export");

    for name in [
        "SwiftNamedPlainSerialize",
        "SwiftNamedPlainDeserialize",
        "SwiftNamedDefaultOptionSerialize",
        "SwiftNamedDefaultOptionDeserialize",
        "SwiftNamedSkipIfSerialize",
        "SwiftNamedSkipIfDeserialize",
        "SwiftNamedNestedOptionalSerialize",
        "SwiftNamedNestedOptionalDeserialize",
    ] {
        assert!(
            exported.contains(&format!("public struct {name}: Codable, Sendable {{")),
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
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    return String(data: try! encoder.encode(value), encoding: .utf8)!
}}

func decode<T: Decodable>(_ json: String, _: T.Type) -> T {{
    try! JSONDecoder().decode(T.self, from: json.data(using: .utf8)!)
}}

// The serialize half writes `null` for a plain option; the deserialize half
// reads both `null` and an absent key.
expect(encode(SwiftNamedPlainSerialize(x: nil, y: 1)), namedPlainNullJSON, "phase plain serialize null")
expect(encode(SwiftNamedPlainSerialize(x: 2, y: 1)), namedPlainSomeJSON, "phase plain serialize some")
check(decode(namedPlainNullJSON, SwiftNamedPlainDeserialize.self).x == nil, "phase plain deserialize null")
check(decode(namedPlainMissingJSON, SwiftNamedPlainDeserialize.self).x == nil, "phase plain deserialize missing")

// A defaulted option is written as `null` by both halves.
expect(encode(SwiftNamedDefaultOptionSerialize(x: nil, y: 1)), namedDefaultNullJSON, "phase default serialize null")
check(decode(namedDefaultMissingJSON, SwiftNamedDefaultOptionDeserialize.self).x == nil, "phase default deserialize missing")

// `skip_serializing_if`: the serialize half omits the absent key, the
// deserialize half still accepts it.
expect(encode(SwiftNamedSkipIfSerialize(x: nil, y: 1)), namedSkipOmittedJSON, "phase skip-if serialize omits")
expect(encode(SwiftNamedSkipIfSerialize(x: 2, y: 1)), namedSkipPresentJSON, "phase skip-if serialize present")
check(decode(namedSkipOmittedJSON, SwiftNamedSkipIfDeserialize.self).x == nil, "phase skip-if deserialize omitted")

// A required struct has the same shape in both phases and is not split.
expect(encode(SwiftNamedRequired(x: 1, y: "a")), namedRequiredJSON, "phase required serialize")
check(decode(namedRequiredJSON, SwiftNamedRequired.self).y == "a", "phase required deserialize")

// Nested optionals split too.
expect(encode(SwiftNamedNestedOptionalSerialize(x: nil, y: 1)), namedNestedNullJSON, "phase nested serialize null")
check(decode(namedNestedNullJSON, SwiftNamedNestedOptionalDeserialize.self).x == nil, "phase nested deserialize null")

print("SWIFT-OK")
"#,
        swift_named_fixtures()
    );

    run_swift(&exported, &harness);
}

/// A bare `#[serde(skip_serializing_if = ...)]` does not make a field
/// absent-tolerant on deserialize: serde omits the key when serializing but
/// still errors on the absent key. The unified codec must reproduce that
/// asymmetry, while a container-level `#[serde(default)]` (which *does*
/// tolerate absence) must still decode an absent key, and an empty named struct
/// must stay constructible and round-trip as `{}`.
#[test]
fn swift_named_skip_if_and_default_decode_tolerance() {
    assert_eq!(
        serde_json::to_string(&SwiftNamedSkipVec { x: vec![], y: 1 }).unwrap(),
        r#"{"y":1}"#
    );
    assert_eq!(
        serde_json::to_string(&SwiftNamedSkipVec { x: vec![1], y: 1 }).unwrap(),
        r#"{"x":[1],"y":1}"#
    );
    assert!(
        serde_json::from_str::<SwiftNamedSkipVec>(r#"{"y":1}"#).is_err(),
        "serde requires the key when deserializing a bare `skip_serializing_if` field"
    );
    assert!(serde_json::from_str::<SwiftNamedSkipVec>(r#"{"x":[],"y":1}"#).is_ok());
    assert!(serde_json::from_str::<SwiftNamedContainerDefault>(r#"{}"#).is_ok());
    assert!(serde_json::from_str::<SwiftNamedFieldDefault>(r#"{"y":1}"#).is_ok());
    assert_eq!(serde_json::to_string(&SwiftNamedEmpty {}).unwrap(), r#"{}"#);
    assert!(serde_json::from_str::<SwiftNamedEmpty>(r#"{}"#).is_ok());

    let types = Types::default()
        .register::<SwiftNamedSkipVec>()
        .register::<SwiftNamedContainerDefault>()
        .register::<SwiftNamedFieldDefault>()
        .register::<SwiftNamedEmpty>();
    let exported = Swift::default()
        .export(&types, specta_serde::Format)
        .expect("named structs should export");
    assert!(
        exported.contains("self.x = try container.decode([UInt8].self, forKey: .x)"),
        "a bare `skip_serializing_if` field must be required on decode:\n{exported}"
    );
    assert!(
        exported.contains("public init() {}"),
        "an empty named struct needs a public initializer:\n{exported}"
    );

    let harness = format!(
        r#"let skipVecOmittedJSON = {}
let skipVecPresentJSON = {}
let containerDefaultEmptyJSON = {}
let fieldDefaultMissingJSON = {}
let emptyJSON = {}
{}
import Foundation

func expect(_ actual: String, _ expected: String, _ label: String) {{
    precondition(actual == expected, "\(label): got \(actual), want \(expected)")
}}

func check(_ condition: Bool, _ label: String) {{
    precondition(condition, label)
}}

func encode<T: Encodable>(_ value: T) -> String {{
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    return String(data: try! encoder.encode(value), encoding: .utf8)!
}}

func decode<T: Decodable>(_ json: String, _: T.Type) -> T {{
    try! JSONDecoder().decode(T.self, from: json.data(using: .utf8)!)
}}

func throwsError(_ body: () throws -> Void) -> Bool {{
    do {{
        try body()
        return false
    }} catch {{
        return true
    }}
}}

// `skip_serializing_if` omits the key when serializing...
expect(encode(SwiftNamedSkipVec(x: nil, y: 1)), skipVecOmittedJSON, "skip-if nil omits key")
expect(encode(SwiftNamedSkipVec(x: [1], y: 1)), skipVecPresentJSON, "skip-if value writes key")
// ...but serde still requires it when deserializing, and so must the codec.
check(
    throwsError {{ _ = try JSONDecoder().decode(SwiftNamedSkipVec.self, from: skipVecOmittedJSON.data(using: .utf8)!) }},
    "skip-if absent key must throw"
)
check(decode(skipVecPresentJSON, SwiftNamedSkipVec.self).x == [1], "skip-if present decodes")

// A container-level `#[serde(default)]` does tolerate an absent key.
check(decode(containerDefaultEmptyJSON, SwiftNamedContainerDefault.self).x == nil, "container default decodes empty")
expect(encode(SwiftNamedContainerDefault(x: nil, y: nil)), containerDefaultEmptyJSON, "container default nil omits both")

// A field-level `#[serde(default)]` on a non-`Option` field is tolerant too.
check(decode(fieldDefaultMissingJSON, SwiftNamedFieldDefault.self).x == nil, "field default decodes missing")

// An empty named struct writes and reads an empty object and is publicly
// constructible.
expect(encode(SwiftNamedEmpty()), emptyJSON, "empty encodes object")
_ = decode(emptyJSON, SwiftNamedEmpty.self)

print("SWIFT-OK")
"#,
        swift_literal(&swift_sorted_json(&SwiftNamedSkipVec { x: vec![], y: 1 })),
        swift_literal(&swift_sorted_json(&SwiftNamedSkipVec { x: vec![1], y: 1 })),
        swift_literal(r#"{}"#),
        swift_literal(r#"{"y":1}"#),
        swift_literal(r#"{}"#),
        "",
    );

    run_swift(&exported, &harness);
}

/// `#[specta(type = ...)]` overrides the exported datatype without changing
/// serde's behaviour. `#[specta(type = u8)] x: Option<u8>` is still an
/// `Option<u8>` to serde: it writes `null` and tolerates an absent key, so the
/// generated Swift must too -- in both the unified shape and the `*Deserialize`
/// half of a phase split. Deriving decode tolerance from the exported datatype
/// nullability alone regressed the `*Deserialize` half to a required key.
#[test]
fn swift_named_override_nullable_matches_serde() {
    assert_eq!(
        serde_json::to_string(&SwiftNamedOverrideNullable { x: None, y: 1 }).unwrap(),
        r#"{"x":null,"y":1}"#
    );
    assert_eq!(
        serde_json::to_string(&SwiftNamedOverrideNullable { x: Some(2), y: 1 }).unwrap(),
        r#"{"x":2,"y":1}"#
    );
    assert!(
        serde_json::from_str::<SwiftNamedOverrideNullable>(r#"{"y":1}"#).is_ok(),
        "serde tolerates the absent key: the Rust type is `Option<u8>`"
    );
    assert_eq!(
        serde_json::from_str::<SwiftNamedOverrideNullable>(r#"{"y":1}"#)
            .unwrap()
            .x,
        None
    );

    let types = Types::default().register::<SwiftNamedOverrideNullable>();

    // Unified: the override must not hide the Rust `Option`, so the property is
    // optional and decoding tolerates an absent key.
    let unified = Swift::default()
        .export(&types, specta_serde::Format)
        .expect("named structs should export");
    assert!(
        unified.contains("public let x: UInt8?"),
        "the override must not hide the Rust `Option`:\n{unified}"
    );
    assert!(
        unified.contains("self.x = try container.decodeIfPresent(UInt8.self, forKey: .x)"),
        "the override must stay absent-tolerant on decode:\n{unified}"
    );

    // Phase-split: the `*Deserialize` half must tolerate the absent key too.
    let phases = Swift::default()
        .export(&types, specta_serde::PhasesFormat)
        .expect("phase-split named structs should export");
    assert!(
        phases.contains("public struct SwiftNamedOverrideNullableDeserialize: Codable, Sendable {"),
        "{phases}"
    );
    assert!(
        phases.contains("self.x = try container.decodeIfPresent(UInt8.self, forKey: .x)"),
        "the deserialize half must stay absent-tolerant:\n{phases}"
    );

    let fixtures = format!(
        "let overrideNullJSON = {}\nlet overrideSomeJSON = {}\nlet overrideMissingJSON = {}\n",
        swift_literal(&swift_sorted_json(&SwiftNamedOverrideNullable {
            x: None,
            y: 1
        })),
        swift_literal(&swift_sorted_json(&SwiftNamedOverrideNullable {
            x: Some(2),
            y: 1,
        })),
        swift_literal(r#"{"y":1}"#),
    );

    let unified_harness = format!(
        r#"{fixtures}
import Foundation

func expect(_ actual: String, _ expected: String, _ label: String) {{
    precondition(actual == expected, "\(label): got \(actual), want \(expected)")
}}

func check(_ condition: Bool, _ label: String) {{
    precondition(condition, label)
}}

func encode<T: Encodable>(_ value: T) -> String {{
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    return String(data: try! encoder.encode(value), encoding: .utf8)!
}}

func decode<T: Decodable>(_ json: String, _: T.Type) -> T {{
    try! JSONDecoder().decode(T.self, from: json.data(using: .utf8)!)
}}

// serde writes `null` for the nil `Option<u8>`, and an absent key still
// decodes to nil.
expect(encode(SwiftNamedOverrideNullable(x: nil, y: 1)), overrideNullJSON, "override nil encodes null")
expect(encode(SwiftNamedOverrideNullable(x: 2, y: 1)), overrideSomeJSON, "override some encodes value")
check(decode(overrideMissingJSON, SwiftNamedOverrideNullable.self).x == nil, "override decode missing")
check(decode(overrideNullJSON, SwiftNamedOverrideNullable.self).x == nil, "override decode null")
check(decode(overrideSomeJSON, SwiftNamedOverrideNullable.self).x == 2, "override decode some")

print("SWIFT-OK")
"#
    );

    let phases_harness = format!(
        r#"{fixtures}
import Foundation

func expect(_ actual: String, _ expected: String, _ label: String) {{
    precondition(actual == expected, "\(label): got \(actual), want \(expected)")
}}

func check(_ condition: Bool, _ label: String) {{
    precondition(condition, label)
}}

func encode<T: Encodable>(_ value: T) -> String {{
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    return String(data: try! encoder.encode(value), encoding: .utf8)!
}}

func decode<T: Decodable>(_ json: String, _: T.Type) -> T {{
    try! JSONDecoder().decode(T.self, from: json.data(using: .utf8)!)
}}

// Both halves keep the `Option<u8>`: the serialize half writes `null`, and the
// deserialize half tolerates the absent key.
expect(encode(SwiftNamedOverrideNullableSerialize(x: nil, y: 1)), overrideNullJSON, "phase override nil encodes null")
expect(encode(SwiftNamedOverrideNullableSerialize(x: 2, y: 1)), overrideSomeJSON, "phase override some encodes value")
check(decode(overrideMissingJSON, SwiftNamedOverrideNullableDeserialize.self).x == nil, "phase override deserialize missing")
check(decode(overrideNullJSON, SwiftNamedOverrideNullableDeserialize.self).x == nil, "phase override deserialize null")
check(decode(overrideSomeJSON, SwiftNamedOverrideNullableDeserialize.self).x == 2, "phase override deserialize some")

print("SWIFT-OK")
"#
    );

    run_swift(&unified, &unified_harness);
    run_swift(&phases, &phases_harness);
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

/// Regression coverage for <https://github.com/specta-rs/specta/issues/616>.
///
/// An internally tagged variant's payload field must use the same absence rules
/// as a named struct. serde tolerates an absent key only for `Option<T>` or a
/// `#[serde(default)]` field, and writes a nil `Option<T>` as `null`. A bare
/// `#[serde(skip_serializing_if = ...)]` omits the key when serializing but
/// still requires it when deserializing.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum SwiftInternalAbsenceSkipIf {
    V {
        #[serde(skip_serializing_if = "Vec::is_empty")]
        x: Vec<u8>,
        y: u8,
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum SwiftInternalAbsenceDefault {
    V {
        #[serde(default)]
        x: Vec<u8>,
        y: u8,
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum SwiftInternalAbsenceOption {
    V { x: Option<u8>, y: u8 },
}

#[test]
fn swift_internal_tag_payload_absence_matches_serde() {
    // Ground truth from serde: a bare `skip_serializing_if` omits on encode but
    // is required on decode.
    assert_eq!(
        serde_json::to_string(&SwiftInternalAbsenceSkipIf::V {
            x: Vec::new(),
            y: 1
        })
        .unwrap(),
        r#"{"t":"V","y":1}"#
    );
    assert!(
        serde_json::to_string(&SwiftInternalAbsenceSkipIf::V { x: vec![1], y: 1 })
            .unwrap()
            .contains(r#""x":[1]"#)
    );
    assert!(
        serde_json::from_str::<SwiftInternalAbsenceSkipIf>(r#"{"t":"V","y":1}"#).is_err(),
        "serde requires the key when deserializing a bare `skip_serializing_if` payload field"
    );
    assert!(
        serde_json::from_str::<SwiftInternalAbsenceSkipIf>(r#"{"t":"V","x":[],"y":1}"#).is_ok()
    );

    // `#[serde(default)]` tolerates the absent key on decode and writes the
    // value on encode.
    assert!(serde_json::from_str::<SwiftInternalAbsenceDefault>(r#"{"t":"V","y":1}"#).is_ok());
    assert_eq!(
        serde_json::to_string(&SwiftInternalAbsenceDefault::V {
            x: Vec::new(),
            y: 1
        })
        .unwrap(),
        r#"{"t":"V","x":[],"y":1}"#
    );

    // A nil `Option<T>` is written as `null`, not omitted.
    assert_eq!(
        serde_json::to_string(&SwiftInternalAbsenceOption::V { x: None, y: 1 }).unwrap(),
        r#"{"t":"V","x":null,"y":1}"#
    );
    assert!(serde_json::from_str::<SwiftInternalAbsenceOption>(r#"{"t":"V","y":1}"#).is_ok());

    let types = Types::default()
        .register::<SwiftInternalAbsenceSkipIf>()
        .register::<SwiftInternalAbsenceDefault>()
        .register::<SwiftInternalAbsenceOption>();
    let exported = Swift::default()
        .export(&types, specta_serde::Format)
        .expect("internally tagged payloads should export");

    assert!(
        exported.contains(r#"try container.decode([UInt8].self, forKey: CodingKeys("x"))"#),
        "a bare `skip_serializing_if` payload field must be required on decode:\n{exported}"
    );
    assert!(
        exported
            .contains(r#"try container.decodeIfPresent([UInt8].self, forKey: CodingKeys("x"))"#),
        "a `#[serde(default)]` payload field must tolerate an absent key:\n{exported}"
    );
    assert!(
        exported.contains(r#"try container.encode(data.x, forKey: CodingKeys("x"))"#),
        "a nil `Option` payload field must be written as `null`, not omitted:\n{exported}"
    );

    let harness = format!(
        r#"let skipIfOmittedJSON = {}
let skipIfPresentJSON = {}
let defaultMissingJSON = {}
let optionNullJSON = {}
import Foundation

func expect(_ actual: String, _ expected: String, _ label: String) {{
    precondition(actual == expected, "\(label): got \(actual), want \(expected)")
}}

func check(_ condition: Bool, _ label: String) {{
    precondition(condition, label)
}}

func encode<T: Encodable>(_ value: T) -> String {{
    let encoder = JSONEncoder()
    encoder.outputFormatting = [.sortedKeys]
    return String(data: try! encoder.encode(value), encoding: .utf8)!
}}

func decode<T: Decodable>(_ json: String, _: T.Type) -> T {{
    try! JSONDecoder().decode(T.self, from: json.data(using: .utf8)!)
}}

func throwsError(_ body: () throws -> Void) -> Bool {{
    do {{
        try body()
        return false
    }} catch {{
        return true
    }}
}}

// A bare `skip_serializing_if` payload field is required on decode, matching
// serde, even though the key is omittable on encode.
check(
    throwsError {{ _ = try JSONDecoder().decode(SwiftInternalAbsenceSkipIf.self, from: skipIfOmittedJSON.data(using: .utf8)!) }},
    "skip-if absent payload key must throw"
)
let skipIfPresent = decode(skipIfPresentJSON, SwiftInternalAbsenceSkipIf.self)
expect(encode(skipIfPresent), skipIfPresentJSON, "skip-if present payload round trip")

// `#[serde(default)]` tolerates the absent key; re-encoding omits a nil
// non-`Option` field, the documented compromise (Swift cannot know the default).
let defaultMissing = decode(defaultMissingJSON, SwiftInternalAbsenceDefault.self)
expect(encode(defaultMissing), defaultMissingJSON, "defaulted payload omits nil element")

// A nil `Option` payload field is written as `null`, not omitted.
let optionNull = decode(optionNullJSON, SwiftInternalAbsenceOption.self)
expect(encode(optionNull), optionNullJSON, "nil Option payload field writes null")

print("SWIFT-OK")
"#,
        serde_json::to_string(r#"{"t":"V","y":1}"#).unwrap(),
        serde_json::to_string(r#"{"t":"V","x":[1],"y":1}"#).unwrap(),
        serde_json::to_string(r#"{"t":"V","y":1}"#).unwrap(),
        serde_json::to_string(r#"{"t":"V","x":null,"y":1}"#).unwrap(),
    );

    run_swift(&exported, &harness);
}
