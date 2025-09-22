use specta::{Type, TypeCollection};
use specta_swift::Swift;

/// Test enum that should serialize as strings (like your JobStatus)
#[derive(Type)]
#[specta(rename_all = "snake_case")]
enum JobStatus {
    Queued,
    Running,
    Paused,
    Completed,
    Failed,
    Cancelled,
}

#[test]
fn test_string_enum_issue() {
    let types = TypeCollection::default().register::<JobStatus>();
    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Current output (tagged union format):\n{}", output);

    // This currently generates:
    // enum JobStatus: Codable {
    //     case queued
    //     case running
    //     case paused
    //     case completed
    //     case failed
    //     case cancelled
    // }
    //
    // But it should generate a string enum that matches serde's behavior
    // The issue is that Specta doesn't automatically detect serde's string serialization
}

#[test]
fn test_workaround_solution() {
    // Workaround: Create a simple string-based type for the API
    #[derive(Type)]
    struct JobStatusResponse {
        status: String,
    }

    let types = TypeCollection::default().register::<JobStatusResponse>();
    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Workaround output (string field):\n{}", output);

    // This generates:
    // struct JobStatusResponse: Codable {
    //     let status: String
    // }
    //
    // This matches the actual JSON format: {"status": "completed"}
}
