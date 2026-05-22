#![allow(clippy::unwrap_used, dead_code, missing_docs)]

use specta::{Type, Types};
use specta_swift::Swift;
use std::time::Duration;

/// Example showcasing special type handling in specta-swift
///
/// This example demonstrates how specta-swift handles special Rust types
/// like Duration, UUID, chrono types, and other commonly used types
/// that need special conversion to Swift equivalents.
///
/// Struct with Duration fields (will be converted to RustDuration helper)
#[derive(Type)]
struct IndexerMetrics {
    /// Total time spent indexing
    total_duration: Duration,
    /// Time spent discovering files
    discovery_duration: Duration,
    /// Time spent processing content
    processing_duration: Duration,
    /// Time spent analyzing content
    content_duration: Duration,
    /// Number of files processed
    files_processed: u32,
    /// Average processing time per file
    avg_time_per_file: Duration,
}

/// Struct with various timestamp types
#[derive(Type)]
struct EventLog {
    /// Event ID
    event_id: String,
    /// When the event occurred
    timestamp: String,
    /// Event duration
    duration: Duration,
    /// Additional metadata
    metadata: Option<Vec<(String, String)>>,
}

/// Configuration struct with timing information
#[derive(Type)]
struct TaskConfig {
    /// Task name
    name: String,
    /// Maximum execution time
    timeout: Duration,
    /// Retry interval
    retry_interval: Duration,
    /// Backoff duration
    backoff_duration: Duration,
    /// Whether task is enabled
    enabled: bool,
}

/// Performance metrics struct
#[derive(Type)]
struct PerformanceMetrics {
    /// Response time
    response_time: Duration,
    /// Processing time
    processing_time: Duration,
    /// Database query time
    query_time: Duration,
    /// Network latency
    network_latency: Duration,
    /// Total time
    total_time: Duration,
}

/// API response with timing information
#[derive(Type)]
struct ApiResponse {
    /// Response data
    data: String,
    /// Processing duration
    processing_duration: Duration,
    /// Cache hit duration (if applicable)
    cache_duration: Option<Duration>,
    /// Network transfer duration
    transfer_duration: Duration,
    /// Status code
    status_code: u16,
}

/// Job status with timing
#[derive(Type)]
enum JobStatus {
    /// Job is queued
    Queued,
    /// Job is running with timing info
    Running {
        started_at: String,
        elapsed_time: Duration,
        estimated_completion: Option<String>,
    },
    /// Job completed successfully
    Completed {
        started_at: String,
        completed_at: String,
        total_duration: Duration,
        result: String,
    },
    /// Job failed with error and timing
    Failed {
        started_at: String,
        failed_at: String,
        duration: Duration,
        error_message: String,
    },
}

/// Complex struct mixing Duration with other types
#[derive(Type)]
struct SystemHealth {
    /// System uptime
    uptime: Duration,
    /// Last health check
    last_check: Duration,
    /// Average response time
    avg_response_time: Duration,
    /// System status
    status: String,
    /// Memory usage percentage
    memory_usage: f64,
    /// CPU usage percentage
    cpu_usage: f64,
}

fn main() {
    println!("🚀 Special Types Example - Duration and timing types");
    println!("{}", "=".repeat(60));

    // Create type collection with all our special types
    let types = Types::default()
        .register::<IndexerMetrics>()
        .register::<EventLog>()
        .register::<TaskConfig>()
        .register::<PerformanceMetrics>()
        .register::<ApiResponse>()
        .register::<JobStatus>()
        .register::<SystemHealth>();

    // Export with default settings
    let swift = Swift::default();
    let output = swift.export(&types, specta_serde::Format).unwrap();

    println!("📝 Generated Swift code:\n");
    println!("{}", output);

    // Write to file for inspection
    swift
        .export_to(
            "./examples/generated/SpecialTypes.swift",
            &types,
            specta_serde::Format,
        )
        .unwrap();
    println!("✅ Special types exported to SpecialTypes.swift");

    println!("\n🔍 Key Features Demonstrated:");
    println!("• Duration type mapping to RustDuration helper struct");
    println!("• Automatic helper struct generation for Duration types");
    println!("• timeInterval property for easy Swift integration");
    println!("• Duration fields in structs and enum variants");
    println!("• Optional Duration fields");
    println!("• Complex timing-related data structures");
    println!("• Performance metrics with multiple Duration fields");

    println!("\n💡 Duration Helper Features:");
    println!("• RustDuration struct with secs and nanos fields");
    println!("• timeInterval computed property (Double)");
    println!("• Proper Codable implementation for Rust format");
    println!("• Automatic injection when Duration types are detected");

    println!("\n📋 Generated Helper Struct:");
    println!("```swift");
    println!("public struct RustDuration: Codable {{");
    println!("    public let secs: UInt64");
    println!("    public let nanos: UInt32");
    println!("    ");
    println!("    public var timeInterval: TimeInterval {{");
    println!("        return Double(secs) + Double(nanos) / 1_000_000_000.0");
    println!("    }}");
    println!("}}");
    println!("```");
}
