// Regression coverage for three validation gaps in `specta-serde`'s
// `validate.rs`:
//
// 1. `#[serde(with/serialize_with/deserialize_with)]` on an *unnamed* (tuple)
//    struct field was never checked against the `#[specta(type = ...)]`
//    override guard that named fields already get, so a codec that changes
//    the wire type silently exported the wrong shape.
// 2. `#[serde(flatten)]` targets were never validated. serde only allows
//    flattening maps and struct-like values (any tag representation of an
//    enum, or an `Option` of one) - flattening a sequence, tuple, or scalar
//    is a *runtime* `serde_json` error ("can only flatten structs and maps").
// 3. A nested untagged enum reached through an internally tagged enum's
//    payload reused the *outer* enum's path when reporting a validation
//    error, naming a variant that doesn't exist on the outer enum.

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;
use std::collections::HashMap;

// --- Gap 1: codec attributes on unnamed (tuple) struct fields ---

mod codec {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &u32, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.collect_str(value)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<u32, D::Error>
    where
        D: Deserializer<'de>,
    {
        String::deserialize(deserializer)?
            .parse()
            .map_err(serde::de::Error::custom)
    }
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct TupleCodecNoOverride(#[serde(with = "codec")] u32);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct TupleCodecWithOverride(
    #[specta(type = String)]
    #[serde(with = "codec")]
    u32,
);

#[test]
fn tuple_struct_codec_without_override_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<TupleCodecNoOverride>(),
            specta_serde::Format,
        )
        .expect_err(
            "a codec on a tuple-struct field without #[specta(type = ...)] should be rejected, \
             just like it already is on named-struct and enum-variant fields",
        );

    assert!(
        err.to_string().contains("Unsupported serde attribute"),
        "unexpected error: {err}"
    );
}

#[test]
fn tuple_struct_codec_with_override_is_accepted_and_exports_override_type() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<TupleCodecWithOverride>(),
            specta_serde::Format,
        )
        .expect("#[specta(type = ...)] override should satisfy the codec guard");

    assert!(
        ts.contains("string"),
        "expected the override type to be exported, got: {ts}"
    );
}

// --- Gap 2: `#[serde(flatten)]` target validation ---

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenVecInvalid {
    a: i32,
    #[serde(flatten)]
    v: Vec<u8>,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenTupleInvalid {
    a: i32,
    #[serde(flatten)]
    v: (u8, u8),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenPrimitiveInvalid {
    a: i32,
    #[serde(flatten)]
    v: u32,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct TupleStructPayload(i32, i32);

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenTupleStructInvalid {
    a: i32,
    #[serde(flatten)]
    v: TupleStructPayload,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct Inner {
    b: i32,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenStructValid {
    a: i32,
    #[serde(flatten)]
    v: Inner,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenOptionStructValid {
    a: i32,
    #[serde(flatten)]
    v: Option<Inner>,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenMapValid {
    a: i32,
    #[serde(flatten)]
    v: HashMap<String, String>,
}

// serde_json evidence (verified with a scratch crate against serde 1.0.228 /
// serde_json 1.0.150): external, internal, and adjacent tagging always
// flatten fine, merging tag/content fields straight into the surrounding map
// (an externally tagged unit variant flattens as `"Variant": null`). An
// untagged enum only flattens when the active variant's payload is
// map-shaped - a variant wrapping e.g. a `u32` fails at runtime - but
// rejecting untagged enums statically would break flattening
// `serde_json::Value`, so they are deliberately accepted too (see
// `flatten_of_json_value_and_json_map_is_accepted` below).
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedPayload {
    A {
        x: i32,
    },
    #[allow(dead_code)]
    B {
        y: i32,
    },
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenUntaggedEnumValid {
    a: i32,
    #[serde(flatten)]
    v: UntaggedPayload,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
enum ExternalPayload {
    A { x: i32 },
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenExternalEnumValid {
    a: i32,
    #[serde(flatten)]
    v: ExternalPayload,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum InternalPayload {
    A { x: i32 },
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenInternalEnumValid {
    a: i32,
    #[serde(flatten)]
    v: InternalPayload,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjacentPayload {
    A { x: i32 },
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenAdjacentEnumValid {
    a: i32,
    #[serde(flatten)]
    v: AdjacentPayload,
}

#[test]
fn flatten_of_vec_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenVecInvalid>(),
            specta_serde::Format,
        )
        .expect_err(
            "serde_json fails at runtime with \"can only flatten structs and maps (got a sequence)\"",
        );

    let msg = err.to_string();
    assert!(msg.contains("flatten"), "unexpected error: {msg}");
    assert!(msg.contains(".v"), "error should point at the field: {msg}");
}

#[test]
fn flatten_of_tuple_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenTupleInvalid>(),
            specta_serde::Format,
        )
        .expect_err(
            "serde_json fails at runtime with \"can only flatten structs and maps (got a tuple)\"",
        );

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

#[test]
fn flatten_of_primitive_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenPrimitiveInvalid>(),
            specta_serde::Format,
        )
        .expect_err(
            "serde_json fails at runtime with \"can only flatten structs and maps (got an integer)\"",
        );

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

#[test]
fn flatten_of_tuple_struct_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenTupleStructInvalid>(),
            specta_serde::Format,
        )
        .expect_err(
            "serde_json fails at runtime with \"can only flatten structs and maps (got a tuple struct)\"",
        );

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

#[test]
fn flatten_of_named_struct_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenStructValid>(),
            specta_serde::Format,
        )
        .expect("flattening a named-field struct is valid serde usage");
}

#[test]
fn flatten_of_option_struct_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenOptionStructValid>(),
            specta_serde::Format,
        )
        .expect("flattening Option<Struct> is valid serde usage (None contributes nothing)");
}

#[test]
fn flatten_of_map_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenMapValid>(),
            specta_serde::Format,
        )
        .expect("flattening a map is valid serde usage");
}

#[test]
fn flatten_of_untagged_enum_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenUntaggedEnumValid>(),
            specta_serde::Format,
        )
        .expect("flattening an untagged enum is valid serde usage");
}

#[test]
fn flatten_of_externally_tagged_enum_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenExternalEnumValid>(),
            specta_serde::Format,
        )
        .expect("flattening an externally tagged enum is valid serde usage");
}

#[test]
fn flatten_of_internally_tagged_enum_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenInternalEnumValid>(),
            specta_serde::Format,
        )
        .expect("flattening an internally tagged enum is valid serde usage");
}

#[test]
fn flatten_of_adjacently_tagged_enum_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenAdjacentEnumValid>(),
            specta_serde::Format,
        )
        .expect("flattening an adjacently tagged enum is valid serde usage");
}

