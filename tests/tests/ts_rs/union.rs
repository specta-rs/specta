use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
enum SimpleEnum1 {
    #[serde(rename = "asdf")]
    A,
    B,
    #[serde(rename_all = "camelCase")]
    C {
        enum_field: (),
    },
}

#[test]
fn test_empty() {
    #[derive(Type)]
    #[specta(collect = false)]
    enum Empty {}

    insta::assert_snapshot!(crate::ts::inline::<Empty>(&Default::default()).unwrap(), @"never");
}

#[test]
fn test_simple_enum() {
    insta::assert_snapshot!(crate::ts::inline::<SimpleEnum1>(&Default::default()).unwrap(), @r#""asdf" | "B" | { C: { enumField: null } }"#);
}
