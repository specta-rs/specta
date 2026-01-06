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
    insta::assert_snapshot!(assert_ts_export2::<TaggedType>().unwrap(), @r#"
    export type TaggedType = { 
    		a: number,
    		b: number,
    		type: "TaggedType",
    	};
    "#);
    //. This might be unexpected but we are inling without NamedDataType so it's correct.
    insta::assert_snapshot!(assert_ts_inline2::<TaggedType>().unwrap(), @r#"{ a: number; b: number }"#);

    // TODO: Better unit tests for this including asserting runtime error for invalid cases
}
