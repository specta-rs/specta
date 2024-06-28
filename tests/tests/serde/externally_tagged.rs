use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(export = false)]
enum A {
    A,
    B { id: String, method: String },
    C(String),
}

#[test]
fn externally_tagged() {
    // There is not way to construct an invalid externally tagged type.

    assert_ts!(
        A,
        "\"A\" | { B: { id: string; method: string } } | { C: string }"
    );
}
