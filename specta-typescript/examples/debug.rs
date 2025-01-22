use std::collections::HashMap;

use serde::{Deserialize, Serialize};
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
#[specta(tag = "tag")]
pub enum Todo {
    A,
    // B(String),
    C {
        a: String,
    }
}

#[derive(Serialize, Type)]
pub enum SkipVariant {
    #[specta(skip)]
    A(String),
    B(i32),
}

// #[derive(Serialize, Type)]
// pub enum SkipVariant2 {
//     #[specta(skip)]
//     A,
// }

#[derive(Type, Serialize, Deserialize)]
struct OptionalOnNamedField(#[specta(optional)] Option<String>); // Should do nothing

fn main() {
    println!("{:?}\n", serde_json::to_string(&SkipVariant::A("Hello".to_string())).unwrap());
    println!("{:?}\n", serde_json::to_string(&SkipVariant::B(32)).unwrap());
    println!("{:?}\n", specta_typescript::export::<SkipVariant>(&Default::default()));

     println!("{:?}\n", serde_json::to_string(&OptionalOnNamedField(Some("Hello".to_string()))).unwrap());


    let a: OptionalOnNamedField = serde_json::from_str("\"Hello\"").unwrap();
    let b: OptionalOnNamedField = serde_json::from_str(r#"null"#).unwrap();
    let c: OptionalOnNamedField = serde_json::from_str(r#"undefined"#).unwrap();
    // println!("{:?}\n", serde_json::to_string(&SkipVariant::A("Hello".to_string())).unwrap());
    // println!("{:?}\n", serde_json::to_string(&SkipVariant::B(32)).unwrap());


    // println!("{:?}\n\n", specta_typescript::inline::<MeNeedInline>(&Default::default()));
    // println!("{:?}\n\n", specta_typescript::export::<MeNeedInline>(&Default::default()));

    // println!("Debug");

    // let i = std::time::Instant::now();
    // let mut types = TypeCollection::default();
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

    // Todo::definition(&mut types);
    // // specta_serde::validate(&types).unwrap();
    // println!("{:?}", specta_typescript::primitives::export(&Default::default(), &types, types.get(Todo::ID).unwrap()));


    // println!("{:?}\n\n", specta_typescript::inline::<Todo<String>>(&Default::default()));
    // println!("{:?}\n\n", specta_typescript::export::<Todo<String>>(&Default::default()));

    // println!("{:?}\n\n", specta_typescript::inline::<Test>(&Default::default()));
    // println!("{:?}\n\n", specta_typescript::export::<Test>(&Default::default()));

     // println!("{:?}\n\n", specta_typescript::inline::<HashMap<MaybeValidKey<String>, ()>>(&Default::default()));
    // println!("{:?}\n\n", specta_typescript::inline::<HashMap<MaybeValidKey<MaybeValidKey<String>>, ()>>(&Default::default()));
}
