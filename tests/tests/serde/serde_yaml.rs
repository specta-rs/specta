use crate::ts::assert_ts_inline2;

#[test]
fn serde_yaml() {
    insta::assert_snapshot!(assert_ts_inline2::<serde_yaml::Value>().unwrap(), @r#"null | boolean | number | string | YamlValue[] | Partial<{ [key in YamlValue]: YamlValue }> | { [key in string]: YamlValue }"#);
    insta::assert_snapshot!(assert_ts_inline2::<serde_yaml::Mapping>().unwrap(), @r#"Partial<{ [key in null | boolean | number | string | YamlValue[] | Partial<{ [key in YamlValue]: YamlValue }> | { [key in string]: YamlValue }]: null | boolean | number | string | YamlValue[] | Partial<{ [key in YamlValue]: YamlValue }> | { [key in string]: YamlValue } }>"#);
    insta::assert_snapshot!(assert_ts_inline2::<serde_yaml::value::TaggedValue>().unwrap(), @r#"{ [key in string]: null | boolean | number | string | YamlValue[] | Partial<{ [key in YamlValue]: YamlValue }> | { [key in string]: YamlValue } }"#);
    insta::assert_snapshot!(assert_ts_inline2::<serde_yaml::Number>().unwrap(), @r#"number"#);
}
