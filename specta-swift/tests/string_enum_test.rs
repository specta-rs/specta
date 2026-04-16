use serde::{Deserialize, Serialize};
use specta::{Type, Types};
use specta_swift::Swift;

/// Test enum with snake_case rename_all.
#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum JobStatus {
    Completed,
    Running,
    Failed,
    PendingApproval,
}

/// Test enum without rename_all.
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
    let string_output = swift.export(&serde_resolved, specta_swift::raw_format()).unwrap();
    let raw_output = swift.export(&raw_resolved, specta_swift::raw_format()).unwrap();

    println!("String enum test output:\n{}", string_output);
    println!("Regular enum test output:\n{}", raw_output);

    assert!(string_output.contains("enum JobStatus: Codable"));
    assert!(string_output.contains("case completed"));
    assert!(string_output.contains("case running"));
    assert!(string_output.contains("case failed"));
    assert!(string_output.contains("case pendingApproval"));

    // RegularEnum should stay a raw Specta enum without serde preprocessing.
    assert!(raw_output.contains("enum RegularEnum: Codable"));
    assert!(raw_output.contains("case option1"));
    assert!(raw_output.contains("case option2"));
    assert!(raw_output.contains("case option3"));
}
