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
