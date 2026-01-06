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

// Test for issue #393 - flatten in enum variant with internal tag
#[derive(Type)]
#[specta(collect = false, tag = "type")]
pub enum MyEnum {
    Variant {
        #[specta(flatten)]
        inner: A,
    },
}

// Test for issue #393 - flatten in enum variant with external tag
#[derive(Type)]
#[specta(collect = false)]
pub enum MyEnumExternal {
    Variant {
        #[specta(flatten)]
        inner: A,
    },
}

// Test for issue #393 - flatten in enum variant with adjacent tag
#[derive(Type)]
#[specta(collect = false, tag = "t", content = "c")]
pub enum MyEnumAdjacent {
    Variant {
        #[specta(flatten)]
        inner: A,
    },
}

// Test for issue #393 - flatten in enum variant with untagged
#[derive(Type)]
#[specta(collect = false, untagged)]
pub enum MyEnumUntagged {
    Variant {
        #[specta(flatten)]
        inner: A,
    },
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
    insta::assert_snapshot!(crate::ts::inline::<C>(&Default::default()).unwrap(), @r"
    (A) & { 
    	b: A,
    }
    ");
    insta::assert_snapshot!(crate::ts::inline::<D>(&Default::default()).unwrap(), @r"
    (A) & { 
    	b: A,
    }
    ");
    // TODO: Assert export
    insta::assert_snapshot!(crate::ts::inline::<E>(&Default::default()).unwrap(), @r"
    (A) & { 
    	b: A,
    }
    ");
    insta::assert_snapshot!(crate::ts::inline::<F>(&Default::default()).unwrap(), @"(A)");
    insta::assert_snapshot!(crate::ts::inline::<G>(&Default::default()).unwrap(), @"(A) & (AA)");
    insta::assert_snapshot!(crate::ts::inline::<H>(&Default::default()).unwrap(), @"{ A: string } | \"B\"");
    insta::assert_snapshot!(crate::ts::inline::<J>(&Default::default()).unwrap(), @r#"{ t: "A"; c: string } | { t: "B" } | { t: "C"; c: A } | { t: "D"; c: A }"#);
    insta::assert_snapshot!(crate::ts::inline::<K>(&Default::default()).unwrap(), @"string | null | A");

    // Test for issue #393 - flatten in enum variants
    insta::assert_snapshot!(crate::ts::inline::<MyEnum>(&Default::default()).unwrap(), @"(A) & { type: \"Variant\" }");
    insta::assert_snapshot!(crate::ts::inline::<MyEnumExternal>(&Default::default()).unwrap(), @"{ Variant: (A) }");
    insta::assert_snapshot!(crate::ts::inline::<MyEnumAdjacent>(&Default::default()).unwrap(), @"{ t: \"Variant\"; c: (A) }");
    insta::assert_snapshot!(crate::ts::inline::<MyEnumUntagged>(&Default::default()).unwrap(), @"(A)");
}
