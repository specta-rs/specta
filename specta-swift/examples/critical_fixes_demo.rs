use specta::{Type, TypeCollection};
use specta_swift::Swift;
use std::time::Duration;

/// Comprehensive demonstration of all critical fixes
/// 
/// This example shows how Specta Swift now properly handles:
/// - Duration fields (converted to TimeInterval)
/// - Tuple structs (converted to proper Swift structs)
/// - Multi-line documentation (properly formatted)
/// - Complex union types with nested structs
/// 
/// All of these previously generated malformed Swift code,
/// but now work correctly with idiomatic Swift output.

/// A struct with Duration fields that previously generated malformed code
/// 
/// Before the fix, this would generate:
/// ```swift
/// let totalDuration:     let secs: UInt64
/// let nanos: UInt32
/// ```
/// 
/// Now it generates:
/// ```swift
/// let totalDuration: TimeInterval
/// ```
#[derive(Type)]
pub struct IndexerMetrics {
    pub total_duration: Duration,
    pub discovery_duration: Duration,
    pub processing_duration: Duration,
    pub content_duration: Duration,
}

/// A tuple struct that previously generated malformed code
/// 
/// Before the fix, this would generate:
/// ```swift
/// String}
/// ```
/// 
/// Now it generates:
/// ```swift
/// struct VolumeFingerprint: Codable {
///     let value: String
/// }
/// ```
#[derive(Type)]
pub struct VolumeFingerprint(pub String);

/// A multi-field tuple struct
#[derive(Type)]
pub struct Point(pub f64, pub f64);

/// A complex enum with multi-line documentation
/// 
/// This demonstrates that multi-line documentation is now properly
/// formatted with /// prefixes on each line, instead of being broken.
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

/// A complex union type with nested structs
#[derive(Type)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: Option<String>,
}

#[derive(Type)]
pub struct Admin {
    pub id: u32,
    pub name: String,
    pub permissions: Vec<String>,
    pub level: u8,
}

#[derive(Type)]
pub enum UserType {
    /// Anonymous user
    Anonymous,
    /// Regular user with basic info
    User(String, u32),
    /// Admin user with full permissions
    Admin {
        id: u32,
        name: String,
        permissions: Vec<String>,
        level: u8,
    },
    /// Registered user with nested struct
    Registered(User),
    /// Super admin with nested admin struct
    SuperAdmin(Admin),
    /// Guest user with session info
    Guest {
        session_id: String,
        expires_at: String,
    },
}

fn main() {
    // Create a type collection with all our types
    let types = TypeCollection::default()
        .register::<IndexerMetrics>()
        .register::<VolumeFingerprint>()
        .register::<Point>()
        .register::<SdPath>()
        .register::<UserType>()
        .register::<User>()
        .register::<Admin>();

    // Export to Swift with default settings
    let swift = Swift::default();
    let output = swift.export(&types).unwrap();
    
    println!("=== CRITICAL FIXES DEMONSTRATION ===\n");
    println!("This demonstrates that all critical issues have been fixed:\n");
    println!("✅ Duration fields now generate TimeInterval instead of malformed syntax");
    println!("✅ Tuple structs now generate proper Swift structs");
    println!("✅ Multi-line documentation is properly formatted");
    println!("✅ Complex unions with nested structs work correctly\n");
    
    println!("Generated Swift code:\n");
    println!("{}", output);
    
    // Also save to file for inspection
    swift.export_to("./CriticalFixesDemo.swift", &types).unwrap();
    println!("\n📁 Also saved to: CriticalFixesDemo.swift");
}
