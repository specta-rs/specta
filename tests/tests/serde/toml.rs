#[cfg(feature = "toml")]
#[test]
fn toml() {
    use crate::ts::assert_ts;

    assert_ts!(
        toml::Value,
        "string | number | boolean | Datetime | TomlValue[] | Partial<{ [key in string]: TomlValue }>"
    );
    assert_ts!(toml::map::Map<String, ()>, "Partial<{ [key in string]: null }>");
    assert_ts!(
        toml::value::Datetime,
        "{ $__toml_private_datetime: string }"
    );
}
