use std::collections::HashMap;

use specta::Type;

#[derive(Type)]
#[specta(export = false, transparent)]
pub struct MaybeValidKey<T>(T);

#[derive(Type)]
#[specta(export = false, transparent)]
pub struct ValidMaybeValidKeyNested(HashMap<MaybeValidKey<MaybeValidKey<String>>, ()>);

#[derive(Type)]
#[specta(export = false)]
pub struct Todo<T> {
    pub field: T,
}


#[derive(Type)]
#[specta(export = false)]
pub struct Todo2<A, B, C> {
    pub a: A,
    pub b: B,
    pub c: C,
}

#[derive(Type)]
#[specta(export = false)]
pub struct Test {
    // #[specta(inline)]
    pub root: Todo<Todo<String>>,
}

fn main() {
    println!("{:?}\n\n", specta_typescript::inline::<Todo<String>>(&Default::default()));
    println!("{:?}\n\n", specta_typescript::export::<Todo<String>>(&Default::default()));

    // println!("{:?}\n\n", specta_typescript::inline::<Test>(&Default::default()));
    // println!("{:?}\n\n", specta_typescript::export::<Test>(&Default::default()));

     // println!("{:?}\n\n", specta_typescript::inline::<HashMap<MaybeValidKey<String>, ()>>(&Default::default()));
    // println!("{:?}\n\n", specta_typescript::inline::<HashMap<MaybeValidKey<MaybeValidKey<String>>, ()>>(&Default::default()));
}
