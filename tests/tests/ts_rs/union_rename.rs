use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
#[specta(rename_all = "lowercase")]
#[specta(rename = "SimpleEnum")]
enum RenamedEnum {
    #[specta(rename = "ASDF")]
    A,
    B,
    C,
}

#[test]
fn test_simple_enum() {
    insta::assert_snapshot!(crate::ts::inline::<RenamedEnum>(&Default::default()).unwrap(), @r#""ASDF" | "b" | "c""#);
}
