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
fn serde_other_requires_phases_format() {
    let err = Typescript::default()
        .export(
            &Types::default().register::<InternalOther>(),
            specta_serde::Format,
        )
        .expect_err("#[serde(other)] should require PhasesFormat");

    assert!(
        err.to_string()
            .contains("`#[serde(other)]` requires `PhasesFormat`")
    );
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
