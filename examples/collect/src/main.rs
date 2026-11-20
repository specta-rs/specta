//! Exports every type reachable through `specta::collect()` to TypeScript.

#![allow(unused)]
// The derive must not declare a generic parameter's bounds in both the list and
// a `where` clause; clippy reports that in the consuming crate, where it looks
// like the user's own struct is at fault. `DisplayValue` below is the guard.
#![deny(clippy::multiple_bound_locations)]

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

/// A bound that the `()` placeholder used by automatic collection could never
/// satisfy. Generic declarations are collected only when the application
/// registers a concrete instantiation.
#[derive(Type)]
pub struct DisplayValue<T: std::fmt::Display> {
    value: T,
}

/// Const-generic declarations are likewise registered at a chosen
/// instantiation rather than guessed during collection.
#[derive(Type)]
pub struct ConstValue<const N: usize> {
    value: [u8; N],
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
