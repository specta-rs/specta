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

// `#[serde(borrow)]` on a field: the path-only form (serde's most common spelling). This already
// worked before the fix — kept as a regression guard for the path-only fall-through.
#[derive(Type, Deserialize)]
#[specta(collect = false)]
struct BorrowFieldPathOnly<'a> {
    #[serde(borrow)]
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

// Adversarial value: the string contains a comma, which must be treated as part of the string
// literal (not a nested-meta separator), and the following `tag` must still take effect.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(expecting = "a, b", tag = "t")]
enum ExpectingCommaThenTag {
    VariantOne { x: String },
}

// Adversarial value: a bound string containing `+`, followed by `rename_all` which must still
// take effect.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(bound = "T: Clone + Serialize", rename_all = "camelCase")]
struct BoundPlusThenRenameAll<T: Clone> {
    the_value: T,
}

// The parenthesized-list form of `bound` (nested metas), followed by a known `rename` that must
// still take effect.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(
    bound(
        serialize = "T: Serialize",
        deserialize = "T: serde::de::DeserializeOwned"
    ),
    rename = "NestedBoundRenamed"
)]
struct NestedBoundThenRename<T> {
    value: T,
}

// `#[serde(crate = "...")]` uses the `crate` keyword as the attribute path, followed by a known
// attribute that must still take effect.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(crate = "serde", rename_all = "camelCase")]
struct CrateThenRenameAll {
    foo_bar: String,
}

// Variant-level unknown-with-value attribute (`parse_variant_meta`'s fall-through), followed by a
// known `rename` that must still take effect.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum VariantLevelUnknown {
    #[serde(bound = "", rename = "renamed_variant")]
    A { foo_bar: String },
}

fn export<T: Type>() -> String {
    Typescript::default()
        .export(&Types::default().register::<T>(), specta_serde::Format)
        .expect("typescript export should succeed")
}

#[test]
fn rename_all_still_applies_after_unknown_valued_attr() {
    insta::assert_snapshot!(
        "serde-unknown-attrs-expecting-with-rename-all",
        export::<ExpectingWithRenameAll>()
    );
}

// Prove the skip consumes exactly the unknown attribute's value and nothing more, even for
// adversarial values (commas and `+` inside strings, nested parens, keyword paths), by asserting
// the *effect* of the known attribute that follows it.
#[test]
fn known_attrs_still_apply_after_adversarial_unknown_values() {
    let ts = export::<ExpectingCommaThenTag>();
    assert!(ts.contains("t: \"VariantOne\""), "tag not applied: {ts}");

    let ts = export::<BoundPlusThenRenameAll<String>>();
    assert!(ts.contains("theValue"), "rename_all not applied: {ts}");

    let ts = export::<NestedBoundThenRename<String>>();
    assert!(
        ts.contains("NestedBoundRenamed"),
        "rename not applied: {ts}"
    );

    let ts = export::<CrateThenRenameAll>();
    assert!(ts.contains("fooBar"), "rename_all not applied: {ts}");

    let ts = export::<VariantLevelUnknown>();
    assert!(
        ts.contains("renamed_variant"),
        "variant rename not applied: {ts}"
    );
}
