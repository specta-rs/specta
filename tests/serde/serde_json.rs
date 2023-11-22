#[cfg(feature = "serde_json")]
#[test]
fn serde_json() {
    use crate::ts::assert_ts;

    assert_ts!(
        serde_json::Value,
        "null | boolean | number | string | Array<any> | Object<any>"
    );
    assert_ts!(serde_json::Map<(), ()>, "");
    assert_ts!(serde_json::Number, "");
}
