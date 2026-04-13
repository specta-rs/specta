//! A playground for quickly reproducing issue.

use serde::Serialize;
use specta::{Type, Types};

#[derive(Serialize, Type)]
pub struct Demo {
    field: String,
}

fn main() {
    let types = Types::default().register::<Demo>();

    let out = specta_typescript::Typescript::new()
        .export(&specta_serde::apply(types).unwrap())
        .unwrap();

    println!("{:?}", out);
}
