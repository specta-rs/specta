use specta::{Type, TypeCollection};
use specta_swift::Swift;

/// Advanced example showcasing complex enum unions and their Swift representations
///
/// This example demonstrates how specta-swift handles complex enum scenarios
/// including nested types, generic enums, and custom Codable implementations.

/// Complex enum with mixed variant types
#[derive(Type)]
enum ApiResult<T, E> {
    /// Success with data and metadata
    Ok {
        data: T,
        status: u16,
        headers: Option<Vec<(String, String)>>,
        timestamp: String,
    },
    /// Error with detailed information
    Err {
        error: E,
        code: u32,
        message: String,
        retry_after: Option<u64>,
    },
    /// Loading state with progress
    Loading {
        progress: f32,
        estimated_completion: Option<String>,
    },
}

/// Enum demonstrating different field patterns
#[derive(Type)]
enum Shape {
    /// Unit variant
    None,
    /// Tuple variant
    Point(f64, f64),
    /// Named fields variant
    Circle { center: Point, radius: f64 },
    /// Referencing another struct
    Rectangle(Rectangle),
    /// Complex nested variant
    Line {
        start: Point,
        end: Point,
        style: LineStyle,
    },
    /// Very complex variant with multiple fields
    Complex {
        vertices: Vec<Point>,
        fill_color: Color,
        stroke_color: Option<Color>,
        stroke_width: f64,
        is_closed: bool,
    },
}

/// Supporting structs for the Shape enum
#[derive(Type)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Type)]
struct Rectangle {
    top_left: Point,
    bottom_right: Point,
}

#[derive(Type)]
struct LineStyle {
    dash_pattern: Option<Vec<f64>>,
    cap_style: String,
    join_style: String,
}

#[derive(Type)]
struct Color {
    red: f64,
    green: f64,
    blue: f64,
    alpha: f64,
}

/// Generic enum with constraints
#[derive(Type)]
enum Container<T> {
    Empty,
    Single(T),
    Multiple(Vec<T>),
    KeyValue(Vec<(String, T)>),
}

/// Enum with recursive references
#[derive(Type)]
enum Tree<T> {
    Leaf(T),
    Branch {
        left: Box<Tree<T>>,
        right: Box<Tree<T>>,
        value: T,
    },
}

/// String enum (will be converted to Swift String enum with Codable)
#[derive(Type)]
enum JobStatus {
    /// Job is queued and waiting to start
    Queued,
    /// Job is currently running
    Running,
    /// Job is paused (can be resumed)
    Paused,
    /// Job completed successfully
    Completed,
    /// Job failed with an error
    Failed,
    /// Job was cancelled by user
    Cancelled,
}

/// Mixed enum with both string-like and data variants
#[derive(Type)]
enum MixedEnum {
    /// Simple string-like variant
    Simple(String),
    /// Complex data variant
    WithFields {
        id: u32,
        name: String,
        metadata: Option<Vec<(String, String)>>,
    },
    /// Another simple variant
    Empty,
}

/// Event system enum
#[derive(Type)]
enum Event {
    /// User-related events
    User {
        user_id: u32,
        action: String,
        timestamp: String,
    },
    /// System events
    System {
        component: String,
        level: String,
        message: String,
    },
    /// Error events
    Error {
        code: u32,
        message: String,
        stack_trace: Option<String>,
    },
}

fn main() {
    println!("🚀 Advanced Unions Example - Complex enum scenarios");
    println!("{}", "=".repeat(60));

    // Create type collection
    let types = TypeCollection::default()
        .register::<ApiResult<String, String>>()
        .register::<Shape>()
        .register::<Point>()
        .register::<Rectangle>()
        .register::<LineStyle>()
        .register::<Color>()
        .register::<Container<String>>()
        .register::<Tree<String>>()
        .register::<JobStatus>()
        .register::<MixedEnum>()
        .register::<Event>();

    // Export with default settings
    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("📝 Generated Swift code:\n");
    println!("{}", output);

    // Write to file for inspection
    swift
        .export_to("./examples/generated/AdvancedUnions.swift", &types)
        .unwrap();
    println!("✅ Advanced unions exported to AdvancedUnions.swift");

    println!("\n🔍 Key Features Demonstrated:");
    println!("• Complex enum variants with named fields");
    println!("• Generic enums with type parameters");
    println!("• String enums with automatic Codable implementation");
    println!("• Mixed enums (both simple and complex variants)");
    println!("• Recursive type definitions");
    println!("• Nested struct references in enum variants");
    println!("• Custom Codable implementations for complex enums");
    println!("• Struct generation for named field variants");
    println!("• Tuple variant handling");
}
