use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
#[serde(rename_all = "lowercase")]
#[serde(rename = "SimpleEnum")]
enum RenamedEnum {
    #[serde(rename = "ASDF")]
    A,
    B,
    C,
}

#[test]
fn test_simple_enum() {
    insta::assert_snapshot!(crate::ts::inline::<RenamedEnum>(&Default::default()).unwrap(), @r#""ASDF" | "b" | "c""#);
}
