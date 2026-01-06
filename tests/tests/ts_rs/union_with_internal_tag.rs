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
fn test_enums_with_internal_tags() {
    insta::assert_snapshot!(crate::ts::inline::<EnumWithInternalTag>(&Default::default()).unwrap(), @r#"{ type: "A"; foo: string } | { type: "B"; bar: number }"#);

    insta::assert_snapshot!(crate::ts::inline::<EnumWithInternalTag2>(&Default::default()).unwrap(), @r#"({ type: "A" } & InnerA) | ({ type: "B" } & InnerB)"#);
}
