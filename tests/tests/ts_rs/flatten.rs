#![allow(dead_code)]

use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
struct FlattenA {
    a: i32,
    b: i32,
}

#[derive(Type)]
#[specta(collect = false)]
struct FlattenB {
    #[specta(flatten)]
    a: FlattenA,
    c: i32,
}

#[derive(Type)]
#[specta(collect = false)]
struct FlattenC {
    #[specta(flatten = true)]
    a: FlattenA,
    c: i32,
}

#[derive(Type)]
#[specta(collect = false)]
struct FlattenD {
    #[specta(flatten = false)]
    a: FlattenA,
    c: i32,
}

#[derive(Type)]
#[specta(collect = false)]
struct FlattenE {
    #[specta(inline)]
    b: FlattenB,
    d: i32,
}

#[derive(Type)]
#[specta(collect = false)]
struct FlattenF {
    #[specta(inline = true)]
    b: FlattenB,
    d: i32,
}

#[derive(Type)]
#[specta(collect = false)]
struct FlattenG {
    #[specta(inline = false)]
    b: FlattenB,
    d: i32,
}

#[test]
fn test_flatten() {
    insta::assert_snapshot!(crate::ts::inline::<FlattenA>(&Default::default()).unwrap(), @"{ a: number; b: number }");
    insta::assert_snapshot!(crate::ts::inline::<FlattenB>(&Default::default()).unwrap(), @"(FlattenA) & { c: number }");
    insta::assert_snapshot!(crate::ts::inline::<FlattenC>(&Default::default()).unwrap(), @"(FlattenA) & { c: number }");
    insta::assert_snapshot!(crate::ts::inline::<FlattenD>(&Default::default()).unwrap(), @"{ a: FlattenA; c: number }");
    insta::assert_snapshot!(crate::ts::inline::<FlattenE>(&Default::default()).unwrap(), @"{ b: (FlattenA) & { c: number }; d: number }");
    insta::assert_snapshot!(crate::ts::inline::<FlattenF>(&Default::default()).unwrap(), @"{ b: (FlattenA) & { c: number }; d: number }");
    insta::assert_snapshot!(crate::ts::inline::<FlattenG>(&Default::default()).unwrap(), @"{ b: FlattenB; d: number }");
}
