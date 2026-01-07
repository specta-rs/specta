//! Comprehensive example demonstrating serde attribute handling with specta-serde
//!
//! This example shows how to use the specta-serde crate to handle various serde attributes
//! and apply transformations for both serialization and deserialization phases.

use serde::{Deserialize, Serialize};
use specta::TypeCollection;
use specta_macros::Type;
use specta_serde::{
    SerdeMode, apply_serde_transformations, process_for_deserialization, process_for_serialization,
};

// Example struct with various serde attributes
#[derive(Type, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub user_id: u64,
    pub first_name: String,
    pub last_name: String,
    #[serde(rename = "emailAddress")]
    pub email: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub phone_number: Option<String>,
    #[serde(default)]
    pub is_active: bool,
}

// Example enum with rename_all
#[derive(Type, Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    NotStarted,
    InProgress,
    Completed,
    #[serde(rename = "failed_with_error")]
    Failed,
}

// Example of internally tagged enum
#[derive(Type, Serialize, Deserialize, Debug)]
#[serde(tag = "type")]
pub enum Message {
    #[serde(rename = "text")]
    Text { content: String },
    #[serde(rename = "image")]
    Image {
        url: String,
        alt_text: Option<String>,
    },
    #[serde(rename = "system")]
    System { message: String },
}

// Example of adjacently tagged enum
#[derive(Type, Serialize, Deserialize, Debug)]
#[serde(tag = "type", content = "data")]
pub enum ApiResponse {
    Success(String),
    Error { code: u32, message: String },
}

// Example of transparent wrapper
#[derive(Type, Serialize, Deserialize, Debug)]
#[serde(transparent)]
pub struct UserId(pub u64);

// Example of flattened fields
#[derive(Type, Serialize, Deserialize, Debug)]
pub struct Address {
    pub street: String,
    pub city: String,
    pub country: String,
}

#[derive(Type, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub struct User {
    pub id: UserId,
    pub name: String,
    #[serde(flatten)]
    pub address: Address,
    pub status: JobStatus,
}

// Example with various skip attributes
#[derive(Type, Serialize, Deserialize, Debug)]
pub struct ApiKey {
    pub id: String,
    #[serde(skip_serializing)]
    pub secret: String,
    #[serde(skip_deserializing)]
    pub created_at: String,
    #[serde(skip)]
    pub internal_notes: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Specta-Serde Transformations Example ===\n");

    // Create a type collection with our example types
    let types = TypeCollection::default()
        .register::<UserProfile>()
        .register::<JobStatus>()
        .register::<Message>()
        .register::<ApiResponse>()
        .register::<UserId>()
        .register::<User>()
        .register::<ApiKey>();

    println!("Original type collection has {} types", types.len());

    // Process for serialization
    println!("\n--- Processing for Serialization ---");
    let ser_types = process_for_serialization(&types)?;
    println!(
        "Serialization type collection has {} types",
        ser_types.len()
    );

    // Process for deserialization
    println!("\n--- Processing for Deserialization ---");
    let de_types = process_for_deserialization(&types)?;
    println!(
        "Deserialization type collection has {} types",
        de_types.len()
    );

    // Example of applying transformations to individual types
    println!("\n--- Individual Type Transformations ---");

    // Get a specific type and transform it
    for ndt in types.into_unsorted_iter() {
        if ndt.name() == "UserProfile" {
            println!("\nTransforming UserProfile:");

            let ser_transformed = apply_serde_transformations(ndt.ty(), SerdeMode::Serialize)?;
            println!("  Serialization transformation: {:?}", ser_transformed);

            let de_transformed = apply_serde_transformations(ndt.ty(), SerdeMode::Deserialize)?;
            println!("  Deserialization transformation: {:?}", de_transformed);
            break;
        }
    }

    // Demonstrate rename_all transformations
    println!("\n--- Rename All Transformations ---");
    for ndt in types.into_unsorted_iter() {
        if ndt.name() == "JobStatus" {
            println!("\nTransforming JobStatus enum:");

            let ser_transformed = apply_serde_transformations(ndt.ty(), SerdeMode::Serialize)?;
            println!("  Serialization: {:?}", ser_transformed);
            break;
        }
    }

    println!("\n=== Example completed successfully ===");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use specta_serde::{SerdeMode, apply_serde_transformations};

    #[test]
    fn test_serde_transformations() {
        let types = TypeCollection::default()
            .register::<UserProfile>()
            .register::<JobStatus>();

        // Test serialization transformations
        let ser_types = process_for_serialization(&types).expect("Serialization processing failed");
        assert_eq!(ser_types.len(), types.len());

        // Test deserialization transformations
        let de_types =
            process_for_deserialization(&types).expect("Deserialization processing failed");
        assert_eq!(de_types.len(), types.len());
    }

    #[test]
    fn test_individual_type_transformation() {
        let types = TypeCollection::default().register::<UserProfile>();

        for ndt in types.into_unsorted_iter() {
            if ndt.name() == "UserProfile" {
                let ser_result = apply_serde_transformations(ndt.ty(), SerdeMode::Serialize);
                assert!(
                    ser_result.is_ok(),
                    "Serialization transformation should succeed"
                );

                let de_result = apply_serde_transformations(ndt.ty(), SerdeMode::Deserialize);
                assert!(
                    de_result.is_ok(),
                    "Deserialization transformation should succeed"
                );
                break;
            }
        }
    }

    #[test]
    fn test_enum_transformations() {
        let types = TypeCollection::default().register::<JobStatus>();

        for ndt in types.into_unsorted_iter() {
            if ndt.name() == "JobStatus" {
                let ser_result = apply_serde_transformations(ndt.ty(), SerdeMode::Serialize);
                assert!(
                    ser_result.is_ok(),
                    "Enum serialization transformation should succeed"
                );

                let de_result = apply_serde_transformations(ndt.ty(), SerdeMode::Deserialize);
                assert!(
                    de_result.is_ok(),
                    "Enum deserialization transformation should succeed"
                );
                break;
            }
        }
    }

    #[test]
    fn test_transparent_wrapper() {
        let types = TypeCollection::default().register::<UserId>();

        for ndt in types.into_unsorted_iter() {
            if ndt.name() == "UserId" {
                let ser_result = apply_serde_transformations(ndt.ty(), SerdeMode::Serialize);
                assert!(
                    ser_result.is_ok(),
                    "Transparent type serialization should succeed"
                );
                break;
            }
        }
    }
}
