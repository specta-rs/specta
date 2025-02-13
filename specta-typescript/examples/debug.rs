use std::collections::HashMap;

use specta::{Type, TypeCollection};

#[derive(Type)]
#[specta(tag = "tag")]
pub enum Todo {
    A,
    // B(String),
    C { a: String },
}

fn main() {
    println!(
        "{:?}\n\n",
        specta_typescript::inline::<Todo>(&Default::default())
    );
    // println!("{:?}\n\n", specta_typescript::export::<HashMap<UnitVariants, ()>>(&Default::default()));

    // Ok("{ tag: \"A\" } | ({ tag: \"B\" } & string) | { tag: \"C\"; a: string }")
}
