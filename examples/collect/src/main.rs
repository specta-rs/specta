#![allow(unused)]

use specta::Type;
use specta_typescript::Typescript;

#[derive(Type)]
pub struct User {
    id: String,
    name: String,
}

#[derive(Type)]
pub struct Post {
    id: String,
    author: User,
    comments: Vec<Comment>,
}

#[derive(Type)]
pub struct Comment {
    body: String,
}

// This type can still derive `Type`, but it is left out of `specta::collect()`.
#[derive(Type)]
#[specta(collect = false)]
pub struct InternalMetrics {
    latency_ms: u64,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = Typescript::default().export(&specta::collect(), specta_serde::Format)?;

    println!("{output}");

    Ok(())
}
