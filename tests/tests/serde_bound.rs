// Regression test for https://github.com/specta-rs/specta/issues/494

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(bound(serialize = "T: serde::Serialize"))]
struct SerdeBoundNested<T>(T);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(bound = "T: serde::Serialize + serde::de::DeserializeOwned")]
struct SerdeBoundFlat<T>(T);

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(bound(
    serialize = "T: serde::Serialize",
    deserialize = "T: serde::de::DeserializeOwned",
))]
struct SerdeBoundBoth<T>(T);

#[test]
fn serde_bound_nested() {
    let mut types = Types::default();
    let _ = SerdeBoundNested::<i32>::definition(&mut types);
}

#[test]
fn serde_bound_flat() {
    let mut types = Types::default();
    let _ = SerdeBoundFlat::<String>::definition(&mut types);
}

#[test]
fn serde_bound_serialize_and_deserialize() {
    let mut types = Types::default();
    let _ = SerdeBoundBoth::<String>::definition(&mut types);
}

// Serde also accepts `bound` on variants and fields, and other
// value-carrying attributes Specta has no use for fail the same way.

#[derive(Serialize, Type)]
#[specta(collect = false)]
enum VariantBound<T> {
    #[serde(bound(serialize = "T: serde::Serialize"))]
    Variant(T),
}

#[derive(Serialize, Type)]
#[specta(collect = false)]
struct FieldBound<T> {
    #[serde(bound(serialize = "T: serde::Serialize"))]
    value: T,
}

#[derive(Serialize, Type)]
#[specta(collect = false)]
#[serde(crate = "serde")]
struct CrateAttr {
    value: i32,
}

#[derive(Deserialize, Type)]
#[specta(collect = false)]
#[serde(expecting = "a wrapper around an integer")]
struct ExpectingAttr {
    value: i32,
}

#[test]
fn unused_serde_attributes_are_ignored() {
    let ts = Typescript::default()
        .export(
            &Types::default()
                .register::<SerdeBoundNested<String>>()
                .register::<SerdeBoundFlat<String>>()
                .register::<SerdeBoundBoth<String>>()
                .register::<VariantBound<String>>()
                .register::<FieldBound<String>>()
                .register::<CrateAttr>()
                .register::<ExpectingAttr>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-bound-typescript", ts);
}
