use serde::{Deserialize, Serialize};
use specta::datatype::DataType;
use specta::{Type, Types};
use specta_swift::Swift;

fn raw_format() -> (
    impl for<'a> Fn(&'a Types) -> Result<std::borrow::Cow<'a, Types>, specta_swift::FormatError>,
    impl for<'a> Fn(
        &'a Types,
        &'a DataType,
    ) -> Result<std::borrow::Cow<'a, DataType>, specta_swift::FormatError>,
) {
    (
        |types| Ok(std::borrow::Cow::Borrowed(types)),
        |_, dt| Ok(std::borrow::Cow::Borrowed(dt)),
    )
}

/// Test enum with snake_case rename_all - should generate string enum
#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
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
    let swift = Swift::default();
    let serde_resolved = Types::default().register::<JobStatus>();
    let raw_resolved = Types::default().register::<RegularEnum>();
    let string_output = swift.export(&serde_resolved, specta_serde::format).unwrap();
    let raw_output = swift.export(&raw_resolved, raw_format()).unwrap();

    println!("String enum test output:\n{}", string_output);
    println!("Regular enum test output:\n{}", raw_output);

    // JobStatus should be a string enum (with String protocol and raw values)
    assert!(string_output.contains("enum JobStatus: String, Codable"));
    assert!(string_output.contains("case completed = \"completed\""));
    assert!(string_output.contains("case running = \"running\""));
    assert!(string_output.contains("case failed = \"failed\""));
    assert!(string_output.contains("case pendingApproval = \"pending_approval\""));

    // RegularEnum should stay a raw Specta enum without serde preprocessing.
    assert!(raw_output.contains("enum RegularEnum: Codable"));
    assert!(raw_output.contains("case option1"));
    assert!(raw_output.contains("case option2"));
    assert!(raw_output.contains("case option3"));
}
