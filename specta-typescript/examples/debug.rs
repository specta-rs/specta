use std::{collections::HashMap, convert::Infallible, sync::Arc};

use specta::{Type, TypeCollection};

#[derive(Type)]
#[specta(export = false)]
pub struct A {
    pub a: String,
}

#[derive(Type)]
#[specta(export = false)]
pub struct B {
    #[specta(flatten)]
    pub a: A,
    #[specta(flatten)]
    pub b: HashMap<String, String>,
    #[specta(flatten)]
    pub c: Arc<A>,
}

fn main() {
    let mut types = TypeCollection::default().register::<B>();
    // .register::<Testing>()
    // .register::<serde_yaml::Value>();
    // println!("{:#?}", types.get(Testing::ID).unwrap());

    // println!("{:#?}", types.get(serde_yaml::Value::ID).unwrap());
    // println!("{:#?}", serde_yaml::Value::definition(&mut types));

    let out = specta_typescript::Typescript::new()
        .bigint(specta_typescript::BigIntExportBehavior::Number)
        .export(&types)
        .unwrap();

    println!("{:?}", out);

    // TODO: GOAL: export type Testing = { A: A } | { B: B };
}
