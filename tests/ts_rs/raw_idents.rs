use specta::Type;

use crate::ts::assert_ts;

#[allow(non_camel_case_types)]
#[derive(Type)]
struct r#struct {
    // r#type: i32, // TS reserved keyword
    r#use: i32,
    r#struct: i32,
    // r#let: i32, // TS reserved keyword
    // r#enum: i32, // TS reserved keyword
}

#[test]
fn raw_idents() {
    assert_ts!(r#struct, "{ use: number; struct: number }");
}
