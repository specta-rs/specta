use serde::{Deserialize, Serialize};
use specta::{Type, TypeCollection};
use specta_serde::{SerdeMode, process_for_deserialization, process_for_serialization};
use specta_typescript::Typescript;

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TestStruct {
    pub field_name: String,
    #[serde(skip_serializing)]
    pub serialize_skip: String,
    #[serde(skip_deserializing)]
    pub deserialize_skip: String,
    #[serde(skip)]
    pub both_skip: String,
    pub normal_field: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let types = TypeCollection::default().register::<TestStruct>();

    println!("=== Original Types ===");
    for ndt in types.into_unsorted_iter() {
        println!("Type: {}", ndt.name());
        println!("DataType: {:?}", ndt.ty());
    }

    println!("\n=== Serialization Processing ===");
    let ser_types = process_for_serialization(&types)?;
    for ndt in ser_types.into_unsorted_iter() {
        println!("Type: {}", ndt.name());
        println!("DataType: {:?}", ndt.ty());
    }

    println!("\n=== Deserialization Processing ===");
    let de_types = process_for_deserialization(&types)?;
    for ndt in de_types.into_unsorted_iter() {
        println!("Type: {}", ndt.name());
        println!("DataType: {:?}", ndt.ty());
    }

    println!("\n=== TypeScript Output (Serialization) ===");
    let ts_ser = Typescript::default()
        .with_serde(SerdeMode::Serialize)
        .export(&types)?;
    println!("{}", ts_ser);

    println!("\n=== TypeScript Output (Deserialization) ===");
    let ts_de = Typescript::default()
        .with_serde(SerdeMode::Deserialize)
        .export(&types)?;
    println!("{}", ts_de);

    println!("\n=== TypeScript Output (No Serde) ===");
    let ts_no_serde = Typescript::default().export(&types)?;
    println!("{}", ts_no_serde);

    Ok(())
}
