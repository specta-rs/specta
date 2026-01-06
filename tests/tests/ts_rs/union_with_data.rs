use serde::Serialize;
use specta::Type;

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct Bar {
    field: i32,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
struct Foo {
    bar: Bar,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
enum SimpleEnum2 {
    A(String),
    B(i32),
    C,
    D(String, i32),
    E(Foo),
    F { a: i32, b: String },
}

#[test]
fn test_stateful_enum() {
    insta::assert_snapshot!(crate::ts::inline::<Bar>(&Default::default()).unwrap(), @r#"{ field: number }"#);

    insta::assert_snapshot!(crate::ts::inline::<Foo>(&Default::default()).unwrap(), @r#"{ bar: Bar }"#);

    insta::assert_snapshot!(crate::ts::inline::<SimpleEnum2>(&Default::default()).unwrap(), @r#"{ A: string } | { B: number } | "C" | { D: [string, number] } | { E: Foo } | { F: { a: number; b: string } }"#);
}
