Easily export your Rust types to other languages

Specta provides a system for type introspection and a set of language exporters which allow you to export your Rust types to other languages!

**Currently we only support exporting to [TypeScript](https://www.typescriptlang.org) but work has begun on other languages.**

## Features
 - Export structs and enums to [Typescript](https://www.typescriptlang.org)
 - Get function types to use in libraries like [tauri-specta](https://github.com/oscartbeaumont/tauri-specta)
 - Supports wide range of common crates in Rust ecosystem
 - Supports type inference - can determine type of `fn demo() -> impl Type`.

## Ecosystem

Specta can be used in your application either directly or through a library which simplifies the process of using it.

- [rspc](https://github.com/oscartbeaumont/rspc) for easily building end-to-end typesafe APIs
- [tauri-specta](https://github.com/oscartbeaumont/tauri-specta) for typesafe Tauri commands

## Example
```rust
use specta::{*, ts::*};

#[derive(Type)]
pub struct MyCustomType {
   pub my_field: String,
}

fn main() {
    assert_eq!(
        ts::export::<MyCustomType>(&ExportConfig::default()).unwrap(),
        "export type MyCustomType = { my_field: string }".to_string()
    );
}
```

## Supported Libraries

If you are using [Prisma Client Rust](https://prisma.brendonovich.dev) you can enable the `rspc` feature on it to allow for Specta support on types coming directly from your database. This includes support for the types created via a selection.

## Feature flags
[//]: # (FEATURE_FLAGS_START)
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
This results in a loss of information and lack of compatability with types from other crates.
