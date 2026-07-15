// Regression test for https://github.com/specta-rs/specta/issues/131

use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Type, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind")]
enum InternalOther {
    #[serde(rename = "known")]
    Known,
    #[serde(other)]
    Other,
}

#[derive(Type, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "kind", content = "data")]
enum AdjacentOther {
    #[serde(rename = "known")]
    Known(String),
    #[serde(other)]
    Other,
}

#[derive(Type, Deserialize)]
#[specta(collect = false)]
enum ExternalOther {
    #[serde(rename = "known")]
    Known,
    #[serde(other)]
    Other,
}

#[derive(Type, Deserialize)]
#[specta(collect = false)]
enum SkippedOther {
    Known,
    #[serde(other, skip)]
    Other,
}

#[derive(Clone, Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(from = "String", into = "String")]
enum ConvertedOther {
    Known,
    #[serde(other)]
    Other,
}

impl From<ConvertedOther> for String {
    fn from(_: ConvertedOther) -> Self {
        "known".to_string()
    }
}

impl From<String> for ConvertedOther {
    fn from(_: String) -> Self {
        Self::Known
    }
}

#[test]
fn serde_other_requires_phases_format() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<InternalOther>(),
            specta_serde::Format,
        )
        .expect_err("unified export cannot soundly exclude known tags");

    assert!(err.to_string().contains("requires `PhasesFormat`"));
}

#[test]
fn skipped_serde_other_does_not_require_phases() {
    Typescript::default()
        .export(
            &Types::default().register::<SkippedOther>(),
            specta_serde::Format,
        )
        .expect("a skipped fallback variant does not widen the wire shape");
}

#[test]
fn conversions_hide_serde_other() {
    Typescript::default()
        .export(
            &Types::default().register::<ConvertedOther>(),
            specta_serde::Format,
        )
        .expect("container conversions replace the declared enum tags");
}

#[test]
fn serde_other_internal_tag_widens_deserialize_tag_to_string() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<InternalOther>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-other-internal-tag-typescript", ts);
}

#[test]
fn serde_other_adjacent_tag_widens_deserialize_tag_to_string() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<AdjacentOther>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-other-adjacent-tag-typescript", ts);
}

#[test]
fn serde_other_external_tag_widens_deserialize_tag_to_string() {
    let ts = Typescript::default()
        .export(
            &Types::default().register::<ExternalOther>(),
            specta_serde::PhasesFormat,
        )
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-other-external-tag-typescript", ts);
}
