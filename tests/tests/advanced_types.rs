//! Tests for difficult types, as an assertion to how Specta handles edge cases.

use std::collections::HashMap;

use specta::Type;

use crate::ts::{assert_ts, assert_ts_export2};

#[derive(Type)]
#[specta(export = false)]
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
#[specta(export = false)]
struct Struct<T> {
    field: HalfGenericA<T>,
}

#[test]
fn test_type_aliases() {
    assert_ts!(NonGeneric, "{ a: number; b: boolean }");
    assert_ts!(HalfGenericA<u8>, "{ a: number; b: boolean }");
    assert_ts!(HalfGenericB<bool>, "{ a: number; b: boolean }");
    assert_ts!(FullGeneric<u8, bool>, "{ a: number; b: boolean }");
    assert_ts!(Another<bool>, "{ a: number; b: boolean }");

    assert_ts!(MapA<u32>, "{ [key in string]: number }");
    assert_ts!(MapB<u32>, "{ [key in number]: string }");
    assert_ts!(
        MapC<u32>,
        "{ [key in string]: { field: Demo<number, boolean> } }"
    );

    assert_ts!(Struct<u32>, "{ field: Demo<number, boolean> }");
}

#[derive(Type)]
#[specta(export = false)]
struct D {
    flattened: u32,
}

#[derive(Type)]
#[specta(export = false)]
struct GenericFlattened<T> {
    generic_flattened: T,
}

#[derive(Type)]
#[specta(export = false)]
struct C {
    a: u32,
    #[specta(flatten)]
    b: D,
}

#[derive(Type)]
#[specta(export = false)]
struct B {
    b: u32,
}

#[derive(Type)]
#[specta(export = false)]
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
#[specta(export = false)]
struct ToBeFlattened {
    a: String,
}

#[derive(Type)]
#[specta(export = false)]
struct DoubleFlattened {
    #[specta(flatten)]
    a: ToBeFlattened,
    #[specta(flatten)]
    b: ToBeFlattened,
}

#[derive(Type)]
#[specta(export = false)]
struct Inner {
    a: i32,
    #[specta(flatten)]
    b: Box<FlattenedInner>,
}

#[derive(Type)]
#[specta(export = false)]
struct FlattenedInner {
    #[specta(flatten)]
    c: Inner,
}

#[derive(Type)]
#[specta(export = false)]
struct BoxedInner {
    a: i32,
}

#[derive(Type)]
#[specta(export = false)]
struct BoxFlattened {
    #[specta(flatten)]
    b: Box<BoxedInner>,
}

#[derive(Type)]
#[specta(export = false)]
struct BoxInline {
    #[specta(inline)]
    c: Box<BoxedInner>,
}

#[test]
fn test_inlining() {
    assert_eq!(
        assert_ts_export2::<A>(),
        Ok(r#"export type A = (B) & (D) & (GenericFlattened<number>) & { a: B; b: B };"#.into())
    );

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
