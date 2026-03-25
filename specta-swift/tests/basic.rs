use specta::{ResolvedTypes, Type, Types};
use specta_swift::Swift;

#[derive(Type)]
struct User {
    name: String,
    age: u32,
    active: bool,
}

#[derive(Type)]
enum Status {
    Active,
    Inactive,
    Pending { reason: String },
    Error(String),
}

#[test]
fn test_basic_export() {
    let types = Types::default().register::<User>().register::<Status>();
    let resolved = ResolvedTypes::from_resolved_types(types);

    let swift = Swift::default();
    let output = swift.export(&resolved).unwrap();

    println!("Generated Swift code:\n{}", output);

    // Basic assertions
    assert!(output.contains("struct User"));
    assert!(output.contains("enum Status"));
    assert!(output.contains("let name: String"));
    assert!(output.contains("let age: UInt32"));
    assert!(output.contains("case active"));
    assert!(output.contains("case pending"));
}
