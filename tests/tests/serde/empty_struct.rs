use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(export = false)]
struct A {}

#[derive(Type)]
#[specta(export = false, tag = "a")]
struct B {}

// https://github.com/oscartbeaumont/specta/issues/174
#[test]
fn empty_enums() {
    assert_ts!(A, "Record<string, never>");
    assert_ts!(B, r#"{ a: "B" }"#);
}
