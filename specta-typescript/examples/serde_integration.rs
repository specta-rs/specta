use serde::{Deserialize, Serialize};
use specta::{Type, TypeCollection};
use specta_serde::SerdeMode;
use specta_typescript::Typescript;

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub user_id: u32,
    pub user_name: String,
    #[serde(skip_serializing)]
    pub password: Option<String>,
    #[serde(default)]
    pub is_active: bool,
}

#[derive(Type, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Status {
    Active { last_seen: String },
    Inactive,
    Banned { reason: String },
}

#[derive(Type, Serialize, Deserialize)]
pub struct Response {
    pub user: User,
    pub status: Status,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let types = TypeCollection::default().register::<Response>();

    // Test serialization mode
    println!("=== Serialization Mode ===");
    let ts_serialize = Typescript::default()
        .with_serde_serialize()
        .export(&types)?;
    println!("{}", ts_serialize);

    // Test deserialization mode
    println!("\n=== Deserialization Mode ===");
    let ts_deserialize = Typescript::default()
        .with_serde_deserialize()
        .export(&types)?;
    println!("{}", ts_deserialize);

    // Test without serde processing
    println!("\n=== No Serde Processing ===");
    let ts_no_serde = Typescript::default().export(&types)?;
    println!("{}", ts_no_serde);

    // Test with custom serde mode
    println!("\n=== Custom Serde Mode ===");
    let ts_custom = Typescript::default()
        .with_serde(SerdeMode::Serialize)
        .export(&types)?;
    println!("{}", ts_custom);

    Ok(())
}
