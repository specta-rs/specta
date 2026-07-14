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

#[derive(Type, Deserialize)]
#[specta(collect = false)]
enum ExternalOther {
    #[serde(rename = "known")]
    Known,
    #[serde(other)]
    Other,
}

#[test]
fn serde_other_unified_format_widens_deserialize_tag() {
    let ts = Typescript::default()
        .export(
            &Types::default()
                .register::<InternalOther>()
                .register::<AdjacentOther>()
                .register::<ExternalOther>(),
            specta_serde::Format,
        )
        .expect("unified typescript export should succeed");

    insta::assert_snapshot!("serde-other-unified-internal-tag-typescript", ts);
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
