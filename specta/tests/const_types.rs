use crate::ts::assert_ts;

#[test]
fn const_types() {
    assert_ts!((String, String), "[string, string]");
    assert_ts!([String; 5], "[string, string, string, string, string]");
    assert_ts!([String; 0], "[]");
}