// --- Gap 2 continued: adversarial edge cases ---

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenOptionVecInvalid {
    a: i32,
    #[serde(flatten)]
    v: Option<Vec<u8>>,
}

#[test]
fn flatten_of_option_vec_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenOptionVecInvalid>(),
            specta_serde::Format,
        )
        .expect_err(
            "serde_json fails at runtime with \"can only flatten structs and maps (got a \
             sequence)\" whenever the option is Some",
        );

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

// A field that never hits the wire can't cause a flatten error, however
// non-flattenable its type is. serde accepts `#[serde(flatten, skip)]` and
// fully omits the field in both directions (ground-truthed: serializes as
// `{"a":1}`, deserializes from it). Full `#[serde(skip)]` erases the field's
// specta type entirely, so it never reaches flatten validation; the pair of
// one-sided skips keeps the type but is just as dead at runtime.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenSkippedVec {
    a: i32,
    #[serde(flatten, skip)]
    v: Vec<u8>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenBothSidedSkipsVec {
    a: i32,
    #[serde(flatten, skip_serializing, skip_deserializing, default)]
    v: Vec<u8>,
}

#[test]
fn serde_json_confirms_skipped_flatten_fields_are_dead() {
    assert_eq!(
        serde_json::to_string(&FlattenSkippedVec { a: 1, v: vec![1] }).unwrap(),
        r#"{"a":1}"#
    );
    assert_eq!(
        serde_json::to_string(&FlattenBothSidedSkipsVec { a: 1, v: vec![1] }).unwrap(),
        r#"{"a":1}"#
    );
    let _: FlattenSkippedVec = serde_json::from_str(r#"{"a":1}"#).unwrap();
    let _: FlattenBothSidedSkipsVec = serde_json::from_str(r#"{"a":1}"#).unwrap();
}

#[test]
fn flatten_of_fully_skipped_vec_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenSkippedVec>(),
            specta_serde::Format,
        )
        .expect("a #[serde(flatten, skip)] field never hits the wire, so its shape is irrelevant");
}

#[test]
fn flatten_of_both_sided_skipped_vec_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenBothSidedSkipsVec>(),
            specta_serde::Format,
        )
        .expect(
            "skip_serializing + skip_deserializing together omit the field in both directions, \
             so its shape is irrelevant",
        );
}

// A one-sided skip is different: the *other* direction still flattens, and
// serde_json fails at runtime there (ground-truthed: deserializing
// `{"a":1}` into a `#[serde(flatten, skip_serializing, default)] Vec<u8>`
// errors with "can only flatten structs and maps").
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenSkipSerializingVecInvalid {
    a: i32,
    #[serde(flatten, skip_serializing, default)]
    v: Vec<u8>,
}

#[test]
fn flatten_of_one_side_skipped_vec_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenSkipSerializingVecInvalid>(),
            specta_serde::PhasesFormat,
        )
        .expect_err("the deserialize direction still flattens the Vec and fails at runtime");

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

/// Renders `T`'s inline (non-reference) datatype through `map_type`, the
/// per-phase entry exporters use for inline/function datatypes. For
/// `PhasesFormat` this renders the *serialize* shape only.
fn phases_map_type_inline<T: Type>() -> Result<(), specta::FormatError> {
    use specta::Format as _;
    use specta::datatype::{DataType, Reference};

    let mut types = Types::default();
    let dt = T::definition(&mut types);
    let DataType::Reference(Reference::Named(reference)) = &dt else {
        panic!("expected a named reference");
    };
    let ty = types
        .get(reference)
        .and_then(|ndt| ndt.ty.clone())
        .expect("definition should be registered");

    // Wrap the struct so the datatype is anonymous rather than structurally
    // equal to the registered definition (which `map_type` intentionally
    // skips - registered definitions are validated by `map_types`). This
    // models an inline/function datatype containing the struct's shape.
    specta_serde::PhasesFormat
        .map_type(&types, &DataType::Nullable(Box::new(ty)))
        .map(|_| ())
}

#[test]
fn phases_map_type_serialize_phase_ignores_deserialize_only_flatten() {
    // `PhasesFormat::map_type` selects and rewrites the *serialize* phase
    // only; a `#[serde(flatten, skip_serializing)]` field is dropped by that
    // rewrite, so its deserialize-only Vec shape is unreachable in this
    // rendering and must not be rejected. (The whole-graph export still
    // rejects the construct - `flatten_of_one_side_skipped_vec_is_rejected`
    // above - because the deserialize phase is rendered there too.)
    phases_map_type_inline::<FlattenSkipSerializingVecInvalid>()
        .expect("the serialize-phase mapping never renders the deserialize-only flattened Vec");
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenSkipDeserializingVecInvalid {
    a: i32,
    #[serde(flatten, skip_deserializing, default)]
    v: Vec<u8>,
}

#[test]
fn phases_map_type_serialize_phase_still_rejects_live_flatten() {
    // Control: with the skip on the *deserialize* side, the Vec is live in
    // the serialize phase that `map_type` renders, so it must still error.
    let err = phases_map_type_inline::<FlattenSkipDeserializingVecInvalid>()
        .expect_err("the flattened Vec is live in the serialize phase");

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

#[test]
fn flatten_of_one_side_skipped_vec_under_unified_format_errors_on_the_skip() {
    // Since #518, unified `Format` rejects one-sided skips outright instead
    // of guessing a shape. The flatten validator must stay out of the way:
    // the field never flattens in unified mode (any skip kills it there), so
    // the error must be the directional-skip incompatibility, not a bogus
    // `InvalidFlattenTarget` fired first for an unreachable shape.
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenSkipSerializingVecInvalid>(),
            specta_serde::Format,
        )
        .expect_err("unified mode rejects one-sided skips since #518");

    let msg = err.to_string();
    assert!(
        msg.contains("Incompatible field skip"),
        "expected the #518 directional-skip error: {msg}"
    );
    assert!(
        !msg.contains("flatten"),
        "flatten validation must not fire for a direction that never flattens: {msg}"
    );
}

// A one-sided skip also means only the *live* phase's shape matters for a
// `specta_serde::Phased` override: `PhasesFormat` removes the field from the
// skipped phase before flatten lowering, so a sequence shape on the skipped
// side is unreachable.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenPhasedSkippedSequenceSideValid {
    a: i32,
    #[serde(flatten, skip_serializing, default)]
    #[specta(type = specta_serde::Phased<Vec<String>, Inner>)]
    v: HashMap<String, String>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenPhasedLiveSequenceSideInvalid {
    a: i32,
    #[serde(flatten, skip_serializing, default)]
    #[specta(type = specta_serde::Phased<Inner, Vec<String>>)]
    v: HashMap<String, String>,
}

#[test]
fn flatten_of_phased_override_with_skipped_sequence_phase_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenPhasedSkippedSequenceSideValid>(),
            specta_serde::PhasesFormat,
        )
        .expect(
            "the Vec is the serialize shape, but serialization is skipped, so only the \
             struct-shaped deserialize side ever flattens",
        );
}

