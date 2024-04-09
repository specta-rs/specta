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

 - Export structs and enums to [Typescript](https://www.typescriptlang.org)
 - Get function types to use in libraries like [tauri-specta](https://github.com/oscartbeaumont/tauri-specta)
 - Supports wide range of common crates in Rust ecosystem
 - Supports type inference - can determine type of `fn demo() -> impl Type`.

## Ecosystem

Specta can be used in your application either directly or through a library which simplifies the process of using it.

 - [rspc](https://github.com/oscartbeaumont/rspc) - Easily building end-to-end typesafe APIs
 - [tauri-specta](https://github.com/oscartbeaumont/tauri-specta) - Typesafe Tauri commands and events
 - [TauRPC](https://github.com/MatsDK/TauRPC) - Tauri extension to give you a fully-typed IPC layer.

## Usage

Add `specta` as a dependency to your project,
enabling the languages you want to export to:

```bash
cargo add specta --features typescript # only 'typescript' is currently supported
```

Then you can use Specta like so:

```rust
use specta::{ts, Type};

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
    assert_eq!(
        ts::export::<TypeOne>(&Default::default()).unwrap(),
        "export type TypeOne = { a: string; b: GenericType<number>; cccccc: MyEnum }".to_string()
    );
}
```

Check out the [docs](https://docs.rs/specta) for more information.


## Motivation

This library was originally created to power the type exporting functionality of [rspc](https://rspc.dev),
but after building it we realized that it could be useful for other projects as well so we decided to move it into a dedicated library.

A huge thanks to [Brendonovich](https://github.com/brendonovich) for doing a heap of development on this library.
