use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
struct UnitA;

#[derive(Type)]
#[specta(collect = false)]
struct UnitB {}

#[derive(Type)]
#[specta(collect = false)]
struct UnitC();

#[test]
fn test() {
    insta::assert_snapshot!(crate::ts::inline::<UnitA>(&Default::default()).unwrap(), @"null");
    insta::assert_snapshot!(crate::ts::inline::<UnitB>(&Default::default()).unwrap(), @"Record<string, never>");
    insta::assert_snapshot!(crate::ts::inline::<UnitC>(&Default::default()).unwrap(), @"[]");
}