#[test]
fn flatten_of_phased_override_with_live_sequence_phase_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenPhasedLiveSequenceSideInvalid>(),
            specta_serde::PhasesFormat,
        )
        .expect_err("the live deserialize side is the Vec, which still flattens and fails");

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

// Same principle for container conversions: a skipped direction's wire shape
// (or its raw fallback) is unreachable. `IntoOnlyId` has a serialize-only
// conversion to a struct wire; with deserialization skipped, the raw u32
// newtype never deserializes, so the flatten is valid (ground-truthed below).
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenIntoOnlySkipDeserializeValid {
    a: i32,
    #[serde(flatten, skip_deserializing, default)]
    v: IntoOnlyId,
}

impl Default for IntoOnlyId {
    fn default() -> Self {
        Self(0)
    }
}

#[test]
fn serde_json_confirms_into_only_with_skipped_deserialize_flattens_fine() {
    assert_eq!(
        serde_json::to_string(&FlattenIntoOnlySkipDeserializeValid {
            a: 1,
            v: IntoOnlyId(7),
        })
        .unwrap(),
        r#"{"a":1,"id":"7"}"#
    );
    let back: FlattenIntoOnlySkipDeserializeValid = serde_json::from_str(r#"{"a":1}"#).unwrap();
    assert_eq!(back.v.0, 0);
}

#[test]
fn flatten_of_into_only_conversion_with_skipped_deserialize_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenIntoOnlySkipDeserializeValid>(),
            specta_serde::PhasesFormat,
        )
        .expect(
            "the live serialize side flattens the struct wire; the raw u32 shape only exists on \
             the skipped deserialize side",
        );
}

// Each side of a `Phased<S, D>` override is only ever exported/rewritten for
// its own phase, so S must only satisfy the *serialize* direction and D the
// *deserialize* direction. `IntoOnlyId`'s serialize wire is a struct (its raw
// u32 shape only matters when it deserializes - which S never does), and
// `FromOnlyVecWire`'s Vec wire is deserialize-only (its raw shape is a named
// struct, which is what the serialize phase flattens).
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenPhasedIntoOnlySerializeSideValid {
    a: i32,
    #[serde(flatten)]
    #[specta(type = specta_serde::Phased<IntoOnlyId, Inner>)]
    v: HashMap<String, String>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenPhasedFromOnlySerializeSideValid {
    a: i32,
    #[serde(flatten)]
    #[specta(type = specta_serde::Phased<FromOnlyVecWire, Inner>)]
    v: HashMap<String, String>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenPhasedVecSerializeWireInvalid {
    a: i32,
    #[serde(flatten)]
    #[specta(type = specta_serde::Phased<VecWire, Inner>)]
    v: HashMap<String, String>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenPhasedFromOnlyDeserializeSideInvalid {
    a: i32,
    #[serde(flatten)]
    #[specta(type = specta_serde::Phased<Inner, FromOnlyVecWire>)]
    v: HashMap<String, String>,
}

#[test]
fn flatten_of_phased_with_into_only_serialize_shape_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenPhasedIntoOnlySerializeSideValid>(),
            specta_serde::PhasesFormat,
        )
        .expect(
            "the serialize phase flattens IntoOnlyId's struct wire; its raw u32 deserialize \
             shape is unreachable because the deserialize phase uses Inner",
        );
}

#[test]
fn flatten_of_phased_with_from_only_serialize_shape_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenPhasedFromOnlySerializeSideValid>(),
            specta_serde::PhasesFormat,
        )
        .expect(
            "the serialize phase flattens FromOnlyVecWire's raw named-field struct; its Vec \
             from-wire only exists on the deserialize side, which uses Inner",
        );
}

#[test]
fn flatten_of_phased_with_vec_serialize_wire_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenPhasedVecSerializeWireInvalid>(),
            specta_serde::PhasesFormat,
        )
        .expect_err("VecWire's serialize wire is a Vec, and the serialize phase is live");

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

#[test]
fn flatten_of_phased_with_from_only_vec_deserialize_shape_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenPhasedFromOnlyDeserializeSideInvalid>(),
            specta_serde::PhasesFormat,
        )
        .expect_err("the deserialize phase flattens FromOnlyVecWire's Vec from-wire");

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

// The same per-phase rule applies one level deeper, to conversion wires
// themselves: in `PhasesFormat` an `into` wire is only ever serialized and a
// `from`/`try_from` wire only ever deserialized (`select_conversion_target`
// picks by phase), so each wire type only needs to satisfy its own direction.
// `NestedWireId` serializes via `FromOnlyVecWire` - whose own `from = Vec`
// conversion is deserialize-only and therefore unreachable here - and
// deserializes via `ConversionWire` (a struct).
#[derive(Type, Serialize, Deserialize, Clone)]
#[specta(collect = false)]
#[serde(into = "FromOnlyVecWire", from = "ConversionWire")]
struct NestedWireId(u32);

impl From<NestedWireId> for FromOnlyVecWire {
    fn from(value: NestedWireId) -> Self {
        Self {
            x: value.0.to_string(),
        }
    }
}

impl From<ConversionWire> for NestedWireId {
    fn from(value: ConversionWire) -> Self {
        Self(value.id.parse().unwrap_or_default())
    }
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenNestedWireValid {
    a: i32,
    #[serde(flatten)]
    v: NestedWireId,
}

#[test]
fn serde_json_confirms_nested_conversion_wires_flatten_per_phase() {
    assert_eq!(
        serde_json::to_string(&FlattenNestedWireValid {
            a: 1,
            v: NestedWireId(7),
        })
        .unwrap(),
        r#"{"a":1,"x":"7"}"#
    );
    let back: FlattenNestedWireValid = serde_json::from_str(r#"{"a":1,"id":"9"}"#).unwrap();
    assert_eq!(back.v.0, 9);
}

#[test]
fn flatten_of_conversion_with_nested_deserialize_only_wire_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenNestedWireValid>(),
            specta_serde::PhasesFormat,
        )
        .expect(
            "the into-wire is only serialized, so its own deserialize-only Vec from-wire is \
             unreachable",
        );
}

