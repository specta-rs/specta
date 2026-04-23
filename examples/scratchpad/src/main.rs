//! A playground for quickly reproducing issue.

use std::{borrow::Cow, ops::Range};

use serde::Serialize;
use specta::{
    Type, Types,
    datatype::{DataType, NamedReferenceType, Reference},
};

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
    // #[specta(inline)]
    a: Box<A>,
    c: C,
}

#[derive(Type)]
// #[specta(inline)]
pub struct C {
    c: D,
}

#[derive(Type)]
pub struct D {
    // This should never show up in output
    d: String,
    // Is recursive but not infinitely recursive.
    // e: E<E<E<String>>>,
}

// #[derive(Type)]
// pub struct B<T> {
//     #[specta(inline)]
//     a: Box<T>,
// }

// B<B<String>>,

// #[derive(Type)]
// pub struct E<EE> {
//     e: EE,
// }

// #[derive(Type)]
// #[specta(inline)] // TODO
// pub struct E2<T> {
//     f: T,
//     // Infinitely recursive
//     #[specta(inline)]
//     e: Box<E2<T>>,
// }

// #[derive(Type)]
// #[specta(inline)] // TODO
// struct E3 {
//     e: E2<String>,
//     // { f: String, e: { f: String, e: { f: String, e: ... } } }
//     ee: E2<E2<String>>,
//     // { f: String, e: { f: String, e: { f: String, e: ... } } }
// }

#[derive(Type)]
struct GG<T>(#[specta(inline)] T);

#[derive(Type)]
struct G {
    #[specta(inline)]
    a: GG<String>,
    b: GG<String>,
}

fn main() {
    let mut types = Types::default()
        .register::<A>()
        // .register::<E3>()
        .register::<G>();

    let def = String::definition(&mut types);
    println!("\n{:?}", def);
    println!(
        "{:?}",
        match def {
            DataType::Reference(Reference::Named(r)) => types.get(&r).unwrap(),
            _ => unreachable!(),
        }
    );

    let def = Range::<i32>::definition(&mut types);
    println!("\n{:?}", def);
    println!(
        "{:?}",
        match def {
            DataType::Reference(Reference::Named(r)) => types.get(&r).unwrap(),
            _ => unreachable!(),
        }
    );

    let def = Cow::<'static, str>::definition(&mut types);
    println!("\n{:?}", def);
    println!(
        "{:?}",
        match def {
            DataType::Reference(Reference::Named(r)) => types.get(&r).unwrap(),
            _ => unreachable!(),
        }
    );

    // println!("{types:#?}");

    // let out = specta_typescript::Typescript::new()
    //     .export(&types, specta_serde::format_phases)
    //     .unwrap();

    // println!("{}", out);
}
