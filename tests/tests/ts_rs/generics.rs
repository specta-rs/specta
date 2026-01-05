#![allow(dead_code)]

use std::{
    collections::{BTreeMap, HashSet},
    rc::Rc,
};

use specta::Type;

#[derive(Type)]
#[specta(collect = false)]
struct Generic1<T: Type> {
    value: T,
    values: Vec<T>,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericAutoBound<T> {
    value: T,
    values: Vec<T>,
}

#[derive(Type)]
#[specta(collect = false)]
struct GenericAutoBound2<T: PartialEq> {
    value: T,
    values: Vec<T>,
}

#[derive(Type)]
#[specta(collect = false)]
struct Container1 {
    foo: Generic1<u32>,
    bar: HashSet<Generic1<u32>>,
    baz: BTreeMap<String, Rc<Generic1<String>>>,
}

#[test]
fn test() {
    insta::assert_snapshot!(crate::ts::export::<Generic1<()>>(&Default::default()).unwrap(), @"export type Generic1<T> = { value: T; values: T[] };");
    insta::assert_snapshot!(crate::ts::export::<GenericAutoBound<()>>(&Default::default()).unwrap(), @"export type GenericAutoBound<T> = { value: T; values: T[] };");
    insta::assert_snapshot!(crate::ts::export::<GenericAutoBound2<()>>(&Default::default()).unwrap(), @"export type GenericAutoBound2<T> = { value: T; values: T[] };");
    insta::assert_snapshot!(crate::ts::export::<Container1>(&Default::default()).unwrap(), @"export type Container1 = { foo: Generic1<number>; bar: Generic1<number>[]; baz: { [key in string]: Generic1<string> } };");
}

#[test]
fn generic_enum() {
    #[derive(Type)]
    #[specta(collect = false)]
    enum Generic2<A, B, C> {
        A(A),
        B(B, B, B),
        C(Vec<C>),
        D(Vec<Vec<Vec<A>>>),
        E { a: A, b: B, c: C },
        X(Vec<i32>),
        Y(i32),
        Z(Vec<Vec<i32>>),
    }

    insta::assert_snapshot!(crate::ts::export::<Generic2::<(), (), ()>>(&Default::default()).unwrap(), @r#"export type Generic2<A, B, C> = { A: A } | { B: [B, B, B] } | { C: C[] } | { D: A[][][] } | { E: { a: A; b: B; c: C } } | { X: number[] } | { Y: number } | { Z: number[][] };"#);
}

#[test]
fn generic_newtype() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct NewType1<T>(Vec<Vec<T>>);

    insta::assert_snapshot!(crate::ts::export::<NewType1::<()>>(&Default::default()).unwrap(), @r#"export type NewType1<T> = T[][];"#);
}

#[test]
fn generic_tuple() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Tuple<T>(T, Vec<T>, Vec<Vec<T>>);

    insta::assert_snapshot!(crate::ts::export::<Tuple::<()>>(&Default::default()).unwrap(), @r#"export type Tuple<T> = [T, T[], T[][]];"#);
}

#[test]
fn generic_struct() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct GenericStruct2<T> {
        a: T,
        b: (T, T),
        c: (T, (T, T)),
        d: [T; 3],
        e: [(T, T); 3],
        f: Vec<T>,
        g: Vec<Vec<T>>,
        h: Vec<[(T, T); 3]>,
    }

    insta::assert_snapshot!(crate::ts::export::<GenericStruct2::<()>>(&Default::default()).unwrap(), @"export type GenericStruct2<T> = { a: T; b: [T, T]; c: [T, [T, T]]; d: [T, T, T]; e: [([T, T]), ([T, T]), ([T, T])]; f: T[]; g: T[][]; h: ([([T, T]), ([T, T]), ([T, T])])[] };");
}

// not currently possible in ts-rs hehe
#[test]
fn inline() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Generic<T> {
        t: T,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct Container {
        g: Generic<String>,
        #[specta(inline)]
        gi: Generic<String>,
        #[specta(flatten)]
        t: Generic<String>,
    }

    insta::assert_snapshot!(crate::ts::export::<Generic::<()>>(&Default::default()).unwrap(), @"export type Generic<T> = { t: T };");
    insta::assert_snapshot!(crate::ts::export::<Container>(&Default::default()).unwrap(), @"export type Container = (Generic<string>) & { g: Generic<string>; gi: { t: string } };");
}

// #[test]
// fn default() {
//     #[derive(Type)]
//     #[specta(collect = false)]
//     struct A<T = String> {
//         t: T,
//     }
//     assert_ts_export!(
//         ts_A::<()>,
//         "export type A<T = string> = { t: T, }"
//     );

//     #[derive(Type)]
//     #[specta(collect = false)]
//     struct B<U = Option<A<i32>>> {
//         u: U,
//     }
//     assert_ts_export!(
//         ts_B::<()>,
//         "export type B<U = A<number> | null>  = { u: U, }"
//     );

//     #[derive(Type)]
//     #[specta(collect = false)]
//     struct Y {
//         a1: A,
//         a2: A<i32>,
// https://github.com/Aleph-Alpha/ts-rs/issues/56
// TODO: fixme
// #[ts(inline)]
// xi: X,
// #[ts(inline)]
// xi2: X<i32>
// }
// assert_ts_export!(
//     ts_Y,
//     "type Y = { a1: A, a2: A<number> }"
// )
// }

// TODO

// #[test]
// fn test_generic_trait_bounds() {
//     #[derive(Type)]
//     struct A<T: ToString = i32> {
//         t: T,
//     }
//     assert_ts_export!(A::<i32>, "export type A<T = number> = { t: T, }");

//     #[derive(Type)]
//     struct B<T: ToString + std::fmt::Debug + Clone + 'static>(T);
//     assert_ts_export!(B::<&'static str>, "export type B<T> = T;");

//     #[derive(Type)]
//     enum C<T: Copy + Clone + PartialEq, K: Copy + PartialOrd = i32> {
//         A { t: T },
//         B(T),
//         C,
//         D(T, K),
//     }
//     assert_ts_export!(
//         C::<&'static str, i32>,
//         "export type C<T, K = number> = { A: { t: T, } } | { B: T } | \"C\" | { D: [T, K] };"
//     );

//     #[derive(Type)]
//     struct D<T: ToString, const N: usize> {
//         t: [T; N],
//     }

//     assert_ts_export!(D::<&str, 41>, "export type D<T> = { t: Array<T>, }")
// }

// https://github.com/specta-rs/specta/issues/400
#[test]
fn generic_parameter_order_preserved() {
    #[derive(Type)]
    #[specta(collect = false)]
    struct Pair<Z, A> {
        first: Z,
        second: A,
    }

    #[derive(Type)]
    #[specta(collect = false)]
    struct Container {
        pair: Pair<i32, String>,
    }

    insta::assert_snapshot!(crate::ts::export::<Container>(&Default::default()).unwrap(), @"export type Container = { pair: Pair<number, string> };");
}
