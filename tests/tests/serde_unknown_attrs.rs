//! Regression tests: `#[derive(Type)]` must accept every valid serde attribute, including ones
//! specta doesn't act on. Attributes that take a `= value` or `(...)` list were previously not
//! consumed by the fall-through branch of `parse_container_meta` / `parse_variant_meta` /
//! `parse_field_meta`, so `syn`'s `parse_nested_meta` errored with `expected `,`` on anything
//! after them. Path-only unknown attributes (e.g. `deny_unknown_fields`) already worked.
//!
//! These types only need to compile to prove the fix; a couple of them are also exported to
//! TypeScript to prove the fix didn't swallow more than the unknown attribute (i.e. a following
//! attribute like `rename_all` still gets parsed and applied).

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;
use std::borrow::Cow;

// `#[serde(expecting = "...")]` on a container: takes a string value serde uses in error
// messages. Specta has no use for it, but must still parse past the `= "..."`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(expecting = "an ExpectingContainer struct")]
struct ExpectingContainer {
    a: String,
}

// `#[serde(bound = "...")]` on a container: overrides the generated bound on the `Serialize`
// impl. Using the same bound serde would generate by default keeps this actually type-correct.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(bound = "T: Serialize")]
struct BoundContainer<T> {
    value: T,
}

// `#[serde(bound(serialize = "..."))]` on a field: the parenthesized-list form of `bound`.
#[derive(Type, Serialize)]
#[specta(collect = false)]
struct BoundField<T> {
    #[serde(bound(serialize = "T: Serialize"))]
    value: T,
}

// `#[serde(borrow = "'a")]` on a field: explicit-lifetime form of `borrow`, used when serde can't
// infer which lifetimes to borrow (e.g. multiple candidates). Realistic use: a zero-copy `Cow`
// field.
#[derive(Type, Deserialize)]
#[specta(collect = false)]
struct BorrowField<'a> {
    #[serde(borrow = "'a")]
    value: Cow<'a, str>,
}

// Combined case: an unknown-with-value attribute (`expecting`) followed by an attribute specta
// *does* act on (`rename_all`). This guards against a fix that consumes too much input and
// swallows the following attribute along with its comma.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(
    expecting = "an ExpectingWithRenameAll struct",
    rename_all = "camelCase"
)]
struct ExpectingWithRenameAll {
    foo_bar: String,
}

#[test]
fn rename_all_still_applies_after_unknown_valued_attr() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<ExpectingWithRenameAll>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-unknown-attrs-expecting-with-rename-all", ts);
}
