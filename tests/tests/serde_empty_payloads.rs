// Regression tests for specta-serde mishandling enum variants / tuple structs
// whose payload is empty, or becomes empty after `#[serde(skip)]` filtering.
//
// serde's actual rules (verified against serde_json below):
// - Only `Fields::Unit` variants omit a payload entirely.
// - Empty tuple variants (`Foo()`) serialize as `[]`.
// - Empty struct variants (`Foo {}`) serialize as `{}`.
// - Declared-multi-field tuple variants/structs stay sequences even when
//   `#[serde(skip)]` reduces the live field count to 0 or 1.
// - Newtype variants/structs (declared arity 1) DO collapse to unit when
//   their single field is skipped.

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

// --- Bug A: adjacently tagged enums must keep `content` for every non-unit
// variant kind, not just ones with a "live" payload. ---

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjEmpty {
    Unit,
    EmptyStruct {},
    EmptyTuple(),
    AllSkipped(#[serde(skip)] u8, #[serde(skip)] u8),
    NewtypeSkip(#[serde(skip)] u8),
}

#[test]
fn adjacent_tagged_empty_payloads_serde_ground_truth() {
    assert_eq!(
        serde_json::to_string(&AdjEmpty::Unit).unwrap(),
        r#"{"t":"Unit"}"#
    );
    assert_eq!(
        serde_json::to_string(&AdjEmpty::EmptyStruct {}).unwrap(),
        r#"{"t":"EmptyStruct","c":{}}"#
    );
    assert_eq!(
        serde_json::to_string(&AdjEmpty::EmptyTuple()).unwrap(),
        r#"{"t":"EmptyTuple","c":[]}"#
    );
    assert_eq!(
        serde_json::to_string(&AdjEmpty::AllSkipped(1, 2)).unwrap(),
        r#"{"t":"AllSkipped","c":[]}"#
    );

    // `content` is required for every non-unit variant, even when empty.
    assert!(serde_json::from_str::<AdjEmpty>(r#"{"t":"EmptyTuple"}"#).is_err());
    assert!(serde_json::from_str::<AdjEmpty>(r#"{"t":"AllSkipped"}"#).is_err());

    // Newtype variant with its sole field skipped: serde collapses this to
    // unit (verified below), unlike the multi-field `AllSkipped` case above.
    assert_eq!(
        serde_json::to_string(&AdjEmpty::NewtypeSkip(1)).unwrap(),
        r#"{"t":"NewtypeSkip"}"#
    );

    // Round trips that *do* succeed.
    assert!(serde_json::from_str::<AdjEmpty>(r#"{"t":"Unit"}"#).is_ok());
    assert!(serde_json::from_str::<AdjEmpty>(r#"{"t":"EmptyStruct","c":{}}"#).is_ok());
    assert!(serde_json::from_str::<AdjEmpty>(r#"{"t":"EmptyTuple","c":[]}"#).is_ok());
    assert!(serde_json::from_str::<AdjEmpty>(r#"{"t":"AllSkipped","c":[]}"#).is_ok());
}

#[test]
fn adjacent_tagged_empty_payloads_typescript() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjEmpty>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    assert!(
        ts.contains(r#"{ t: "Unit" }"#),
        "unit variant must not have a `c` key:\n{ts}"
    );
    assert!(
        ts.contains("c:") && ts.contains(r#""EmptyStruct""#),
        "empty struct variant must keep `c`:\n{ts}"
    );
    assert!(
        ts.contains(r#""EmptyTuple""#) && ts.contains("c: []"),
        "empty tuple variant must have `c: []`, not omit `c` or render `null`:\n{ts}"
    );
    assert!(
        ts.contains(r#""AllSkipped""#) && ts.contains("c: []"),
        "all-skipped tuple variant must have `c: []`:\n{ts}"
    );
}

// --- Bug B: externally tagged multi-field tuple variants with all fields
// skipped must stay `{ A: [] }`, not collapse to a bare string literal. ---

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExtSkipped {
    A(#[serde(skip)] u8, #[serde(skip)] u8),
    B(String),
}

#[test]
fn external_tagged_all_skipped_tuple_serde_ground_truth() {
    assert_eq!(
        serde_json::to_string(&ExtSkipped::A(1, 2)).unwrap(),
        r#"{"A":[]}"#
    );

    // A bare string is *not* accepted for a declared-arity-2 tuple variant.
    assert!(serde_json::from_str::<ExtSkipped>(r#""A""#).is_err());
    assert!(serde_json::from_str::<ExtSkipped>(r#"{"A":[]}"#).is_ok());
}

#[test]
fn external_tagged_all_skipped_tuple_typescript() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<ExtSkipped>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    assert!(
        !ts.contains(r#""A""#) || ts.contains("A: []"),
        "must not collapse a declared arity-2 tuple variant to a bare string literal:\n{ts}"
    );
    assert!(
        ts.contains("A: []"),
        "must render `{{ A: [] }}` for the all-skipped tuple variant:\n{ts}"
    );
}

// --- Bug C: an externally tagged empty tuple variant must render its
// payload as `[]`, not `null`. ---

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExtEmpty {
    EmptyTuple(),
}

#[test]
fn external_tagged_empty_tuple_serde_ground_truth() {
    assert_eq!(
        serde_json::to_string(&ExtEmpty::EmptyTuple()).unwrap(),
        r#"{"EmptyTuple":[]}"#
    );

    assert!(serde_json::from_str::<ExtEmpty>(r#"{"EmptyTuple":null}"#).is_err());
    assert!(serde_json::from_str::<ExtEmpty>(r#"{"EmptyTuple":[]}"#).is_ok());
}

#[test]
fn external_tagged_empty_tuple_typescript() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<ExtEmpty>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    assert!(
        ts.contains("EmptyTuple: []"),
        "empty tuple variant payload must render as `[]`, not `null`:\n{ts}"
    );
    assert!(
        !ts.contains("EmptyTuple: null"),
        "empty tuple variant payload must not render as `null`:\n{ts}"
    );
}

// --- Bug D: a tuple struct with skipped fields that reduce its live arity
// to 0 or 1 must still render as an array, since its declared arity is > 1. ---

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct TupleSkip(String, #[serde(skip)] String);

#[test]
fn tuple_struct_skipped_field_serde_ground_truth() {
    assert_eq!(
        serde_json::to_string(&TupleSkip("a".into(), "b".into())).unwrap(),
        r#"["a"]"#
    );

    assert!(serde_json::from_str::<TupleSkip>(r#""a""#).is_err());
    assert!(serde_json::from_str::<TupleSkip>(r#"["a"]"#).is_ok());
}

#[test]
fn tuple_struct_skipped_field_typescript() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<TupleSkip>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    assert!(
        ts.contains("[string]"),
        "declared arity-2 tuple struct with one live field must render as `[string]`, not a bare `string`:\n{ts}"
    );
    assert!(
        !ts.contains("type TupleSkip = string;"),
        "must not collapse to a bare newtype:\n{ts}"
    );
}
