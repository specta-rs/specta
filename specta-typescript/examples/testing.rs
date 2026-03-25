use std::{collections::HashMap, iter};

use specta::{
    ResolvedTypes, Type, Types,
    datatype::{DataType, Reference},
};
use specta_typescript::{Typescript, primitives};

#[derive(Type)]
// #[specta(inline)]
pub struct Bruh<T>(T);

fn main() {
    let mut types = Types::default();
    let ty = Bruh::<HashMap<String, String>>::definition(&mut types);
    let resolved_types = ResolvedTypes::from_resolved_types(types.clone());

    println!(
        "{:?}",
        primitives::inline(&Typescript::default(), &resolved_types, &ty).unwrap()
    );
    println!(
        "{:?}",
        primitives::export(
            &Typescript::default(),
            &resolved_types,
            iter::once(match ty {
                DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
                _ => todo!(),
            }),
            ""
        )
        .unwrap()
    );
    // println!("{:?}", primitives::inline(&Default::default(), &types, &ty));
}
