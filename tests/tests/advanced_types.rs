//! Tests for difficult types, as an assertion to how Specta handles edge cases.

use std::collections::HashMap;

use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
struct Demo<A, B> {
    a: A,
    b: B,
}

type NonGeneric = Demo<u8, bool>;
type HalfGenericA<T> = Demo<T, bool>;
type HalfGenericB<T> = Demo<u8, T>;
type FullGeneric<T, U> = Demo<T, U>;

type Another<T> = FullGeneric<u8, T>;

type MapA<A> = HashMap<String, A>;
type MapB<B> = HashMap<B, String>;
type MapC<B> = HashMap<String, Struct<B>>;

#[derive(Type)]
#[specta(collect = false)]
struct Struct<T> {
    field: HalfGenericA<T>,
}

#[test]
fn test_type_aliases() {
    insta::assert_snapshot!(crate::ts::inline::<NonGeneric>(&Default::default()).unwrap(), @"{ a: number; b: boolean }");
    insta::assert_snapshot!(crate::ts::inline::<HalfGenericA<u8>>(&Default::default()).unwrap(), @"{ a: number; b: boolean }");
    insta::assert_snapshot!(crate::ts::inline::<HalfGenericB<bool>>(&Default::default()).unwrap(), @"{ a: number; b: boolean }");
    insta::assert_snapshot!(crate::ts::inline::<FullGeneric<u8, bool>>(&Default::default()).unwrap(), @"{ a: number; b: boolean }");
    insta::assert_snapshot!(crate::ts::inline::<Another<bool>>(&Default::default()).unwrap(), @"{ a: number; b: boolean }");

    insta::assert_snapshot!(crate::ts::inline::<MapA<u32>>(&Default::default()).unwrap(), @"{ [key in string]: number }");
    insta::assert_snapshot!(crate::ts::inline::<MapB<u32>>(&Default::default()).unwrap(), @"{ [key in number]: string }");
    insta::assert_snapshot!(crate::ts::inline::<MapC<u32>>(&Default::default()).unwrap(), @"{ [key in string]: { field: Demo<number, boolean> } }");

    insta::assert_snapshot!(crate::ts::inline::<Struct<u32>>(&Default::default()).unwrap(), @"{ field: Demo<number, boolean> }");
}

#[derive(Type)]
#[specta(collect = false)]
struct D {
    flattened: u32,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericFlattened<T> {
    generic_flattened: T,
}

#[derive(Type)]
#[specta(collect = false)]
struct C {
    a: u32,
    #[specta(flatten)]
    b: D,
}

#[derive(Type)]
#[specta(collect = false)]
struct B {
    b: u32,
}

#[derive(Type)]
#[specta(collect = false)]
struct A {
    a: B,
    #[specta(inline)]
    b: B,
    #[specta(flatten)]
    c: B,
    #[specta(inline, flatten)]
    d: D,
    #[specta(inline, flatten)]
    e: GenericFlattened<u32>,
}

#[derive(Type)]
#[specta(collect = false)]
struct ToBeFlattened {
    a: String,
}

#[derive(Type)]
#[specta(collect = false)]
struct DoubleFlattened {
    #[specta(flatten)]
    a: ToBeFlattened,
    #[specta(flatten)]
    b: ToBeFlattened,
}

#[derive(Type)]
#[specta(collect = false)]
struct Inner {
    a: i32,
    #[specta(flatten)]
    b: Box<FlattenedInner>,
}

#[derive(Type)]
#[specta(collect = false)]
struct FlattenedInner {
    #[specta(flatten)]
    c: Inner,
}

#[derive(Type)]
#[specta(collect = false)]
struct BoxedInner {
    a: i32,
}

#[derive(Type)]
#[specta(collect = false)]
struct BoxFlattened {
    #[specta(flatten)]
    b: Box<BoxedInner>,
}

#[derive(Type)]
#[specta(collect = false)]
struct BoxInline {
    #[specta(inline)]
    c: Box<BoxedInner>,
}

#[test]
fn test_inlining() {
    insta::assert_snapshot!(crate::ts::export::<A>(&Default::default()).unwrap(), @r#"export type A = (B) & (D) & (GenericFlattened<number>) & { a: B; b: B };"#);

    // assert_ts!(
    //     A,
    //     "({ b: number }) & ({ flattened: number }) & ({ generic_flattened: number }) & { a: B; b: { b: number } }"
    // );
    // assert_ts!(DoubleFlattened, "({ a: string }) & ({ a: string })");

    // TODO: All of these currently fail.
    // assert_ts!(FlattenedInner, ""); // TODO: This is wrong
    // assert_ts!(BoxFlattened, ""); // TODO: This is wrong
    // assert_ts!(BoxInline, ""); // TODO: This is wrong
}

#[test]
fn test_types_with_lifetimes() {
    // TODO

    // TODO: Detect duplicate detection with lifetimes.
    // TODO: Test duplicates with generic types.
}
