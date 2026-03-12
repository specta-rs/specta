use specta::{Type, TypeCollection};

/// Example showcasing ReScript's built-in `result<ok, err>` type.
///
/// This is a key advantage of ReScript over TypeScript: `result<ok, err>` is
/// a first-class built-in type. specta-rescript detects any enum with exactly
/// two variants named `Ok(T)` and `Err(E)` and emits `result<t, e>`.
///
/// This is especially useful for Tauri apps where commands return `Result<T, E>`.

/// Custom Result-shaped enum — detected and emitted as `result<t, e>`
#[derive(Type)]
enum MyResult<T, E> {
    Ok(T),
    Err(E),
}

/// Tauri-style command payload
#[derive(Type)]
struct UserData {
    id: u32,
    name: String,
    email: String,
}

#[derive(Type)]
struct AppError {
    code: String,
    message: String,
}

/// A struct whose field uses our Result type
#[derive(Type)]
struct CommandResponse {
    request_id: String,
    result: MyResult<UserData, AppError>,
}

/// Result can appear in arrays (e.g. batch operations)
#[derive(Type)]
struct BatchResponse {
    results: Vec<MyResult<UserData, AppError>>,
    total: u32,
    failed_count: u32,
}

/// Richer error type
#[derive(Type)]
enum CommandError {
    NotFound(String),
    Unauthorized,
    Validation { field: String, message: String },
    Internal(String),
}

#[derive(Type)]
struct FetchResult<T> {
    data: MyResult<T, CommandError>,
    cached: bool,
}

/// Option type — maps to ReScript's `option<t>`
#[derive(Type)]
struct SearchResult {
    /// option<userData> — None means not found
    user: Option<UserData>,
    /// option<string> — None means no next page
    next_cursor: Option<String>,
    total_count: u32,
}

pub fn types() -> TypeCollection {
    TypeCollection::default()
        .register::<MyResult<String, String>>()
        .register::<UserData>()
        .register::<AppError>()
        .register::<CommandResponse>()
        .register::<BatchResponse>()
        .register::<CommandError>()
        .register::<FetchResult<UserData>>()
        .register::<SearchResult>()
}

#[cfg(not(test))]
fn main() {
    use specta_rescript::ReScript;

    println!("Result Types Example - result<ok, err> and option<t> in ReScript");
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
                "/examples/generated/ResultTypes.res"
            ),
            &types,
        )
        .unwrap();
    println!("Result types exported to ResultTypes.res");

    println!("\nKey Features Demonstrated:");
    println!("• Ok(T)/Err(E) enum pattern -> result<t, e> (ReScript built-in)");
    println!("• Option<T> -> option<t> (ReScript built-in)");
    println!("• result<t, e> in struct fields");
    println!("• array<myResult<t, e>> for batch responses");
    println!("• Generic result types with type parameters");
    println!("• Realistic Tauri IPC command response patterns");
    println!();
    println!("Note: ReScript result uses Ok/Error constructors.");
    println!("Rust serializes as {{\"Ok\": ...}}/{{\"Err\": ...}}.");
    println!("Use a decode adapter in your ReScript code as needed.");
}
