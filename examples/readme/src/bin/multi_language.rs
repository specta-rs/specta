//! The multi-language example from the README. Kept as a compile check.

#![allow(missing_docs)]

use serde::Serialize;
use specta::{Type, Types};
use specta_swift::Swift;
use specta_typescript::Typescript;

#[derive(Type, Serialize)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: Option<String>,
}

fn main() {
    let types = Types::default().register::<User>();

    let typescript = Typescript::default()
        .export(&types, specta_serde::Format)
        .unwrap();
    let swift = Swift::default()
        .export(&types, specta_serde::Format)
        .unwrap();

    assert!(typescript.contains("email?: string | null"), "{typescript}");
    assert!(swift.contains("public struct User"), "{swift}");
}
