use std::collections::HashMap;

use serde::Serialize;
use specta::{ResolvedTypes, Type, Types};

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
    let types = Types::default().register::<B>();
    // .register::<Testing>()
    // .register::<serde_yaml::Value>();
    // println!("{:#?}", types.get(Testing::ID).unwrap());

    // println!("{:#?}", types.get(serde_yaml::Value::ID).unwrap());
    // println!("{:#?}", serde_yaml::Value::definition(&mut types));

    let out = specta_typescript::Typescript::new()
        .bigint(specta_typescript::BigIntExportBehavior::Number)
        .export(&ResolvedTypes::from_resolved_types(types))
        .unwrap();

    println!("{:?}", out);

    // TODO: GOAL: export type Testing = { A: A } | { B: B };
}
