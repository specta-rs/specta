#![allow(dead_code)]

use specta::Type;

use crate::ts::assert_ts;

#[derive(Type)]
#[specta(export = false)]
struct FlattenA {
    a: i32,
    b: i32,
}

#[derive(Type)]
#[specta(export = false)]
struct FlattenB {
    #[specta(flatten)]
    a: FlattenA,
    c: i32,
}

#[derive(Type)]
#[specta(export = false)]
struct FlattenC {
    #[specta(flatten = true)]
    a: FlattenA,
    c: i32,
}

#[derive(Type)]
#[specta(export = false)]
struct FlattenD {
    #[specta(flatten = false)]
    a: FlattenA,
    c: i32,
}

#[derive(Type)]
#[specta(export = false)]
struct FlattenE {
    #[specta(inline)]
    b: FlattenB,
    d: i32,
}

#[derive(Type)]
#[specta(export = false)]
struct FlattenF {
    #[specta(inline = true)]
    b: FlattenB,
    d: i32,
}

#[derive(Type)]
#[specta(export = false)]
struct FlattenG {
    #[specta(inline = false)]
    b: FlattenB,
    d: i32,
}

#[test]
fn test_flatten() {
    assert_ts!(FlattenA, "{ a: number; b: number }");
    assert_ts!(FlattenB, "{ a: number; b: number; c: number }");
    assert_ts!(FlattenC, "{ a: number; b: number; c: number }");
    assert_ts!(FlattenD, "{ a: FlattenA; c: number }");
    assert_ts!(
        FlattenE,
        "{ b: { a: number; b: number; c: number }; d: number }"
    );
    assert_ts!(
        FlattenF,
        "{ b: { a: number; b: number; c: number }; d: number }"
    );
    assert_ts!(FlattenG, "{ b: FlattenB; d: number }");
}
