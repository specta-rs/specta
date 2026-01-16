#![allow(deprecated)]

use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
#[deprecated]
struct DeprecatedType {
    a: i32,
}

#[derive(Type)]
#[specta(collect = false)]
#[deprecated = "Look at you big man using a deprecation message"]
struct DeprecatedTypeWithMsg {
    a: i32,
}

#[derive(Type)]
#[specta(collect = false)]
#[deprecated(note = "Look at you big man using a deprecation message")]
struct DeprecatedTypeWithMsg2 {
    a: i32,
}

#[derive(Type)]
#[specta(collect = false)]
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
#[specta(collect = false)]
pub struct DeprecatedTupleVariant(
    #[deprecated] String,
    #[deprecated = "Nope"] String,
    #[deprecated(note = "Nope")] i32,
);

#[derive(Type)]
#[specta(collect = false)]
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
    insta::assert_snapshot!(crate::ts::export::<DeprecatedType>(&Default::default()).unwrap(), @"/**\n * @deprecated\n */\nexport type DeprecatedType = { a: number };");
    insta::assert_snapshot!(crate::ts::export::<DeprecatedTypeWithMsg>(&Default::default()).unwrap(), @"/**\n * @deprecated Look at you big man using a deprecation message\n */\nexport type DeprecatedTypeWithMsg = { a: number };");
    insta::assert_snapshot!(crate::ts::export::<DeprecatedTypeWithMsg2>(&Default::default()).unwrap(), @"/**\n * @deprecated Look at you big man using a deprecation message\n */\nexport type DeprecatedTypeWithMsg2 = { a: number };");
    insta::assert_snapshot!(crate::ts::export::<DeprecatedFields>(&Default::default()).unwrap(), @"export type DeprecatedFields = { a: number; \n/**\n * @deprecated\n */\nb: string; \n/**\n * @deprecated This field is cringe!\n */\nc: string; \n/**\n * @deprecated This field is cringe!\n */\nd: string };");
    insta::assert_snapshot!(crate::ts::export::<DeprecatedTupleVariant>(&Default::default()).unwrap(), @"export type DeprecatedTupleVariant = [\n/**\n * @deprecated\n */\nstring, \n/**\n * @deprecated Nope\n */\nstring, \n/**\n * @deprecated Nope\n */\nnumber];");
    insta::assert_snapshot!(crate::ts::export::<DeprecatedEnumVariants>(&Default::default()).unwrap(), @"export type DeprecatedEnumVariants = \n/**\n * @deprecated\n */\n\"A\" | \n/**\n * @deprecated Nope\n */\n\"B\" | \n/**\n * @deprecated Nope\n */\n\"C\";");
}
