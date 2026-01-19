use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
struct Skip {
    a: i32,
    b: i32,
    #[specta(skip)]
    c: String,
}

#[test]
fn test_def() {
    insta::assert_snapshot!(crate::ts::inline::<Skip>(&Default::default()).unwrap(), @"{ a: number; b: number }");
}
