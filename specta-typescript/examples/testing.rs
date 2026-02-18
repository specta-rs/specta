use std::collections::HashMap;

use specta::{
    Type, TypeCollection,
    datatype::{DataType, Reference},
};
use specta_typescript::{Typescript, primitives};

#[derive(Type)]
// #[specta(inline)]
pub struct Bruh<T>(T);

fn main() {
    let mut types = TypeCollection::default();
    let ty = Bruh::<HashMap<String, String>>::definition(&mut types);

    println!(
        "{:?}",
        primitives::inline(&Typescript::default(), &types, &ty).unwrap()
    );
    println!(
        "{:?}",
        primitives::export(
            &Typescript::default(),
            &types,
            match ty {
                DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
                _ => todo!(),
            }
        )
        .unwrap()
    );
    // println!("{:?}", primitives::inline(&Default::default(), &types, &ty));
}
