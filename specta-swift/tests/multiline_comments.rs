use specta::{Type, TypeCollection};
use specta_swift::Swift;

/// A path within the Spacedrive Virtual Distributed File System
///
/// This is the core abstraction that enables cross-device operations.
/// An SdPath can represent:
/// - A physical file at a specific path on a specific device
/// - A content-addressed file that can be sourced from any device
///
/// This enum-based approach enables resilient file operations by allowing
/// content-based paths to be resolved to optimal physical locations at runtime.
#[derive(Type)]
enum SdPath {
    /// A physical file path on a specific device
    Physical {
        /// The device ID where this file is located
        device_id: String,
        /// The file path on the device
        path: String,
    },
    /// A content-addressed file that can be sourced from any device
    Content {
        /// The content hash of the file
        hash: String,
        /// Optional preferred device for this content
        preferred_device: Option<String>,
    },
}

/// A simple struct with a single-line comment
#[derive(Type)]
struct SimpleStruct {
    /// The name of the struct
    name: String,
}

#[test]
fn test_multiline_comments() {
    let types = TypeCollection::default()
        .register::<SdPath>()
        .register::<SimpleStruct>();

    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Generated Swift code with comments:\n{}", output);

    // Test that multi-line comments are properly formatted
    assert!(output.contains("/// A path within the Spacedrive Virtual Distributed File System"));
    assert!(
        output.contains("/// This is the core abstraction that enables cross-device operations.")
    );
    assert!(output.contains("/// An SdPath can represent:"));
    assert!(output.contains("/// - A physical file at a specific path on a specific device"));
    assert!(output.contains("/// - A content-addressed file that can be sourced from any device"));
    assert!(output.contains("/// This enum-based approach enables resilient file operations"));

    // Test that single-line comments work too
    assert!(output.contains("/// A simple struct with a single-line comment"));

    // Note: Field-level comments are not currently supported by Specta
    // The enum cases and struct fields don't have individual comments
    // because Specta doesn't extract field-level documentation by default
}
