use specta::Type;

#[allow(non_camel_case_types)]
#[derive(Type)]
#[specta(collect = false)]
struct r#struct {
    // r#type: i32, // TS reserved keyword
    r#use: i32,
    r#struct: i32,
    // r#let: i32, // TS reserved keyword
    // r#enum: i32, // TS reserved keyword
}

#[test]
fn raw_idents() {
    insta::assert_snapshot!(crate::ts::inline::<r#struct>(&Default::default()).unwrap(), @"{ use: number; struct: number }");
}
