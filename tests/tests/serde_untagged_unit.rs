//! Regression tests for https://github.com/specta-rs/specta bug: container-level
//! `#[serde(untagged)]` enums export unit variants as their name string literal
//! instead of `null`.
//!
//! serde serializes a unit variant of an untagged enum as `null` (no
//! discriminant is ever written for untagged enums). Before the fix,
//! `rewrite_enum_repr_for_phase` early-returned for `EnumRepr::Untagged`
//! without touching any variant, so the exporter fell back to its
//! serde-agnostic default for a `Fields::Unit` enum variant: the variant's
//! name as a string literal (e.g. `"A"`). Every assertion below is anchored to
//! `serde_json::to_string` runtime evidence, not assumption.

use serde::Serialize;
use specta::{Type, Types};
use specta_serde::{Format, PhasesFormat};
use specta_typescript::Typescript;

/// A container-untagged enum covering all variant shapes: a true unit
/// variant, a newtype variant, a struct (named-fields) variant, and a
/// zero-arg tuple variant. serde gives each of these a *different* wire
/// shape, so this pins down that the fix only changes the unit-variant case.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum ContainerUntagged {
    Unit,
    Newtype(String),
    Struct { a: String },
    EmptyTuple(),
}

#[test]
fn serde_json_wire_shapes_for_container_untagged() {
    // Ground truth: what does serde actually put on the wire for each
    // variant kind of a container-level `#[serde(untagged)]` enum?
    assert_eq!(
        serde_json::to_string(&ContainerUntagged::Unit).unwrap(),
        "null",
        "serde serializes a unit variant of an untagged enum as `null`"
    );
    assert_eq!(
        serde_json::to_string(&ContainerUntagged::Newtype("hi".into())).unwrap(),
        "\"hi\""
    );
    assert_eq!(
        serde_json::to_string(&ContainerUntagged::Struct { a: "x".into() }).unwrap(),
        "{\"a\":\"x\"}"
    );
    assert_eq!(
        serde_json::to_string(&ContainerUntagged::EmptyTuple()).unwrap(),
        "[]",
        "a zero-arg tuple variant is NOT the same wire shape as a unit variant"
    );
}

#[test]
fn container_untagged_unit_variant_exports_as_null() {
    let ts = Typescript::default()
        .export(&Types::default().register::<ContainerUntagged>(), Format)
        .expect("typescript export should succeed");

    // Matches the serde_json evidence above: null | string | { a: string } | [].
    insta::assert_snapshot!("serde-untagged-unit-container", ts);
}

#[test]
fn container_untagged_unit_variant_exports_as_null_under_phases_format() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<ContainerUntagged>(),
            PhasesFormat,
        )
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-untagged-unit-container-phases", ts);
}

/// Same shapes as [`ContainerUntagged`], but the unit variant carries an
/// asymmetric `#[serde(rename(serialize = ..., deserialize = ...))]`. Renames
/// are wire-irrelevant for untagged enums, but the asymmetry is enough to
/// flag `variant_has_local_difference`, forcing `PhasesFormat` to split this
/// enum into `_Serialize`/`_Deserialize` definitions.
///
/// `PhasesFormat` rewrites a split type's `DataType` twice: once when first
/// building the `_Serialize`/`_Deserialize` clone, and again in a second pass
/// once all split types are registered (so cross-references resolve). This
/// type exists purely to exercise that double-rewrite path and confirm the
/// unit -> `null` rewrite doesn't double-transform (e.g. re-wrapping the
/// already-rewritten variant, or erroring out).
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum ContainerUntaggedForcedSplit {
    #[serde(rename(serialize = "UNIT_ON_SERIALIZE", deserialize = "unit_on_deserialize"))]
    Unit,
    Newtype(String),
    Struct {
        a: String,
    },
    EmptyTuple(),
}

#[test]
fn container_untagged_unit_variant_survives_double_rewrite_when_split() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<ContainerUntaggedForcedSplit>(),
            PhasesFormat,
        )
        .expect("typescript export should succeed");

    // Both the `_Serialize` and `_Deserialize` definitions must independently
    // land on `null | string | { a: string } | []` -- the rename asymmetry
    // only forces the split, it doesn't change the wire shape.
    insta::assert_snapshot!("serde-untagged-unit-container-forced-split", ts);
}

