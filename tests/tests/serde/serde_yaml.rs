use crate::ts::assert_ts_inline2;

#[test]
fn serde_yaml() {
    assert_eq!(assert_ts_inline2::<serde_yaml::Value>(), Ok(r#"null | boolean | number | string | YamlValue[] | Partial<{ [key in string]: unknown }>"#.into()));
    assert_eq!(
        assert_ts_inline2::<serde_yaml::Mapping>(),
        Ok(r#"unknown"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<serde_yaml::value::TaggedValue>(),
        Ok(r#"Partial<{ [key in string]: unknown }>"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<serde_yaml::Number>(),
        Ok(r#"number"#.into())
    );
}
