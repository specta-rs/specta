use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct A {
    pub a: String,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct AA {
    pub a: i32,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
pub struct B {
    #[serde(flatten)]
    pub a: A,
    pub b: HashMap<String, String>,
    pub c: Box<A>,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct C {
    pub a: A,
    #[specta(inline)]
    pub b: A,
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
pub struct D {
    pub a: A,
    #[specta(inline)]
    pub b: A,
}

// #[derive(Type, Serialize, Deserialize)]
// #[specta(collect = false)]
// #[serde(untagged)]
// pub struct E {
//     pub a: A,
//     #[specta(inline)]
//     pub b: A,
// }

// Flattening a struct multiple times
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct F {
    pub a: A,
    pub b: A,
}

// Two fields with the same name (`a`) but different types
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct G {
    pub a: A,
    pub b: AA,
}

// Serde can't serialize this
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub enum H {
    A(String),
    B,
}

// Test for issue #393 - flatten in enum variant with internal tag
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
pub enum MyEnum {
    Variant {
        #[serde(flatten)]
        inner: A,
    },
}

// Test for issue #393 - flatten in enum variant with external tag
#[derive(Type, Serialize)]
#[specta(collect = false)]
pub enum MyEnumExternal {
    Variant {
        #[serde(flatten)]
        inner: A,
    },
}

// Test for issue #393 - flatten in enum variant with adjacent tag
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
pub enum MyEnumAdjacent {
    Variant {
        #[serde(flatten)]
        inner: A,
    },
}

// Test for issue #393 - flatten in enum variant with untagged
#[derive(Type, Serialize)]
#[specta(collect = false)]
#[serde(untagged)]
pub enum MyEnumUntagged {
    Variant {
        #[serde(flatten)]
        inner: A,
    },
}

// TODO: Invalid Serde type but unit test this at the datamodel level cause it might be valid in other langs.
// #[derive(Type, Serialize, Deserialize)]
// #[specta(collect = false)]
// #[serde(tag = "type")]
// pub enum I {
//     A(String),
//     B,
//     #[specta(inline)]
//     C(A),
//     D(#[serde(flatten)] A),
// }

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "t", content = "c")]
pub enum J {
    A(String),
    B,
    #[specta(inline)]
    C(A),
    D(A),
}

#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
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
    // insta::assert_snapshot!(crate::ts::inline::<E>(&Default::default()).unwrap(), @"(A) & { b: { a: string } }");
    insta::assert_snapshot!(crate::ts::inline::<F>(&Default::default()).unwrap(), @"(A)");
    insta::assert_snapshot!(crate::ts::inline::<G>(&Default::default()).unwrap(), @"(A) & (AA)");
    insta::assert_snapshot!(crate::ts::inline::<H>(&Default::default()).unwrap(), @"{ A: string } | \"B\"");
    insta::assert_snapshot!(crate::ts::inline::<J>(&Default::default()).unwrap(), @"{ t: \"A\"; c: string } | { t: \"B\" } | { t: \"C\"; c: { a: string } } | { t: \"D\"; c: A }");
    insta::assert_snapshot!(crate::ts::inline::<K>(&Default::default()).unwrap(), @"string | null | { a: string } | A");

    // Test for issue #393 - flatten in enum variants
    insta::assert_snapshot!(crate::ts::inline::<MyEnum>(&Default::default()).unwrap(), @"(A) & { type: \"Variant\" }");
    insta::assert_snapshot!(crate::ts::inline::<MyEnumExternal>(&Default::default()).unwrap(), @"{ Variant: (A) }");
    insta::assert_snapshot!(crate::ts::inline::<MyEnumAdjacent>(&Default::default()).unwrap(), @"{ t: \"Variant\"; c: (A) }");
    insta::assert_snapshot!(crate::ts::inline::<MyEnumUntagged>(&Default::default()).unwrap(), @"(A)");
}
