use serde::Serialize;
use specta::{
    Type, TypeCollection,
    datatype::{DataType, Reference},
};
use specta_typescript::Typescript;

#[derive(Serialize, Type)]
#[serde(rename = "HelloWorld2")]
pub struct HelloWorld {
    #[serde(rename = "a_renamed")]
    a: String,

    #[serde(rename(serialize = "b_serialize_name"))]
    b: String,
}

#[derive(Serialize, Type)]
#[serde(rename = "HelloWorld2")]
pub struct NotPhaseSpecific {
    #[serde(rename = "b")]
    a: String,
}

#[derive(Serialize, Type)]
pub struct NotPhaseSpecificButReferencing {
    a: HelloWorld,
}

#[derive(Serialize, Type)]
pub struct TestingFlatten {
    a: String,
    #[serde(flatten)]
    flattened: TestingFlattenFlattened,
}

#[derive(Serialize, Type)]
pub struct TestingFlattenFlattened {
    b: String,
}

#[derive(Serialize, Type)]
pub enum SerdeExternalExample {
    Unit,
    Newtype(String),
    Struct { value: String },
}

#[derive(Serialize, Type)]
#[serde(tag = "type")]
pub enum SerdeInternalExample {
    Unit,
    Struct { value: String },
}

#[derive(Serialize, Type)]
#[serde(tag = "type", content = "data")]
pub enum SerdeAdjacentExample {
    Unit,
    Newtype(String),
    Struct { value: String },
}

#[derive(Serialize, Type)]
#[serde(untagged)]
pub enum SerdeUntaggedExample {
    String(String),
    Object { value: String },
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
        );

        println!("{:#?}", types);
        println!("Types Count: {}", types.len());
    }

    {
        let types = TypeCollection::default()
            .register::<NotPhaseSpecific>()
            .register::<NotPhaseSpecificButReferencing>()
            .register::<HelloWorld>()
            .register::<TestingFlatten>()
            .register::<SerdeExternalExample>()
            .register::<SerdeInternalExample>()
            .register::<SerdeAdjacentExample>()
            .register::<SerdeUntaggedExample>();
        println!("RAW:\n{}", Typescript::default().export(&types).unwrap());
        // println!(
        //     "specta_serde::apply(...): `{}",
        //     Typescript::default()
        //         .export(&specta_serde::apply(types.clone()))
        //         .unwrap()
        // );
        println!(
            "specta_serde::apply_phases(...):\n{}",
            Typescript::default()
                .export(&specta_serde::apply_phases(types))
                .unwrap()
        );
    }
}
