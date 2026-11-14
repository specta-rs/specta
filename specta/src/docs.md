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

Features for third-party crates carry that crate's major version in their name (`uuid1`, `chrono04`, `bevy_ecs020`), so support for a new major of a crate can arrive as a new feature in a minor release of Specta rather than a major one. The unversioned names remain as aliases of the current versioned feature.

[//]: # (FEATURE_FLAGS_START)
Core Features

- `std` - Enable support for the Rust standard library. Enabled by default.
- `derive` - Derive macros
- `collect` - Support for collecting up a global type map

Features

- `function` - Support for exporting the types of Rust functions.

Compatibility

- `serde_json1` - Support for [serde-json](https://github.com/serde-rs/json) 1.x
- `serde_yaml09` - Support for [serde_yaml](https://github.com/dtolnay/serde-yaml) 0.9.x
- `toml1` - Support for [toml](https://github.com/toml-rs/toml) 1.x

External types

- `uuid1` - [uuid](https://docs.rs/uuid) 1.x
- `ulid3` - [ulid](https://docs.rs/ulid) 3.x
- `chrono04` - [chrono](https://docs.rs/chrono) 0.4.x
- `time03` - [time](https://docs.rs/time) 0.3.x
- `jiff02` - [jiff](https://docs.rs/jiff) 0.2.x
- `bigdecimal04` - [bigdecimal](https://docs.rs/bigdecimal) 0.4.x
- `rust_decimal1` - [rust_decimal](https://docs.rs/rust_decimal) 1.x
- `indexmap2` - [indexmap](https://docs.rs/indexmap) 2.x
- `ordered-float` - [ordered-float](https://docs.rs/ordered-float) 3.x through 5.x. The version range is the policy for this crate, so the feature name carries no major.
- `heapless` - [heapless](https://docs.rs/heapless) 0.7.x through 0.9.x. The version range is the policy for this crate, so the feature name carries no major.
- `semver1` - [semver](https://docs.rs/semver) 1.x
- `smol_str03` - [smol_str](https://docs.rs/smol_str) 0.3.x
- `arrayvec` - [arrayvec](https://docs.rs/arrayvec) 0.6.x and 0.7.x. The version range is the policy for this crate, so the feature name carries no major.
- `smallvec1` - [smallvec](https://docs.rs/smallvec) 1.x
- `ipnetwork021` - [ipnetwork](https://docs.rs/ipnetwork) 0.21.x
- `mac_address1` - [mac_address](https://docs.rs/mac_address) 1.x
- `bit-vec010` - [bit-vec](https://docs.rs/bit-vec) 0.10.x
- `bson3` - [bson](https://docs.rs/bson) 3.x
- `bytes1` - [bytes](https://docs.rs/bytes) 1.x
- `uhlc09` - [uhlc](https://docs.rs/uhlc) 0.9.x
- `bytesize2` - [bytesize](https://docs.rs/bytesize) 2.x
- `glam034` - [glam](https://docs.rs/glam) 0.34.x
- `tokio1` - [tokio](https://docs.rs/tokio) 1.x
- `url2` - [url](https://docs.rs/url) 2.x
- `either1` - [either](https://docs.rs/either) 1.x
- `error-stack08` - [error-stack](https://docs.rs/error-stack) 0.8.x
- `bevy_ecs020` - [bevy_ecs](https://docs.rs/bevy_ecs) 0.20.x
- `bevy_input020` - [bevy_input](https://docs.rs/bevy_input) 0.20.x
- `camino1` - [camino](https://docs.rs/camino) 1.x
- `geojson1` - [geojson](https://docs.rs/geojson) 1.x

Unversioned aliases. These are the pre-2.0 names; each enables the versioned feature it is named after. New manifests should use the versioned name.

- `serde_json` - Alias of `serde_json1`.
- `serde_yaml` - Alias of `serde_yaml09`.
- `toml` - Alias of `toml1`.
- `uuid` - Alias of `uuid1`.
- `ulid` - Alias of `ulid3`.
- `chrono` - Alias of `chrono04`.
- `time` - Alias of `time03`.
- `jiff` - Alias of `jiff02`.
- `bigdecimal` - Alias of `bigdecimal04`.
- `rust_decimal` - Alias of `rust_decimal1`.
- `indexmap` - Alias of `indexmap2`.
- `semver` - Alias of `semver1`.
- `smol_str` - Alias of `smol_str03`.
- `smallvec` - Alias of `smallvec1`.
- `ipnetwork` - Alias of `ipnetwork021`.
- `mac_address` - Alias of `mac_address1`.
- `bit-vec` - Alias of `bit-vec010`.
- `bson` - Alias of `bson3`.
- `bytes` - Alias of `bytes1`.
- `uhlc` - Alias of `uhlc09`.
- `bytesize` - Alias of `bytesize2`.
- `glam` - Alias of `glam034`.
- `tokio` - Alias of `tokio1`.
- `url` - Alias of `url2`.
- `either` - Alias of `either1`.
- `error-stack` - Alias of `error-stack08`.
- `bevy_ecs` - Alias of `bevy_ecs020`.
- `bevy_input` - Alias of `bevy_input020`.
- `camino` - Alias of `camino1`.
- `geojson` - Alias of `geojson1`.

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
