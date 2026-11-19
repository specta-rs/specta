//! Exports every type reachable through `specta::collect()` to TypeScript.

#![allow(unused)]

use specta::Type;
use specta_typescript::Typescript;

/// A user of the example application.
#[derive(Type)]
pub struct User {
    id: String,
    name: String,
}

/// A post written by a [`User`].
#[derive(Type)]
pub struct Post {
    id: String,
    author: User,
    comments: Vec<Comment>,
}

/// A comment left on a [`Post`].
#[derive(Type)]
pub struct Comment {
    body: String,
}

/// A type which can still derive `Type`, but is left out of `specta::collect()`.
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
