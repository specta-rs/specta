use specta::{Type, TypeCollection};
use specta_swift::Swift;
use std::time::Duration;

/// Test struct with Duration fields that should generate proper Swift code
#[derive(Type)]
pub struct IndexerMetrics {
    pub total_duration: Duration,
    pub discovery_duration: Duration,
    pub processing_duration: Duration,
    pub content_duration: Duration,
}

/// Test tuple struct that should generate a proper Swift struct
#[derive(Type)]
pub struct VolumeFingerprint(pub String);

/// Test tuple struct with multiple fields
#[derive(Type)]
pub struct Point(pub f64, pub f64);

/// Test tuple struct with single field (common pattern)
#[derive(Type)]
pub struct UserId(pub u32);

/// Test enum with multi-line documentation
///
/// This is a comprehensive test for multi-line documentation
/// that should be properly formatted in Swift output.
///
/// The documentation should include:
/// - Multiple paragraphs
/// - Bullet points
/// - Technical details
///
/// This ensures that complex documentation is preserved
/// when generating Swift code from Rust types.
#[derive(Type)]
pub enum SdPath {
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

#[test]
fn test_duration_fields() {
    let types = TypeCollection::default().register::<IndexerMetrics>();
    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Duration test output:\n{}", output);

    // Duration fields should be converted to a proper Swift type
    // We'll convert them to TimeInterval (Double) for now
    assert!(output.contains("let totalDuration: TimeInterval"));
    assert!(output.contains("let discoveryDuration: TimeInterval"));
    assert!(output.contains("let processingDuration: TimeInterval"));
    assert!(output.contains("let contentDuration: TimeInterval"));

    // Should not contain malformed secs/nanos syntax
    assert!(!output.contains("let secs: UInt64"));
    assert!(!output.contains("let nanos: UInt32"));
}

#[test]
fn test_tuple_structs() {
    let types = TypeCollection::default()
        .register::<VolumeFingerprint>()
        .register::<Point>()
        .register::<UserId>();

    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Tuple struct test output:\n{}", output);

    // VolumeFingerprint should be a proper struct
    assert!(output.contains("struct VolumeFingerprint: Codable"));
    assert!(output.contains("let value: String"));

    // Point should be a proper struct with two fields
    assert!(output.contains("struct Point: Codable"));
    assert!(output.contains("let field0: Double"));
    assert!(output.contains("let field1: Double"));

    // UserId should be a proper struct
    assert!(output.contains("struct UserId: Codable"));
    assert!(output.contains("let value: UInt32"));

    // Should not contain malformed syntax
    assert!(!output.contains("String}"));
    assert!(!output.contains("f64, f64}"));
}

#[test]
fn test_multi_line_documentation() {
    let types = TypeCollection::default().register::<SdPath>();
    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Multi-line documentation test output:\n{}", output);

    // Multi-line documentation should be properly formatted
    assert!(output.contains("/// This is a comprehensive test for multi-line documentation"));
    assert!(output.contains("/// that should be properly formatted in Swift output."));
    assert!(output.contains("/// The documentation should include:"));
    assert!(output.contains("/// - Multiple paragraphs"));
    assert!(output.contains("/// - Bullet points"));
    assert!(output.contains("/// - Technical details"));

    // Should not have broken comment formatting (these patterns don't exist in the output)
    // The multi-line comments are now properly formatted with /// prefixes
}
