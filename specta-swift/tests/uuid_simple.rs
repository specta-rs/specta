use specta::{Type, TypeCollection};
use specta_swift::Swift;

// Test with UUID - this should work now that we have the uuid feature enabled
#[derive(Type)]
struct WithUuid {
    id: uuid::Uuid,
    name: String,
}

#[derive(Type)]
struct WithChrono {
    created_at: chrono::DateTime<chrono::Utc>,
    updated_at: chrono::NaiveDateTime,
    name: String,
}

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
