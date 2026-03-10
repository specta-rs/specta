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

#[derive(Serialize, Type)]
#[serde(rename = "HelloWorld2")]
pub struct NotPhaseSpecific {
    #[serde(rename = "b")]
    a: String,
}

fn main() {
    {
        let mut types = TypeCollection::default();
        println!("{:#?}", HelloWorld::definition(&mut types));
        println!(
            "{:#?}",
            match HelloWorld::definition(&mut types) {
                DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
                _ => unreachable!(),
            }
        );
    }

    {
        let mut types = TypeCollection::default().register::<NotPhaseSpecific>();
        let def = HelloWorld::definition(&mut types);
        let types = specta_serde::apply_phases(types);
        println!(
            "{:#?}",
            match def {
                DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
                _ => unreachable!(),
            }
        ); // TODO: We need to solve referential integrity

        println!("{:#?}", types);
        println!("Types Count: {}", types.len());
    }

    // {
    //     let mut types = TypeCollection::default();
    //     let def = HelloWorld::definition(&mut types);
    //     let types = specta_serde::apply(types);
    //     println!(
    //         "{:#?}",
    //         match def {
    //             DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
    //             _ => unreachable!(),
    //         }
    //     );
    // }

    // println!("{}", Typescript::default().export(&types).unwrap());
    // println!(
    //     "{}",
    //     Typescript::default()
    //         .export(&specta_serde::testing(types))
    //         .unwrap()
    // ); // TODO
}
