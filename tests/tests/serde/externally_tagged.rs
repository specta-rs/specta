use specta::Type;

use crate::ts::assert_ts_inline2;

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

    assert_eq!(
        assert_ts_inline2::<A>(),
        Ok(r#""A" | { B: { id: string; method: string } } | { C: string }"#.into())
    );
}
