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
fn serde_other_requires_format_phases() {
    let (map_types, _) = specta_serde::format;
    let err = map_types(&Types::default().register::<InternalOther>())
        .map(|types| types.into_owned())
        .expect_err("#[serde(other)] should require format_phases");

    assert!(
        err.to_string()
            .contains("`#[serde(other)]` requires `format_phases`")
    );
}

#[test]
fn serde_other_internal_tag_widens_deserialize_tag_to_string() {
    let (map_types, _) = specta_serde::format_phases;
    let types = map_types(&Types::default().register::<InternalOther>())
        .map(|types| types.into_owned())
        .expect("format_phases should support internally tagged #[serde(other)] enums");
    let ts = Typescript::default()
        .export(&types, crate::raw_format)
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-other-internal-tag-typescript", ts);
}

#[test]
fn serde_other_adjacent_tag_widens_deserialize_tag_to_string() {
    let (map_types, _) = specta_serde::format_phases;
    let types = map_types(&Types::default().register::<AdjacentOther>())
        .map(|types| types.into_owned())
        .expect("format_phases should support adjacently tagged #[serde(other)] enums");
    let ts = Typescript::default()
        .export(&types, crate::raw_format)
        .expect("typescript export should succeed");

    insta::assert_snapshot!("serde-other-adjacent-tag-typescript", ts);
}
