//! Integration tests for Swift UUID and chrono support.

#![allow(clippy::unwrap_used, dead_code, missing_docs)]

use specta::{Type, Types};
use specta_swift::Swift;

#[derive(Type)]
struct WithUuid {
    id: uuid::Uuid,
    name: String,
}

#[test]
fn test_uuid_support() {
    let types = Types::default().register::<WithUuid>();
    let swift = Swift::default();
    let output = swift.export(&types, specta_serde::Format).unwrap();

    println!("UUID support test:\n{}", output);

    // UUID should be converted to String in Swift
    assert!(output.contains("let id: String"));
    assert!(output.contains("let name: String"));
}

#[derive(Type)]
struct WithChrono {
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::NaiveDateTime,
    name: String,
}

#[test]
fn test_chrono_support() {
    let types = Types::default().register::<WithChrono>();
    let swift = Swift::default();
    let output = swift.export(&types, specta_serde::Format).unwrap();

    println!("Chrono support test:\n{}", output);

    // Chrono types should be converted to String in Swift
    assert!(output.contains("let createdAt: String"));
    assert!(output.contains("let updatedAt: String"));
    assert!(output.contains("let name: String"));
}
