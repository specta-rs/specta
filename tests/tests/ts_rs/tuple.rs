use specta::Type;

#[test]
fn test_tuple() {
    type Tuple = (String, i32, (i32, i32));
    insta::assert_snapshot!(crate::ts::inline::<Tuple>(&Default::default()).unwrap(), @"[string, number, [number, number]]");
}

#[test]
fn test_newtype() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct NewType(String);

    insta::assert_snapshot!(crate::ts::inline::<NewType>(&Default::default()).unwrap(), @"string");
}

#[test]
fn test_tuple_newtype() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct TupleNewType(String, i32, (i32, i32));
    insta::assert_snapshot!(crate::ts::inline::<TupleNewType>(&Default::default()).unwrap(), @"[string, number, [number, number]]");
}
