#[test]
fn const_types() {
    insta::assert_snapshot!(crate::ts::inline::<(String, String)>(&Default::default()).unwrap(), @"[string, string]");
    insta::assert_snapshot!(crate::ts::inline::<[String; 5]>(&Default::default()).unwrap(), @"[string, string, string, string, string]");
    insta::assert_snapshot!(crate::ts::inline::<[String; 0]>(&Default::default()).unwrap(), @"[]");
}
