use specta::{Type, TypeCollection};
use specta_swift::Swift;

// Test with common types that might not have Type implementations
#[derive(Type)]
struct TestStruct {
    // Basic types that should work
    id: u32,
    name: String,
    email: Option<String>,
    // These might not have Type implementations
    // uuid: uuid::Uuid,  // Commented out - likely not supported
    // created_at: chrono::DateTime<chrono::Utc>,  // Commented out - likely not supported
}

#[derive(Type)]
enum TestEnum {
    Unit,
    Tuple(String, u32),
    Named { id: u32, name: String },
}

#[test]
fn test_common_types() {
    let types = TypeCollection::default()
        .register::<TestStruct>()
        .register::<TestEnum>();

    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Generated Swift code:\n{}", output);

    // Test that basic types work
    assert!(output.contains("struct TestStruct"));
    assert!(output.contains("enum TestEnum"));
    assert!(output.contains("let id: UInt32"));
    assert!(output.contains("let name: String"));
    assert!(output.contains("let email: String?"));
}

// Test what happens when we try to use unsupported types
#[test]
fn test_unsupported_types() {
    // This test will fail to compile if UUID doesn't have Type implementation
    // Uncomment to test:
    /*
    #[derive(Type)]
    struct WithUuid {
        id: uuid::Uuid,
    }

    let types = TypeCollection::default().register::<WithUuid>();
    let swift = Swift::default();
    let output = swift.export(&types).unwrap();
    println!("UUID support: {}", output);
    */
}
