use crate::ts::assert_ts;

use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum EnumWithInternalTag {
    A { foo: String },
    B { bar: i32 },
}

#[derive(Type)]
#[specta(collect = false)]
struct InnerA {
    foo: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct InnerB {
    bar: i32,
}

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "type")]
enum EnumWithInternalTag2 {
    A(InnerA),
    B(InnerB),
}

#[test]
#[cfg(feature = "serde")]
fn test_enums_with_internal_tags() {
    assert_ts!(
        EnumWithInternalTag,
        r#"{ type: "A"; foo: string } | { type: "B"; bar: number }"#
    );

    assert_ts!(
        EnumWithInternalTag2,
        r#"({ type: "A" } & InnerA) | ({ type: "B" } & InnerB)"#
    );
}