/// Control: a *variant-level* `#[serde(untagged)]` unit variant (mixed in
/// with tagged variants) already rendered as `null` before this fix --
/// `transform_untagged_variant` already handled that case correctly. This
/// must keep working.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum MixedTaggedAndUntaggedUnitControl {
    Tagged {
        value: String,
    },
    #[serde(untagged)]
    Empty,
}

#[test]
fn variant_level_untagged_unit_still_renders_null() {
    assert_eq!(
        serde_json::to_string(&MixedTaggedAndUntaggedUnitControl::Empty).unwrap(),
        "null"
    );

    let ts = Typescript::default()
        .export(
            &Types::default().register::<MixedTaggedAndUntaggedUnitControl>(),
            Format,
        )
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-untagged-unit-variant-level-control", ts);
}

/// A container-untagged enum where every variant is a unit variant: every
/// variant serializes to the exact same wire value (`null`), so the exported
/// union should dedup down to a single `null`, matching serde exactly (there
/// is no way to distinguish `A`, `B`, or `C` on the wire).
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum AllUnitUntagged {
    A,
    B,
    C,
}

#[test]
fn all_unit_untagged_variants_dedup_to_a_single_null() {
    for value in [AllUnitUntagged::A, AllUnitUntagged::B, AllUnitUntagged::C] {
        assert_eq!(serde_json::to_string(&value).unwrap(), "null");
    }

    let ts = Typescript::default()
        .export(&Types::default().register::<AllUnitUntagged>(), Format)
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-untagged-unit-all-unit", ts);
}

/// A container-untagged enum with a `#[serde(skip)]`ped unit variant. Skipped
/// variants are filtered out (`filter_enum_variants_for_phase` runs before
/// `rewrite_enum_repr_for_phase`) so the skipped unit variant must be dropped
/// entirely -- not rendered as `null`.
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntaggedWithSkippedUnit {
    A(String),
    #[serde(skip)]
    B,
}

#[test]
fn skipped_unit_variant_of_untagged_enum_stays_dropped() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<UntaggedWithSkippedUnit>(),
            Format,
        )
        .expect("typescript export should succeed");

    assert!(
        ts.contains("export type UntaggedWithSkippedUnit = string;"),
        "skipped unit variant must not surface as `null`: {ts}"
    );
}

/// A container-untagged enum with a unit variant serializes to `null`, which
/// serde_json rejects as a map key at runtime (`"key must be a string"`).
/// Before this fix the unit variant survived as `Fields::Unit`, which map-key
/// validation accepts (it is a valid string key for *tagged* enums), so specta
/// exported a `"A" | "B"` key type that serde could never produce. Now that
/// the variant is rewritten to its real `null` wire shape, export fails
/// loudly instead of emitting unsound bindings.
#[derive(Type, Serialize, PartialEq, Eq, Hash)]
#[specta(collect = false)]
#[serde(untagged)]
enum AllUnitUntaggedKey {
    A,
    B,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct UntaggedUnitMapKey {
    map: std::collections::HashMap<AllUnitUntaggedKey, String>,
}

#[test]
fn untagged_unit_enum_as_map_key_fails_export_like_serde_fails_at_runtime() {
    // Ground truth: serde_json cannot serialize this map at all.
    let mut map = std::collections::HashMap::new();
    map.insert(AllUnitUntaggedKey::A, "x".to_string());
    let runtime = serde_json::to_string(&map).unwrap_err();
    assert!(
        runtime.to_string().contains("key must be a string"),
        "unexpected serde_json error: {runtime}"
    );

    let err = Typescript::default()
        .export(&Types::default().register::<UntaggedUnitMapKey>(), Format)
        .expect_err("a null-serializing map key must be rejected at export time");

    assert!(
        err.to_string().contains("Invalid map key"),
        "unexpected export error: {err}"
    );
}
