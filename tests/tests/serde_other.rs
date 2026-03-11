// Regression test for https://github.com/specta-rs/specta/issues/131

use serde::Deserialize;
use specta::{Type, TypeCollection};
use specta_typescript::{BigIntExportBehavior, Typescript};

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

#[test]
fn serde_other_requires_apply_phases() {
    let err = specta_serde::apply(TypeCollection::default().register::<InternalOther>())
        .expect_err("#[serde(other)] should require apply_phases");

    assert!(err
        .to_string()
        .contains("`#[serde(other)]` requires `apply_phases`"));
}

#[test]
fn serde_other_internal_tag_widens_deserialize_tag_to_string() {
    let types = specta_serde::apply_phases(TypeCollection::default().register::<InternalOther>())
        .expect("apply_phases should support internally tagged #[serde(other)] enums");
    let ts = Typescript::default()
        .bigint(BigIntExportBehavior::Number)
        .export(&types)
        .expect("typescript export should succeed");

    assert!(ts.contains("InternalOther_Serialize"));
    assert!(ts.contains("InternalOther_Deserialize"));
    assert!(ts.contains("kind: string"));
    assert!(ts.contains("kind: \"known\""));
}

#[test]
fn serde_other_adjacent_tag_widens_deserialize_tag_to_string() {
    let types = specta_serde::apply_phases(TypeCollection::default().register::<AdjacentOther>())
        .expect("apply_phases should support adjacently tagged #[serde(other)] enums");
    let ts = Typescript::default()
        .bigint(BigIntExportBehavior::Number)
        .export(&types)
        .expect("typescript export should succeed");

    assert!(ts.contains("AdjacentOther_Serialize"));
    assert!(ts.contains("AdjacentOther_Deserialize"));
    assert!(ts.contains("kind: string"));
    assert!(ts.contains("data: string"));
}
