use specta::{Type, TypeCollection};
use specta_swift::Swift;

/// Test enum with snake_case rename_all - should generate string enum
#[derive(Type)]
#[specta(rename_all = "snake_case")]
enum JobStatus {
    Completed,
    Running,
    Failed,
    PendingApproval,
}

/// Test enum without rename_all - should generate tagged union
#[derive(Type)]
enum RegularEnum {
    Option1,
    Option2,
    Option3,
}

#[test]
fn test_string_enum_generation() {
    let types = TypeCollection::default()
        .register::<JobStatus>()
        .register::<RegularEnum>();

    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("String enum test output:\n{}", output);

    // JobStatus should be a string enum (with String protocol and raw values)
    assert!(output.contains("enum JobStatus: String, Codable"));
    assert!(output.contains("case completed = \"completed\""));
    assert!(output.contains("case running = \"running\""));
    assert!(output.contains("case failed = \"failed\""));
    assert!(output.contains("case pendingApproval = \"pending_approval\""));

    // RegularEnum should be a tagged union
    assert!(output.contains("enum RegularEnum: Codable"));
    assert!(output.contains("case option1"));
    assert!(output.contains("case option2"));
    assert!(output.contains("case option3"));
}
