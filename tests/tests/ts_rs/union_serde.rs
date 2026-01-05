use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "kind", content = "d")]
enum SimpleEnumA {
    A,
    B,
}

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "kind", content = "data")]
enum ComplexEnum {
    A,
    B { foo: String, bar: f64 },
    W(SimpleEnumA),
    F { nested: SimpleEnumA },
    T(i32, SimpleEnumA),
}

#[derive(Type)]
#[specta(collect = false)]
#[serde(untagged)]
enum Untagged {
    Foo(String),
    Bar(i32),
    None,
}

#[test]
fn test_serde_enum() {
    insta::assert_snapshot!(crate::ts::inline::<SimpleEnumA>(&Default::default()).unwrap(), @r#"{ kind: "A" } | { kind: "B" }"#);
    insta::assert_snapshot!(crate::ts::inline::<ComplexEnum>(&Default::default()).unwrap(), @r#"{ kind: "A" } | { kind: "B"; data: { foo: string; bar: number } } | { kind: "W"; data: SimpleEnumA } | { kind: "F"; data: { nested: SimpleEnumA } } | { kind: "T"; data: [number, SimpleEnumA] }"#);
    insta::assert_snapshot!(crate::ts::inline::<Untagged>(&Default::default()).unwrap(), @r#"string | number | null"#);
}
