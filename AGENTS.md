Specta is a Rust library for easily exporting Rust types to other languages (TypeScript, Swift, Rust, OpenAPI, etc.). It's a workspace with multiple crates:

```
specta/
├── specta/              # Core library
├── specta-macros/       # Macros for the core library
├── specta-typescript/   # TypeScript exporter (stable)
├── specta-swift/        # Swift exporter  (stable)
├── specta-openapi/      # OpenAPI (partial)
├── specta-serde/        # Serde utilities
├── specta-util/         # Utilities for end-users. Less semver guarantees.
├── tests/               # Integration tests
├── Cargo.toml           # Workspace manifest
```

Specta is format agnostic so the `specta` and `specta-macros` crate should avoid hardcoding serde-specific behaviors.
We use `insta` for snapshot testing
Use Rust 2024 edition
Document feature gates with `#[cfg_attr(docsrs, doc(cfg(feature = "...")))]`
Prefer to put tests in the dedicated crate
Don't run `cargo doc --open` as it opens a browser you can't read. Maybe prefer a web fetch to https://docs.rs/{crate_name}
Prefer Rust module guidelines including using `module.rs` instead of `module/mod.rs`
When testing bugs create a unit test to ensure it's fixed. Ensure this test has a link back to the original GitHub issue. Don't use `/tmp` for temporary projects.

You are a senior engineer. You should follow Clippy and Rust best practices. Write code that is concise and readable. Make use of Rust's method chaining where it would result in cleaner code.
