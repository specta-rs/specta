//! Demonstration of into_phases functionality
//!
//! This example shows how to use into_phases to create separate type definitions
//! for serialization and deserialization.

use specta::TypeCollection;
use specta_macros::Type;
use specta_serde::into_phases;

#[derive(Type, serde::Serialize, serde::Deserialize)]
struct User {
    pub id: u64,
    pub name: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    #[serde(skip_deserializing)]
    pub computed_field: String,
}

#[derive(Type, serde::Serialize, serde::Deserialize)]
struct Post {
    pub id: u64,
    pub title: String,
    pub author: User,
    #[serde(skip_serializing)]
    pub draft_content: Option<String>,
}

fn main() {
    println!("=== into_phases Demo ===\n");

    // Register types in a collection
    let mut types = TypeCollection::default();
    types = types.register::<User>();
    types = types.register::<Post>();

    println!("Original collection has {} types", types.len());
    for ndt in types.into_sorted_iter() {
        println!("  - {}", ndt.name());
    }
    println!();

    // Transform into phases
    let types = TypeCollection::default()
        .register::<User>()
        .register::<Post>();

    let phased = into_phases(types).expect("Failed to create phases");

    println!("After into_phases, collection has {} types", phased.len());
    for ndt in phased.into_sorted_iter() {
        println!("  - {}", ndt.name());
    }
    println!();

    println!("=== Key Points ===");
    println!("• Original types remain unchanged");
    println!("• _Serialize versions skip fields marked with skip_serializing");
    println!("• _Deserialize versions skip fields marked with skip_deserializing");
    println!("• References are updated: Post_Serialize references User_Serialize");
}
