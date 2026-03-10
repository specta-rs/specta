use serde::Serialize;
use specta::{
    datatype::{DataType, Reference},
    Type, TypeCollection,
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

    println!("{:?}", Typescript::default().export())
}
