use specta::{Type, Types};
use std::collections::HashMap;

/// Comprehensive example showcasing basic Rust types and their ReScript equivalents.
///
/// This example demonstrates how specta-rescript handles fundamental Rust types
/// and converts them to appropriate ReScript types.

/// All supported primitive types
#[derive(Type)]
struct Primitives {
    // Integer types — all map to ReScript `int`
    small_int: i8,
    unsigned_small: u8,
    short_int: i16,
    unsigned_short: u16,
    regular_int: i32,
    unsigned_int: u32,
    long_int: i64,
    unsigned_long: u64,
    signed_size: isize,
    unsigned_size: usize,

    // Float types — map to ReScript `float`
    single_precision: f32,
    double_precision: f64,

    // Boolean maps to `bool`
    is_active: bool,

    // char and String both map to `string`
    single_char: char,
    name: String,
}

/// Optional fields — map to `option<t>`
#[derive(Type)]
struct WithOptionals {
    required: String,
    optional_string: Option<String>,
    optional_int: Option<i32>,
    optional_list: Option<Vec<String>>,
    nested_optional: Option<Option<String>>,
}

/// Collections
#[derive(Type)]
struct Collections {
    // Vec<T> -> `array<t>`
    tags: Vec<String>,
    scores: Vec<f64>,
    ids: Vec<u32>,

    // Nested arrays
    matrix: Vec<Vec<f64>>,
    tags_per_user: Vec<Vec<String>>,

    // Fixed-length arrays (still map to `array<t>`)
    rgb: [u8; 3],
}

/// Maps — only string-keyed maps supported; map to `dict<v>`
#[derive(Type)]
struct WithMaps {
    headers: HashMap<String, String>,
    settings: HashMap<String, i32>,
    nested: HashMap<String, Vec<String>>,
}

/// Tuple types
#[derive(Type)]
struct WithTuples {
    pair: (String, i32),
    triple: (String, i32, bool),
    nested: (String, (i32, f64)),
}

/// Nested structs
#[derive(Type)]
struct Address {
    street: String,
    city: String,
    country: String,
    postal_code: Option<String>,
}

#[derive(Type)]
struct UserProfile {
    display_name: String,
    bio: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Type)]
struct User {
    id: u32,
    email: String,
    profile: UserProfile,
    address: Option<Address>,
    tags: Vec<String>,
}

pub fn types() -> Types {
    Types::default()
        .register::<Primitives>()
        .register::<WithOptionals>()
        .register::<Collections>()
        .register::<WithMaps>()
        .register::<WithTuples>()
        .register::<Address>()
        .register::<UserProfile>()
        .register::<User>()
}

#[cfg(not(test))]
fn main() {
    use specta_rescript::ReScript;

    println!("Basic Types Example - Generating ReScript from Rust types");
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
                "/examples/generated/BasicTypes.res"
            ),
            &types,
        )
        .unwrap();
    println!("Basic types exported to BasicTypes.res");

    println!("\nKey Features Demonstrated:");
    println!("• All integer types -> int");
    println!("• f32/f64 -> float");
    println!("• bool -> bool");
    println!("• char/String -> string");
    println!("• Option<T> -> option<t>");
    println!("• Vec<T> -> array<t>");
    println!("• HashMap<String, V> -> dict<v>");
    println!("• Tuples -> (t1, t2, ...)");
    println!("• Nested struct references");
}
