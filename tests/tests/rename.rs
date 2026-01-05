use specta::Type;

#[derive(Type)]
#[specta(collect = false, rename = "StructNew", tag = "t")]
pub struct Struct {
    a: String,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct Struct2 {
    #[specta(rename = "b")]
    a: String,
}

#[derive(Type)]
#[specta(collect = false, rename = "EnumNew", tag = "t")]
pub enum Enum {
    A,
    B,
}

#[derive(Type)]
#[specta(collect = false, rename = "EnumNew", tag = "t")]
pub enum Enum2 {
    #[specta(rename = "C")]
    A,
    B,
}

#[derive(Type)]
#[specta(collect = false, rename = "EnumNew", tag = "t")]
pub enum Enum3 {
    A {
        #[specta(rename = "b")]
        a: String,
    },
}

#[test]
fn rename() {
    insta::assert_snapshot!(crate::ts::inline::<Struct>(&Default::default()).unwrap(), @"{ a: string }");
    insta::assert_snapshot!(crate::ts::export::<Struct>(&Default::default()).unwrap(), @"export type StructNew = { a: string; t: \"StructNew\" };");

    insta::assert_snapshot!(crate::ts::inline::<Struct2>(&Default::default()).unwrap(), @"{ b: string }");

    insta::assert_snapshot!(crate::ts::inline::<Enum>(&Default::default()).unwrap(), @"{ t: \"A\" } | { t: \"B\" }");
    insta::assert_snapshot!(crate::ts::export::<Enum>(&Default::default()).unwrap(), @"export type EnumNew = { t: \"A\" } | { t: \"B\" };");

    insta::assert_snapshot!(crate::ts::inline::<Enum2>(&Default::default()).unwrap(), @"{ t: \"C\" } | { t: \"B\" }");
    insta::assert_snapshot!(crate::ts::inline::<Enum3>(&Default::default()).unwrap(), @"{ t: \"A\"; b: string }");
}