// Controls: a wire whose *own* direction's shape is invalid must stay
// rejected on both sides.
#[derive(Type, Serialize, Deserialize, Clone)]
#[specta(collect = false)]
#[serde(into = "VecWire", from = "ConversionWire")]
struct BadSerializeWireId(u32);

impl From<BadSerializeWireId> for VecWire {
    fn from(value: BadSerializeWireId) -> Self {
        Self {
            x: value.0.to_string(),
        }
    }
}

impl From<ConversionWire> for BadSerializeWireId {
    fn from(value: ConversionWire) -> Self {
        Self(value.id.parse().unwrap_or_default())
    }
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenBadSerializeWireInvalid {
    a: i32,
    #[serde(flatten)]
    v: BadSerializeWireId,
}

#[derive(Type, Serialize, Deserialize, Clone)]
#[specta(collect = false)]
#[serde(into = "ConversionWire", from = "FromOnlyVecWire")]
struct BadDeserializeWireId(u32);

impl From<BadDeserializeWireId> for ConversionWire {
    fn from(value: BadDeserializeWireId) -> Self {
        Self {
            id: value.0.to_string(),
        }
    }
}

impl From<FromOnlyVecWire> for BadDeserializeWireId {
    fn from(value: FromOnlyVecWire) -> Self {
        Self(value.x.parse().unwrap_or_default())
    }
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenBadDeserializeWireInvalid {
    a: i32,
    #[serde(flatten)]
    v: BadDeserializeWireId,
}

#[test]
fn flatten_of_conversion_with_vec_serialize_side_of_wire_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenBadSerializeWireInvalid>(),
            specta_serde::PhasesFormat,
        )
        .expect_err("VecWire's own serialize side is its Vec into-wire, which is reachable");

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

#[test]
fn flatten_of_conversion_with_vec_deserialize_side_of_wire_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenBadDeserializeWireInvalid>(),
            specta_serde::PhasesFormat,
        )
        .expect_err(
            "FromOnlyVecWire's own deserialize side is its Vec from-wire, which is reachable \
             when it is used as the outer from target",
        );

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

// Variant-level skips gate flattened fields the same way field-level skips
// do: `PhasesFormat` drops a one-side-skipped variant from that phase
// (`filter_enum_variants_for_phase`), so a phase in which the variant doesn't
// exist can't flatten anything, and a `Phased` override's shape on that side
// is unreachable.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum VariantSkipSerializingEnum {
    #[serde(skip_serializing)]
    V {
        #[serde(flatten)]
        #[specta(type = specta_serde::Phased<Vec<String>, Inner>)]
        v: HashMap<String, String>,
    },
    Other {
        x: i32,
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum VariantSkipDeserializingEnum {
    #[serde(skip_deserializing)]
    V {
        #[serde(flatten)]
        #[specta(type = specta_serde::Phased<Vec<String>, Inner>)]
        v: HashMap<String, String>,
    },
    Other {
        x: i32,
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum VariantUnskippedEnum {
    V {
        #[serde(flatten)]
        #[specta(type = specta_serde::Phased<Vec<String>, Inner>)]
        v: HashMap<String, String>,
    },
    Other {
        x: i32,
    },
}

#[test]
fn flatten_in_serialize_skipped_variant_only_validates_deserialize_shape() {
    Typescript::default()
        .export(
            &Types::default().register::<VariantSkipSerializingEnum>(),
            specta_serde::PhasesFormat,
        )
        .expect(
            "the Vec is the serialize shape, but the whole variant is dropped from the serialize \
             phase, so only the struct-shaped deserialize side ever flattens",
        );
}

#[test]
fn flatten_in_deserialize_skipped_variant_still_validates_live_serialize_shape() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<VariantSkipDeserializingEnum>(),
            specta_serde::PhasesFormat,
        )
        .expect_err("the live serialize side is the Vec, which still flattens and fails");

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

// Unified mode drops a variant skipped in *either* direction entirely
// (`variant_is_skipped_for_mode`: `Unified => skip_serializing ||
// skip_deserializing`), so nothing in it ever flattens there - even a plain
// Vec target must be accepted.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum VariantSkipUnifiedEnum {
    #[serde(skip_serializing)]
    V {
        #[serde(flatten)]
        v: Vec<u8>,
    },
    Other {
        x: i32,
    },
}

#[test]
fn flatten_in_one_side_skipped_variant_under_unified_format_errors_on_the_skip() {
    // Since #518, unified `Format` rejects one-sided variant skips outright.
    // The flatten validator must stay out of the way: the variant never
    // exists in unified mode, so the error must be the directional-skip
    // incompatibility, not a bogus `InvalidFlattenTarget` for the Vec.
    let err = Typescript::default()
        .export(
            &Types::default().register::<VariantSkipUnifiedEnum>(),
            specta_serde::Format,
        )
        .expect_err("unified mode rejects one-sided variant skips since #518");

    let msg = err.to_string();
    assert!(
        msg.contains("Incompatible variant skip"),
        "expected the #518 directional-skip error: {msg}"
    );
    assert!(
        !msg.contains("flatten"),
        "flatten validation must not fire for a variant that never exists in unified mode: {msg}"
    );
}

#[test]
fn flatten_in_unskipped_variant_validates_both_shapes() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<VariantUnskippedEnum>(),
            specta_serde::PhasesFormat,
        )
        .expect_err("both phases are live, and the serialize side is a Vec");

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

// The mirror case: a *deserialize-only* conversion to a non-flattenable wire
// on a field whose deserialization is skipped. Serialization flattens the raw
// named-field struct (fine); deserialization would use the Vec wire, but that
// direction is dead, so the export must succeed.
#[derive(Type, Serialize, Deserialize, Default)]
#[specta(collect = false)]
#[serde(from = "Vec<String>")]
struct FromOnlyVecWire {
    x: String,
}

impl From<Vec<String>> for FromOnlyVecWire {
    fn from(value: Vec<String>) -> Self {
        Self {
            x: value.into_iter().next().unwrap_or_default(),
        }
    }
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenFromOnlySkipDeserializeValid {
    a: i32,
    #[serde(flatten, skip_deserializing, default)]
    v: FromOnlyVecWire,
}

#[test]
fn serde_json_confirms_from_only_with_skipped_deserialize_flattens_fine() {
    assert_eq!(
        serde_json::to_string(&FlattenFromOnlySkipDeserializeValid {
            a: 1,
            v: FromOnlyVecWire { x: "hi".into() },
        })
        .unwrap(),
        r#"{"a":1,"x":"hi"}"#
    );
    let back: FlattenFromOnlySkipDeserializeValid = serde_json::from_str(r#"{"a":1}"#).unwrap();
    assert_eq!(back.v.x, "");
}

#[test]
fn flatten_of_from_only_conversion_with_skipped_deserialize_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenFromOnlySkipDeserializeValid>(),
            specta_serde::PhasesFormat,
        )
        .expect(
            "serialization flattens the raw named-field struct; the Vec wire only exists on the \
             skipped deserialize side",
        );
}

