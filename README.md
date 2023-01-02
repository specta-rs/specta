<div align="center">
    <img height="150" src=".github/logo.png" alt="Specta Logo"></img>
    <h1>Specta</h1>
    <p><b>Specta allows you to easily export your Rust types to other languages.</b></p>
    <a href="https://discord.gg/4V9M5sksw8"><img src="https://img.shields.io/discord/1011665225809924136?style=flat-square" alt="Discord"></a>
    <a href="https://crates.io/crates/specta"><img src="https://img.shields.io/crates/d/specta?style=flat-square" alt="Crates.io"></a>
    <a href="https://crates.io/crates/rspc"><img src="https://img.shields.io/crates/v/rspc.svg?style=flat-square"
    alt="crates.io" /></a>
    <a href="https://docs.rs/specta"><img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square" alt="docs.rs" /></a>
    <a href="/LICENSE.md"><img src="https://img.shields.io/crates/l/specta?style=flat-square" alt="License"></a>
</div>

<br>

## Features

 - Export to [Typescript](https://www.typescriptlang.org)
 - Export functions
 - Export structs and enums
 - Supports wide range of common crates in Rust ecosystem
 - Supports type inference. Can determine type of `fn demo() -> impl Type`.

## Usage

Create a new project and run.

```bash
cargo add specta
```

Then you can use Specta like so:

```rust
use specta::{ts, Type};

#[derive(Type)]
pub struct MyCustomType<A> {
    pub my_field: String,
    pub generic: A,
}

fn main() {
    assert_eq!(
        ts::export::<MyCustomType<()>>(),
        Ok("export interface MyCustomType<A> { my_field: string, generic: A }".to_string())
    );
}
```

Check out the [docs](https://docs.rs/specta) for more information.


## Motivation

This library was originally created to power the type exporting functionality of [rspc](https://rspc.dev), but after building it we realized that it could be useful for other projects as well so we decided to move it into a dedicated library.

A huge thanks to [Brendonovich](https://github.com/brendonovich) for doing a heap of development on this library.
