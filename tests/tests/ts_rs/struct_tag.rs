use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(export = false)]
#[serde(tag = "type")]
struct TaggedType {
    a: i32,
    b: i32,
}

#[test]
fn test_struct_tagging() {
    assert_ts!(
        TaggedType,
        r#"{ type: "TaggedType"; a: number; b: number }"#
    );

    // TODO: Better unit tests for this including asserting runtime error for invalid cases
}
