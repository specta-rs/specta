Easily export your Rust types to other languages.

Specta provides a system for type introspection and a set of language exporters which allow you to export your Rust types to other languages!

**Get started** by checking out the language exporter's, start with [Typescript](https://docs.rs/specta-typescript).

## Features

- Export structs and enums to [Typescript](https://www.typescriptlang.org)
- Get function types to use in libraries like [tauri-specta](https://github.com/specta-rs/tauri-specta)
- Supports wide range of common crates in Rust ecosystem
- Supports type inference - can determine type of `fn demo() -> impl Type`.
 
## Ecosystem

Specta can be used in your application either directly or through a library which simplifies the process of using it.

- [rspc](https://github.com/specta-rs/rspc) - A framework for building typesafe web backends in Rust
- [tauri-specta](https://github.com/specta-rs/tauri-specta) - Completely typesafe Tauri commands
- [TauRPC](https://github.com/MatsDK/TauRPC) - Typesafe IPC layer for Tauri applications
- [orpc-rs](https://github.com/ahonn/orpc-rs) - Rust implementation of [oRPC](https://orpc.dev) — type-safe RPC with first-class Tauri support.

## Languages

Specta is designed to be able to export from Rust to any other language.

| Language                                               | Status        |
| ------------------------------------------------------ | ------------- |
| [specta-typescript](https://docs.rs/specta-typescript) | **stable**    |
| [specta-go](https://docs.rs/specta-go)                 | alpha         |
| [specta-swift](https://docs.rs/specta-swift)           | alpha         |
| [specta-openapi](https://docs.rs/specta-openapi)       | wip           |
| [specta-jsonschema](https://docs.rs/specta-jsonschema) | wip           |
| [specta-java](https://docs.rs/specta-java)             | alpha         |
| [specta-zod](https://docs.rs/specta-zod)               | wip           |
| [specta-valibot](https://docs.rs/specta-valibot)       | wip           |
| [specta-kotlin](https://docs.rs/specta-kotlin)         | alpha         |
| specta-jsdoc                                           | _coming soon_ |
| specta-rust                                            | _coming soon_ |

## Formats

Specta is format agnostic. Format-specific behavior is handled by companion
crates before handing transformed types to language exporters.

| Format | Status        |
| ------ | ------------- |
| [specta-serde](https://docs.rs/specta-serde) |  **stable**    |

## Wire format

Specta models the [Serde](https://serde.rs) *data model* with a single static type
graph, so every built-in `Type` impl describes the shape a JSON-style serializer
sees. This is independent of any concrete output format:

- `CString`/`CStr` serialize as a byte list (`serialize_bytes`), not a string.
- `OsString`/`OsStr` serialize as an externally tagged enum: the `Unix` newtype
  variant carries the `as_bytes()` sequence on Unix, and the `Windows` newtype
  variant carries UTF-16 code units on Windows. Unlike the `CString` case, the
  Unix payload is an ordinary byte sequence, not `serialize_bytes`. Serde provides
  no encoding on other targets, so Specta keeps the historical `str` mapping there
  to stay source-compatible.
- `serde_yaml::value::TaggedValue` serializes as a single-entry map keyed by the
  tag string, e.g. `{"!Thing": true}`. A dynamic one-entry key cannot be expressed
  statically, so Specta maps it to a string-keyed map of `serde_yaml::Value`.
- `toml::value::Datetime` serializes as the
  `{"$__toml_private_datetime": "2026-01-01"}` struct emitted by `toml`.
- `bit_vec::BitVec<B>` serializes via its derived `Serialize` layout, i.e. the
  `{ storage: [B], nbits: usize }` struct, not a sequence of booleans.

Native YAML (`!Thing true`) and native TOML (`2026-01-01`) encode tags and
datetimes natively instead. Specta has no target-format-aware lowering, so the
JSON/serde-data-model shape above is what gets exported and the native encodings
are **not** emitted; an exporter that needs a native encoding must special-case
these types.

## Feature flags

[//]: # (FEATURE_FLAGS_START)

- `function` - Support for exporting the types of Rust functions.
- `collect` - Support for collecting up a global type map

Languages

- `typescript` - Support for [TypeScript](https://www.typescriptlang.org) language exporting
- `js_doc` - Support for [JSDoc](https://jsdoc.app) exporting helpers. Also requires `typescript` feature to be enabled.

Compatibility

- `serde` - Support for [serde](https://serde.rs)
- `serde_json` - Support for [serde-json](https://github.com/serde-rs/json)
- `serde_yaml` - Support for [serde_yaml](https://github.com/dtolnay/serde-yaml)
- `toml` - Support for [toml](https://github.com/toml-rs/toml)
- `tauri` - Support for [Tauri](https://tauri.app). This is required when using the `#[specta]` attribute with Tauri Commands.

External types

- `ulid` - [ulid](https://docs.rs/ulid) crate
- `uuid` - [uuid](https://docs.rs/uuid) crate
- `chrono` - [chrono](https://docs.rs/chrono) crate
- `time` - [time](https://docs.rs/time) crate
- `jiff` - [jiff](https://docs.rs/jiff) crate
- `bigdecimal` - [bigdecimal](https://docs.rs/bigdecimal) crate
- `rust_decimal` - [rust_decimal](https://docs.rs/rust_decimal) crate
- `indexmap` - [indexmap](https://docs.rs/indexmap) crate
- `ordered-float` - [ordered-float](https://docs.rs/ordered-float) crate
- `heapless` - [heapless](https://docs.rs/heapless) crate
- `semver` - [semver](https://docs.rs/semver) crate
- `smol_str` - [smol_str](https://docs.rs/smol_str) crate
- `arrayvec` - [arrayvec](https://docs.rs/arrayvec) crate
- `smallvec` - [smallvec](https://docs.rs/smallvec) crate
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
but it has a few limitations which became a problem when I was building [rspc](https://github.com/specta-rs/rspc).
Namely it deals with types individually which means it is not possible to export a type and all of the other types it depends on.

#### Why not Typeshare?

[Typeshare](https://github.com/1Password/typeshare) is also great, but its approach is fundamentally different.
While Specta uses traits and runtime information, Typeshare statically analyzes your Rust
files.
This results in a loss of information and lack of compatibility with types from other crates.
