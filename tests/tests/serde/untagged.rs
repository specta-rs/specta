use specta::Type;

use crate::ts::assert_ts_inline2;

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

    assert_eq!(
        assert_ts_inline2::<A>(),
        Ok(r#"{ id: string } | string | [string, string]"#.into())
    );
    assert_eq!(assert_ts_inline2::<B>(), Ok(r#"null"#.into()));
}
