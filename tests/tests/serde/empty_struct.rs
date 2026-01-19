use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ts::{assert_ts_export2, assert_ts_inline2};

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct A {}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "a")]
struct B {}

// https://github.com/specta-rs/specta/issues/174
#[test]
fn empty_enums() {
    insta::assert_snapshot!(assert_ts_export2::<A>().unwrap(), @r#"export type A = Record<string, never>;"#);
    insta::assert_snapshot!(assert_ts_inline2::<A>().unwrap(), @r#"Record<string, never>"#);
    insta::assert_snapshot!(assert_ts_export2::<B>().unwrap(), @r#"export type B = { "a": "B" };"#);
    // This may seem unexpected but without a NamedDataType the tag is not set
    insta::assert_snapshot!(assert_ts_inline2::<B>().unwrap(), @r#"Record<string, never>"#);
}
