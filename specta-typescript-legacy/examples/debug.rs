use std::collections::HashMap;

use specta::Type;

#[derive(Type)]
pub enum Hello {
    A,
    B(String),
    C {
        a: String,
    }
}

fn main() {
    println!("{:?}\n\n", specta_typescript_legacy::inline::<Hello>(&Default::default()));
    // println!("{:?}\n\n", specta_typescript::export::<HashMap<UnitVariants, ()>>(&Default::default()));

    // Ok("\"A\" | { B: string } | { C: { a: string } }")
}
