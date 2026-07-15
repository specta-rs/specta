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
//! That heuristic was replaced (PR #522) with an explicit
//! `ENUM_REPR_REWRITTEN_MARKER` attribute set by every synthetic-enum
//! builder in `specta-serde` once its output reaches its final exported
//! shape. A Codex P1 review comment on the merged flatten-`Option` PR (#519)
//! flagged that `flatten_intersection_with_optionals` was missed by that
//! marker rollout: it also builds a synthetic union `Enum`, so an internally
//! tagged enum variant with both a flattened `Option<T>` field and a
//! phase-split trigger (e.g. `skip_serializing_if`) got its flatten-union
//! payload re-treated as an unrewritten user enum on `PhasesFormat`'s second
//! pass. See `internal_flatten_opt_phase_split_keeps_tag_shape` below.
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

// Codex P1 review comment on merged PR #519:
// https://github.com/specta-rs/specta/pull/519#discussion_r3584368095
//
// "When this path is used for a `PhasesFormat`-split internally tagged enum
// (for example a variant that also has `skip_serializing_if`), `map_types`
// rewrites the generated serialize/deserialize types a second time, and
// `enum_repr_already_rewritten` only recognizes an already-rewritten internal
// payload when the unnamed field is an `Intersection`. This helper
// [`flatten_intersection_with_optionals`] now returns a top-level `Enum`
// union, so the second pass treats the variant as externally tagged and
// exports `{ A: ... }` instead of the serde shape with the `t` tag."
//
// `flatten_intersection_with_optionals` (used both for flattened structs and
// for internally-tagged enum variants with a flattened `Option<T>` field)
// builds a synthetic union `Enum` but - unlike every other synthetic-enum
// builder in this file (`string_literal_datatype`, `alias_field_union`,
// `rewrite_identifier_enum_for_phase`'s output) - never marked it with
// `ENUM_REPR_REWRITTEN_MARKER`. `PhasesFormat` only walks into this payload
// on its *second* rewrite pass over split generated types, which only
// happens when something else on the same enum forces a serialize/deserialize
// split (e.g. a field with `skip_serializing_if`); a flattened `Option` field
// on its own is phase-symmetric and never triggers the split (see
// `flat_opt_phases_format_does_not_split` in `serde_flatten_option.rs`), which
// is why this bug wasn't caught by the PR #519 test suite. Confirmed against
// `origin/main` (pre-fix) that this produces exactly the double-wrap Codex
// predicted - `{ A: { t: "A", ... } & ... }` - instead of the real serde wire
// shape `{"t":"A",...}`; on unfixed `PR #522` it instead hard-errors on
// export ("anonymous named-field enum variants cannot be exported").
#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct FlattenPhaseSplitInner {
    x: i32,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum InternalFlattenOptPhaseSplit {
    A {
        #[serde(flatten)]
        inner: Option<FlattenPhaseSplitInner>,
        #[serde(skip_serializing_if = "Option::is_none")]
        trigger: Option<String>,
    },
}

#[test]
fn internal_flatten_opt_phase_split_keeps_tag_shape() {
    let none = InternalFlattenOptPhaseSplit::A {
        inner: None,
        trigger: None,
    };
    assert_eq!(serde_json::to_string(&none).unwrap(), r#"{"t":"A"}"#);

    let some = InternalFlattenOptPhaseSplit::A {
        inner: Some(FlattenPhaseSplitInner { x: 1 }),
        trigger: Some("hi".to_string()),
    };
    assert_eq!(
        serde_json::to_string(&some).unwrap(),
        r#"{"t":"A","x":1,"trigger":"hi"}"#
    );

    let de_none: InternalFlattenOptPhaseSplit = serde_json::from_str(r#"{"t":"A"}"#).unwrap();
    match de_none {
        InternalFlattenOptPhaseSplit::A { inner, trigger } => {
            assert!(inner.is_none());
            assert!(trigger.is_none());
        }
    }

    let rendered = Typescript::default()
        .export(
            &Types::default().register::<InternalFlattenOptPhaseSplit>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    let normalized = normalize_ts(&rendered);

    // The bug wraps the whole variant a second time as if it were externally
    // tagged, e.g. `{ A: { t: "A", ... } & ... }`.
    assert!(
        !normalized.contains("{ A:"),
        "variant `A` was externally double-wrapped by the second `PhasesFormat` \
         rewrite pass over the flatten-option union payload: {rendered}"
    );
    // The correct shape keeps the internal `t` tag directly on every branch.
    assert!(
        normalized.contains("t: \"A\""),
        "expected the internally tagged shape to retain the `t` tag on every \
         union branch, got: {rendered}"
    );
    assert!(
        normalized.contains("FlattenPhaseSplitInner"),
        "expected the flattened `Some` branch to still merge in \
         `FlattenPhaseSplitInner`'s fields, got: {rendered}"
    );
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
