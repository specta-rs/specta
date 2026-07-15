// Regression tests for internally tagged newtype variants. Serde routes their
// payload through `TaggedSerializer`, whose accepted shapes and enum encoding
// differ from the payload's standalone representation.

use serde::{Deserialize, Serialize};
use specta::{Format as _, Type, Types, datatype::DataType};

#[derive(Debug, PartialEq, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Inner {
    value: i32,
}

#[derive(Debug, PartialEq, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Newtype(Inner);

#[derive(Debug, PartialEq, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Unit;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct HiddenNewtype(#[specta(skip)] i32);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct HiddenNamed {
    #[specta(skip)]
    value: i32,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct HiddenNamedNewtype(HiddenNamed);

#[derive(Debug, PartialEq, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum NewtypePayloads {
    Newtype(Newtype),
    Unit(Unit),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum HiddenNewtypePayload {
    Value(HiddenNewtype),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum DirectHiddenPayload {
    Value(#[specta(skip)] i32),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum HiddenNamedPayload {
    Value(HiddenNamedNewtype),
}

#[derive(Debug, PartialEq, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum GenericPayload<T> {
    Value(T),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum WrappedGenericPayload<T> {
    Value(GenericNewtype<T>),
}

#[derive(Debug, PartialEq, Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExternalPayload {
    Unit,
    Newtype(i32),
    Tuple(i32, i32),
    Struct { value: i32 },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExternalWithoutUnit {
    Newtype(i32),
    Tuple(i32, i32),
    Struct { value: i32 },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum DirectionalExternalUnit {
    #[serde(skip_serializing)]
    Unit,
    Live {
        value: i32,
    },
}

#[derive(Debug, PartialEq, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum ExternalPayloadWrapper {
    Value(ExternalPayload),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum GenericExternalPayload<T> {
    Unit,
    Newtype(T),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum ConcreteExternalPayloadWrapper {
    Value(GenericExternalPayload<i32>),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum InlineGenericExternalPayloadWrapper {
    Value(#[specta(inline)] GenericExternalPayload<i32>),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct GenericNewtype<T>(T);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum GenericNewtypeExternalWrapper {
    Value(GenericNewtype<ExternalPayload>),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum NestedGenericNewtypeExternalWrapper {
    Value(GenericNewtype<GenericNewtype<ExternalPayload>>),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExternalWithSerdeAttrs {
    Renamed {
        #[serde(rename = "wire_value")]
        value: i32,
    },
    Skipped(#[serde(skip)] i32),
    #[serde(untagged)]
    Raw {
        raw_value: i32,
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum ExternalWithSerdeAttrsWrapper {
    Value(ExternalWithSerdeAttrs),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExternalWithSkippedFlatten {
    #[serde(skip)]
    Dead {
        #[serde(flatten)]
        inner: Inner,
    },
    Live {
        value: i32,
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum ExternalWithSkippedFlattenWrapper {
    Value(ExternalWithSkippedFlatten),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExternalFlattenOnly {
    Struct {
        #[serde(flatten)]
        inner: Inner,
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum ExternalFlattenOnlyWrapper {
    Value(ExternalFlattenOnly),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExternalWithHiddenPayload {
    Value(#[specta(skip)] i32),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum ExternalWithHiddenPayloadWrapper {
    Value(ExternalWithHiddenPayload),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum InlineExternalPayloadWrapper {
    Value(#[specta(inline)] ExternalPayload),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExternalWithUntaggedUnitInline {
    Known {
        value: i32,
    },
    #[serde(untagged)]
    Unit,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum InlineExternalWithUntaggedUnitWrapper {
    Value(#[specta(inline)] ExternalWithUntaggedUnitInline),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
enum ExternalWithInvalidUntagged {
    #[serde(untagged)]
    Scalar(i32),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
enum ExternalWithUntaggedUnit {
    #[serde(untagged)]
    Unit,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
enum ExternalWithUntaggedEmptyStruct {
    #[serde(untagged)]
    Empty {},
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum UntaggedDirectionalMap {
    #[serde(untagged)]
    Value(#[serde(skip_serializing)] Inner),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedExternalPayload {
    External(ExternalPayload),
    Struct { raw: i32 },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum UntaggedExternalPayloadWrapper {
    Value(UntaggedExternalPayload),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExternalWithOther {
    Known,
    #[serde(other)]
    Other,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum InlineExternalWithOtherWrapper {
    Value(#[specta(inline)] ExternalWithOther),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum InvalidUntaggedExternalWrapper {
    Value(ExternalWithInvalidUntagged),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct InvalidTuple(i32, i32);

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct InvalidEmptyTuple();

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false, transparent = false)]
#[serde(transparent)]
struct TransparentWithSkippedSibling(Inner, #[serde(skip)] u8);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false, transparent = false)]
#[serde(transparent)]
struct NamedTransparentUnit {
    inner: Unit,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false, transparent = false)]
#[serde(transparent)]
struct NamedTransparentExternal {
    inner: ExternalPayload,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum TransparentWithSkippedSiblingWrapper {
    Value(TransparentWithSkippedSibling),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum InvalidTuplePayloads {
    Tuple(InvalidTuple),
    Empty(InvalidEmptyTuple),
}

#[test]
fn serde_accepts_newtype_and_unit_struct_payloads() {
    let newtype = NewtypePayloads::Newtype(Newtype(Inner { value: 1 }));
    assert_eq!(
        serde_json::to_value(&newtype).unwrap(),
        serde_json::json!({ "kind": "Newtype", "value": 1 })
    );
    assert_eq!(
        serde_json::from_value::<NewtypePayloads>(serde_json::to_value(&newtype).unwrap()).unwrap(),
        newtype
    );

    let unit = NewtypePayloads::Unit(Unit);
    assert_eq!(
        serde_json::to_value(&unit).unwrap(),
        serde_json::json!({ "kind": "Unit" })
    );
    assert_eq!(
        serde_json::from_value::<NewtypePayloads>(serde_json::to_value(&unit).unwrap()).unwrap(),
        unit
    );

    assert_eq!(
        serde_json::to_value(TransparentWithSkippedSiblingWrapper::Value(
            TransparentWithSkippedSibling(Inner { value: 1 }, 0)
        ))
        .unwrap(),
        serde_json::json!({ "kind": "Value", "value": 1 })
    );
}

#[test]
fn format_accepts_newtype_unit_and_generic_payloads() {
    specta_serde::Format
        .map_types(&Types::default().register::<NewtypePayloads>())
        .expect("newtype and unit structs are valid internally tagged payloads");
    specta_serde::Format
        .map_types(&Types::default().register::<TransparentWithSkippedSiblingWrapper>())
        .expect("transparent tuple structs delegate through their sole live field");
    specta_serde::Format
        .map_types(&Types::default().register::<GenericPayload<Inner>>())
        .expect("generic payloads must be validated at their concrete use site");
    specta_serde::Format
        .map_types(&Types::default().register::<GenericPayload<ExternalWithoutUnit>>())
        .expect("external payloads without unit-like variants need no contextual rewrite");
    specta_serde::Format
        .map_types(&Types::default().register::<GenericPayload<ExternalWithUntaggedEmptyStruct>>())
        .expect("an untagged empty object already merges with an internal tag correctly");

    let err = specta_serde::Format
        .map_types(&Types::default().register::<GenericPayload<i32>>())
        .expect_err("a concrete scalar generic payload is invalid for TaggedSerializer");
    assert!(
        err.to_string().contains("payload cannot be merged"),
        "unexpected error: {err}"
    );

    for types in [
        Types::default().register::<GenericPayload<Unit>>(),
        Types::default().register::<GenericPayload<()>>(),
        Types::default().register::<GenericPayload<NamedTransparentUnit>>(),
        Types::default().register::<GenericPayload<NamedTransparentExternal>>(),
    ] {
        let err = specta_serde::Format
            .map_types(&types)
            .expect_err("a standalone null-like generic argument cannot be intersected with a tag");
        assert!(
            err.to_string().contains("context-sensitive enum encoding"),
            "unexpected error: {err}"
        );
    }

    let err = specta_serde::Format
        .map_types(&Types::default().register::<GenericPayload<ExternalPayload>>())
        .expect_err("a generic definition cannot contextually re-encode a concrete enum argument");
    assert!(
        err.to_string().contains("context-sensitive enum encoding"),
        "unexpected error: {err}"
    );

    let err = specta_serde::Format
        .map_types(&Types::default().register::<WrappedGenericPayload<ExternalPayload>>())
        .expect_err("a generic wrapper must not hide context-sensitive enum encoding");
    assert!(
        err.to_string().contains("context-sensitive enum encoding"),
        "unexpected error: {err}"
    );

    let err = specta_serde::Format
        .map_types(&Types::default().register::<GenericPayload<ExternalWithUntaggedUnit>>())
        .expect_err("an untagged unit contributes no payload under TaggedSerializer");
    assert!(
        err.to_string().contains("context-sensitive enum encoding"),
        "unexpected error: {err}"
    );

    let err = specta_serde::PhasesFormat
        .map_types(&Types::default().register::<GenericPayload<DirectionalExternalUnit>>())
        .expect_err("deserialize-only unit variants still need contextual generic encoding");
    assert!(
        err.to_string().contains("context-sensitive enum encoding"),
        "unexpected error: {err}"
    );

    let err = specta_serde::PhasesFormat
        .map_types(&Types::default().register::<GenericPayload<UntaggedDirectionalMap>>())
        .expect_err("a phase-skipped untagged newtype becomes a contextual unit contribution");
    assert!(
        err.to_string().contains("context-sensitive enum encoding"),
        "unexpected error: {err}"
    );
}

#[test]
fn tagged_serializer_uses_map_encoding_for_external_enums() {
    let cases = [
        (
            ExternalPayloadWrapper::Value(ExternalPayload::Unit),
            serde_json::json!({ "kind": "Value", "Unit": null }),
        ),
        (
            ExternalPayloadWrapper::Value(ExternalPayload::Newtype(1)),
            serde_json::json!({ "kind": "Value", "Newtype": 1 }),
        ),
        (
            ExternalPayloadWrapper::Value(ExternalPayload::Tuple(1, 2)),
            serde_json::json!({ "kind": "Value", "Tuple": [1, 2] }),
        ),
        (
            ExternalPayloadWrapper::Value(ExternalPayload::Struct { value: 1 }),
            serde_json::json!({ "kind": "Value", "Struct": { "value": 1 } }),
        ),
    ];

    for (value, expected) in cases {
        assert_eq!(serde_json::to_value(&value).unwrap(), expected);
        assert_eq!(
            serde_json::from_value::<ExternalPayloadWrapper>(expected).unwrap(),
            value
        );
    }
}

#[test]
fn format_inlines_external_enum_tagged_serializer_shape() {
    let types = Types::default().register::<ExternalPayloadWrapper>();
    let mapped = specta_serde::Format
        .map_types(&types)
        .expect("external enum payload is supported by TaggedSerializer")
        .into_owned();
    let wrapper = mapped
        .into_unsorted_iter()
        .find(|ndt| ndt.name == "ExternalPayloadWrapper")
        .and_then(|ndt| ndt.ty.as_ref())
        .expect("mapped wrapper definition should exist");

    assert!(
        contains_named_field(wrapper, "Unit", |ty| {
            matches!(ty, DataType::Tuple(tuple) if tuple.elements.is_empty())
        }),
        "the contextual external unit variant must become `Unit: null`: {wrapper:#?}"
    );
    assert!(
        contains_named_field(wrapper, "Tuple", |ty| {
            matches!(ty, DataType::Tuple(tuple) if tuple.elements.len() == 2)
        }),
        "tuple variants remain map entries with tuple values: {wrapper:#?}"
    );
}

#[test]
fn contextual_external_enum_shape_substitutes_concrete_generics() {
    for (name, types) in [
        (
            "ConcreteExternalPayloadWrapper",
            Types::default().register::<ConcreteExternalPayloadWrapper>(),
        ),
        (
            "InlineGenericExternalPayloadWrapper",
            Types::default().register::<InlineGenericExternalPayloadWrapper>(),
        ),
    ] {
        let mapped = specta_serde::Format
            .map_types(&types)
            .expect("concrete generic external payload should remain concrete")
            .into_owned();
        let wrapper = mapped
            .into_unsorted_iter()
            .find(|ndt| ndt.name == name)
            .and_then(|ndt| ndt.ty.as_ref())
            .expect("mapped wrapper definition should exist");

        assert!(
            contains_named_field(wrapper, "Newtype", |ty| {
                matches!(ty, DataType::Primitive(specta::datatype::Primitive::i32))
            }),
            "the concrete generic argument must replace the inner placeholder: {wrapper:#?}"
        );
    }
}

#[test]
fn generic_newtype_wrapper_does_not_hide_external_enum_shape() {
    for (name, types) in [
        (
            "GenericNewtypeExternalWrapper",
            Types::default().register::<GenericNewtypeExternalWrapper>(),
        ),
        (
            "NestedGenericNewtypeExternalWrapper",
            Types::default().register::<NestedGenericNewtypeExternalWrapper>(),
        ),
    ] {
        let mapped = specta_serde::Format
            .map_types(&types)
            .expect("generic newtype wrappers delegate into TaggedSerializer")
            .into_owned();
        let wrapper = mapped
            .into_unsorted_iter()
            .find(|ndt| ndt.name == name)
            .and_then(|ndt| ndt.ty.as_ref())
            .expect("mapped wrapper definition should exist");

        assert!(
            contains_named_field(wrapper, "Unit", |ty| {
                matches!(ty, DataType::Tuple(tuple) if tuple.elements.is_empty())
            }),
            "the wrapper must expose the contextual `Unit: null` entry: {wrapper:#?}"
        );
    }
}

#[test]
fn contextual_external_enum_applies_serde_field_and_variant_semantics() {
    assert_eq!(
        serde_json::to_value(ExternalWithSerdeAttrsWrapper::Value(
            ExternalWithSerdeAttrs::Renamed { value: 1 }
        ))
        .unwrap(),
        serde_json::json!({ "kind": "Value", "Renamed": { "wire_value": 1 } })
    );
    assert_eq!(
        serde_json::to_value(ExternalWithSerdeAttrsWrapper::Value(
            ExternalWithSerdeAttrs::Skipped(1)
        ))
        .unwrap(),
        serde_json::json!({ "kind": "Value", "Skipped": null })
    );
    assert_eq!(
        serde_json::to_value(ExternalWithSerdeAttrsWrapper::Value(
            ExternalWithSerdeAttrs::Raw { raw_value: 1 }
        ))
        .unwrap(),
        serde_json::json!({ "kind": "Value", "raw_value": 1 })
    );

    let types = Types::default().register::<ExternalWithSerdeAttrsWrapper>();
    let mapped = specta_serde::Format
        .map_types(&types)
        .expect("contextual lowering should apply serde field and variant attributes")
        .into_owned();
    let wrapper = mapped
        .into_unsorted_iter()
        .find(|ndt| ndt.name == "ExternalWithSerdeAttrsWrapper")
        .and_then(|ndt| ndt.ty.as_ref())
        .expect("mapped wrapper definition should exist");

    assert!(contains_named_field(wrapper, "wire_value", |ty| matches!(
        ty,
        DataType::Primitive(specta::datatype::Primitive::i32)
    )));
    assert!(contains_named_field(wrapper, "Skipped", |ty| matches!(
        ty,
        DataType::Tuple(tuple) if tuple.elements.is_empty()
    )));
    assert!(contains_named_field(wrapper, "raw_value", |ty| matches!(
        ty,
        DataType::Primitive(specta::datatype::Primitive::i32)
    )));
}

#[test]
fn skipped_variant_does_not_trigger_contextual_flatten_error() {
    let types = Types::default().register::<ExternalWithSkippedFlattenWrapper>();
    let mapped = specta_serde::Format
        .map_types(&types)
        .expect("attributes on skipped variants are not part of the wire shape")
        .into_owned();
    let mapped = mapped.into_unsorted_iter().collect::<Vec<_>>();
    let payload = mapped
        .iter()
        .find(|ndt| ndt.name == "ExternalWithSkippedFlatten")
        .and_then(|ndt| ndt.ty.as_ref())
        .expect("mapped payload definition should exist");

    assert!(contains_named_field(payload, "value", |ty| matches!(
        ty,
        DataType::Primitive(specta::datatype::Primitive::i32)
    )));
    assert!(!contains_named_field(payload, "Dead", |_| true));
}

#[test]
fn exact_external_map_payload_bypasses_contextual_rebuilding() {
    assert_eq!(
        serde_json::to_value(ExternalFlattenOnlyWrapper::Value(
            ExternalFlattenOnly::Struct {
                inner: Inner { value: 1 },
            }
        ))
        .unwrap(),
        serde_json::json!({ "kind": "Value", "Struct": { "value": 1 } })
    );
    specta_serde::Format
        .map_types(&Types::default().register::<ExternalFlattenOnlyWrapper>())
        .expect("an external enum with only map variants already has the contextual wire shape");
}

#[test]
fn specta_hidden_newtype_payloads_are_rejected() {
    for types in [
        Types::default().register::<HiddenNewtypePayload>(),
        Types::default().register::<DirectHiddenPayload>(),
        Types::default().register::<HiddenNamedPayload>(),
        Types::default().register::<ExternalWithHiddenPayloadWrapper>(),
    ] {
        assert!(
            specta_serde::Format.map_types(&types).is_err(),
            "serde still transports the payload hidden from Specta, so its shape is unknown"
        );
    }
}

#[test]
fn inline_external_enum_keeps_tagged_serializer_shape() {
    let types = Types::default().register::<InlineExternalPayloadWrapper>();
    let mapped = specta_serde::Format
        .map_types(&types)
        .expect("inline external enums use the same contextual representation")
        .into_owned();
    let wrapper = mapped
        .into_unsorted_iter()
        .find(|ndt| ndt.name == "InlineExternalPayloadWrapper")
        .and_then(|ndt| ndt.ty.as_ref())
        .expect("mapped wrapper definition should exist");

    assert!(contains_named_field(wrapper, "Unit", |ty| matches!(
        ty,
        DataType::Tuple(tuple) if tuple.elements.is_empty()
    )));
}

#[test]
fn inline_external_untagged_unit_contributes_an_empty_object() {
    assert_eq!(
        serde_json::to_value(InlineExternalWithUntaggedUnitWrapper::Value(
            ExternalWithUntaggedUnitInline::Unit
        ))
        .unwrap(),
        serde_json::json!({ "kind": "Value" })
    );

    let mapped = specta_serde::Format
        .map_types(&Types::default().register::<InlineExternalWithUntaggedUnitWrapper>())
        .expect("an untagged unit contributes no fields under TaggedSerializer")
        .into_owned();
    let wrapper = mapped
        .into_unsorted_iter()
        .find(|ndt| ndt.name == "InlineExternalWithUntaggedUnitWrapper")
        .and_then(|ndt| ndt.ty.as_ref())
        .expect("mapped wrapper definition should exist");

    assert!(contains_empty_object(wrapper));
}

#[test]
fn nested_untagged_enum_propagates_contextual_external_shape() {
    let mapped = specta_serde::Format
        .map_types(&Types::default().register::<UntaggedExternalPayloadWrapper>())
        .expect("nested untagged payloads propagate contextual replacements")
        .into_owned();
    let wrapper = mapped
        .into_unsorted_iter()
        .find(|ndt| ndt.name == "UntaggedExternalPayloadWrapper")
        .and_then(|ndt| ndt.ty.as_ref())
        .expect("mapped wrapper definition should exist");

    assert!(contains_named_field(wrapper, "Unit", |ty| matches!(
        ty,
        DataType::Tuple(tuple) if tuple.elements.is_empty()
    )));
}

#[test]
fn inline_external_other_is_rejected_in_contextual_deserialize_shape() {
    assert!(
        specta_serde::PhasesFormat
            .map_types(&Types::default().register::<InlineExternalWithOtherWrapper>())
            .is_err(),
        "a widened unknown map key cannot be represented without constraining the outer tag"
    );
}

#[test]
fn contextual_untagged_scalar_payload_is_rejected() {
    assert!(
        serde_json::to_value(InvalidUntaggedExternalWrapper::Value(
            ExternalWithInvalidUntagged::Scalar(1)
        ))
        .is_err()
    );

    let err = specta_serde::Format
        .map_types(&Types::default().register::<InvalidUntaggedExternalWrapper>())
        .expect_err("TaggedSerializer rejects scalar untagged payloads");
    assert!(
        err.to_string().contains("cannot be merged"),
        "unexpected error: {err}"
    );
}

#[test]
fn direct_tuple_struct_payloads_remain_invalid() {
    assert!(serde_json::to_value(InvalidTuplePayloads::Tuple(InvalidTuple(1, 2))).is_err());
    assert!(serde_json::to_value(InvalidTuplePayloads::Empty(InvalidEmptyTuple())).is_err());

    let err = specta_serde::Format
        .map_types(&Types::default().register::<InvalidTuplePayloads>())
        .expect_err("TaggedSerializer rejects tuple struct payloads");
    assert!(
        err.to_string().contains("payload cannot be merged"),
        "unexpected error: {err}"
    );
}

fn contains_named_field(
    ty: &DataType,
    expected_name: &str,
    expected_ty: impl Copy + Fn(&DataType) -> bool,
) -> bool {
    match ty {
        DataType::Struct(strct) => match &strct.fields {
            specta::datatype::Fields::Named(fields) => fields.fields.iter().any(|(name, field)| {
                (name == expected_name && field.ty.as_ref().is_some_and(expected_ty))
                    || field
                        .ty
                        .as_ref()
                        .is_some_and(|ty| contains_named_field(ty, expected_name, expected_ty))
            }),
            specta::datatype::Fields::Unnamed(fields) => fields.fields.iter().any(|field| {
                field
                    .ty
                    .as_ref()
                    .is_some_and(|ty| contains_named_field(ty, expected_name, expected_ty))
            }),
            specta::datatype::Fields::Unit => false,
        },
        DataType::Enum(enm) => enm
            .variants
            .iter()
            .any(|(_, variant)| match &variant.fields {
                specta::datatype::Fields::Named(fields) => {
                    fields.fields.iter().any(|(name, field)| {
                        (name == expected_name && field.ty.as_ref().is_some_and(expected_ty))
                            || field.ty.as_ref().is_some_and(|ty| {
                                contains_named_field(ty, expected_name, expected_ty)
                            })
                    })
                }
                specta::datatype::Fields::Unnamed(fields) => fields.fields.iter().any(|field| {
                    field
                        .ty
                        .as_ref()
                        .is_some_and(|ty| contains_named_field(ty, expected_name, expected_ty))
                }),
                specta::datatype::Fields::Unit => false,
            }),
        DataType::Intersection(parts) => parts
            .iter()
            .any(|ty| contains_named_field(ty, expected_name, expected_ty)),
        DataType::Nullable(ty) => contains_named_field(ty, expected_name, expected_ty),
        DataType::Tuple(tuple) => tuple
            .elements
            .iter()
            .any(|ty| contains_named_field(ty, expected_name, expected_ty)),
        DataType::List(list) => contains_named_field(&list.ty, expected_name, expected_ty),
        DataType::Map(map) => {
            contains_named_field(map.key_ty(), expected_name, expected_ty)
                || contains_named_field(map.value_ty(), expected_name, expected_ty)
        }
        DataType::Primitive(_) | DataType::Reference(_) | DataType::Generic(_) => false,
    }
}

fn contains_empty_object(ty: &DataType) -> bool {
    match ty {
        DataType::Struct(strct) => match &strct.fields {
            specta::datatype::Fields::Named(fields) => {
                fields.fields.is_empty()
                    || fields
                        .fields
                        .iter()
                        .any(|(_, field)| field.ty.as_ref().is_some_and(contains_empty_object))
            }
            specta::datatype::Fields::Unnamed(fields) => fields
                .fields
                .iter()
                .any(|field| field.ty.as_ref().is_some_and(contains_empty_object)),
            specta::datatype::Fields::Unit => false,
        },
        DataType::Enum(enm) => enm
            .variants
            .iter()
            .any(|(_, variant)| match &variant.fields {
                specta::datatype::Fields::Named(fields) => fields
                    .fields
                    .iter()
                    .any(|(_, field)| field.ty.as_ref().is_some_and(contains_empty_object)),
                specta::datatype::Fields::Unnamed(fields) => fields
                    .fields
                    .iter()
                    .any(|field| field.ty.as_ref().is_some_and(contains_empty_object)),
                specta::datatype::Fields::Unit => false,
            }),
        DataType::Intersection(parts)
        | DataType::Tuple(specta::datatype::Tuple {
            elements: parts, ..
        }) => parts.iter().any(contains_empty_object),
        DataType::Nullable(ty) => contains_empty_object(ty),
        DataType::List(list) => contains_empty_object(&list.ty),
        DataType::Map(map) => {
            contains_empty_object(map.key_ty()) || contains_empty_object(map.value_ty())
        }
        DataType::Primitive(_) | DataType::Reference(_) | DataType::Generic(_) => false,
    }
}
