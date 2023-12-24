use specta::Type;

use types::A;

mod types;

#[derive(Type)]
pub struct Demo {
    // Both the same type but imported different ways
    a: A,
    b: types::A,
}

fn main() {
    println!("Hello, world!");
}
