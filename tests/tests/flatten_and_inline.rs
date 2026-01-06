use std::{collections::HashMap, sync::Arc};

use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
pub struct A {
    pub a: String,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct AA {
    pub a: i32,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct B {
    #[specta(flatten)]
    pub a: A,
    #[specta(flatten)]
    pub b: HashMap<String, String>,
    #[specta(flatten)]
    pub c: Arc<A>,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct C {
    #[specta(flatten)]
    pub a: A,
    #[specta(inline)]
    pub b: A,
}

#[derive(Type)]
#[specta(collect = false, tag = "type")]
pub struct D {
    #[specta(flatten)]
    pub a: A,
    #[specta(inline)]
    pub b: A,
}

#[derive(Type)]
#[specta(collect = false)]
#[serde(untagged)]
pub struct E {
    #[specta(flatten)]
    pub a: A,
    #[specta(inline)]
    pub b: A,
}

// Flattening a struct multiple times
#[derive(Type)]
#[specta(collect = false)]
pub struct F {
    #[specta(flatten)]
    pub a: A,
    #[specta(flatten)]
    pub b: A,
}

// Two fields with the same name (`a`) but different types
#[derive(Type)]
#[specta(collect = false)]
pub struct G {
    #[specta(flatten)]
    pub a: A,
    #[specta(flatten)]
    pub b: AA,
}

// Serde can't serialize this
#[derive(Type)]
#[specta(collect = false)]
pub enum H {
    A(String),
    B,
}

// TODO: Invalid Serde type but unit test this at the datamodel level cause it might be valid in other langs.
// #[derive(Type)]
// #[specta(collect = false, tag = "type")]
// pub enum I {
//     A(String),
//     B,
//     #[specta(inline)]
//     C(A),
//     D(#[specta(flatten)] A),
// }

#[derive(Type)]
#[specta(collect = false, tag = "t", content = "c")]
pub enum J {
    A(String),
    B,
    #[specta(inline)]
    C(A),
    D(A),
}

#[derive(Type)]
#[specta(collect = false, untagged)]
pub enum K {
    A(String),
    B,
    #[specta(inline)]
    C(A),
    D(A),
}

#[test]
fn serde() {
    insta::assert_snapshot!(crate::ts::inline::<B>(&Default::default()).unwrap(), @"(A) & ({ [key in string]: string })");
    insta::assert_snapshot!(crate::ts::inline::<C>(&Default::default()).unwrap(), @"(A) & { b: { a: string } }");
    insta::assert_snapshot!(crate::ts::inline::<D>(&Default::default()).unwrap(), @"(A) & { b: { a: string } }");
    // assert_ts!(D, "(A) & { b: { a: string } }"); // TODO: Assert export
    insta::assert_snapshot!(crate::ts::inline::<E>(&Default::default()).unwrap(), @"(A) & { b: { a: string } }");
    insta::assert_snapshot!(crate::ts::inline::<F>(&Default::default()).unwrap(), @"(A)");
    insta::assert_snapshot!(crate::ts::inline::<G>(&Default::default()).unwrap(), @"(A) & (AA)");
    insta::assert_snapshot!(crate::ts::inline::<H>(&Default::default()).unwrap(), @"{ A: string } | \"B\"");
    insta::assert_snapshot!(crate::ts::inline::<J>(&Default::default()).unwrap(), @"{ t: \"A\"; c: string } | { t: \"B\" } | { t: \"C\"; c: { a: string } } | { t: \"D\"; c: A }");
    insta::assert_snapshot!(crate::ts::inline::<K>(&Default::default()).unwrap(), @"string | null | { a: string } | A");
}
