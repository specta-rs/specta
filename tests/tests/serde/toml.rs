use crate::ts::assert_ts_inline2;

#[test]
fn toml() {
    insta::assert_snapshot!(assert_ts_inline2::<toml::Value>().unwrap(), @r#"string | number | boolean | Datetime | TomlValue[] | { [key in string]: TomlValue }"#);
    insta::assert_snapshot!(assert_ts_inline2::<toml::map::Map<String, ()>>().unwrap(), @r#"{ [key in string]: null }"#);
    insta::assert_snapshot!(assert_ts_inline2::<toml::value::Datetime>().unwrap(), @r#"{ $__toml_private_datetime: string }"#);
}
