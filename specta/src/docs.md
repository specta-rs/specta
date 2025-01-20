Easily export your Rust types to other languages.

Specta provides a system for type introspection and a set of language exporters which allow you to export your Rust types to other languages!

**Get started** by checking out the language exporter's, start with [Typescript](../specta_typescript/index.html).

## Features

- Export structs and enums to [Typescript](https://www.typescriptlang.org)
- Get function types to use in libraries like [tauri-specta](https://github.com/oscartbeaumont/tauri-specta)
- Supports wide range of common crates in Rust ecosystem
- Supports type inference - can determine type of `fn demo() -> impl Type`.

## Ecosystem

Specta can be used in your application either directly or through a library which simplifies the process of using it.

- [rspc](https://github.com/specta-rs/rspc) - A framework for building typesafe web backends in Rust
- [tauri-specta](https://github.com/specta-rs/tauri-specta) - Completely typesafe Tauri commands
- [TauRPC](https://github.com/MatsDK/TauRPC) - Typesafe IPC layer for Tauri applications

## Languages

Specta is designed to be able to export from Rust to any other language.

| Language                                               | Status        |
| ------------------------------------------------------ | ------------- |
| [specta-typescript](https://docs.rs/specta-typescript) | **Supported** |
| [specta-jsdoc](https://docs.rs/specta-jsdoc)           | wip           |
| [specta-rust](https://docs.rs/specta-rust)             | wip           |
| [specta-go](https://docs.rs/specta-go)                 | _coming soon_ |
| [specta-kotlin](https://docs.rs/specta-kotlin)         | _coming soon_ |
| [specta-swift](https://docs.rs/specta-swift)           | _coming soon_ |
| [specta-openapi](https://docs.rs/specta-openapi)       | wip           |
| [specta-jsonschema](https://docs.rs/specta-jsonschema) | wip           |
| [specta-zod](https://docs.rs/specta-zod)               | wip           |
| [specta-valibot](https://docs.rs/specta-valibot)       | _coming soon_ |

## Feature flags

[//]: # (FEATURE_FLAGS_START)

- `function` - Support for exporting the types of Rust functions.
- `export` - Support for collecting up a global type map

Languages

- `typescript` - Support for [TypeScript](https://www.typescriptlang.org) language exporting
- `js_doc` - Support for [JSDoc](https://jsdoc.app) exporting helpers. Also requires `typescript` feature to be enabled.

Compatibility

- `serde` - Support for [serde](https://serde.rs)
- `serde_json` - Support for [serde-json](https://github.com/serde-rs/json)
- `serde_yaml` - Support for [serde_yaml](https://github.com/dtolnay/serde-yaml)
- `toml` - Support for [toml](https://github.com/toml-rs/toml)
- `tauri` - Support for [Tauri](https://tauri.app). This is required when using [`specta::function`](macro@crate::specta) with Tauri Commands.

External types

- `ulid` - [ulid](https://docs.rs/ulid) crate
- `uuid` - [uuid](https://docs.rs/uuid) crate
- `chrono` - [chrono](https://docs.rs/chrono) crate
- `time` - [time](https://docs.rs/time) crate
- `jiff` - [jiff](https://docs.rs/jiff) crate
- `bigdecimal` - [bigdecimal](https://docs.rs/bigdecimal) crate
- `rust_decimal` - [rust_decimal](https://docs.rs/rust_decimal) crate
- `indexmap` - [indexmap](https://docs.rs/indexmap) crate
- `ipnetwork` - [ipnetwork](https://docs.rs/ipnetwork) crate
- `mac_address` - [mac_address](https://docs.rs/mac_address) crate
- `bit-vec` - [bit-vec](https://docs.rs/bit-vec) crate
- `bson` - [bson](https://docs.rs/bson) crate
- `uhlc` - [uhlc](https://docs.rs/uhlc) crate
- `bytesize` - [bytesize](https://docs.rs/bytesize) crate
- `glam` - [glam](https://docs.rs/glam) crate
- `tokio` - [tokio](https://docs.rs/tokio) crate
- `url` - [url](https://docs.rs/url) crate
- `either` - [either](https://docs.rs/either) crate
- `bevy_ecs` - [bevy_ecs](https://docs.rs/bevy_ecs) crate

[//]: # (FEATURE_FLAGS_END)
## Alternatives

#### Why not ts-rs?

[ts-rs](https://github.com/Aleph-Alpha/ts-rs) is a great library,
but it has a few limitations which became a problem when I was building [rspc](https://github.com/oscartbeaumont/rspc).
Namely it deals with types individually which means it is not possible to export a type and all of the other types it depends on.

#### Why not Typeshare?

[Typeshare](https://github.com/1Password/typeshare) is also great, but its approach is fundamentally different.
While Specta uses traits and runtime information, Typeshare statically analyzes your Rust
files.
This results in a loss of information and lack of compatibility with types from other crates.
