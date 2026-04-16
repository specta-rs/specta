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
    //     .export(&specta_serde::apply(types).unwrap(), specta_typescript::serde::format)
    //     .unwrap();
    // println!("{}", out);

    let out = specta_typescript::Typescript::new()
        .export(
            &Types::default().register::<Demo>(),
            specta_typescript::serde::format_phases,
        )
        .unwrap();
    println!("{}", out);

    let mut types = Types::default();
    let dt = Demo::definition(&mut types);
    let dt = match dt {
        DataType::Reference(Reference::Named(r)) => r.ty(&types).unwrap().clone(),
        _ => panic!("Expected a named reference"),
    };

    // println!(
    //     "{:?}",
    //     // TODO: This should error?
    //     specta_typescript::primitives::inline(
    //         &specta_typescript::Typescript::new(),
    //         specta_typescript::serde::map_types(&types)
    //             .unwrap()
    //             .as_ref(),
    //         specta_typescript::serde::map_datatype(&types, &dt)
    //             .unwrap()
    //             .as_ref()
    //     )
    // );
    println!(
        "{:?}",
        specta_typescript::primitives::inline(
            &specta_typescript::Typescript::new(),
            specta_typescript::serde::map_phases_types(&types)
                .unwrap()
                .as_ref(),
            specta_typescript::serde::map_phases_datatype(&types, &dt)
                .unwrap()
                .as_ref()
        )
    );
}
