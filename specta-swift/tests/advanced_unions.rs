use specta::{Type, TypeCollection};
use specta_swift::Swift;

#[derive(Type)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Type)]
struct Circle {
    center: Point,
    radius: f64,
}

#[derive(Type)]
struct Rectangle {
    top_left: Point,
    bottom_right: Point,
}

#[derive(Type)]
struct Line {
    start: Point,
    end: Point,
}

#[derive(Type)]
enum Shape {
    // Unit variant
    None,

    // Simple tuple variant
    Point(f64, f64),

    // Named fields variant
    Circle {
        center: Point,
        radius: f64,
    },

    // Nested struct variant
    Rectangle(Rectangle),

    // Complex nested variant
    Line {
        start: Point,
        end: Point,
    },

    // Mixed variant with multiple nested types
    Complex {
        shapes: Vec<Shape>,
        metadata: Option<String>,
    },
}

#[derive(Type)]
enum ApiResponse<T> {
    // Success with data
    Success {
        data: T,
        status: u16,
        headers: Vec<(String, String)>,
    },

    // Error with details
    Error {
        code: u32,
        message: String,
        details: Option<T>,
    },

    // Loading state
    Loading {
        progress: f32,
        estimated_time: Option<u64>,
    },

    // Redirect
    Redirect {
        url: String,
        permanent: bool,
    },
}

#[derive(Type)]
enum DatabaseResult<T, E> {
    // Success with data and metadata
    Ok {
        data: T,
        affected_rows: u64,
        execution_time: f64,
    },

    // Error with error type and context
    Err {
        error: E,
        query: String,
        retry_count: u32,
    },

    // Connection issues
    ConnectionError {
        host: String,
        port: u16,
        reason: String,
    },

    // Timeout
    Timeout {
        duration: f64,
        operation: String,
    },
}

#[test]
fn test_complex_unions() {
    let types = TypeCollection::default()
        .register::<Point>()
        .register::<Circle>()
        .register::<Rectangle>()
        .register::<Line>()
        .register::<Shape>()
        .register::<ApiResponse<String>>()
        .register::<DatabaseResult<String, String>>();

    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Complex unions Swift code:\n{}", output);

    // Test that all types are generated
    assert!(output.contains("struct Point"));
    assert!(output.contains("struct Circle"));
    assert!(output.contains("struct Rectangle"));
    assert!(output.contains("struct Line"));
    assert!(output.contains("enum Shape"));
    assert!(output.contains("enum ApiResponse"));
    assert!(output.contains("enum DatabaseResult"));

    // Test Shape enum variants
    assert!(output.contains("case none"));
    assert!(output.contains("case point(Double, Double)"));
    assert!(output.contains("case circle"));
    assert!(output.contains("case rectangle(Rectangle)"));
    assert!(output.contains("case line"));
    assert!(output.contains("case complex"));

    // Test ApiResponse enum variants
    assert!(output.contains("case success"));
    assert!(output.contains("case error"));
    assert!(output.contains("case loading"));
    assert!(output.contains("case redirect"));

    // Test DatabaseResult enum variants
    assert!(output.contains("case ok"));
    assert!(output.contains("case err"));
    assert!(output.contains("case connectionError"));
    assert!(output.contains("case timeout"));

    // Test that nested structs are properly referenced
    assert!(output.contains("center: Point"));
    assert!(output.contains("radius: Double"));
    assert!(output.contains("topLeft: Point"));
    assert!(output.contains("bottomRight: Point"));
    assert!(output.contains("start: Point"));
    assert!(output.contains("end: Point"));
}

#[test]
fn test_union_with_generics() {
    let types = TypeCollection::default()
        .register::<ApiResponse<String>>()
        .register::<DatabaseResult<i32, String>>();

    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Generic unions Swift code:\n{}", output);

    // Test generic enum definitions
    assert!(output.contains("enum ApiResponse<T>"));
    assert!(output.contains("enum DatabaseResult<T, E>"));

    // Test that Codable is added via extension
    assert!(output.contains("extension ApiResponse: Codable"));
    assert!(output.contains("extension DatabaseResult: Codable"));

    // Test that generic types are used in variants
    assert!(output.contains("data: T"));
    assert!(output.contains("error: E"));
    assert!(output.contains("details: T?"));
}

#[test]
fn test_union_naming_conventions() {
    let types = TypeCollection::default().register::<Shape>();

    // Test with different naming conventions
    let swift_pascal = Swift::new().naming(specta_swift::NamingConvention::PascalCase);
    let swift_snake = Swift::new().naming(specta_swift::NamingConvention::SnakeCase);

    let output_pascal = swift_pascal.export(&types).unwrap();
    let output_snake = swift_snake.export(&types).unwrap();

    println!("PascalCase output:\n{}", output_pascal);
    println!("SnakeCase output:\n{}", output_snake);

    // PascalCase should have camelCase enum cases
    assert!(output_pascal.contains("case none"));
    assert!(output_pascal.contains("case point"));
    assert!(output_pascal.contains("case circle"));

    // SnakeCase should have snake_case enum cases
    assert!(output_snake.contains("case none"));
    assert!(output_snake.contains("case point"));
    assert!(output_snake.contains("case circle"));
}
