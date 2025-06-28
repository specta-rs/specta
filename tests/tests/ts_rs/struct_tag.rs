use specta::Type;

use crate::ts::{assert_ts_export2, assert_ts_inline2};

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "type")]
struct TaggedType {
    a: i32,
    b: i32,
}

#[test]
fn test_struct_tagging() {
    assert_eq!(
        assert_ts_export2::<TaggedType>(),
        Ok(r#"export type TaggedType = { a: number; b: number; type: "TaggedType" };"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<TaggedType>(),
        //. This might be unexpected but we are inling without NamedDataType so it's correct.
        Ok(r#"{ a: number; b: number }"#.into())
    );

    // TODO: Better unit tests for this including asserting runtime error for invalid cases
}
