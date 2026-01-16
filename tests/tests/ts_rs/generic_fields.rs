#![allow(dead_code, clippy::box_collection)]

use std::borrow::Cow;

use specta::Type;

#[test]
fn newtype() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Newtype1(Vec<Cow<'static, i32>>);
    insta::assert_snapshot!(crate::ts::inline::<Newtype1>(&Default::default()).unwrap(), @"number[]");
}

#[test]
fn newtype_nested() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Newtype2(Vec<Vec<i32>>);
    insta::assert_snapshot!(crate::ts::inline::<Newtype2>(&Default::default()).unwrap(), @"number[][]");
}

#[test]
fn alias() {
    type Alias1 = Vec<String>;
    insta::assert_snapshot!(crate::ts::inline::<Alias1>(&Default::default()).unwrap(), @"string[]");
}

#[test]
fn alias_nested() {
    type Alias2 = Vec<Vec<String>>;
    insta::assert_snapshot!(crate::ts::inline::<Alias2>(&Default::default()).unwrap(), @"string[][]");
}

#[test]
fn named() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Struct1 {
        a: Box<Vec<String>>,
        b: (Vec<String>, Vec<String>),
        c: [Vec<String>; 3],
    }
    insta::assert_snapshot!(crate::ts::inline::<Struct1>(&Default::default()).unwrap(), @"{ a: string[]; b: [string[], string[]]; c: [string[], string[], string[]] }");
}

#[test]
fn named_nested() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Struct2 {
        a: Vec<Vec<String>>,
        b: (Vec<Vec<String>>, Vec<Vec<String>>),
        c: [Vec<Vec<String>>; 3],
    }
    insta::assert_snapshot!(crate::ts::inline::<Struct2>(&Default::default()).unwrap(), @"{ a: string[][]; b: [string[][], string[][]]; c: [string[][], string[][], string[][]] }");
}

#[test]
fn tuple() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Tuple1(Vec<i32>, (Vec<i32>, Vec<i32>), [Vec<i32>; 3]);
    insta::assert_snapshot!(crate::ts::inline::<Tuple1>(&Default::default()).unwrap(), @"[number[], [number[], number[]], [number[], number[], number[]]]");
}

#[test]
fn tuple_nested() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Tuple2(
        Vec<Vec<i32>>,
        (Vec<Vec<i32>>, Vec<Vec<i32>>),
        [Vec<Vec<i32>>; 3],
    );
    insta::assert_snapshot!(crate::ts::inline::<Tuple2>(&Default::default()).unwrap(), @"[number[][], [number[][], number[][]], [number[][], number[][], number[][]]]");
}
