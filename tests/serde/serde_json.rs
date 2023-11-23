#[cfg(feature = "serde_json")]
#[test]
fn serde_json() {
    use crate::ts::assert_ts;

    assert_ts!(
        serde_json::Value,
        "null | boolean | number | string | JsonValue[] | { [key in string]: JsonValue }"
    );
    assert_ts!(serde_json::Map<String, ()>, "{ [key in string]: null }");
    assert_ts!(serde_json::Number, "number");
}
