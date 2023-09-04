use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(export = false, tag = "t", content = "c")]
enum A {
    A,
    B { id: String, method: String },
    C(String),
}

#[test]
fn adjacently_tagged() {
    // There is not way to construct an invalid adjacently tagged type.

    assert_ts!(
        A,
        "{ t: \"A\" } | { t: \"B\"; c: { id: string; method: string } } | { t: \"C\"; c: string }"
    );
}
