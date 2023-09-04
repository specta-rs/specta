use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(export = false, untagged)]
enum A {
    A { id: String },
    C(String),
    D(String, String),
}

#[derive(Type)]
#[serde(export = false, untagged)]
pub enum B {
    A,
    B,
}

#[test]
fn untagged() {
    // There is not way to construct an invalid untagged type.

    assert_ts!(A, "{ id: string } | string | [string, string]");
    assert_ts!(B, "null")
}
