use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ts::{assert_ts_export2, assert_ts_inline2};

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum A {
    A,
    B { id: String, method: String },
    C(String),
}

#[test]
fn adjacently_tagged() {
    // There is not way to construct an invalid adjacently tagged type.

    insta::assert_snapshot!(
        assert_ts_export2::<A>().unwrap(),
        @r#"export type A = { t: "A" } | { t: "B"; c: { id: string; method: string } } | { t: "C"; c: string };"#
    );
    insta::assert_snapshot!(
        assert_ts_inline2::<A>().unwrap(),
        @r#"{ t: "A" } | { t: "B"; c: { id: string; method: string } } | { t: "C"; c: string }"#
    );
}

// Test for https://github.com/specta-rs/specta/issues/395
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "event",
    content = "data"
)]
enum LoadProjectEvent {
    Started {
        project_name: String,
    },
    ProgressTest {
        project_name: String,
        status: String,
        progress: i32,
    },
    Finished {
        project_name: String,
    },
}

#[test]
fn adjacently_tagged_rename_all_fields() {
    // Test for https://github.com/specta-rs/specta/issues/395
    // The `rename_all_fields = "camelCase"` should convert field names to camelCase
    insta::assert_snapshot!(
        assert_ts_export2::<LoadProjectEvent>().unwrap(),
        @r#"export type LoadProjectEvent = { event: "started"; data: { projectName: string } } | { event: "progressTest"; data: { projectName: string; status: string; progress: number } } | { event: "finished"; data: { projectName: string } };"#
    );
    insta::assert_snapshot!(
        assert_ts_inline2::<LoadProjectEvent>().unwrap(),
        @r#"{ event: "started"; data: { projectName: string } } | { event: "progressTest"; data: { projectName: string; status: string; progress: number } } | { event: "finished"; data: { projectName: string } }"#
    );
}
