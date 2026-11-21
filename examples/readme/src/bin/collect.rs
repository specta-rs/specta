//! The collection example from the README. Kept as a compile check.

#![allow(missing_docs)]

use specta::Type;
use specta_typescript::Typescript;

#[derive(Type)]
pub enum MyEither<L, R> {
    Left(L),
    Right(R),
}

#[derive(Type)]
pub struct GenericType<A> {
    pub my_field: String,
    pub generic: A,
}

#[derive(Type)]
pub enum MyEnum {
    A,
    B,
    C,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct DontExportMe {
    #[allow(dead_code)]
    field: String,
}

fn main() {
    let types = specta::collect();
    let output = Typescript::default()
        .export(&types, specta::Identity)
        .unwrap();

    assert!(output.contains("MyEnum"), "{output}");
    assert!(!output.contains("DontExportMe"), "{output}");
}
