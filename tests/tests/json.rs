use serde::Serialize;
use specta::json;

use crate::ts::assert_ts;

#[derive(Serialize)]
pub struct Demo {
    a: String,
}

// TODO: Assert types
// TODO: Assert JSON results are correct

#[test]
fn test_json_macro() {
    assert_ts!(() => json!(null), "null");
    assert_ts!(() => json!(true), "true");
    assert_ts!(() => json!(false), "false");

    assert_ts!(() => json!({}), "Record<string, never>");

    // TODO: Fix these
    // assert_ts!(() => json!({ "a": "b" }), r#"{ "a": "b" }"#);
    // assert_ts!(() => json!({ "hello": "world" }), "{ hello: string }");
    // assert_ts!(() => json!({
    //     "hello": "world",
    // }), "{ hello: string }");
    // assert_ts!(() => json!({ "a": 5, "c": true, "d": false, "e": 42u8, "f": 2.7 }), r#""#);
    // assert_ts!(() => json!([]), "[]");
    // assert_ts!(() => json!(["a", "b", "c"]), "string[]");
    // assert_ts!(() => json!([{}, {}, {}]), "");
    // assert_ts!(() => json!([{ "n": "0" }, { "n": "1" }, { "n": "2" }]), "");
}
