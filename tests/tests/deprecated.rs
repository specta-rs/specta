#![allow(deprecated)]

use specta::Type;

use crate::ts::assert_ts_export;

#[derive(Type)]
#[specta(export = false)]
#[deprecated]
struct DeprecatedType {
    a: i32,
}

#[derive(Type)]
#[specta(export = false)]
#[deprecated = "Look at you big man using a deprecation message"]
struct DeprecatedTypeWithMsg {
    a: i32,
}

#[derive(Type)]
#[specta(export = false)]
#[deprecated(note = "Look at you big man using a deprecation message")]
struct DeprecatedTypeWithMsg2 {
    a: i32,
}

#[derive(Type)]
#[specta(export = false)]
struct DeprecatedFields {
    a: i32,
    #[deprecated]
    b: String,
    #[deprecated = "This field is cringe!"]
    c: String,
    #[deprecated(note = "This field is cringe!")]
    d: String,
}

#[derive(Type)]
#[specta(export = false)]
pub struct DeprecatedTupleVariant(
    #[deprecated] String,
    #[deprecated = "Nope"] String,
    #[deprecated(note = "Nope")] i32,
);

#[derive(Type)]
#[specta(export = false)]
pub enum DeprecatedEnumVariants {
    #[deprecated]
    A,
    #[deprecated = "Nope"]
    B,
    #[deprecated(note = "Nope")]
    C,
}

#[test]
fn test_deprecated_types() {
    assert_ts_export!(
        DeprecatedType,
        "/**\n * @deprecated\n */\nexport type DeprecatedType = { a: number };"
    );
    assert_ts_export!(
        DeprecatedTypeWithMsg,
        "/**\n * @deprecated Look at you big man using a deprecation message\n */\nexport type DeprecatedTypeWithMsg = { a: number };"
    );
    assert_ts_export!(DeprecatedTypeWithMsg2, "/**\n * @deprecated Look at you big man using a deprecation message\n */\nexport type DeprecatedTypeWithMsg2 = { a: number };");
    assert_ts_export!(DeprecatedFields, "export type DeprecatedFields = { a: number; \n/**\n * @deprecated\n */\nb: string; \n/**\n * @deprecated This field is cringe!\n */\nc: string; \n/**\n * @deprecated This field is cringe!\n */\nd: string };");
    assert_ts_export!(DeprecatedTupleVariant, "export type DeprecatedTupleVariant = [\n/**\n * @deprecated\n */\nstring, \n/**\n * @deprecated Nope\n */\nstring, \n/**\n * @deprecated Nope\n */\nnumber];");
    assert_ts_export!(DeprecatedEnumVariants, "export type DeprecatedEnumVariants = \n/**\n * @deprecated\n */\n\"A\" | \n/**\n * @deprecated Nope\n */\n\"B\" | \n/**\n * @deprecated Nope\n */\n\"C\";");
}
