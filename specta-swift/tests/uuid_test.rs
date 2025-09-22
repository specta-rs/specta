use specta::{Type, TypeCollection};
use specta_swift::Swift;

// Test with UUID - this should work if the uuid feature is enabled
#[cfg(feature = "uuid")]
#[derive(Type)]
struct WithUuid {
    id: uuid::Uuid,
    name: String,
}

#[cfg(feature = "uuid")]
#[test]
fn test_uuid_support() {
    let types = TypeCollection::default().register::<WithUuid>();
    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("UUID support test:\n{}", output);

    // UUID should be converted to String in Swift
    assert!(output.contains("let id: String"));
    assert!(output.contains("let name: String"));
}

#[cfg(not(feature = "uuid"))]
#[test]
fn test_uuid_not_available() {
    println!("UUID feature not enabled - this is expected");
    // This test passes when UUID feature is not enabled
}

// Test with chrono - this should work if the chrono feature is enabled
#[cfg(feature = "chrono")]
#[derive(Type)]
struct WithChrono {
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::NaiveDateTime,
    name: String,
}

#[cfg(feature = "chrono")]
#[test]
fn test_chrono_support() {
    let types = TypeCollection::default().register::<WithChrono>();
    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Chrono support test:\n{}", output);

    // Chrono types should be converted to String in Swift
    assert!(output.contains("let createdAt: String"));
    assert!(output.contains("let updatedAt: String"));
    assert!(output.contains("let name: String"));
}

#[cfg(not(feature = "chrono"))]
#[test]
fn test_chrono_not_available() {
    println!("Chrono feature not enabled - this is expected");
    // This test passes when chrono feature is not enabled
}
