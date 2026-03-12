use specta::{Type, TypeCollection};

/// Example showcasing ReScript's enum/variant representation.
///
/// ReScript has two distinct enum representations:
/// - **Polymorphic variants** `[ #A | #B ]` — for all-unit enums (no data)
/// - **Regular variants** `| A | B(t)` — for enums with data-carrying variants
///
/// specta-rescript automatically picks the right representation.

// ── Polymorphic variants (all unit) ─────────────────────────────────────────

/// All unit variants -> `[ #Active | #Inactive | #Suspended ]`
/// Perfect for string-like status types.
#[derive(Type)]
enum Status {
    Active,
    Inactive,
    Suspended,
}

/// HTTP methods
#[derive(Type)]
enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
}

/// Log levels
#[derive(Type)]
enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

// ── Regular variants (data-carrying) ────────────────────────────────────────

/// Single unnamed field variants
#[derive(Type)]
enum Notification {
    Email(String),
    Sms(String),
    Push(String),
}

/// Multi-field tuple variants
#[derive(Type)]
enum Coordinate {
    TwoD(f64, f64),
    ThreeD(f64, f64, f64),
}

/// Named-field variants generate auxiliary record types
#[derive(Type)]
enum Shape {
    /// Unit variant — no data
    None,
    /// Tuple variant — inline types
    Circle(f64),
    /// Tuple variant — multiple fields become a tuple payload
    Rect(f64, f64),
    /// Named fields — generates `shapeLineFields` record type
    Line { x1: f64, y1: f64, x2: f64, y2: f64 },
}

/// Mixed enum: unit + data variants
#[derive(Type)]
enum ApiError {
    /// Simple variant — no payload
    NotFound,
    Unauthorized,
    /// Data variants with context
    BadRequest(String),
    InternalError {
        message: String,
        code: u32,
    },
    RateLimit {
        retry_after_secs: u32,
    },
}

/// Referencing other types in variants
#[derive(Type)]
struct Point {
    x: f64,
    y: f64,
}

#[derive(Type)]
enum DrawCommand {
    MoveTo(Point),
    LineTo(Point),
    CurveTo {
        control1: Point,
        control2: Point,
        end: Point,
    },
    Close,
}

pub fn types() -> TypeCollection {
    TypeCollection::default()
        .register::<Status>()
        .register::<HttpMethod>()
        .register::<LogLevel>()
        .register::<Notification>()
        .register::<Coordinate>()
        .register::<Shape>()
        .register::<ApiError>()
        .register::<Point>()
        .register::<DrawCommand>()
}

#[cfg(not(test))]
fn main() {
    use specta_rescript::ReScript;

    println!("Variants Example - Enum patterns in ReScript");
    println!("{}", "=".repeat(60));

    let types = types();

    let rescript = ReScript::default().without_serde();
    let output = rescript.export(&types).unwrap();

    println!("Generated ReScript code:\n");
    println!("{}", output);

    rescript
        .export_to(
            concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/examples/generated/Variants.res"
            ),
            &types,
        )
        .unwrap();
    println!("Variants exported to Variants.res");

    println!("\nKey Features Demonstrated:");
    println!("• All-unit enums -> polymorphic variants [ #A | #B ]");
    println!("• Single-field variants -> VariantName(type)");
    println!("• Multi-field tuple variants -> VariantName(t1, t2)");
    println!("• Named-field variants -> auxiliary record types + VariantName(recordType)");
    println!("• Mixed enums -> regular variants throughout");
    println!("• References to other named types in variants");
}
