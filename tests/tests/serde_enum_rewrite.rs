//! Regression tests for the `enum_repr_already_rewritten` idempotency guard
//! in `specta-serde`, which used to shape-sniff variants to decide whether an
//! enum had already been rewritten to its serde representation. That
//! heuristic was unsound in both directions:
//!
//! - False positive: shapes the transform produces (e.g. a single unnamed
//!   `Primitive::str` field, or a named field whose type is `Primitive::str`)
//!   are also valid *untransformed* user shapes (e.g. `&'static str` fields),
//!   so the guard could skip the rewrite entirely and leave the enum
//!   untagged in the exported bindings.
//! - False negative: `PhasesFormat` runs the rewrite pass twice over split
//!   generated types. The untagged-variant branch didn't clear
//!   `transformed_variant.attributes`, so the second pass's guard check
//!   failed and the whole enum was rewritten a second time, double-wrapping
//!   already-tagged variants.
//!
//! Each test below asserts the exported TypeScript describes the same shape
//! that `serde_json` actually produces at runtime.

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

/// Collapses whitespace (including the multi-line pretty-printing the
/// exporter uses for nested object types) and drops trailing commas before a
/// closing brace, so substring checks on the exported TypeScript aren't
/// defeated by formatting details that are irrelevant to the shape being
/// asserted on.
fn normalize_ts(rendered: &str) -> String {
    rendered
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace(", }", " }")
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
enum StrNewtype {
    A(&'static str),
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
enum GuardStrField {
    A { s: &'static str },
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
enum GuardField {
    Message {
        #[serde(rename = "Message")]
        text: String,
    },
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum MixedUntagged {
    #[serde(alias = "a")]
    A(String),
    #[serde(untagged)]
    Other(u32),
}

// Bug 1a: a single unnamed `&'static str` field lowers to exactly
// `Primitive::str`, which the old heuristic mistook for the transform's own
// output shape (`transform_external_variant`'s string-literal tag), so the
// enum was never externally tagged.
#[test]
fn str_newtype_variant_is_externally_tagged() {
    let json = serde_json::to_string(&StrNewtype::A("hi")).unwrap();
    assert_eq!(json, r#"{"A":"hi"}"#);

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<StrNewtype>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    let normalized = normalize_ts(&rendered);

    assert!(
        !normalized.contains("StrNewtype = string"),
        "enum was left untagged, exported bindings do not match serde_json's object shape: {rendered}"
    );
    assert!(
        normalized.contains("{ A: string }"),
        "expected externally tagged shape `{{ A: string }}`, got: {rendered}"
    );
}

// Bug 1b: same false-positive guard, but for a named field whose type is
// `Primitive::str`.
#[test]
fn named_str_field_variant_is_externally_tagged() {
    let json = serde_json::to_string(&GuardStrField::A { s: "hi" }).unwrap();
    assert_eq!(json, r#"{"A":{"s":"hi"}}"#);

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<GuardStrField>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    let normalized = normalize_ts(&rendered);

    assert!(
        !normalized.contains("GuardStrField = { s: string }"),
        "enum was left untagged, exported bindings do not match serde_json's object shape: {rendered}"
    );
    assert!(
        normalized.contains("A: { s: string }"),
        "expected externally tagged shape `{{ A: {{ s: string }} }}`, got: {rendered}"
    );
}

// Bug 1c: field renames run before the guard, so a field renamed to match
// its variant name (`text` -> `Message`) was mistaken for the transform's
// own tag-matches-name output shape, leaving the enum untagged.
#[test]
fn field_renamed_to_variant_name_is_externally_tagged() {
    let json = serde_json::to_string(&GuardField::Message {
        text: "hi".to_string(),
    })
    .unwrap();
    assert_eq!(json, r#"{"Message":{"Message":"hi"}}"#);

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<GuardField>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    let normalized = normalize_ts(&rendered);

    assert!(
        !normalized.contains("GuardField = { Message: string }"),
        "enum was left untagged, exported bindings do not match serde_json's object shape: {rendered}"
    );
    assert!(
        normalized.contains("Message: { Message: string }"),
        "expected externally tagged shape `{{ Message: {{ Message: string }} }}`, got: {rendered}"
    );
}

// Bug 2: the untagged-variant branch of `rewrite_enum_repr_for_phase` didn't
// clear the transformed variant's attributes before `continue`-ing, so on
// `PhasesFormat`'s second rewrite pass over the split generated types, the
// guard's shape check failed for the whole enum and it was rewritten a
// second time as `EnumRepr::External`, double-wrapping the `A` variant.
#[test]
fn mixed_untagged_enum_is_not_double_wrapped() {
    let json_a = serde_json::to_string(&MixedUntagged::A("x".to_string())).unwrap();
    assert_eq!(json_a, r#"{"A":"x"}"#);
    let json_other = serde_json::to_string(&MixedUntagged::Other(5)).unwrap();
    assert_eq!(json_other, "5");

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<MixedUntagged>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    // Collapse all whitespace so multi-line TS formatting doesn't defeat
    // substring checks (the exporter pretty-prints nested object types).
    let normalized = normalize_ts(&rendered);

    assert!(
        !normalized.contains("A: { A:"),
        "variant `A` was double-wrapped by a second rewrite pass: {rendered}"
    );
    assert!(
        normalized.contains("{ A: string }"),
        "expected `A` to be tagged with a single-level object shape `{{ A: string }}`, got: {rendered}"
    );
    assert!(
        normalized.contains("number"),
        "expected the untagged `Other` variant to appear as a bare `number`, got: {rendered}"
    );
}