// `#[serde(transparent)]` selects the single non-skipped field as the wire
// shape, ignoring `#[serde(skip)]`-ed siblings (ground-truthed: the wrapper
// below serializes as `[1,2]`, and flattening it fails at runtime with "can
// only flatten structs and maps (got a sequence)"). Skipped fields carry no
// specta type, so the transparent resolution's live-field collection already
// matches serde's selection rule and must keep rejecting this. (A one-sided
// skip on the extra field doesn't compile in serde: "#[serde(transparent)]
// requires struct to have at most one transparent field".)
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct TransparentSkipVec(#[serde(skip)] u32, Vec<u8>);

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenTransparentSkipVecInvalid {
    a: i32,
    #[serde(flatten)]
    v: TransparentSkipVec,
}

#[test]
fn flatten_of_transparent_wrapper_with_skipped_sibling_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenTransparentSkipVecInvalid>(),
            specta_serde::Format,
        )
        .expect_err("serde's wire shape is the non-skipped Vec, which can't be flattened");

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

// A tuple struct with exactly one field is a newtype: serde delegates
// (de)serialization straight to the inner value with no `#[serde(transparent)]`
// required, so flattening one is exactly as valid as flattening its inner type
// (verified against serde_json 1.x: newtype-of-struct flattens fine,
// newtype-of-integer fails with "can only flatten structs and maps").
#[derive(Type, Serialize)]
#[specta(collect = false)]
struct NewtypeOfStruct(Inner);

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct NewtypeOfNewtype(NewtypeOfStruct);

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct NewtypeOfPrimitive(u32);

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenNewtypeStructValid {
    a: i32,
    #[serde(flatten)]
    v: NewtypeOfStruct,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenNewtypeChainValid {
    a: i32,
    #[serde(flatten)]
    v: NewtypeOfNewtype,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenNewtypePrimitiveInvalid {
    a: i32,
    #[serde(flatten)]
    v: NewtypeOfPrimitive,
}

#[test]
fn flatten_of_newtype_struct_wrapping_struct_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenNewtypeStructValid>(),
            specta_serde::Format,
        )
        .expect("serde delegates a newtype straight to its inner value, which is a struct here");
}

#[test]
fn flatten_of_newtype_chain_wrapping_struct_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenNewtypeChainValid>(),
            specta_serde::Format,
        )
        .expect("newtype delegation applies at every level of the chain");
}

#[test]
fn flatten_of_newtype_struct_wrapping_primitive_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenNewtypePrimitiveInvalid>(),
            specta_serde::Format,
        )
        .expect_err(
            "serde_json fails at runtime with \"can only flatten structs and maps (got an \
             integer)\"",
        );

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenBoxStructValid {
    a: i32,
    #[serde(flatten)]
    v: Box<Inner>,
}

#[test]
fn flatten_of_boxed_struct_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenBoxStructValid>(),
            specta_serde::Format,
        )
        .expect("Box<T> is invisible to serde, so flattening Box<Struct> is valid");
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenGeneric<T> {
    a: i32,
    #[serde(flatten)]
    t: T,
}

#[test]
fn flatten_of_generic_parameter_is_accepted() {
    // The generic definition's flatten field is `DataType::Generic`, whose
    // shape is unknowable at validation time. serde allows this whenever the
    // instantiation is struct/map-shaped, so rejecting it would be a false
    // positive that breaks valid code.
    Typescript::default()
        .export(
            &Types::default().register::<FlattenGeneric<Inner>>(),
            specta_serde::Format,
        )
        .expect("flattening a generic parameter instantiated with a struct is valid serde usage");
}

// A *reference* to a generic type, on the other hand, carries concrete
// generic arguments, so the flatten target's shape is knowable: the argument
// must be substituted into the resolved definition instead of accepting the
// bare `DataType::Generic` placeholder.
// https://github.com/specta-rs/specta/pull/524#discussion_r-codex (generic
// substitution when resolving flatten targets)
#[derive(Type, Serialize)]
#[specta(collect = false)]
struct GenericNewtype<T>(T);

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenGenericNewtypeVecInvalid {
    a: i32,
    #[serde(flatten)]
    w: GenericNewtype<Vec<u8>>,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenGenericNewtypeStructValid {
    a: i32,
    #[serde(flatten)]
    w: GenericNewtype<Inner>,
}

#[test]
fn serde_json_confirms_flatten_of_generic_newtype_over_vec_fails() {
    let err = serde_json::to_string(&FlattenGenericNewtypeVecInvalid {
        a: 1,
        w: GenericNewtype(vec![1]),
    })
    .expect_err("newtype delegation exposes the Vec, which serde can't flatten");
    assert!(
        err.to_string()
            .contains("can only flatten structs and maps"),
        "unexpected serde_json error: {err}"
    );

    serde_json::to_string(&FlattenGenericNewtypeStructValid {
        a: 1,
        w: GenericNewtype(Inner { b: 2 }),
    })
    .expect("newtype delegation exposes the struct, which flattens fine");
}

#[test]
fn flatten_of_generic_newtype_instantiated_with_vec_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenGenericNewtypeVecInvalid>(),
            specta_serde::Format,
        )
        .expect_err(
            "the reference carries the concrete Vec<u8> argument, so the flatten target is \
             knowably a sequence",
        );

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

#[test]
fn flatten_of_generic_newtype_instantiated_with_struct_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenGenericNewtypeStructValid>(),
            specta_serde::Format,
        )
        .expect("substitution must not reject a struct-shaped instantiation");
}

// `#[serde(into = ..., from = ...)]` substitute the serde *wire* type for the
// declared shape, so a newtype over a primitive that converts to a named-field
// wire struct is a perfectly valid flatten target - the raw tuple shape never
// hits the wire. Ground-truthed below with serde_json.
// https://github.com/specta-rs/specta/pull/524 (Codex round 2)
#[derive(Type, Serialize, Deserialize, Clone)]
#[specta(collect = false)]
#[serde(into = "ConversionWire", from = "ConversionWire")]
struct ConvertedId(u64);

#[derive(Type, Serialize, Deserialize, Clone)]
#[specta(collect = false)]
struct ConversionWire {
    id: String,
}

impl From<ConvertedId> for ConversionWire {
    fn from(value: ConvertedId) -> Self {
        Self {
            id: value.0.to_string(),
        }
    }
}

