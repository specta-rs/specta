use crate::ts::assert_ts_inline2;

#[test]
fn serde_json() {
    assert_eq!(assert_ts_inline2::<serde_json::Value>(), Ok(r#"null | boolean | number | string | JsonValue[] | Partial<{ [key in string]: JsonValue }>"#.into()));
    assert_eq!(
        assert_ts_inline2::<serde_json::Map<String, ()>>(),
        Ok(r#"Partial<{ [key in string]: null }>"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<serde_json::Number>(),
        Ok(r#"number"#.into())
    );
}
