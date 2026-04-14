//! A playground for quickly reproducing issue.

use serde::Serialize;
use specta::{
    Type, Types,
    datatype::{DataType, Reference},
};

#[derive(Serialize, Type)]
pub struct Demo {
    // #[serde(rename = "field2")]
    #[serde(rename(serialize = "field_ser", deserialize = "field_der"))]
    field: String,
}

fn main() {
    // let types = Types::default().register::<Demo>();
    // let out = specta_typescript::Typescript::new()
    //     .export(&specta_serde::apply(types).unwrap())
    //     .unwrap();

    // let out = specta_typescript::Typescript::new()
    //     // .format(specta_serde::format)
    //     .format(specta_serde::format_phases)
    //     .export(&Types::default().register::<Demo>())
    //     .unwrap();

    // println!("{}", out);

    let mut types = Types::default();
    let dt = Demo::definition(&mut types);
    let dt = match dt {
        DataType::Reference(Reference::Named(r)) => r.ty(&types).unwrap().clone(),
        _ => panic!("Expected a named reference"),
    };

    println!(
        "{:?}",
        // TODO: This should error?
        specta_typescript::primitives::inline(
            &specta_typescript::Typescript::new().format(specta_serde::format),
            &types,
            &dt
        )
    );
    println!(
        "{:?}",
        specta_typescript::primitives::inline(
            &specta_typescript::Typescript::new().format(specta_serde::format_phases),
            &types,
            &dt
        )
    );
}
