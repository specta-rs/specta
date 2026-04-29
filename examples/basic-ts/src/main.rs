use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use serde_with::{DisplayFromStr, OneOrMany, serde_as};
use specta::{
    Type, Types,
    datatype::{DataType, Reference},
};
use specta_typescript::Typescript;
use std::convert::TryFrom;

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

#[derive(Serialize, Deserialize, Type)]
pub struct UserWire {
    id: String,
}

#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(into = "UserWire")]
pub struct UserInto {
    id: String,
}

impl From<UserInto> for UserWire {
    fn from(value: UserInto) -> Self {
        Self { id: value.id }
    }
}

#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(from = "UserWire")]
pub struct UserFrom {
    id: String,
}

impl From<UserWire> for UserFrom {
    fn from(value: UserWire) -> Self {
        Self { id: value.id }
    }
}

#[derive(Clone, Serialize, Deserialize, Type)]
#[serde(try_from = "UserWire")]
pub struct UserTryFrom {
    id: String,
}

impl TryFrom<UserWire> for UserTryFrom {
    type Error = String;

    fn try_from(value: UserWire) -> Result<Self, Self::Error> {
        Ok(Self { id: value.id })
    }
}

#[derive(Serialize, Deserialize, Type)]
pub struct UsesSerdeConversions {
    into_user: UserInto,
    from_user: UserFrom,
    try_from_user: UserTryFrom,
}

#[derive(Serialize_repr, Deserialize_repr, Type, PartialEq, Debug)]
#[specta(type = u8)]
#[repr(u8)]
enum SmallPrime {
    Two = 2,
    Three = 3,
    Five = 5,
    Seven = 7,
}

#[serde_as]
#[derive(Serialize, Deserialize, Type)]
pub struct SerdeWithDisplayFromStr {
    #[serde_as(as = "DisplayFromStr")]
    #[specta(type = String)]
    build_number: u64,
}

#[derive(Serialize, Deserialize, Type)]
#[serde(untagged)]
pub enum OneOrManyString {
    One(String),
    Many(Vec<String>),
}

#[serde_as]
#[derive(Serialize, Deserialize, Type)]
pub struct SerdeWithOneOrMany {
    #[serde_as(as = "OneOrMany<_>")]
    #[specta(type = specta_serde::Phased<Vec<String>, OneOrManyString>)]
    tags: Vec<String>,
}

fn main() {
    {
        let mut types = Types::default();
        println!("{:#?}", HelloWorld::definition(&mut types));
        println!(
            "{:#?}",
            match HelloWorld::definition(&mut types) {
                DataType::Reference(Reference::Named(r)) => types.get(&r).unwrap(),
                _ => unreachable!(),
            }
        );
    }

    // TODO
    // {
    //     let mut types = Types::default().register::<NotPhaseSpecific>();
    //     let def = HelloWorld::definition(&mut types);
    //     let types = map_types(&types).unwrap().into_owned();
    //     println!(
    //         "{:#?}",
    //         match def {
    //             DataType::Reference(Reference::Named(r)) => r.get(&types).unwrap(),
    //             _ => unreachable!(),
    //         }
    //     );

    //     println!("{:#?}", types);
    //     println!("Types Count: {}", types.len());
    // }

    // TODO
    // {
    //     let types = Types::default()
    //         .register::<NotPhaseSpecific>()
    //         .register::<NotPhaseSpecificButReferencing>()
    //         .register::<HelloWorld>()
    //         .register::<TestingFlatten>()
    //         .register::<SerdeExternalExample>()
    //         .register::<SerdeInternalExample>()
    //         .register::<SerdeAdjacentExample>()
    //         .register::<SerdeUntaggedExample>()
    //         .register::<UserWire>()
    //         .register::<UserInto>()
    //         .register::<UserFrom>()
    //         .register::<UserTryFrom>()
    //         .register::<UsesSerdeConversions>()
    //         .register::<SmallPrime>()
    //         .register::<SerdeWithDisplayFromStr>()
    //         .register::<SerdeWithOneOrMany>();
    //     println!(
    //         "RAW:\n{}",
    //         Typescript::default()
    //             .export(
    //                 &types,
    //                 // You don't want to copy this.
    //                 // Use `specta_serde`. This is just good for internal testing.
    //                 (
    //                     |types| Ok(Cow::Borrowed(types)),
    //                     |_, dt| Ok(Cow::Borrowed(dt)),
    //                 )
    //             )
    //             .unwrap()
    //     );
    //     match map_types(&types) {
    //         Ok(_) => println!(
    //             "specta_serde::Format(...):\n{}",
    //             Typescript::default()
    //                 .export(&types, specta_serde::Format)
    //                 .unwrap()
    //         ),
    //         Err(err) => println!("specta_serde::Format(...) ERROR: {err}"),
    //     }
    //     println!(
    //         "specta_serde::PhasesFormat(...):\n{}",
    //         Typescript::default()
    //             .export(&types, specta_serde::PhasesFormat)
    //             .unwrap()
    //     );
    // }

    {
        let serde_with_types = Types::default()
            .register::<SerdeWithDisplayFromStr>()
            .register::<SerdeWithOneOrMany>();
        println!(
            "serde_with + specta_serde::PhasesFormat(...):\n{}",
            Typescript::default()
                .export(&serde_with_types, specta_serde::PhasesFormat)
                .unwrap()
        );
    }
}
