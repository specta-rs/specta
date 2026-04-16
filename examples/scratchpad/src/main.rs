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
    //     .export(&types, specta_serde::format)
    //     .unwrap();
    // println!("{}", out);

    let out = specta_typescript::Typescript::new()
        .export(
            &Types::default().register::<Demo>(),
            specta_serde::format_phases,
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
    //         specta_serde::map_types(&types)
    //             .unwrap()
    //             .as_ref(),
    //         specta_serde::map_datatype(&types, &dt)
    //             .unwrap()
    //             .as_ref()
    //     )
    // );
    println!("{:?}", {
        let (map_types, map_datatype) = specta_serde::format_phases;
        let mapped_types = map_types(&types).unwrap();
        let mapped_dt = map_datatype(&types, &dt).unwrap();
        specta_typescript::primitives::inline(
            &specta_typescript::Typescript::new(),
            mapped_types.as_ref(),
            mapped_dt.as_ref(),
        )
    });
}
