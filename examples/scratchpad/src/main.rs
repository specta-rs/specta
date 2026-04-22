//! A playground for quickly reproducing issue.

use serde::Serialize;
use specta::{Type, Types};

#[derive(Type)]
// #[specta(inline)]
pub struct A {
    #[specta(inline)]
    b: B,
    c: C,
}

#[derive(Type)]
// #[specta(inline)]
pub struct B {
    #[specta(inline)]
    a: Box<A>,
    c: C,
}

#[derive(Type)]
pub struct C {
    c: D,
}

#[derive(Type)]
pub struct D {
    // This should never show up in output
    d: String,
    // Is recursive but not infinitely recursive.
    e: E<E<E<String>>>,
}

#[derive(Type)]
pub struct E<EE> {
    e: EE,
}

#[derive(Type)]
pub struct E2<T> {
    f: T,
    // Infinitely recursive
    #[specta(inline)]
    e: Box<E2<T>>,
}

#[derive(Type)]
struct E3 {
    e: E2<String>,
    // { f: String, e: { f: String, e: { f: String, e: ... } } }
    ee: E2<E2<String>>,
    // { f: String, e: { f: String, e: { f: String, e: ... } } }
}

fn main() {
    let types = Types::default()
        // .register::<A>()
        .register::<E3>();

    let out = specta_typescript::Typescript::new()
        .export(&types, specta_serde::format_phases)
        .unwrap();

    println!("{}", out);
}
