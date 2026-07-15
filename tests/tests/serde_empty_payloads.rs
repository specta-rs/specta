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

    // Newtype variant with its sole field skipped: serde's *serializer*
    // collapses this to unit (no `c` key), unlike the multi-field
    // `AllSkipped` case above...
    assert_eq!(
        serde_json::to_string(&AdjEmpty::NewtypeSkip(1)).unwrap(),
        r#"{"t":"NewtypeSkip"}"#
    );
    // ...but serde's *deserializer* is asymmetric: it still requires the `c`
    // key to be present and exactly `null`.
    assert!(serde_json::from_str::<AdjEmpty>(r#"{"t":"NewtypeSkip"}"#).is_err());
    assert!(serde_json::from_str::<AdjEmpty>(r#"{"t":"NewtypeSkip","c":null}"#).is_ok());
    assert!(serde_json::from_str::<AdjEmpty>(r#"{"t":"NewtypeSkip","c":[]}"#).is_err());

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
    assert!(
        ts.contains(r#"{ t: "NewtypeSkip"; c?: null }"#),
        "collapsed newtype variant is ser/de asymmetric; unified mode must \
         render an optional `c?: null`:\n{ts}"
    );
}

#[test]
fn adjacent_tagged_newtype_skip_phases() {
    // Serialize omits `c`; deserialize requires `c: null` -- `PhasesFormat`
    // must split the enum so each phase gets its exact shape.
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjEmpty>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    assert!(
        ts.contains(r#"AdjEmpty_Serialize"#) && ts.contains(r#"AdjEmpty_Deserialize"#),
        "enum must split into per-phase types:\n{ts}"
    );

    let serialize_ty = ts
        .lines()
        .find(|line| line.contains("AdjEmpty_Serialize ="))
        .expect("serialize type must be exported");
    assert!(
        serialize_ty.contains(r#"{ t: "NewtypeSkip" }"#),
        "serialize phase must omit `c` for the collapsed newtype variant:\n{ts}"
    );

    let deserialize_ty = ts
        .lines()
        .find(|line| line.contains("AdjEmpty_Deserialize ="))
        .expect("deserialize type must be exported");
    assert!(
        deserialize_ty.contains(r#"{ t: "NewtypeSkip"; c: null }"#),
        "deserialize phase must require `c: null` for the collapsed newtype variant:\n{ts}"
    );
}

// A skipped sole field of type `Option<T>` is the exception to the
// `content: null` deserialize requirement above: serde's `missing_field`
// helper special-cases `Option` (a missing `c` deserializes as `None`), so
// both `{"t":"V"}` and `{"t":"V","c":null}` are accepted. `c` must therefore
// stay *optional* in every phase, and no per-phase split is needed --
// `{ t: "V"; c?: null }` describes both directions exactly.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjOptionSkip {
    V(#[serde(skip)] Option<u8>),
    W(String),
}

#[test]
fn adjacent_tagged_option_skip_serde_ground_truth() {
    assert_eq!(
        serde_json::to_string(&AdjOptionSkip::V(Some(1))).unwrap(),
        r#"{"t":"V"}"#
    );

    // Unlike the non-`Option` `NewtypeSkip` case above, a missing `c` is
    // accepted (deserialized as `None`), and `c: null` is too.
    assert!(serde_json::from_str::<AdjOptionSkip>(r#"{"t":"V"}"#).is_ok());
    assert!(serde_json::from_str::<AdjOptionSkip>(r#"{"t":"V","c":null}"#).is_ok());
    assert!(serde_json::from_str::<AdjOptionSkip>(r#"{"t":"V","c":1}"#).is_err());
}

#[test]
fn adjacent_tagged_option_skip_typescript() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjOptionSkip>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    assert!(
        ts.contains(r#"{ t: "V"; c?: null }"#),
        "skipped `Option` sole field must render an optional `c?: null`:\n{ts}"
    );
}

#[test]
fn adjacent_tagged_option_skip_phases() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjOptionSkip>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    assert!(
        !ts.contains("AdjOptionSkip_Serialize") && !ts.contains("AdjOptionSkip_Deserialize"),
        "`c?: null` is exact for both phases, so the enum must not split:\n{ts}"
    );
    assert!(
        ts.contains(r#"{ t: "V"; c?: null }"#),
        "deserialize must not require `c` for a skipped `Option` sole field:\n{ts}"
    );
    assert!(
        !ts.contains(r#"{ t: "V"; c: null }"#),
        "deserialize must not require `c` for a skipped `Option` sole field:\n{ts}"
    );
}

// One-sided `skip_deserializing` on an `Option` sole field: the serializer
// still emits the live payload, while the deserializer accepts a missing or
// `null` `c` (serde's `missing_field` `Option` special case again).
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjOptionSkipDe {
    V(#[serde(skip_deserializing)] Option<u8>),
}

#[test]
fn adjacent_tagged_option_skip_deserializing_serde_ground_truth() {
    assert_eq!(
        serde_json::to_string(&AdjOptionSkipDe::V(Some(1))).unwrap(),
        r#"{"t":"V","c":1}"#
    );
    assert_eq!(
        serde_json::to_string(&AdjOptionSkipDe::V(None)).unwrap(),
        r#"{"t":"V","c":null}"#
    );

    assert!(serde_json::from_str::<AdjOptionSkipDe>(r#"{"t":"V"}"#).is_ok());
    assert!(serde_json::from_str::<AdjOptionSkipDe>(r#"{"t":"V","c":null}"#).is_ok());
    assert!(serde_json::from_str::<AdjOptionSkipDe>(r#"{"t":"V","c":1}"#).is_err());
}

#[test]
fn adjacent_tagged_option_skip_deserializing_phases() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjOptionSkipDe>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    let serialize_ty = ts
        .lines()
        .find(|line| line.contains("AdjOptionSkipDe_Serialize ="))
        .expect("serialize type must be exported");
    assert!(
        serialize_ty.contains(r#"{ t: "V"; c: number | null }"#),
        "serialize phase keeps the live payload:\n{ts}"
    );

    let deserialize_ty = ts
        .lines()
        .find(|line| line.contains("AdjOptionSkipDe_Deserialize ="))
        .expect("deserialize type must be exported");
    assert!(
        deserialize_ty.contains(r#"{ t: "V"; c?: null }"#),
        "deserialize phase must keep `c` optional for a skipped `Option` sole field:\n{ts}"
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

// The arity fix must not change the datatype's top-level kind: exporters like
// specta-swift only emit named definitions for `Struct`/`Enum` datatypes, so
// rewriting the definition to a bare `DataType::Tuple` would make the type
// silently disappear from their output.
#[test]
fn tuple_struct_skipped_field_swift() {
    let swift = specta_swift::Swift::default()
        .export(
            &Types::default().register::<TupleSkip>(),
            specta_serde::Format,
        )
        .expect("swift export should succeed");

    assert!(
        swift.contains("TupleSkip"),
        "the tuple struct must not disappear from Swift output:\n{swift}"
    );
}

// A container `#[serde(rename = "...")]` must survive the declared-arity
// rewrite. The rewrite replaces the `DataType::Struct` (which carries the
// container attributes) with a bare `DataType::Tuple` before
// `rewrite_named_type_for_phase` reads the rename, so on this branch the
// rename is lost. PR #525 (fix/serde-enum-container-rename) reorders both
// `map_types` drivers to compute the container rename BEFORE
// `rewrite_datatype_for_phase` runs, which fixes this for every
// shape-changing rewrite (verified against a scratch merge of that branch).
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "Wire")]
struct RenamedTupleSkip(String, #[serde(skip)] String);

#[test]
#[ignore = "container rename for shape-rewritten types requires #525 (fix/serde-enum-container-rename)"]
fn tuple_struct_skipped_field_container_rename() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<RenamedTupleSkip>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    assert!(
        ts.contains("export type Wire = [string];"),
        "container rename must survive the declared-arity tuple rewrite:\n{ts}"
    );
    assert!(
        !ts.contains("RenamedTupleSkip"),
        "the un-renamed Rust name must not leak into the export:\n{ts}"
    );
}

// A serde-transparent tuple struct is the exception to the arity rule above:
// serde serializes it as the bare inner value, so it must NOT be rewritten to
// an array even though its declared arity is > 1. `#[specta(transparent =
// false)]` keeps Specta from resolving the container to its inner type at
// derive time, so specta-serde sees the struct with its
// `serde:container:transparent` attribute intact.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(transparent)]
#[specta(transparent = false)]
struct TransparentTupleSkip(String, #[serde(skip)] u8);

#[test]
fn transparent_tuple_struct_skipped_field_serde_ground_truth() {
    assert_eq!(
        serde_json::to_string(&TransparentTupleSkip("a".into(), 1)).unwrap(),
        r#""a""#
    );

    assert!(serde_json::from_str::<TransparentTupleSkip>(r#""a""#).is_ok());
    assert!(serde_json::from_str::<TransparentTupleSkip>(r#"["a"]"#).is_err());
}

#[test]
fn transparent_tuple_struct_skipped_field_typescript() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<TransparentTupleSkip>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    assert!(
        ts.contains("type TransparentTupleSkip = string;"),
        "serde-transparent tuple struct must stay a bare `string`, not `[string]`:\n{ts}"
    );
    assert!(
        !ts.contains("[string]"),
        "must not rewrite a transparent struct to an array shape:\n{ts}"
    );
}
