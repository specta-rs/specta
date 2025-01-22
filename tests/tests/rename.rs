use specta::Type;

use crate::ts::{assert_ts, assert_ts_export};

#[derive(Type)]
#[specta(export = false, rename = "StructNew", tag = "t")]
pub struct Struct {
    a: String,
}

#[derive(Type)]
#[specta(export = false)]
pub struct Struct2 {
    #[specta(rename = "b")]
    a: String,
}

#[derive(Type)]
#[specta(export = false, rename = "EnumNew", tag = "t")]
pub enum Enum {
    A,
    B,
}

#[derive(Type)]
#[specta(export = false, rename = "EnumNew", tag = "t")]
pub enum Enum2 {
    #[specta(rename = "C")]
    A,
    B,
}

#[derive(Type)]
#[specta(export = false, rename = "EnumNew", tag = "t")]
pub enum Enum3 {
    A {
        #[specta(rename = "b")]
        a: String,
    },
}

#[test]
fn rename() {
    assert_ts!(Struct, "{ t: \"StructNew\"; a: string }");
    assert_ts_export!(
        Struct,
        "export type StructNew = { t: \"StructNew\"; a: string };"
    );

    assert_ts!(Struct2, "{ b: string }");

    assert_ts!(Enum, "({ t: \"A\" }) | ({ t: \"B\" })");
    assert_ts_export!(
        Enum,
        "export type EnumNew = ({ t: \"A\" }) | ({ t: \"B\" });"
    );

    assert_ts!(Enum2, "({ t: \"C\" }) | ({ t: \"B\" })");
    assert_ts!(Enum3, "({ t: \"A\"; b: string })");
}
