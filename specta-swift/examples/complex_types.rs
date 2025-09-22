use specta::{Type, TypeCollection};
use specta_swift::Swift;

// Basic types
#[derive(Type)]
struct User {
    id: u32,
    name: String,
    email: Option<String>,
    is_active: bool,
    metadata: UserMetadata,
    tags: Vec<String>,
    scores: Vec<f64>,
}

#[derive(Type)]
struct UserMetadata {
    created_at: String,
    last_login: Option<String>,
    preferences: UserPreferences,
}

#[derive(Type)]
struct UserPreferences {
    theme: String,
    notifications: bool,
    language: String,
}

// Admin types
#[derive(Type)]
struct Admin {
    id: u32,
    name: String,
    permissions: Vec<String>,
    level: u8,
}

#[derive(Type)]
struct Guest {
    session_id: String,
    expires_at: String,
}

// Complex union types
#[derive(Type)]
enum UserType {
    // Unit variant
    Anonymous,

    // Tuple variant
    User(String, u32),

    // Named fields variant
    Admin {
        id: u32,
        name: String,
        permissions: Vec<String>,
    },

    // Nested struct variant
    Registered(User),

    // Complex nested struct variant
    SuperAdmin(Admin),

    // Mixed variant with nested struct
    Guest {
        info: Guest,
        created_at: String,
    },
}

// API response types
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

// Database result types
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

// Shape types for geometric operations
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

// Complex union with all variant types
#[derive(Type)]
enum ComplexUnion {
    // Simple unit
    None,

    // Tuple with multiple types
    Tuple(String, u32, bool),

    // Named fields
    NamedFields {
        id: u32,
        name: String,
        active: bool,
    },

    // Nested struct
    UserStruct(User),

    // Nested enum
    UserType(UserType),

    // Complex nested structure
    Complex {
        user: User,
        metadata: Vec<String>,
        settings: Option<Admin>,
    },
}

fn main() {
    let types = TypeCollection::default()
        .register::<User>()
        .register::<UserMetadata>()
        .register::<UserPreferences>()
        .register::<Admin>()
        .register::<Guest>()
        .register::<UserType>()
        .register::<ApiResponse<String>>()
        .register::<DatabaseResult<i32, String>>()
        .register::<Point>()
        .register::<Circle>()
        .register::<Rectangle>()
        .register::<Line>()
        .register::<Shape>()
        .register::<ComplexUnion>();

    let swift = Swift::default();
    let output = swift.export(&types).unwrap();

    println!("Generated Swift code:\n{}", output);

    // Also write to file
    swift.export_to("./Types.swift", &types).unwrap();
    println!("Types written to Types.swift");
}
