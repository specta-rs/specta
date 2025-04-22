use crate::ts::{assert_ts, assert_ts_inline2};

#[test]
fn toml() {
    assert_eq!(
        assert_ts_inline2::<toml::Value>(),
        Ok(r#"string | number | boolean | Datetime | TomlValue[] | { [key in string]: TomlValue }"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<toml::map::Map<String, ()>>(),
        Ok(r#"{ [key in string]: null }"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<toml::value::Datetime>(),
        Ok(r#"{ $__toml_private_datetime: string }"#.into())
    );
}