impl From<ConversionWire> for ConvertedId {
    fn from(value: ConversionWire) -> Self {
        Self(value.id.parse().unwrap_or_default())
    }
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenConvertedNewtypeValid {
    a: i32,
    #[serde(flatten)]
    v: ConvertedId,
}

#[test]
fn serde_json_confirms_flatten_of_converted_newtype_uses_wire_shape() {
    let json = serde_json::to_string(&FlattenConvertedNewtypeValid {
        a: 1,
        v: ConvertedId(7),
    })
    .expect("into-conversion makes the flatten target a named-field struct");
    assert_eq!(json, r#"{"a":1,"id":"7"}"#);

    let back: FlattenConvertedNewtypeValid =
        serde_json::from_str(&json).expect("from-conversion deserializes the wire shape back");
    assert_eq!(back.v.0, 7);
}

#[test]
fn flatten_of_converted_newtype_with_struct_wire_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenConvertedNewtypeValid>(),
            specta_serde::Format,
        )
        .expect("the wire shape (a named-field struct) is what serde flattens, not the raw tuple");
}

// Control: the inverse - a raw shape that would be fine, converting to a wire
// shape serde can't flatten - must be rejected by wire shape too.
#[derive(Type, Serialize, Deserialize, Clone)]
#[specta(collect = false)]
#[serde(into = "Vec<String>", from = "Vec<String>")]
struct VecWire {
    x: String,
}

impl From<VecWire> for Vec<String> {
    fn from(value: VecWire) -> Self {
        vec![value.x]
    }
}

impl From<Vec<String>> for VecWire {
    fn from(value: Vec<String>) -> Self {
        Self {
            x: value.into_iter().next().unwrap_or_default(),
        }
    }
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenConvertedVecWireInvalid {
    a: i32,
    #[serde(flatten)]
    v: VecWire,
}

#[test]
fn flatten_of_converted_struct_with_vec_wire_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenConvertedVecWireInvalid>(),
            specta_serde::Format,
        )
        .expect_err(
            "the declared shape is a named-field struct, but the wire shape is a Vec, which \
             serde_json rejects at runtime with \"can only flatten structs and maps\"",
        );

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

// One-sided conversion: `into` only affects serialization; without `from`,
// deserialization still uses the raw declared shape. The flatten target must
// therefore be valid in *both* directions - here the deserialize direction is
// the raw newtype over u32, which serde can't flatten a map into.
#[derive(Type, Serialize, Deserialize, Clone)]
#[specta(collect = false)]
#[serde(into = "ConversionWire")]
struct IntoOnlyId(u32);

impl From<IntoOnlyId> for ConversionWire {
    fn from(value: IntoOnlyId) -> Self {
        Self {
            id: value.0.to_string(),
        }
    }
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenIntoOnlyInvalid {
    a: i32,
    #[serde(flatten)]
    v: IntoOnlyId,
}

#[test]
fn flatten_of_into_only_conversion_still_checks_raw_deserialize_shape() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenIntoOnlyInvalid>(),
            specta_serde::PhasesFormat,
        )
        .expect_err(
            "serialize-wire is a struct, but deserialization uses the raw u32 newtype, which \
             can't be flattened",
        );

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenNestedGenericNewtypeVecInvalid {
    a: i32,
    #[serde(flatten)]
    w: GenericNewtype<GenericNewtype<Vec<u8>>>,
}

#[test]
fn flatten_of_nested_generic_newtype_over_vec_is_rejected() {
    // The substitution environment is lexically scoped: the outer wrapper's
    // argument (`GenericNewtype<Vec<u8>>`) must be resolved in the scope it
    // was written in, then the inner wrapper's argument (`Vec<u8>`) in turn.
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenNestedGenericNewtypeVecInvalid>(),
            specta_serde::Format,
        )
        .expect_err("newtype delegation bottoms out at the Vec through both wrappers");

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

// A self-recursive generic newtype must terminate: the definition's inner
// reference (`GenericSelfRecursive<T>`) is a fixed syntactic key, so the
// visited set catches the cycle on the second unfolding. (Non-regular
// recursion like `struct N<T>(Box<N<Option<T>>>)` - where lazy substitution
// is what guarantees the visited keys stay finite - can't even be derived:
// rustc hits its recursion limit monomorphizing `Type` for it.)
#[derive(Type, Serialize)]
#[specta(collect = false)]
struct GenericSelfRecursive<T>(Box<GenericSelfRecursive<T>>, std::marker::PhantomData<T>);

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenGenericSelfRecursive {
    a: i32,
    #[serde(flatten)]
    v: GenericSelfRecursive<Inner>,
}

#[test]
fn flatten_of_self_recursive_generic_terminates() {
    let _ = with_timeout(std::time::Duration::from_secs(10), || {
        Typescript::default().export(
            &Types::default().register::<FlattenGenericSelfRecursive>(),
            specta_serde::Format,
        )
    });
}

// Explicit `specta_serde::Phased` overrides replace the field's datatype with
// an opaque reference holding both phase shapes; a flatten check must look
// inside rather than trusting the opaque wrapper, since serde still has to
// flatten the real value in both directions at runtime.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenPhasedVecInvalid {
    a: i32,
    #[serde(flatten)]
    #[specta(type = specta_serde::Phased<Vec<String>, Inner>)]
    v: HashMap<String, String>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenPhasedMapValid {
    a: i32,
    #[serde(flatten)]
    #[specta(type = specta_serde::Phased<HashMap<String, String>, Inner>)]
    v: HashMap<String, String>,
}

#[test]
fn flatten_of_phased_override_with_sequence_phase_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenPhasedVecInvalid>(),
            specta_serde::PhasesFormat,
        )
        .expect_err("the serialize phase is a Vec, which serde can't flatten at runtime");

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

#[test]
fn flatten_of_phased_override_with_map_and_struct_phases_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<FlattenPhasedMapValid>(),
            specta_serde::PhasesFormat,
        )
        .expect("both phases are map/struct-shaped, so the flatten is valid in both directions");
}

