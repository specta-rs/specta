use std::{collections::HashMap, sync::Arc};

use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(export = false)]
pub struct A {
    pub a: String,
}

#[derive(Type)]
#[specta(export = false)]
pub struct AA {
    pub a: i32,
}

#[derive(Type)]
#[specta(export = false)]
pub struct B {
    #[specta(flatten)]
    pub a: A,
    #[specta(flatten)]
    pub b: HashMap<String, String>,
    #[specta(flatten)]
    pub c: Arc<A>,
}

#[derive(Type)]
#[specta(export = false)]
pub struct C {
    #[specta(flatten)]
    pub a: A,
    #[specta(inline)]
    pub b: A,
}

#[derive(Type)]
#[specta(export = false, tag = "type")]
pub struct D {
    #[specta(flatten)]
    pub a: A,
    #[specta(inline)]
    pub b: A,
}

#[derive(Type)]
#[specta(export = false)]
#[serde(untagged)]
pub struct E {
    #[specta(flatten)]
    pub a: A,
    #[specta(inline)]
    pub b: A,
}

// Flattening a struct multiple times
#[derive(Type)]
#[specta(export = false)]
pub struct F {
    #[specta(flatten)]
    pub a: A,
    #[specta(flatten)]
    pub b: A,
}

// Two fields with the same name (`a`) but different types
#[derive(Type)]
#[specta(export = false)]
pub struct G {
    #[specta(flatten)]
    pub a: A,
    #[specta(flatten)]
    pub b: AA,
}

// Serde can't serialize this
#[derive(Type)]
#[specta(export = false)]
pub enum H {
    A(String),
    B,
}

// TODO: Invalid Serde type but unit test this at the datamodel level cause it might be valid in other langs.
// #[derive(Type)]
// #[specta(export = false, tag = "type")]
// pub enum I {
//     A(String),
//     B,
//     #[specta(inline)]
//     C(A),
//     D(#[specta(flatten)] A),
// }

#[derive(Type)]
#[specta(export = false, tag = "t", content = "c")]
pub enum J {
    A(String),
    B,
    #[specta(inline)]
    C(A),
    D(A),
}

#[derive(Type)]
#[specta(export = false, untagged)]
pub enum K {
    A(String),
    B,
    #[specta(inline)]
    C(A),
    D(A),
}

#[test]
fn serde() {
    assert_ts!(
        B,
        "({ a: string }) & (Partial<{ [key in string]: string }>) & ({ a: string })"
    );
    assert_ts!(C, "({ a: string }) & { b: { a: string } }");
    assert_ts!(D, "({ a: string }) & { b: { a: string }; type: \"D\" }");
    assert_ts!(E, "({ a: string }) & { b: { a: string } }");
    assert_ts!(F, "({ a: string }) & ({ a: string })");
    assert_ts!(G, "({ a: string }) & ({ a: number })");
    assert_ts!(H, "{ A: string } | \"B\"");
    assert_ts!(J, "{ t: \"A\"; c: string } | { t: \"B\" } | { t: \"C\"; c: { a: string } } | { t: \"D\"; c: A }");
    assert_ts!(K, "string | null | { a: string } | A");
}
