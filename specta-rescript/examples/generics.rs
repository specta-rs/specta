use specta::{Type, TypeCollection};

/// Example showcasing generic types in ReScript.
///
/// ReScript generic type parameters use apostrophe-prefixed lowercase names:
/// `type myType<'a, 'b> = ...`
/// Rust's `T`, `E`, `K`, `V` become `'t`, `'e`, `'k`, `'v`.

/// Single type parameter
#[derive(Type)]
struct Wrapper<T> {
    value: T,
}

/// Multiple type parameters
#[derive(Type)]
struct Pair<A, B> {
    first: A,
    second: B,
}

/// Generic with multiple type parameters — common API response pattern
#[derive(Type)]
struct ApiResponse<T, E> {
    data: Option<T>,
    error: Option<E>,
    status_code: u32,
    request_id: String,
}

/// Paginated response — common pattern in APIs
#[derive(Type)]
struct Page<T> {
    items: Vec<T>,
    total: u32,
    page: u32,
    page_size: u32,
    has_next: bool,
    has_prev: bool,
}

/// Generic enum (non-Result shape)
#[derive(Type)]
enum Container<T> {
    Empty,
    Single(T),
    Multiple(Vec<T>),
}

/// Generic struct referencing another generic struct
#[derive(Type)]
struct CachedValue<T> {
    inner: Wrapper<T>,
    cached_at: String,
    ttl_secs: Option<u32>,
}

/// Concrete types used with generics
#[derive(Type)]
struct Article {
    id: u32,
    title: String,
    content: String,
    author_id: u32,
}

#[derive(Type)]
struct ValidationError {
    field: String,
    message: String,
}

pub fn types() -> TypeCollection {
    // Register with concrete type arguments so all referenced types are included
    TypeCollection::default()
        .register::<Wrapper<String>>()
        .register::<Pair<String, i32>>()
        .register::<ApiResponse<Article, ValidationError>>()
        .register::<Page<Article>>()
        .register::<Container<String>>()
        .register::<CachedValue<Article>>()
        .register::<Article>()
        .register::<ValidationError>()
}

#[cfg(not(test))]
fn main() {
    use specta_rescript::ReScript;

    println!("Generics Example - Generic types in ReScript");
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
                "/examples/generated/Generics.res"
            ),
            &types,
        )
        .unwrap();
    println!("Generics exported to Generics.res");

    println!("\nKey Features Demonstrated:");
    println!("• Generic type parameters: Rust T -> ReScript 'a");
    println!("• Multiple type parameters: <A, B> -> <'a, 'b>");
    println!("• Generic structs with option<'a> and array<'a> fields");
    println!("• Generic enums with data variants");
    println!("• Nested generic types (CachedValue<T> wrapping Wrapper<T>)");
    println!("• Common API patterns (Page<T>, ApiResponse<T, E>)");
}
