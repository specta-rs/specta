use specta::Type;

use crate::ts::{assert_ts_export2, assert_ts_inline2};

#[derive(Type)]
#[specta(export = false)]
struct A {}

#[derive(Type)]
#[specta(export = false, tag = "a")]
struct B {}

// https://github.com/oscartbeaumont/specta/issues/174
#[test]
fn empty_enums() {
    assert_eq!(
        assert_ts_export2::<A>(),
        Ok(r#"export type A = Record<string, never>;"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<A>(),
        Ok(r#"Record<string, never>"#.into())
    );
    assert_eq!(
        assert_ts_export2::<B>(),
        Ok(r#"export type B = { "a": "B" };"#.into())
    );
    assert_eq!(
        assert_ts_inline2::<B>(),
        // This may seem unexpected but without a NamedDataType the tag is not set
        Ok(r#"Record<string, never>"#.into())
    );
}
