use specta::Type;

use crate::ts::{assert_ts_export2, assert_ts_inline2};

#[derive(Type)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
enum A {
    A,
    B { id: String, method: String },
    C(String),
}

#[test]
fn adjacently_tagged() {
    // There is not way to construct an invalid adjacently tagged type.

    // insta::assert_debug_snapshot!(assert_ts_export2::<A>());
    // insta::assert_debug_snapshot!(assert_ts_inline2::<A>());

    assert_eq!(
        assert_ts_export2::<A>(),
        Ok(r#"export type A = { t: "A" } | { t: "B"; c: { id: string; method: string } } | { t: "C"; c: string };"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<A>(),
        Ok(
            r#"{ t: "A" } | { t: "B"; c: { id: string; method: string } } | { t: "C"; c: string }"#
                .into()
        )
    );
}
