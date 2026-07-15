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
// serde_json 1.0.150): every one of these four representations flattens
// fine, merging tag/content fields straight into the surrounding map.
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
