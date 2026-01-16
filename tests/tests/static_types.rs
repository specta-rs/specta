use specta_typescript::{Any, Unknown};

#[test]
fn static_types() {
    insta::assert_snapshot!(crate::ts::inline::<Any>(&Default::default()).unwrap(), @"any");
    insta::assert_snapshot!(crate::ts::inline::<Unknown>(&Default::default()).unwrap(), @"unknown");

    insta::assert_snapshot!(crate::ts::inline::<Any<String>>(&Default::default()).unwrap(), @"any");
    insta::assert_snapshot!(crate::ts::inline::<Unknown<String>>(&Default::default()).unwrap(), @"unknown");
}
