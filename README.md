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
- Get function types to use in libraries like [tauri-specta](https://github.com/oscartbeaumont/tauri-specta)
- Supports wide range of common crates in Rust ecosystem
- Supports type inference - can determine type of `fn demo() -> impl Type`

## Language Support

| Language        | Status         | Exporter                                                          | Features                                          |
| --------------- | -------------- | ----------------------------------------------------------------- | ------------------------------------------------- |
| **TypeScript**  | ✅ **Stable**  | [`specta-typescript`](https://crates.io/crates/specta-typescript) | Full type support, generics, unions               |
| **Swift**       | ✅ **Stable**  | [`specta-swift`](https://crates.io/crates/specta-swift)           | Idiomatic Swift, custom Codable, Duration support |
| **Rust**        | 🚧 **Partial** | [`specta-rust`](https://crates.io/crates/specta-rust)             | Basic types work, structs/enums in progress       |
| **OpenAPI**     | 🚧 **Partial** | [`specta-openapi`](https://crates.io/crates/specta-openapi)       | Primitives work, complex types in progress        |
| **Go**          | 🚧 **Planned** | [`specta-go`](https://crates.io/crates/specta-go)                 | Go structs and interfaces                         |
| **Kotlin**      | 🚧 **Planned** | [`specta-kotlin`](https://crates.io/crates/specta-kotlin)         | Kotlin data classes and sealed classes            |
| **JSON Schema** | 🚧 **Planned** | [`specta-jsonschema`](https://crates.io/crates/specta-jsonschema) | JSON Schema generation                            |
| **Zod**         | 🚧 **Planned** | [`specta-zod`](https://crates.io/crates/specta-zod)               | Zod schema validation                             |
| **Python**      | 🚧 **Planned** | `specta-python`                                                   | Python dataclasses and type hints                 |
| **C#**          | 🚧 **Planned** | `specta-csharp`                                                   | C# classes and enums                              |
| **Java**        | 🚧 **Planned** | `specta-java`                                                     | Java POJOs and enums                              |

### Legend

- ✅ **Stable**: Production-ready with comprehensive test coverage
- 🚧 **Partial**: Basic functionality implemented, complex types in progress
- 🚧 **Planned**: In development or planned for future release

## Implementation Status

The Specta ecosystem is actively developed with varying levels of completeness:

- **Production Ready (2)**: TypeScript and Swift exporters are fully functional with comprehensive test coverage
- **Partially Implemented (2)**: Rust and OpenAPI exporters have basic functionality working, with complex types in progress
- **Planned (7)**: Go, Kotlin, JSON Schema, Zod, Python, C#, and Java exporters are in development

For the most up-to-date status of each exporter, check the individual crate documentation and issue trackers.

## Ecosystem

Specta can be used in your application either directly or through a library which simplifies the process of using it.

- [rspc](https://github.com/oscartbeaumont/rspc) - Easily building end-to-end typesafe APIs
- [tauri-specta](https://github.com/oscartbeaumont/tauri-specta) - Typesafe Tauri commands and events
- [TauRPC](https://github.com/MatsDK/TauRPC) - Tauri extension to give you a fully-typed IPC layer.

## Usage

Add the [`specta`](https://docs.rs/specta) crate along with any Specta language exporter crate:

```bash
# Core Specta library
cargo add specta

# Language exporters (choose one or more)
cargo add specta_typescript  # TypeScript (stable)
cargo add specta_swift       # Swift (stable)
cargo add specta_rust        # Rust (partial - basic types)
cargo add specta_openapi     # OpenAPI/Swagger (partial - primitives)
# cargo add specta_go          # Go (planned)
# cargo add specta_kotlin      # Kotlin (planned)
# cargo add specta_jsonschema  # JSON Schema (planned)
# cargo add specta_zod         # Zod schemas (planned)
```

Then you can use Specta like following:

### TypeScript Example

```rust
use specta::{Type, TypeCollection};
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
    let types = TypeCollection::default()
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
use specta::{Type, TypeCollection};
use specta_typescript::Typescript;
use specta_swift::Swift;

#[derive(Type)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub email: Option<String>,
}

fn main() {
    let types = TypeCollection::default()
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
#[specta(export = false)]
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
