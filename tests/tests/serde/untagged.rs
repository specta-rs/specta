use specta::Type;

use crate::ts::assert_ts_inline2;

#[derive(Type)]
#[specta(collect = false, untagged)]
enum A {
    A { id: String },
    C(String),
    D(String, String),
}

#[derive(Type)]
#[serde(collect = false, untagged)]
pub enum B {
    A,
    B,
}

#[test]
fn untagged() {
    // There is not way to construct an invalid untagged type.

    insta::assert_snapshot!(
        assert_ts_inline2::<A>().unwrap(),
        @r#"{ id: string } | string | [string, string]"#
    );
    insta::assert_snapshot!(assert_ts_inline2::<B>().unwrap(), @r#"null"#);
}
