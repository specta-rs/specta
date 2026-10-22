// Regression test: `#[serde(flatten)]` on a named field inside an externally
// tagged, adjacently tagged, or untagged enum variant.
//
// Serde merges the flattened field's keys into the variant payload object, but
// Specta used to copy the field verbatim and export a literal nested key that
// never appears on the wire (the export succeeded, it was just wrong). Only
// internally tagged variants lowered flatten. Named payloads now share the
// struct flatten/alias pipeline for every representation.
//
// https://github.com/specta-rs/specta/issues/592

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Debug, Clone, PartialEq, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct Inner {
    b: i32,
}

impl Inner {
    fn is_zero(&self) -> bool {
        self.b == 0
    }
}

#[derive(Debug, Clone, PartialEq, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct GenericInner<T> {
    value: T,
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExtMandatory {
    V {
        x: i32,
        #[serde(flatten)]
        inner: Inner,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjMandatory {
    V {
        x: i32,
        #[serde(flatten)]
        inner: Inner,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntMandatory {
    V {
        x: i32,
        #[serde(flatten)]
        inner: Inner,
    },
}

/// Internally tagged control: this path already lowered flatten before the fix.
#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum IntMandatory {
    V {
        x: i32,
        #[serde(flatten)]
        inner: Inner,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExtOptional {
    V {
        x: i32,
        #[serde(flatten)]
        inner: Option<Inner>,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjOptional {
    V {
        x: i32,
        #[serde(flatten)]
        inner: Option<Inner>,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntOptional {
    V {
        x: i32,
        #[serde(flatten)]
        inner: Option<Inner>,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExtConditional {
    V {
        x: i32,
        #[serde(flatten, skip_serializing_if = "Inner::is_zero")]
        inner: Inner,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjConditional {
    V {
        x: i32,
        #[serde(flatten, skip_serializing_if = "Inner::is_zero")]
        inner: Inner,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntConditional {
    V {
        x: i32,
        #[serde(flatten, skip_serializing_if = "Inner::is_zero")]
        inner: Inner,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum IntConditional {
    V {
        x: i32,
        #[serde(flatten, skip_serializing_if = "Inner::is_zero")]
        inner: Inner,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExtMap {
    V {
        x: i32,
        #[serde(flatten)]
        rest: HashMap<String, i32>,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntMap {
    V {
        x: i32,
        #[serde(flatten)]
        rest: HashMap<String, i32>,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExtGeneric {
    V {
        x: i32,
        #[serde(flatten)]
        inner: GenericInner<String>,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntGeneric {
    V {
        x: i32,
        #[serde(flatten)]
        inner: GenericInner<String>,
    },
}

/// A `#[serde(skip)]` sibling must not resurrect a synthetic key, and must not
/// stop the remaining flattened field from being merged.
#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExtSkipped {
    V {
        x: i32,
        #[serde(skip)]
        gone: i32,
        #[serde(flatten)]
        inner: Inner,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntSkipped {
    V {
        x: i32,
        #[serde(skip)]
        gone: i32,
        #[serde(flatten)]
        inner: Inner,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasedFlattenPayload {
    #[serde(alias = "outer_key")]
    value: String,
}

/// The flattened payload's alias collides with an outer key.
#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExtAliasCollision {
    V {
        outer_key: String,
        #[serde(flatten)]
        inner: AliasedFlattenPayload,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum AdjAliasCollision {
    V {
        outer_key: String,
        #[serde(flatten)]
        inner: AliasedFlattenPayload,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
enum UntAliasCollision {
    V {
        outer_key: String,
        #[serde(flatten)]
        inner: AliasedFlattenPayload,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct AliasedSiblingPayload {
    y: String,
}

/// An alias on a sibling of the flattened field.
#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
enum ExtAliasSibling {
    V {
        #[serde(alias = "old_x")]
        x: String,
        #[serde(flatten)]
        payload: AliasedSiblingPayload,
    },
}

#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum IntAliasSibling {
    V {
        #[serde(alias = "old_x")]
        x: String,
        #[serde(flatten)]
        payload: AliasedSiblingPayload,
    },
}

/// A variant-level `#[serde(untagged)]` inside an internally tagged container
/// drops the container's tag, so its payload is flattened like any other
/// untagged variant rather than lowered against the tag.
#[derive(Debug, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t")]
enum IntWithUntaggedVariant {
    Other {
        y: i32,
    },
    #[serde(untagged)]
    V {
        x: i32,
        #[serde(flatten)]
        inner: Inner,
    },
}

fn export_unified<T: Type>() -> String {
    Typescript::default()
        .export(&Types::default().register::<T>(), specta_serde::Format)
        .expect("export should succeed")
}

fn export_phased<T: Type>() -> String {
    Typescript::default()
        .export(
            &Types::default().register::<T>(),
            specta_serde::PhasesFormat,
        )
        .expect("export should succeed")
}

/// The bug's signature: a literal `inner`/`rest` key that never appears on the
/// serde wire.
fn assert_no_synthetic_key(rendered: &str, key: &str) {
    for needle in [format!("\t{key}:"), format!("{{{key}:")] {
        assert!(
            !rendered.contains(&needle),
            "synthetic `{key}` key leaked into the export:\n{rendered}"
        );
    }
}

// --- Serde wire evidence, so the exported types can be checked against what
// serde actually emits rather than against assumptions. ---

#[test]
fn mandatory_flatten_serde_wire_evidence() {
    assert_eq!(
        serde_json::to_string(&ExtMandatory::V {
            x: 1,
            inner: Inner { b: 2 },
        })
        .unwrap(),
        r#"{"V":{"x":1,"b":2}}"#
    );
    assert_eq!(
        serde_json::to_string(&AdjMandatory::V {
            x: 1,
            inner: Inner { b: 2 },
        })
        .unwrap(),
        r#"{"t":"V","c":{"x":1,"b":2}}"#
    );
    assert_eq!(
        serde_json::to_string(&UntMandatory::V {
            x: 1,
            inner: Inner { b: 2 },
        })
        .unwrap(),
        r#"{"x":1,"b":2}"#
    );
    assert_eq!(
        serde_json::to_string(&IntMandatory::V {
            x: 1,
            inner: Inner { b: 2 },
        })
        .unwrap(),
        r#"{"t":"V","x":1,"b":2}"#
    );
}

#[test]
fn optional_flatten_serde_wire_evidence() {
    // `None` contributes nothing; `Some` merges the payload's keys.
    assert_eq!(
        serde_json::to_string(&ExtOptional::V { x: 1, inner: None }).unwrap(),
        r#"{"V":{"x":1}}"#
    );
    assert_eq!(
        serde_json::to_string(&ExtOptional::V {
            x: 1,
            inner: Some(Inner { b: 2 }),
        })
        .unwrap(),
        r#"{"V":{"x":1,"b":2}}"#
    );
    assert_eq!(
        serde_json::to_string(&AdjOptional::V { x: 1, inner: None }).unwrap(),
        r#"{"t":"V","c":{"x":1}}"#
    );
    assert_eq!(
        serde_json::to_string(&AdjOptional::V {
            x: 1,
            inner: Some(Inner { b: 2 }),
        })
        .unwrap(),
        r#"{"t":"V","c":{"x":1,"b":2}}"#
    );
    assert_eq!(
        serde_json::to_string(&UntOptional::V { x: 1, inner: None }).unwrap(),
        r#"{"x":1}"#
    );
    assert_eq!(
        serde_json::to_string(&UntOptional::V {
            x: 1,
            inner: Some(Inner { b: 2 }),
        })
        .unwrap(),
        r#"{"x":1,"b":2}"#
    );

    let de: ExtOptional = serde_json::from_str(r#"{"V":{"x":1}}"#).unwrap();
    match de {
        ExtOptional::V { inner, .. } => assert!(inner.is_none()),
    }
}

#[test]
fn conditional_flatten_serde_wire_evidence() {
    for omitted in [
        serde_json::to_string(&ExtConditional::V {
            x: 1,
            inner: Inner { b: 0 },
        })
        .unwrap(),
        serde_json::to_string(&AdjConditional::V {
            x: 1,
            inner: Inner { b: 0 },
        })
        .unwrap(),
        serde_json::to_string(&UntConditional::V {
            x: 1,
            inner: Inner { b: 0 },
        })
        .unwrap(),
        serde_json::to_string(&IntConditional::V {
            x: 1,
            inner: Inner { b: 0 },
        })
        .unwrap(),
    ] {
        assert!(!omitted.contains("\"b\""), "{omitted}");
    }

    assert_eq!(
        serde_json::to_string(&ExtConditional::V {
            x: 1,
            inner: Inner { b: 2 },
        })
        .unwrap(),
        r#"{"V":{"x":1,"b":2}}"#
    );
    assert_eq!(
        serde_json::to_string(&UntConditional::V {
            x: 1,
            inner: Inner { b: 2 },
        })
        .unwrap(),
        r#"{"x":1,"b":2}"#
    );
}

#[test]
fn map_flatten_serde_wire_evidence() {
    let rest: HashMap<String, i32> = [("k".to_string(), 9)].into_iter().collect();
    assert_eq!(
        serde_json::to_string(&ExtMap::V {
            x: 1,
            rest: rest.clone(),
        })
        .unwrap(),
        r#"{"V":{"x":1,"k":9}}"#
    );
    assert_eq!(
        serde_json::to_string(&UntMap::V { x: 1, rest }).unwrap(),
        r#"{"x":1,"k":9}"#
    );
}

#[test]
fn generic_flatten_serde_wire_evidence() {
    assert_eq!(
        serde_json::to_string(&ExtGeneric::V {
            x: 1,
            inner: GenericInner { value: "s".into() },
        })
        .unwrap(),
        r#"{"V":{"x":1,"value":"s"}}"#
    );
    assert_eq!(
        serde_json::to_string(&UntGeneric::V {
            x: 1,
            inner: GenericInner { value: "s".into() },
        })
        .unwrap(),
        r#"{"x":1,"value":"s"}"#
    );
}

#[test]
fn skipped_sibling_serde_wire_evidence() {
    assert_eq!(
        serde_json::to_string(&ExtSkipped::V {
            x: 1,
            gone: 7,
            inner: Inner { b: 2 },
        })
        .unwrap(),
        r#"{"V":{"x":1,"b":2}}"#
    );
    assert_eq!(
        serde_json::to_string(&UntSkipped::V {
            x: 1,
            gone: 7,
            inner: Inner { b: 2 },
        })
        .unwrap(),
        r#"{"x":1,"b":2}"#
    );
}

// --- Exported TypeScript, checked against the serde evidence above. ---

#[test]
fn mandatory_flatten_merges_into_every_variant_payload() {
    let expected = [
        (
            "ExtMandatory",
            export_unified::<ExtMandatory>(),
            "export type ExtMandatory = { V: {\n\tx: number,\n} & Inner };",
        ),
        (
            "AdjMandatory",
            export_unified::<AdjMandatory>(),
            "export type AdjMandatory = { t: \"V\"; c: {\n\tx: number,\n} & Inner };",
        ),
        (
            "UntMandatory",
            export_unified::<UntMandatory>(),
            "export type UntMandatory = {\n\tx: number,\n} & Inner;",
        ),
        (
            "IntMandatory",
            export_unified::<IntMandatory>(),
            "export type IntMandatory = {\n\tt: \"V\",\n} & {\n\tx: number,\n} & Inner;",
        ),
    ];

    for (name, rendered, expected) in expected {
        assert!(
            rendered.contains(expected),
            "{name}: expected:\n{expected}\ngot:\n{rendered}"
        );
        assert_no_synthetic_key(&rendered, "inner");
    }

    // The unified and phased shapes agree for a symmetric payload.
    assert_eq!(
        export_phased::<ExtMandatory>(),
        export_unified::<ExtMandatory>()
    );
    assert_eq!(
        export_phased::<AdjMandatory>(),
        export_unified::<AdjMandatory>()
    );
    assert_eq!(
        export_phased::<UntMandatory>(),
        export_unified::<UntMandatory>()
    );
}

#[test]
fn optional_flatten_becomes_present_or_absent_branches() {
    let ext = export_unified::<ExtOptional>();
    assert!(
        ext.contains(
            "export type ExtOptional = { V: {\n\tx: number,\n} & Inner | {\n\tx: number,\n} };"
        ),
        "{ext}"
    );
    assert_no_synthetic_key(&ext, "inner");

    let adj = export_unified::<AdjOptional>();
    assert!(
        adj.contains(
            "export type AdjOptional = { t: \"V\"; c: {\n\tx: number,\n} & Inner | {\n\tx: number,\n} };"
        ),
        "{adj}"
    );

    let unt = export_unified::<UntOptional>();
    assert!(
        unt.contains("export type UntOptional = {\n\tx: number,\n} & Inner | {\n\tx: number,\n};"),
        "{unt}"
    );
    assert_no_synthetic_key(&unt, "inner");

    // A flattened `Option<T>` never admits a bare `null` on the wire.
    for rendered in [&ext, &adj, &unt] {
        assert!(!rendered.contains("| null"), "{rendered}");
    }
}

#[test]
fn conditional_flatten_splits_serialize_and_deserialize() {
    let unified = export_unified::<ExtConditional>();
    assert!(
        unified.contains(
            "export type ExtConditional = { V: {\n\tx: number,\n} & Inner | {\n\tx: number,\n} };"
        ),
        "{unified}"
    );
    assert_no_synthetic_key(&unified, "inner");

    let phased = export_phased::<ExtConditional>();
    assert!(
        phased.contains("export type ExtConditional_Serialize = { V: {\n\tx: number,\n} & Inner | {\n\tx: number,\n} };"),
        "{phased}"
    );
    assert!(
        phased.contains(
            "export type ExtConditional_Deserialize = { V: {\n\tx: number,\n} & Inner };"
        ),
        "{phased}"
    );
    assert_no_synthetic_key(&phased, "inner");

    let unt = export_phased::<UntConditional>();
    assert!(
        unt.contains("export type UntConditional_Serialize = {\n\tx: number,\n} & Inner | {\n\tx: number,\n};"),
        "{unt}"
    );
    assert!(
        unt.contains("export type UntConditional_Deserialize = {\n\tx: number,\n} & Inner;"),
        "{unt}"
    );
}

#[test]
fn map_flatten_merges_the_catch_all() {
    let ext = export_unified::<ExtMap>();
    assert!(
        ext.contains(
            "export type ExtMap = { V: {\n\tx: number,\n} & { [key in string]: number } };"
        ),
        "{ext}"
    );
    assert_no_synthetic_key(&ext, "rest");

    let unt = export_unified::<UntMap>();
    assert!(
        unt.contains("export type UntMap = {\n\tx: number,\n} & { [key in string]: number };"),
        "{unt}"
    );
    assert_no_synthetic_key(&unt, "rest");
}

#[test]
fn generic_flatten_keeps_the_instantiation() {
    let ext = export_unified::<ExtGeneric>();
    assert!(
        ext.contains("export type ExtGeneric = { V: {\n\tx: number,\n} & GenericInner<string> };"),
        "{ext}"
    );
    assert_no_synthetic_key(&ext, "inner");

    let unt = export_unified::<UntGeneric>();
    assert!(
        unt.contains("export type UntGeneric = {\n\tx: number,\n} & GenericInner<string>;"),
        "{unt}"
    );
}

#[test]
fn skipped_siblings_do_not_resurrect_keys() {
    let ext = export_unified::<ExtSkipped>();
    assert!(
        ext.contains("export type ExtSkipped = { V: {\n\tx: number,\n} & Inner };"),
        "{ext}"
    );
    assert_no_synthetic_key(&ext, "inner");
    assert_no_synthetic_key(&ext, "gone");

    let unt = export_unified::<UntSkipped>();
    assert!(
        unt.contains("export type UntSkipped = {\n\tx: number,\n} & Inner;"),
        "{unt}"
    );
    assert_no_synthetic_key(&unt, "gone");
}

#[test]
fn flattened_alias_collisions_relax_the_outer_spelling() {
    let value = ExtAliasCollision::V {
        outer_key: "outer".into(),
        inner: AliasedFlattenPayload {
            value: "inner".into(),
        },
    };
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        serde_json::json!({ "V": { "outer_key": "outer", "value": "inner" } })
    );

    for rendered in [
        export_unified::<ExtAliasCollision>(),
        export_unified::<AdjAliasCollision>(),
        export_unified::<UntAliasCollision>(),
    ] {
        // The flattened payload must not exclude the colliding outer key: the
        // canonical `value` spelling has to stay satisfiable.
        assert!(
            rendered.contains(
                "({\n\tvalue: string,\n} | {\n\touter_key: string,\n\tvalue?: never,\n})"
            ),
            "{rendered}"
        );
        assert_no_synthetic_key(&rendered, "inner");
    }

    // Deserialize genuinely rejects the duplicate spelling, so the phase shape
    // keeps the exclusion.
    let phased = export_phased::<ExtAliasCollision>();
    assert!(
        phased.contains(
            "export type ExtAliasCollision_Deserialize = { V: {\n\touter_key: string,\n} & ({\n\tvalue: string,\n\touter_key?: never,\n} | {\n\touter_key: string,\n\tvalue?: never,\n}) };"
        ),
        "{phased}"
    );
    assert!(
        phased.contains("export type ExtAliasCollision_Serialize = { V: {\n\touter_key: string,\n} & {\n\tvalue: string,\n} };"),
        "{phased}"
    );
}

#[test]
fn variant_level_untagged_inside_an_internal_tag_is_flattened() {
    assert_eq!(
        serde_json::to_string(&IntWithUntaggedVariant::V {
            x: 1,
            inner: Inner { b: 2 },
        })
        .unwrap(),
        r#"{"x":1,"b":2}"#
    );

    for rendered in [
        export_unified::<IntWithUntaggedVariant>(),
        export_phased::<IntWithUntaggedVariant>(),
    ] {
        assert!(
            rendered.contains(
                "export type IntWithUntaggedVariant = { t: \"Other\"; y: number } | {\n\tx: number,\n} & Inner;"
            ),
            "{rendered}"
        );
        assert_no_synthetic_key(&rendered, "inner");
    }
}

#[test]
fn aliases_beside_a_flattened_field_are_preserved() {
    let value = ExtAliasSibling::V {
        x: "x".into(),
        payload: AliasedSiblingPayload { y: "y".into() },
    };
    assert_eq!(
        serde_json::to_value(value).unwrap(),
        serde_json::json!({ "V": { "x": "x", "y": "y" } })
    );

    let expected = "export type ExtAliasSibling = { V: ({\n\tx: string,\n\told_x?: never,\n} | {\n\told_x: string,\n\tx?: never,\n}) & AliasedSiblingPayload };";
    let ext = export_unified::<ExtAliasSibling>();
    assert!(ext.contains(expected), "{ext}");
    assert_no_synthetic_key(&ext, "payload");

    // The internally tagged control lowers the same payload identically.
    let int = export_unified::<IntAliasSibling>();
    assert!(
        int.contains("{\n\tx: string,\n\told_x?: never,\n} | {\n\told_x: string,\n\tx?: never,\n}"),
        "{int}"
    );
}
