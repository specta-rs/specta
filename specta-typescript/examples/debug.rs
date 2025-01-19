use std::collections::HashMap;

use specta::Type;

#[derive(Type)]
#[specta(export = false)]
enum UnitVariants {
    A,
    B,
    C,
}
// TODO: Test recursive inline

fn main() {
    println!("{:?}\n\n", specta_typescript::inline::<HashMap<UnitVariants, ()>>(&Default::default()));
    // println!("{:?}\n\n", specta_typescript::export::<HashMap<UnitVariants, ()>>(&Default::default()));
}
