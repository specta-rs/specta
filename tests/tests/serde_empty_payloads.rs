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
}

// A newtype variant whose skipped sole field is NOT `Option` is
// direction-asymmetric: serde's serializer omits `c` (like a unit variant),
// but its deserializer requires `c: null`. No unified shape can represent
// both directions, so `specta_serde::Format` rejects it and `PhasesFormat`
// splits it into exact per-phase shapes.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjNewtypeSkip {
    NewtypeSkip(#[serde(skip)] u8),
    Live(String),
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
        serde_json::to_string(&AdjNewtypeSkip::NewtypeSkip(1)).unwrap(),
        r#"{"t":"NewtypeSkip"}"#
    );
    // ...but serde's *deserializer* is asymmetric: it still requires the `c`
    // key to be present and exactly `null`.
    assert!(serde_json::from_str::<AdjNewtypeSkip>(r#"{"t":"NewtypeSkip"}"#).is_err());
    assert!(serde_json::from_str::<AdjNewtypeSkip>(r#"{"t":"NewtypeSkip","c":null}"#).is_ok());
    assert!(serde_json::from_str::<AdjNewtypeSkip>(r#"{"t":"NewtypeSkip","c":[]}"#).is_err());

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

#[test]
fn adjacent_tagged_newtype_skip_unified_errors() {
    // Serialize omits `c`; deserialize requires `c: null` -- no unified
    // shape can represent both directions, so `Format` must reject it
    // (matching the #518 policy for one-sided renames/skips) rather than
    // guess.
    let result = Typescript::default().export(
        &Types::default().register::<AdjNewtypeSkip>(),
        specta_serde::Format,
    );

    let err = result.expect_err("unified export must reject the asymmetric shape");
    let message = err.to_string();
    assert!(
        message.contains("PhasesFormat"),
        "error must point at `PhasesFormat`:\n{message}"
    );
}

#[test]
fn adjacent_tagged_newtype_skip_phases() {
    // Serialize omits `c`; deserialize requires `c: null` -- `PhasesFormat`
    // must split the enum so each phase gets its exact shape.
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjNewtypeSkip>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    assert!(
        ts.contains(r#"AdjNewtypeSkip_Serialize"#) && ts.contains(r#"AdjNewtypeSkip_Deserialize"#),
        "enum must split into per-phase types:\n{ts}"
    );

    let serialize_ty = ts
        .lines()
        .find(|line| line.contains("AdjNewtypeSkip_Serialize ="))
        .expect("serialize type must be exported");
    assert!(
        serialize_ty.contains(r#"{ t: "NewtypeSkip" }"#),
        "serialize phase must omit `c` for the collapsed newtype variant:\n{ts}"
    );

    let deserialize_ty = ts
        .lines()
        .find(|line| line.contains("AdjNewtypeSkip_Deserialize ="))
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

// serde's `missing_field` `Option` special case applies to LIVE newtype
// `Option` payloads too, without any skip attrs: the serializer always emits
// `c` (`c: 1` / `c: null`), but the deserializer accepts a missing `c` as
// `None`. Deserialize-facing shapes must keep `c` optional (mirroring the
// `#[serde(default)]` convention: optional in deserialize/unified, required
// in serialize). Non-`Option` payloads keep `c` required.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjLiveOption {
    V(Option<u8>),
    NonOpt(u8),
}

#[test]
fn adjacent_tagged_live_option_serde_ground_truth() {
    assert_eq!(
        serde_json::to_string(&AdjLiveOption::V(Some(1))).unwrap(),
        r#"{"t":"V","c":1}"#
    );
    assert_eq!(
        serde_json::to_string(&AdjLiveOption::V(None)).unwrap(),
        r#"{"t":"V","c":null}"#
    );

    assert!(matches!(
        serde_json::from_str::<AdjLiveOption>(r#"{"t":"V"}"#),
        Ok(AdjLiveOption::V(None))
    ));
    assert!(serde_json::from_str::<AdjLiveOption>(r#"{"t":"V","c":null}"#).is_ok());
    assert!(serde_json::from_str::<AdjLiveOption>(r#"{"t":"V","c":1}"#).is_ok());
    // The non-Option control still hard-requires `c`.
    assert!(serde_json::from_str::<AdjLiveOption>(r#"{"t":"NonOpt"}"#).is_err());
}

#[test]
fn adjacent_tagged_live_option_unified() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjLiveOption>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    assert!(
        ts.contains(r#"{ t: "V"; c?: number | null }"#),
        "a live newtype `Option` payload accepts a missing `c` on deserialize; \
         unified must keep it optional:\n{ts}"
    );
    assert!(
        ts.contains(r#"{ t: "NonOpt"; c: number }"#),
        "a non-`Option` payload keeps `c` required:\n{ts}"
    );
}

// Codex's reported case: `skip_serializing` on a live-on-deserialize
// `Option` payload. The enum splits (one-sided skip); the deserialize shape
// must keep `c` optional, since `{"t":"V"}` deserializes to `V(None)`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjOptionSkipSer {
    V(#[serde(skip_serializing)] Option<u8>),
}

#[test]
fn adjacent_tagged_option_skip_serializing_serde_ground_truth() {
    assert_eq!(
        serde_json::to_string(&AdjOptionSkipSer::V(Some(1))).unwrap(),
        r#"{"t":"V"}"#
    );

    assert!(matches!(
        serde_json::from_str::<AdjOptionSkipSer>(r#"{"t":"V"}"#),
        Ok(AdjOptionSkipSer::V(None))
    ));
    assert!(serde_json::from_str::<AdjOptionSkipSer>(r#"{"t":"V","c":null}"#).is_ok());
    assert!(serde_json::from_str::<AdjOptionSkipSer>(r#"{"t":"V","c":1}"#).is_ok());
}

#[test]
fn adjacent_tagged_option_skip_serializing_phases() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjOptionSkipSer>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    let serialize_ty = ts
        .lines()
        .find(|line| line.contains("AdjOptionSkipSer_Serialize ="))
        .expect("serialize type must be exported");
    assert!(
        serialize_ty.contains(r#"{ t: "V" }"#),
        "serialize omits `c` for the ser-skipped payload:\n{ts}"
    );

    let deserialize_ty = ts
        .lines()
        .find(|line| line.contains("AdjOptionSkipSer_Deserialize ="))
        .expect("deserialize type must be exported");
    assert!(
        deserialize_ty.contains(r#"{ t: "V"; c?: number | null }"#),
        "deserialize must keep `c` optional for a live `Option` payload:\n{ts}"
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

// Variant-level `#[serde(untagged)]` bypasses the tag/content representation
// entirely: the variant serializes as its bare payload. A skipped newtype
// payload collapses to serde's *unit* representation, which is `null` under
// untagged -- NOT `[]` -- while zero-arg and multi-field all-skipped tuple
// variants stay `[]`. The adjacent-collapse unified rejection must not fire
// for untagged variants (there is no `c` key to require or split).
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjWithUntaggedSkip {
    Normal(String),
    #[serde(untagged)]
    U(#[serde(skip)] u8),
    #[serde(untagged)]
    M(#[serde(skip)] u8, #[serde(skip)] u8),
}

#[test]
fn adjacent_tagged_untagged_skip_serde_ground_truth() {
    assert_eq!(
        serde_json::to_string(&AdjWithUntaggedSkip::U(7)).unwrap(),
        "null"
    );
    assert_eq!(
        serde_json::to_string(&AdjWithUntaggedSkip::M(1, 2)).unwrap(),
        "[]"
    );

    assert!(matches!(
        serde_json::from_str::<AdjWithUntaggedSkip>("null"),
        Ok(AdjWithUntaggedSkip::U(_))
    ));
    assert!(matches!(
        serde_json::from_str::<AdjWithUntaggedSkip>("[]"),
        Ok(AdjWithUntaggedSkip::M(..))
    ));
    // The tag/content forms do not exist for untagged variants.
    assert!(serde_json::from_str::<AdjWithUntaggedSkip>(r#"{"t":"U"}"#).is_err());
    assert!(serde_json::from_str::<AdjWithUntaggedSkip>(r#"{"t":"U","c":null}"#).is_err());
}

#[test]
fn adjacent_tagged_untagged_skip_unified() {
    // The adjacent asymmetric-collapse rejection must NOT fire: the untagged
    // variants never carry a `c` key.
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjWithUntaggedSkip>(),
            specta_serde::Format,
        )
        .expect("untagged variants have no content key; unified export must succeed");

    assert!(
        ts.contains(r#"{ t: "Normal"; c: string }"#),
        "tagged variant keeps its adjacent shape:\n{ts}"
    );
    assert!(
        ts.contains("null"),
        "untagged skipped newtype must render serde's unit payload (`null`):\n{ts}"
    );
    assert!(
        ts.contains("[]"),
        "untagged all-skipped multi-field tuple must stay `[]`:\n{ts}"
    );
    // The `null` must come from U, not from a `[]`-shaped U member.
    assert!(
        ts.contains("null") && ts.matches("[]").count() == 1,
        "exactly one `[]` member (M); U must be `null`, not `[]`:\n{ts}"
    );
}

#[test]
fn adjacent_tagged_untagged_skip_phases_no_split() {
    // Nothing here is phase-asymmetric: untagged variants have no `c` key.
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjWithUntaggedSkip>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    assert!(
        !ts.contains("AdjWithUntaggedSkip_Serialize"),
        "no phase split is needed for untagged variants:\n{ts}"
    );
}

// `#[specta(skip)]` is invisible to serde: the wire still carries the full
// payload symmetrically (`{"t":"V","c":7}` round-trips; a missing `c` and
// `c: null` are both rejected). The skip only *hides* the field from the
// export, so the variant renders as `{ t: "V" }` (no fabricated `c`), with
// no serde asymmetry: no unified-mode error and no `PhasesFormat` split.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjSpectaSkip {
    V(#[specta(skip)] u8),
    Live(String),
}

#[test]
fn adjacent_tagged_specta_skip_serde_ground_truth() {
    assert_eq!(
        serde_json::to_string(&AdjSpectaSkip::V(7)).unwrap(),
        r#"{"t":"V","c":7}"#
    );

    assert!(serde_json::from_str::<AdjSpectaSkip>(r#"{"t":"V","c":7}"#).is_ok());
    // serde never collapses this variant: `c` is required with a real value.
    assert!(serde_json::from_str::<AdjSpectaSkip>(r#"{"t":"V"}"#).is_err());
    assert!(serde_json::from_str::<AdjSpectaSkip>(r#"{"t":"V","c":null}"#).is_err());
}

#[test]
fn adjacent_tagged_specta_skip_unified() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjSpectaSkip>(),
            specta_serde::Format,
        )
        .expect("specta-only skips are not serde-asymmetric; unified export must succeed");

    assert!(
        ts.contains(r#"{ t: "V" }"#),
        "the hidden payload must omit `c` entirely (specta-skip hides the field):\n{ts}"
    );
    assert!(
        !ts.contains("c?: null") && !ts.contains(r#"{ t: "V"; c: null }"#),
        "must not fabricate a `null` content for a field serde still transports:\n{ts}"
    );
}

#[test]
fn adjacent_tagged_specta_skip_phases_no_split() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjSpectaSkip>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    assert!(
        !ts.contains("AdjSpectaSkip_Serialize"),
        "specta-only skips are symmetric on the wire; no phase split:\n{ts}"
    );
    assert!(
        ts.contains(r#"{ t: "V" }"#),
        "the hidden payload must omit `c` entirely:\n{ts}"
    );
}

// Multi-field variants hidden entirely with `#[specta(skip)]` follow the
// hidden-field convention too: serde still transports the values
// (`{"V":[7,8]}` externally, `{"t":"V","c":[7,8]}` adjacently -- both `"V"`
// and `[]` are rejected on deserialize), so the export must NOT fabricate an
// `[]` payload. It hides the payload like the pre-rewrite behavior did:
// external renders the bare variant string, adjacent omits `c`.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExtSpectaHidden {
    V(#[specta(skip)] u8, #[specta(skip)] u8),
    Live(String),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjSpectaHidden {
    V(#[specta(skip)] u8, #[specta(skip)] u8),
    Live(String),
}

#[test]
fn specta_hidden_multi_field_serde_ground_truth() {
    assert_eq!(
        serde_json::to_string(&ExtSpectaHidden::V(7, 8)).unwrap(),
        r#"{"V":[7,8]}"#
    );
    assert!(serde_json::from_str::<ExtSpectaHidden>(r#""V""#).is_err());
    assert!(serde_json::from_str::<ExtSpectaHidden>(r#"{"V":[]}"#).is_err());

    assert_eq!(
        serde_json::to_string(&AdjSpectaHidden::V(7, 8)).unwrap(),
        r#"{"t":"V","c":[7,8]}"#
    );
    assert!(serde_json::from_str::<AdjSpectaHidden>(r#"{"t":"V","c":[]}"#).is_err());
}

#[test]
fn specta_hidden_multi_field_external_typescript() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<ExtSpectaHidden>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    assert!(
        ts.contains(r#""V""#) && !ts.contains("V: []"),
        "hidden payload must collapse to the bare variant string, not fabricate `{{ V: [] }}`:\n{ts}"
    );
}

#[test]
fn specta_hidden_multi_field_adjacent_typescript() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjSpectaHidden>(),
            specta_serde::Format,
        )
        .expect("typescript export should succeed");

    assert!(
        ts.contains(r#"{ t: "V" }"#) && !ts.contains("c: []"),
        "hidden payload must omit `c`, not fabricate `c: []`:\n{ts}"
    );
}

// A user-defined type whose path merely ENDS in `Option` must not be
// mistaken for the std `Option`: serde's `missing_field` special case only
// applies to the real `std`/`core` `Option`, so a skipped `wire::Option<u8>`
// sole field behaves exactly like the non-`Option` case (serialize omits
// `c`, deserialize requires `c: null`).
mod wire {
    #[derive(serde::Serialize, serde::Deserialize, Default)]
    pub struct Option<T>(pub T);
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjCustomOptionSkip {
    V(#[serde(skip)] wire::Option<u8>),
    Live(String),
}

#[test]
fn adjacent_tagged_custom_option_skip_serde_ground_truth() {
    assert_eq!(
        serde_json::to_string(&AdjCustomOptionSkip::V(wire::Option(7))).unwrap(),
        r#"{"t":"V"}"#
    );

    // Unlike the real `Option`, a missing `c` is rejected; only `c: null`
    // is accepted.
    assert!(serde_json::from_str::<AdjCustomOptionSkip>(r#"{"t":"V"}"#).is_err());
    assert!(serde_json::from_str::<AdjCustomOptionSkip>(r#"{"t":"V","c":null}"#).is_ok());
}

#[test]
fn adjacent_tagged_custom_option_skip_unified_errors() {
    // The asymmetric non-`Option` rejection must apply: `wire::Option` gets
    // no `missing_field` special case from serde.
    let result = Typescript::default().export(
        &Types::default().register::<AdjCustomOptionSkip>(),
        specta_serde::Format,
    );

    let err = result.expect_err("unified export must reject the asymmetric shape");
    assert!(
        err.to_string().contains("PhasesFormat"),
        "error must point at `PhasesFormat`:\n{err}"
    );
}

#[test]
fn adjacent_tagged_custom_option_skip_phases() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjCustomOptionSkip>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    let deserialize_ty = ts
        .lines()
        .find(|line| line.contains("AdjCustomOptionSkip_Deserialize ="))
        .expect("deserialize type must be exported (the enum must split)");
    assert!(
        deserialize_ty.contains(r#"{ t: "V"; c: null }"#),
        "a custom `*::Option` skipped sole field must keep `c` required:\n{ts}"
    );
}

// Fully-qualified spellings of the real `Option` must still be recognized.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjQualifiedOptionSkip {
    A(#[serde(skip)] std::option::Option<u8>),
    B(#[serde(skip)] ::core::option::Option<u8>),
}

#[test]
fn adjacent_tagged_qualified_option_skip_unified() {
    assert!(serde_json::from_str::<AdjQualifiedOptionSkip>(r#"{"t":"A"}"#).is_ok());
    assert!(serde_json::from_str::<AdjQualifiedOptionSkip>(r#"{"t":"B"}"#).is_ok());

    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjQualifiedOptionSkip>(),
            specta_serde::Format,
        )
        .expect("qualified std/core Option must still be recognized as Option");

    assert!(
        ts.contains(r#"{ t: "A"; c?: null }"#) && ts.contains(r#"{ t: "B"; c?: null }"#),
        "qualified Option spellings must keep `c` optional:\n{ts}"
    );
}

// A `#[specta(type = Option<u8>)]` override must NOT be mistaken for a real
// `Option` field: serde only sees the actual Rust type (`u8`), so its
// deserializer still requires `c` to be present. `Option`-ness must be
// derived from the real field syntax, never from the exported datatype.
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjNullableOverrideSkipDe {
    V(
        #[serde(skip_deserializing)]
        #[specta(type = Option<u8>)]
        u8,
    ),
}

#[test]
fn adjacent_tagged_nullable_override_skip_deserializing_serde_ground_truth() {
    assert_eq!(
        serde_json::to_string(&AdjNullableOverrideSkipDe::V(7)).unwrap(),
        r#"{"t":"V","c":7}"#
    );

    // The real field is `u8`, so unlike a genuine `Option` the deserializer
    // rejects a missing `c` (serde's `missing_field` Option special case
    // does not apply) and only accepts `c: null`.
    assert!(serde_json::from_str::<AdjNullableOverrideSkipDe>(r#"{"t":"V"}"#).is_err());
    assert!(serde_json::from_str::<AdjNullableOverrideSkipDe>(r#"{"t":"V","c":null}"#).is_ok());
}

#[test]
fn adjacent_tagged_nullable_override_skip_deserializing_phases() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjNullableOverrideSkipDe>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    let deserialize_ty = ts
        .lines()
        .find(|line| line.contains("AdjNullableOverrideSkipDe_Deserialize ="))
        .expect("deserialize type must be exported");
    assert!(
        deserialize_ty.contains(r#"{ t: "V"; c: null }"#),
        "a nullable *override* on a non-Option field must keep `c` required:\n{ts}"
    );
    assert!(
        !deserialize_ty.contains("c?:"),
        "a nullable *override* on a non-Option field must not make `c` optional:\n{ts}"
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
// container attributes) with a bare `DataType::Tuple`, so the rename must be
// computed before the shape-changing rewrite runs (#525).
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(rename = "Wire")]
struct RenamedTupleSkip(String, #[serde(skip)] String);

#[test]
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
