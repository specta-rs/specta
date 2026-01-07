use std::collections::HashMap;

use serde::Serialize;
use specta::{Type, TypeCollection};

#[derive(Type, Serialize)]
#[specta(collect = false)]
pub struct A {
    pub a: String,
}

#[derive(Type, Serialize)]
#[specta(collect = false)]
pub struct B {
    #[serde(flatten)]
    pub a: A,
    #[serde(flatten)]
    pub b: HashMap<String, String>,
    #[serde(flatten)]
    pub c: Box<A>,
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
