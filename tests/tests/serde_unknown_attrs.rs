//! specta models only a subset of serde's attribute namespace, and serde validates that
//! namespace itself - so attributes specta does not model must be ignored, not rejected.
//! Ignoring a name-value or list attribute still requires consuming its tokens: returning
//! from the `parse_nested_meta` callback without doing so makes syn fail the whole derive
//! with "expected `,`".
//!
//! The trigger case was `#[serde(remote = "Self")]`, which makes the serde derives emit
//! inherent `serialize`/`deserialize` functions so the trait impls can be written by hand
//! (e.g. to migrate a document before parsing it). Container `bound`/`expecting`, variant
//! `bound`, and field `getter` hit the same paths in each attribute scope.

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(remote = "Self")]
struct RemoteSelf {
    value: i32,
    #[serde(default)]
    label: Option<String>,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(bound = "T: Serialize + Clone")]
struct ContainerBound<T> {
    inner: T,
}

#[derive(Type, Deserialize)]
#[specta(collect = false)]
#[serde(expecting = "a remote self")]
struct ContainerExpecting {
    id: u32,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
enum VariantBound<T> {
    #[serde(bound = "T: Serialize")]
    Value(T),
    Empty,
}

struct Seconds(i32);

impl Seconds {
    fn value(&self) -> i32 {
        self.0
    }
}

// The remote-definition pattern: serde reads the real type's fields through getters.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(remote = "Seconds")]
struct SecondsDef {
    #[serde(getter = "Seconds::value")]
    value: i32,
}

#[test]
fn unknown_serde_attributes_are_ignored() {
    let rendered = Typescript::default()
        .export(
            &Types::default().register::<RemoteSelf>(),
            specta_serde::Format,
        )
        .expect("remote = \"Self\" must not affect the exported type");
    assert!(rendered.contains("value: number"), "got: {rendered}");
    assert!(rendered.contains("label"), "got: {rendered}");

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ContainerBound<String>>(),
            specta_serde::Format,
        )
        .expect("container bound must not affect the exported type");
    assert!(rendered.contains("inner"), "got: {rendered}");

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<ContainerExpecting>(),
            specta_serde::Format,
        )
        .expect("container expecting must not affect the exported type");
    assert!(rendered.contains("id: number"), "got: {rendered}");

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<VariantBound<String>>(),
            specta_serde::Format,
        )
        .expect("variant bound must not affect the exported type");
    assert!(rendered.contains("Value"), "got: {rendered}");

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<SecondsDef>(),
            specta_serde::Format,
        )
        .expect("field getter must not affect the exported type");
    assert!(rendered.contains("value: number"), "got: {rendered}");
}
