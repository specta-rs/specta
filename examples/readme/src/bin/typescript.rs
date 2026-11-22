//! The TypeScript example from the README. Kept as a compile check.

#![allow(missing_docs)]

use serde::Serialize;
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Type, Serialize)]
pub struct TypeOne {
    pub a: String,
    pub b: GenericType<i32>,
    #[serde(rename = "cccccc")]
    pub c: MyEnum,
}

#[derive(Type, Serialize)]
pub struct GenericType<A> {
    pub my_field: String,
    pub generic: A,
}

#[derive(Type, Serialize)]
pub enum MyEnum {
    A,
    B,
    C,
}

fn main() {
    // GenericType and MyEnum do not need registering: resolving TypeOne
    // registers every named type it references.
    let types = Types::default().register::<TypeOne>();

    let output = Typescript::default()
        .export(&types, specta_serde::Format)
        .unwrap();

    assert!(output.contains("cccccc"), "{output}");
}
