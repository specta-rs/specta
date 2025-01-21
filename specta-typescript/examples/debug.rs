use std::collections::HashMap;

use specta::{Type, NamedType, TypeCollection};

// #[derive(Type)]
// #[specta(export = false, transparent)]
// pub struct MaybeValidKey<T>(T);

// #[derive(Type)]
// #[specta(export = false, transparent)]
// pub struct ValidMaybeValidKeyNested(HashMap<MaybeValidKey<MaybeValidKey<String>>, ()>);

// #[derive(Type)]
// #[specta(export = false)]
// pub struct Todo<T> {
//     pub field: T,
// }


// #[derive(Type)]
// #[specta(export = false)]
// pub struct Todo2<A, B, C> {
//     pub a: A,
//     pub b: B,
//     pub c: C,
// }

// #[derive(Type)]
// #[specta(export = false)]
// pub struct Test {
//     // #[specta(inline)]
//     pub root: Todo<Todo<String>>,
// }

#[derive(Type)]
pub struct Demo {
    pub a: String,
}

#[derive(Type)]
#[specta(inline)]
pub struct MeNeedInline {
    #[specta(inline)]
    pub a: Demo
}

#[derive(Type)]
pub struct Generic<T, U> {
    pub a: T,
    pub b: U
}

#[derive(Type)]
pub enum Hello {
    A,
    B(String),
    C {
        a: String,
    }
}

#[derive(Type)]
// #[specta(tag = "tag")]
pub enum Todo {
    A,
}

fn main() {
    // println!("{:?}\n\n", specta_typescript::inline::<MeNeedInline>(&Default::default()));
    // println!("{:?}\n\n", specta_typescript::export::<MeNeedInline>(&Default::default()));

    // println!("Debug");

    // let i = std::time::Instant::now();
    let mut types = TypeCollection::default();
    // let dt = MeNeedInline::definition(&mut types);
    // println!("A {:?}", i.elapsed());
    // println!("{:?}", specta_typescript::primitives::inline(&Default::default(), &types, &dt));

    // let i = std::time::Instant::now();
    // println!("{:?}", specta_typescript::primitives::export(&Default::default(), &types, types.get(MeNeedInline::ID).unwrap()));
    // println!("B {:?}", i.elapsed());

    // Generic::<String, i32>::definition(&mut types);
    // println!("{:?}", specta_typescript::primitives::export(&Default::default(), &types, types.get(Generic::<String, i32>::ID).unwrap()));

    // Hello::definition(&mut types);
    // println!("{:?}", specta_typescript::primitives::export(&Default::default(), &types, types.get(Hello::ID).unwrap()));

    Todo::definition(&mut types);
    // specta_serde::validate(&types).unwrap();
    println!("{:?}", specta_typescript::primitives::export(&Default::default(), &types, types.get(Todo::ID).unwrap()));


    // println!("{:?}\n\n", specta_typescript::inline::<Todo<String>>(&Default::default()));
    // println!("{:?}\n\n", specta_typescript::export::<Todo<String>>(&Default::default()));

    // println!("{:?}\n\n", specta_typescript::inline::<Test>(&Default::default()));
    // println!("{:?}\n\n", specta_typescript::export::<Test>(&Default::default()));

     // println!("{:?}\n\n", specta_typescript::inline::<HashMap<MaybeValidKey<String>, ()>>(&Default::default()));
    // println!("{:?}\n\n", specta_typescript::inline::<HashMap<MaybeValidKey<MaybeValidKey<String>>, ()>>(&Default::default()));
}