// A flatten field *declared inside* a generic definition is unknowable at the
// definition (`FlattenGeneric`'s own walk accepts the bare placeholder - see
// `flatten_of_generic_parameter_is_accepted`), but a reference to it carries
// concrete arguments, so the referencing type's walk must substitute them and
// reject non-object instantiations. serde_json ground truth below.
#[derive(Type, Serialize)]
#[specta(collect = false)]
struct UsesWrapVecInvalid {
    w: FlattenGeneric<Vec<u8>>,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct UsesWrapStructValid {
    w: FlattenGeneric<Inner>,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct UsesWrapMapValid {
    w: FlattenGeneric<HashMap<String, String>>,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct UsesWrapNestedVecInvalid {
    w: FlattenGeneric<FlattenGeneric<Vec<u8>>>,
}

#[test]
fn serde_json_confirms_generic_flatten_instantiations() {
    let err = serde_json::to_string(&UsesWrapVecInvalid {
        w: FlattenGeneric { a: 1, t: vec![1u8] },
    })
    .expect_err("flattening the Vec instantiation fails at runtime");
    assert!(
        err.to_string()
            .contains("can only flatten structs and maps"),
        "unexpected serde_json error: {err}"
    );

    assert_eq!(
        serde_json::to_string(&UsesWrapStructValid {
            w: FlattenGeneric {
                a: 1,
                t: Inner { b: 2 },
            },
        })
        .unwrap(),
        r#"{"w":{"a":1,"b":2}}"#
    );
}

#[test]
fn flatten_of_generic_instantiated_with_vec_through_reference_is_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<UsesWrapVecInvalid>(),
            specta_serde::Format,
        )
        .expect_err(
            "the reference carries the concrete Vec<u8> argument for the definition's \
             flattened T",
        );

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

#[test]
fn flatten_of_generic_instantiated_with_struct_through_reference_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<UsesWrapStructValid>(),
            specta_serde::Format,
        )
        .expect("a struct-shaped instantiation is valid serde usage");
}

#[test]
fn flatten_of_generic_instantiated_with_map_through_reference_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<UsesWrapMapValid>(),
            specta_serde::Format,
        )
        .expect("a map-shaped instantiation is valid serde usage");
}

#[test]
fn flatten_of_nested_generic_instantiation_over_vec_is_rejected() {
    // The inner `FlattenGeneric<Vec<u8>>` appears as a generic *argument*;
    // the walk must not false-skip it behind the outer instantiation.
    let err = Typescript::default()
        .export(
            &Types::default().register::<UsesWrapNestedVecInvalid>(),
            specta_serde::Format,
        )
        .expect_err("the inner instantiation flattens a Vec");

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

// The reference memo must be keyed per *substitution*, not per syntactic
// reference: `Outer<T>`'s body names `FlattenGeneric<T>` once, but reaching
// it under two different outer instantiations in one walk must validate the
// flattened `T` against each of them - whichever order the walk sees them in.
#[derive(Type, Serialize)]
#[specta(collect = false)]
struct Outer<T> {
    inner: FlattenGeneric<T>,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct BothGoodFirst {
    good: Outer<Inner>,
    bad: Outer<Vec<u8>>,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct BothBadFirst {
    bad: Outer<Vec<u8>>,
    good: Outer<Inner>,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct OnlyGood {
    good: Outer<Inner>,
}

#[test]
fn flatten_of_generic_reached_under_two_substitutions_rejects_the_bad_one() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<BothGoodFirst>(),
            specta_serde::Format,
        )
        .expect_err(
            "the struct-shaped instantiation being walked first must not mask the Vec \
             instantiation of the same syntactic reference",
        );
    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );

    // Order-sensitivity check: the bad instantiation first must not pass by
    // accident of walk order either.
    let err = Typescript::default()
        .export(
            &Types::default().register::<BothBadFirst>(),
            specta_serde::Format,
        )
        .expect_err("the Vec instantiation must be rejected regardless of walk order");
    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

#[test]
fn flatten_of_generic_reached_under_one_good_substitution_is_accepted() {
    Typescript::default()
        .export(
            &Types::default().register::<OnlyGood>(),
            specta_serde::Format,
        )
        .expect("the struct-shaped instantiation alone is valid");
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct TransparentOuter(TransparentInner);

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(transparent)]
struct TransparentInner(Vec<u8>);

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenNestedTransparentVecInvalid {
    a: i32,
    #[serde(flatten)]
    v: TransparentOuter,
}

#[test]
fn flatten_of_nested_transparent_wrappers_around_vec_is_rejected() {
    // Transparent resolution must recurse: two `#[serde(transparent)]`
    // wrappers still delegate straight down to the `Vec<u8>`, which
    // serde_json rejects at runtime with "can only flatten structs and maps
    // (got a sequence)".
    let err = Typescript::default()
        .export(
            &Types::default().register::<FlattenNestedTransparentVecInvalid>(),
            specta_serde::Format,
        )
        .expect_err("nested transparent wrappers still serialize as a sequence");

    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenSelfRecursive {
    a: i32,
    #[serde(flatten)]
    s: Box<FlattenSelfRecursive>,
}

#[test]
fn flatten_of_self_referential_struct_terminates() {
    // Serializing this at runtime would recurse forever, but that's not a
    // flatten-shape problem - the target resolves to a named struct. What
    // matters here is that the flatten target resolution's cycle guard
    // terminates instead of hanging the export.
    let result = with_timeout(std::time::Duration::from_secs(10), || {
        Typescript::default().export(
            &Types::default().register::<FlattenSelfRecursive>(),
            specta_serde::Format,
        )
    });
    result.expect("self-referential flatten target is struct-shaped, so validation accepts it");
}

/// Runs `f` on a background thread, panicking if it doesn't finish within
/// `timeout`, so a recursion regression fails the test instead of hanging CI.
fn with_timeout<T: Send + 'static>(
    timeout: std::time::Duration,
    f: impl FnOnce() -> T + Send + 'static,
) -> T {
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(f());
    });
    rx.recv_timeout(timeout)
        .expect("operation timed out - likely an infinite recursion regression")
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct FlattenJsonValue {
    a: i32,
    #[serde(flatten)]
    v: serde_json::Value,
    #[serde(flatten)]
    extra: HashMap<String, serde_json::Value>,
}

#[test]
fn flatten_of_json_value_and_json_map_is_accepted() {
    // `serde_json::Value` is modelled as an untagged enum. Flattening it only
    // works at runtime when the value is an object, but rejecting it would
    // break the extremely common "extra fields" pattern, so enum targets are
    // deliberately accepted (see `validate_flatten_target`).
    //
    // Validation is exercised via `Format::map_types` rather than a full
    // TypeScript export because `Value` contains `i64`, which trips the
    // exporter's unrelated BigInt policy after validation succeeds.
    use specta::Format as _;

    specta_serde::Format
        .map_types(&Types::default().register::<FlattenJsonValue>())
        .expect("flattening serde_json::Value / HashMap<String, Value> must stay accepted");
}

// --- Gap 1 continued: error paths use declaration indices ---

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct TupleCodecAfterSkipped(#[serde(skip)] u32, #[serde(with = "codec")] u32);

#[test]
fn tuple_struct_codec_error_uses_declaration_index() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<TupleCodecAfterSkipped>(),
            specta_serde::Format,
        )
        .expect_err("codec without override must still be rejected after a skipped field");

    let msg = err.to_string();
    assert!(
        msg.contains("[1]"),
        "error should use the declaration index (1), not the post-filter index (0): {msg}"
    );
}

