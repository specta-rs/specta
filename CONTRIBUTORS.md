# Contributing to Specta

Specta is a Rust workspace for exporting Rust type information to other
languages and schema formats. This guide is meant to help contributors find the
right place to make a change, run the same checks as CI, and update the tests
that protect generated output.

## Workspace Map

- `specta/` is the core library. It owns the format-agnostic type model,
  `Type`, `Types`, `DataType`, formatter hooks, and optional collection/function
  support.
- `specta-macros/` implements the derive and function macros re-exported by
  `specta`. Keep this crate focused on parsing Rust attributes and producing
  Specta type metadata.
- `specta-serde/` applies Serde semantics to Specta type graphs. Serde-specific
  behavior should usually live here, not in `specta` or `specta-macros`.
- `specta-typescript/` and `specta-swift/` are the stable exporters. They are
  good references for exporter structure, generated-output tests, and error
  reporting.
- `specta-openapi/`, `specta-jsonschema/`, `specta-zod/`, `specta-valibot/`, `specta-go/`, and
  `specta-kotlin/` are format crates at different stages of completeness.
- `specta-util/` contains end-user helpers with fewer semver guarantees than the
  core crates.
- `tests/` is the main integration test crate. Prefer adding regression tests
  there when behavior crosses crate boundaries or exercises exported output.
- `examples/` contains small usage examples and scratchpads for manual checks.

The workspace uses Rust 2024 edition throughout.

## Design Notes

Specta is intentionally format agnostic. The core type graph should describe
Rust type structure without assuming TypeScript, Swift, OpenAPI, Serde wire
shapes, or any other target format.

When making changes, prefer these boundaries:

- Put core type modeling, registration, and traversal behavior in `specta/`.
- Put derive parsing/code generation in `specta-macros/`.
- Put Serde attribute interpretation, validation, and phased serialize versus
  deserialize behavior in `specta-serde/`.
- Put language-specific rendering, reserved words, layout choices, imports, and
  exporter errors in the target exporter crate.
- Put broad behavior regressions in `tests/` unless a crate-local unit test is a
  clearer fit.

If a bug report comes from GitHub, include the issue or PR link in a nearby test
comment so future contributors can recover the context.

## Formatting and Lints

Format Rust code before opening a PR:

```sh
cargo fmt --all
```

The workspace opts into shared Rust and Clippy lints from the root
`Cargo.toml`, including warnings for missing docs, `unwrap`, `panic`, `todo`,
and `panic_in_result_fn`. CI runs Clippy with all features:

```sh
cargo clippy --all-features
```

Feature-gated public APIs should document the gate for docs.rs:

```rust
#[cfg(feature = "some-feature")]
#[cfg_attr(docsrs, doc(cfg(feature = "some-feature")))]
pub fn some_feature_api() {}
```

Use Rust module layout conventions used in this repository, including
`module.rs` instead of `module/mod.rs` for new modules.

## Running Tests

Run the full feature test suite before submitting substantial changes:

```sh
cargo test --all-features
```

CI also checks that the workspace builds without default features:

```sh
cargo build --all --no-default-features
```

For faster iteration, run the relevant crate or integration test first, then run
the full suite before marking the work ready.

## Snapshot Tests

Generated code and schema output is commonly tested with
[`insta`](https://insta.rs/). Most workspace snapshots live under
`tests/tests/snapshots/`, with additional crate-local snapshots where useful.

Run the test that produces the snapshot, then review pending snapshots:

```sh
cargo test -p specta-tests --test test --all-features
cargo insta review
```

Accept snapshots only after reading the generated diff carefully. Snapshot
changes are API and compatibility signals, not just test fixtures.

If you do not have `cargo-insta` installed, install it with:

```sh
cargo install cargo-insta
```

## Trybuild Tests

Macro diagnostics are tested with [`trybuild`](https://docs.rs/trybuild).
The main compile-fail entrypoint is `tests/tests/mod.rs`, which runs files in
`tests/tests/macro/` and compares their `.stderr` output.

Run the macro compile-fail tests with:

```sh
cargo test -p specta-tests --test test compile_errors --all-features
```

When an intentional diagnostic changes, rerun with `TRYBUILD=overwrite` and
review the updated `.stderr` files:

```sh
TRYBUILD=overwrite cargo test -p specta-tests --test test compile_errors --all-features
```

Only commit updated `.stderr` files when the diagnostic is intentionally better
or the accepted compiler output changed.

## Documentation

Public APIs should have concise docs because the workspace warns on missing
documentation. For docs.rs-specific checks, prefer building docs locally instead
of opening a browser:

```sh
RUSTDOCFLAGS="--cfg docsrs" cargo +nightly doc --all-features --no-deps
```

Do not use `cargo doc --open` in automated or agent-driven workflows.

## Pull Request Checklist

Before opening a PR, check the items that match the change:

- `cargo fmt --all`
- `cargo clippy --all-features`
- `cargo test --all-features`
- `cargo build --all --no-default-features`
- `cargo insta review` if snapshots changed
- `TRYBUILD=overwrite ...` only when intentionally updating compile-fail output
- New or updated tests for bug fixes and behavior changes
- Feature-gate docs for new feature-gated public APIs
