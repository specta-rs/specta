#[cfg(feature = "serde_yaml")]
#[test]
fn serde_yaml() {
    use crate::ts::assert_ts;

    assert_ts!(
        serde_yaml::Value,
        "null | boolean | number | string | YamlValue[] | unknown | { [key in string]: unknown }"
    );
    assert_ts!(serde_yaml::Mapping, "unknown");
    assert_ts!(
        serde_yaml::value::TaggedValue,
        "{ [key in string]: unknown }"
    );
    assert_ts!(serde_yaml::Number, "number");
}
