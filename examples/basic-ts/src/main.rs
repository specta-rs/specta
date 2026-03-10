use serde::Serialize;
use specta::{
    Type, TypeCollection,
    datatype::{DataType, Reference},
};

#[derive(Serialize, Type)]
#[serde(rename = "HelloWorld2")]
pub struct HelloWorld {
    #[serde(rename = "b")]
    a: String,

    #[serde(rename(serialize = "ser_name"))]
    b: String,
}

fn main() {
    let mut types = TypeCollection::default();
    println!("{:#?}", HelloWorld::definition(&mut types));
    println!(
        "{:#?}",
        match HelloWorld::definition(&mut types) {
            DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
            _ => unreachable!(),
        }
    );

    let def = HelloWorld::definition(&mut types);
    let types = specta_serde::testing(types);
    println!(
        "{:#?}",
        match def {
            DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
            _ => unreachable!(),
        }
    );

    // println!("{}", Typescript::default().export(&types).unwrap());
    // println!(
    //     "{}",
    //     Typescript::default()
    //         .export(&specta_serde::testing(types))
    //         .unwrap()
    // ); // TODO
}
