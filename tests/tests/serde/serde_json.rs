use crate::ts::assert_ts_inline2;

#[test]
fn serde_json() {
    insta::assert_snapshot!(assert_ts_inline2::<serde_json::Value>().unwrap(), @r#"null | boolean | number | string | JsonValue[] | { [key in string]: JsonValue }"#);
    insta::assert_snapshot!(assert_ts_inline2::<serde_json::Map<String, ()>>().unwrap(), @r#"{ [key in string]: null }"#);
    insta::assert_snapshot!(assert_ts_inline2::<serde_json::Number>().unwrap(), @r#"number"#);
}