// --- Gap 3: nested untagged enum inside an internally tagged enum reports
// the inner type in its error path ---

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedPrim {
    P(u32),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum OuterTag {
    X(UntaggedPrim),
}

#[test]
fn nested_untagged_enum_error_names_the_inner_type() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<OuterTag>(),
            specta_serde::Format,
        )
        .expect_err(
            "serde fails at runtime with \"cannot serialize tagged newtype variant OuterTag::X \
             containing an integer\", so specta-serde should reject this too",
        );

    let msg = err.to_string();
    assert!(
        msg.contains("UntaggedPrim"),
        "error should name the inner untagged enum type, not just the outer enum, got: {msg}"
    );
    assert!(
        msg.contains("OuterTag"),
        "error should still mention the outer container it was reached from, got: {msg}"
    );
    // The reported "variant" is `UntaggedPrim::P` (the field that's actually
    // incompatible with an internal tag), not `OuterTag::X` - `OuterTag`
    // doesn't have a variant named `P`, which was the original bug.
    assert!(
        msg.contains("variant 'P'"),
        "error should name the inner enum's variant, not a variant of the outer enum, got: {msg}"
    );
}

// A field skipped for a phase is removed before the rewrite ever visits its
// type in that phase, so flatten checks *inside* the field's type must
// inherit the field's skips. In this construct the containing field kills
// serialization and the flattened Vec's own skip kills deserialization - the
// Vec never flattens in either phase (ground-truthed below), so a per-phase
// rendering must accept it. The nested struct is built by hand because a
// *registered* equivalent is validated standalone too - and rejected
// correctly, since serializing it directly does flatten the Vec (also pinned
// below).
fn nested_dead_flatten_datatype(
    types: &mut Types,
    container_field_attr: Option<&'static str>,
) -> specta::datatype::DataType {
    use specta::datatype::{Field, Struct};

    let mut v = Field::new(<Vec<u8> as Type>::definition(types));
    v.attributes.insert("serde:field:flatten", true);
    v.attributes.insert("serde:field:skip_deserializing", true);
    let inner_struct = Struct::named().field("v", v).build();

    let mut inner_field = Field::new(inner_struct);
    if let Some(attr) = container_field_attr {
        inner_field.attributes.insert(attr, true);
    }

    Struct::named()
        .field("a", Field::new(<i32 as Type>::definition(types)))
        .field("inner", inner_field)
        .build()
}

#[test]
fn phases_map_type_nested_flatten_dead_in_both_phases_is_accepted() {
    use specta::Format as _;

    let mut types = Types::default();
    let dt = nested_dead_flatten_datatype(&mut types, Some("serde:field:skip_serializing"));

    specta_serde::PhasesFormat.map_type(&types, &dt).expect(
        "the containing field kills serialize and the flatten's own skip kills deserialize",
    );
}

#[test]
fn phases_map_type_nested_flatten_through_live_field_is_rejected() {
    use specta::Format as _;

    let mut types = Types::default();
    let dt = nested_dead_flatten_datatype(&mut types, None);

    let err = specta_serde::PhasesFormat
        .map_type(&types, &dt)
        .expect_err("through a live field the flattened Vec is live on the serialize side");
    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}

#[test]
fn phases_map_type_same_inner_through_dead_and_live_fields_is_rejected() {
    use specta::Format as _;
    use specta::datatype::{Field, Struct};

    // The same inner shape reached through a skipped field AND a live field:
    // the live occurrence must still be validated (and rejected), in both
    // declaration orders, so a dead-direction visit can't mask a live one.
    for dead_first in [true, false] {
        let mut types = Types::default();

        let make_inner = |types: &mut Types| {
            let mut v = Field::new(<Vec<u8> as Type>::definition(types));
            v.attributes.insert("serde:field:flatten", true);
            v.attributes.insert("serde:field:skip_deserializing", true);
            Struct::named().field("v", v).build()
        };

        let mut dead_field = Field::new(make_inner(&mut types));
        dead_field
            .attributes
            .insert("serde:field:skip_serializing", true);
        let live_field = Field::new(make_inner(&mut types));

        let builder = Struct::named();
        let dt = if dead_first {
            builder
                .field("dead", dead_field)
                .field("live", live_field)
                .build()
        } else {
            builder
                .field("live", live_field)
                .field("dead", dead_field)
                .build()
        };

        let err = specta_serde::PhasesFormat.map_type(&types, &dt).expect_err(
            "the live field's flattened Vec is live on the serialize side regardless of order",
        );
        assert!(
            err.to_string().contains("flatten"),
            "unexpected error (dead_first={dead_first}): {err}"
        );
    }
}

// The registered (derive) equivalent stays rejected by the whole-graph
// export: the inner type is a standalone export there, and serializing it
// directly *does* flatten the Vec at runtime.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct NestedDeadFlattenInner {
    #[serde(flatten, skip_deserializing, default)]
    v: Vec<u8>,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct NestedDeadFlattenContainer {
    a: i32,
    #[serde(skip_serializing)]
    inner: NestedDeadFlattenInner,
}

#[test]
fn serde_json_confirms_nested_dead_flatten_runtime_behavior() {
    // The container never flattens the Vec in either direction...
    assert_eq!(
        serde_json::to_string(&NestedDeadFlattenContainer {
            a: 1,
            inner: NestedDeadFlattenInner { v: vec![1] },
        })
        .unwrap(),
        r#"{"a":1}"#
    );
    let _: NestedDeadFlattenContainer = serde_json::from_str(r#"{"a":1,"inner":{}}"#).unwrap();

    // ...but the inner type alone still fails when serialized directly,
    // which is why its *registered* form is correctly rejected.
    let err = serde_json::to_string(&NestedDeadFlattenInner { v: vec![1] })
        .expect_err("serializing the inner type directly flattens the Vec");
    assert!(
        err.to_string()
            .contains("can only flatten structs and maps"),
        "unexpected serde_json error: {err}"
    );
}

#[test]
fn registered_nested_dead_flatten_graph_stays_rejected() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<NestedDeadFlattenContainer>(),
            specta_serde::PhasesFormat,
        )
        .expect_err("the inner type is registered standalone and is invalid on its own");
    assert!(
        err.to_string().contains("flatten"),
        "unexpected error: {err}"
    );
}
