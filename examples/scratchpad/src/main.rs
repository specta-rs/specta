//! A playground for quickly reproducing issue.
#![allow(warnings)]

use specta::Types;

fn main() {
    let mut types = Types::default();

    let out = specta_typescript::Typescript::new()
        .export(&types, specta_serde::PhasesFormat)
        .unwrap();

    println!("{}", out);
}
