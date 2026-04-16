// Regression test for https://github.com/specta-rs/specta/issues/131

use serde::Deserialize;
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

#[test]
fn serde_other_requires_apply_phases() {
    let err = specta_serde::apply(Types::default().register::<InternalOther>())
        .expect_err("#[serde(other)] should require apply_phases");

    assert!(
        err.to_string()
            .contains("`#[serde(other)]` requires `apply_phases`")
    );
}

#[test]
fn serde_other_internal_tag_widens_deserialize_tag_to_string() {
    let types = specta_serde::apply_phases(Types::default().register::<InternalOther>())
        .expect("apply_phases should support internally tagged #[serde(other)] enums");
    let ts = Typescript::default()
        .export(&types, crate::raw_format)
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-other-internal-tag-typescript", ts);
}

#[test]
fn serde_other_adjacent_tag_widens_deserialize_tag_to_string() {
    let types = specta_serde::apply_phases(Types::default().register::<AdjacentOther>())
        .expect("apply_phases should support adjacently tagged #[serde(other)] enums");
    let ts = Typescript::default()
        .export(&types, crate::raw_format)
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-other-adjacent-tag-typescript", ts);
}
