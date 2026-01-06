use specta::Type;

use crate::ts::assert_ts_inline2;

#[derive(Type)]
#[specta(collect = false)]
enum A {
    A,
    B { id: String, method: String },
    C(String),
}

#[test]
fn externally_tagged() {
    // There is not way to construct an invalid externally tagged type.

    insta::assert_snapshot!(
        assert_ts_inline2::<A>().unwrap(),
        @r#""A" | { B: { id: string; method: string } } | { C: string }"#
    );
}
