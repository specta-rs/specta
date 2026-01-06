use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
struct ExampleId(pub ulid::Ulid);

#[test]
fn ulid() {
    insta::assert_snapshot!(crate::ts::inline::<ulid::Ulid>(&Default::default()).unwrap(), @"string");
    insta::assert_snapshot!(crate::ts::inline::<ExampleId>(&Default::default()).unwrap(), @"string");
}
