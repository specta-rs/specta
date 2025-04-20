use crate::ts::{assert_ts, assert_ts_inline2};

#[test]
fn toml() {
    assert_eq!(
        assert_ts_inline2::<toml::Value>(),
        Ok(r#""A" | { B: [number] }"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<toml::map::Map<String, ()>>(),
        Ok(r#""A" | { B: [number] }"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<toml::value::Datetime>(),
        Ok(r#"{ $__toml_private_datetime: string }"#.into())
    );
}
