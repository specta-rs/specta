// Regression test for https://github.com/specta-rs/specta/issues/494

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Serialize, Type)]
#[specta(collect = false)]
#[serde(bound(serialize = "T: serde::Serialize"))]
struct NestedBound<T>(T);

#[derive(Serialize, Type)]
#[specta(collect = false)]
#[serde(bound = "T: serde::Serialize")]
struct StringBound<T>(T);

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
                .register::<NestedBound<String>>()
                .register::<StringBound<String>>()
                .register::<VariantBound<String>>()
                .register::<FieldBound<String>>()
                .register::<CrateAttr>()
                .register::<ExpectingAttr>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-bound-typescript", ts);
}
