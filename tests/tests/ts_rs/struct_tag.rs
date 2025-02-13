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
        r#"{ a: number; b: number; type: "TaggedType" }"#
    );

    // TODO: Better unit tests for this including asserting runtime error for invalid cases
}
