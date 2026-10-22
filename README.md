<div align="center">
    <img height="150" src=".github/logo.png" alt="Specta Logo"></img>
    <h1>Specta</h1>
    <p><b>Easily export your Rust types to other languages</b></p>
    <a href="https://discord.com/invite/5M6fpszrry"><img src="https://img.shields.io/discord/1011665225809924136?style=flat-square" alt="Discord"></a>
    <a href="https://crates.io/crates/specta"><img src="https://img.shields.io/crates/d/specta?style=flat-square" alt="Crates.io"></a>
    <a href="https://crates.io/crates/specta"><img src="https://img.shields.io/crates/v/specta.svg?style=flat-square"
    alt="crates.io" /></a>
    <a href="https://docs.rs/specta"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs" /></a>
    <a href="/LICENSE.md"><img src="https://img.shields.io/crates/l/specta?style=flat-square" alt="License"></a>
</div>

<br>

## Features

- Export structs and enums to multiple languages
- Get function types to use in libraries like [tauri-specta](https://github.com/specta-rs/tauri-specta)
- Supports wide range of common crates in Rust ecosystem
- Supports type inference - can determine type of `fn demo() -> impl Type`

## Language Support

| Language        | Status              | Exporter                                                          | Features                                          |
| --------------- | ------------------- | ----------------------------------------------------------------- | ------------------------------------------------- |
| **TypeScript**  | ✅ **Stable**       | [`specta-typescript`](https://crates.io/crates/specta-typescript) | Full type support, generics, unions               |
| **Swift**       | ✅ **Stable**       | [`specta-swift`](https://crates.io/crates/specta-swift)           | Idiomatic Swift, custom Codable, Duration support |
| **Rust**        | 🧪 **Experimental** | [`specta-rust`](https://crates.io/crates/specta-rust)             | Rust source, modules, generics; structural wire-only shapes error |
| **OpenAPI**     | 🧪 **Experimental** | [`specta-openapi`](https://crates.io/crates/specta-openapi)       | OpenAPI 3.1 schemas by default (3.0 opt-in), generics, JSON and YAML |
| **Go**          | 🧪 **Experimental** | [`specta-go`](https://crates.io/crates/specta-go)                 | Structs, generics, string enums, layouts, Serde   |
| **Kotlin**      | 🧪 **Experimental** | [`specta-kotlin`](https://crates.io/crates/specta-kotlin)         | Data/sealed classes, generics, optional Kotlinx   |
| **JSON Schema** | 🧪 **Experimental** | [`specta-jsonschema`](https://crates.io/crates/specta-jsonschema) | Draft 7/2019-09/2020-12, generics, Serde          |
| **Zod**         | 🧪 **Experimental** | [`specta-zod`](https://crates.io/crates/specta-zod)               | Zod 4 schemas, inferred types, generics, layouts  |
| **Valibot**     | 🧪 **Experimental** | [`specta-valibot`](https://crates.io/crates/specta-valibot)       | Valibot schemas, inferred types, generics, layouts |
| **Python**      | 🧪 **Experimental** | [`specta-python`](./specta-python)                                | Python 3.13 TypedDicts and type hints             |
| **C#**          | 🧪 **Experimental** | `specta-csharp`                                                   | Records, enums, generics, namespaces, file layouts |
| **Java**        | 🧪 **Experimental** | `specta-java`                                                     | Java 17 records, enums, and sealed interfaces     |

### Legend

- ✅ **Stable**: Production-ready and widely used, with comprehensive test coverage.
- 🧪 **Experimental**: Pre-1.0 and still evolving. The API and generated output may change, and the range of Rust and Serde shapes each exporter supports differs substantially — check the Features column and the crate's documentation before relying on one.

## Implementation Status

The Specta exporter crates are pre-1.0 and actively developed, and their maturity varies widely. TypeScript and Swift are stable and production-ready. The remaining exporters are experimental and each supports a different subset of Rust and Serde shapes, so there is no single "completeness" ranking between them — see the Features column above and each crate's documentation and issue tracker for the shapes a given exporter supports.

## Ecosystem

Specta can be used in your application either directly or through a library which simplifies the process of using it.

- [rspc](https://github.com/specta-rs/rspc) - Easily building end-to-end typesafe APIs
- [tauri-specta](https://github.com/specta-rs/tauri-specta) - Typesafe Tauri commands and events
- [TauRPC](https://github.com/MatsDK/TauRPC) - Tauri extension to give you a fully-typed IPC layer.

## Usage

Add the [`specta`](https://docs.rs/specta) crate along with any Specta language exporter crate:

```bash
# Core Specta library
cargo add specta

# Language exporters (choose one or more)
cargo add specta_typescript  # TypeScript (stable)
cargo add specta_swift       # Swift (stable)
cargo add specta_rust        # Rust (experimental)
cargo add specta_openapi     # OpenAPI 3.1 schemas by default, 3.0 opt-in (JSON or YAML)
cargo add specta_java        # Java 17 (experimental)
cargo add specta_go           # Go (experimental)
cargo add specta-python      # Python 3.13 type hints (experimental)
cargo add specta_kotlin       # Kotlin (experimental)
cargo add specta_jsonschema   # JSON Schema (experimental)
cargo add specta_zod          # Zod schemas (experimental)
cargo add specta-valibot      # Valibot schemas (experimental)
cargo add specta-csharp       # C# (experimental)
```

Then you can use Specta like following:

### TypeScript Example

```rust
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Type)]
pub struct TypeOne {
    pub a: String,
    pub b: GenericType<i32>,
    #[serde(rename = "cccccc")]
    pub c: MyEnum,
}

#[derive(Type)]
pub struct GenericType<A> {
    pub my_field: String,
    pub generic: A,
}

#[derive(Type)]
pub enum MyEnum {
    A,
    B,
    C,
}

fn main() {
    let types = Types::default()
        // You don't need to specify `GenericType` or `MyEnum` because they are referenced by `TypeOne`
        .register::<TypeOne>();

    Typescript::default()
        .export_to("./bindings.ts", &types)
        .unwrap();

    // if you need more control over file saving
    assert_eq!(
        Typescript::default().export(&types).unwrap(),
        r#"// This file has been generated by Specta. DO NOT EDIT.

export type GenericType<A> = { my_field: string; generic: A };

export type MyEnum = "A" | "B" | "C";

export type TypeOne = { a: string; b: GenericType<number>; cccccc: MyEnum };

"#
    );
}

```

### Multi-Language Export Example

You can export the same types to multiple languages:

```rust
use specta::{Type, Types};
use specta_typescript::Typescript;
use specta_swift::Swift;

#[derive(Type)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: Option<String>,
}

fn main() {
    let types = Types::default()
        .register::<User>();

    // Export to TypeScript (stable)
    Typescript::default()
        .export_to("./types.ts", &types)
        .unwrap();

    // Export to Swift (stable)
    Swift::default()
        .export_to("./Types.swift", &types)
        .unwrap();

    // Note: Other exporters are in development
}
```

A common use case is to export all types for which `specta::Type` is derived into a single file:

```rust
//! NOTE: This example requires the `export` feature on the `specta` crate
use specta::Type;
use specta_typescript::Typescript;

#[derive(Type)]
pub enum MyEither<L, R> {
    Left(L),
    Right(R),
}

#[derive(Type)]
pub struct GenericType<A> {
    pub my_field: String,
    pub generic: A,
}

#[derive(Type)]
pub enum MyEnum {
    A,
    B,
    C,
}

#[derive(Type)]
#[specta(collect = false)]
pub struct DontExportMe {
    field: String,
}

fn main() {
    Typescript::default()
        .export_to("./bindings.ts", &specta::export())
        .unwrap();
}
```

Check out the [docs](https://docs.rs/specta) for more information.

## Motivation

This library was originally created to power the type exporting functionality of [rspc](https://rspc.dev),
but after building it we realized that it could be useful for other projects as well so we decided to move it into a dedicated library.

A huge thanks to [Brendonovich](https://github.com/brendonovich) for doing a heap of early development on this library.

## Releasing

Publishing to [crates.io](https://crates.io/crates/specta) uses
[crates.io trusted publishing](https://crates.io/docs/trusted-publishing): the
`release` workflow exchanges its GitHub OIDC identity for a short-lived crates.io
token, so **no long-lived `CARGO_REGISTRY_TOKEN` is stored** in this repository or
in Actions secrets.

**One-time setup** — a crate owner adds a trusted publisher in the crates.io UI
(the crate's **Settings → Trusted Publishing**) for every crate in the workspace.
These values must match `.github/workflows/release.yml` exactly, or the token
exchange is refused:

| Field | Value |
| --- | --- |
| Owner | `specta-rs` |
| Repository | `specta` |
| Workflow filename | `release.yml` |
| Environment | `release` |

The workflow publishes the whole workspace, so each of the following crates needs
its own trusted publisher: `specta`, `specta-macros`, `specta-serde`,
`specta-util`, `specta-typescript`, `specta-swift`, `specta-openapi`,
`specta-jsonschema`, `specta-kotlin`, `specta-csharp`, `specta-go`,
`specta-python`, `specta-java`, `specta-zod`, `specta-valibot`, `specta-rust` and
`specta-rescript`.

A trusted publisher can only be configured once a crate exists on crates.io, and
the initial publish requires an API token. `specta-valibot`, `specta-rust` and
`specta-rescript` have never been released, so publish each of them once from your
machine before adding its trusted publisher.

Adding required reviewers to the `release` GitHub environment is recommended: it
turns publishing into an approved action rather than something any tag push does.

**Cutting a release:**

1. Bump the `version` of every workspace crate (the tag names the `specta` crate
   version) and merge to `main`. The exporters pin `specta` to an exact version,
   so each one has to be republished whenever `specta` changes, and Cargo refuses
   to publish a version that already exists on crates.io.
2. Tag the merge commit and push the tag:

   ```bash
   git tag v2.0.0-rc.27
   git push origin v2.0.0-rc.27
   ```

The `release` workflow then verifies the release (the tag matches the `specta`
crate version, and the workspace builds and tests), publishes every crate to
crates.io, and creates the GitHub release.
