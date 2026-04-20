# Specta Serde

[Serde](https://serde.rs) support for [Specta](https://github.com/specta-rs/specta).

This allows for apply Serde macro attributes on your types to the Specta generated types!

## Using with Specta TypeScript

`specta-serde` exposes two format implementation for usage with any of the exporter crates (like  `specta-typescript`):

- `specta_serde::format`: unified shape for both serialize and deserialize.
- `specta_serde::format_phases`: split serialize/deserialize shapes.

## `format` (unified shape)

Use `format` when serde behavior is symmetric and only a single TypeScript type is produced.

Note: This will error with certain Serde attributes like `#[serde(rename(serialize = "a", deserialize = "b"))]` as it's unclear what is correct.

```rust
use specta::Types;
use specta_typescript::Typescript;

#[derive(specta::Type, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct User {
    user_id: u32,
}

let types = Types::default().register::<User>();

let output = Typescript::default()
    .export(&types, specta_serde::format)
    .unwrap();

assert!(output.contains("export type User"));
assert!(output.contains("userId: number"));
```

You should always prefer `format_phases` where possible as it will generate a more accurate type.

## `format_phases` (split by direction)

Use `format_phases` when the wire format could between serialization and deserialization. This may product two different types `TypeName_Serialize` and `TypeName_Deserialize` to accurately represent both phases. It will produce `TypeName` which is `TypeName_Serialize | TypeName_Deserialize` so the type can be used in a general format when needed.

This is common with directional serde metadata (`serialize_with`,
`deserialize_with`, `from`, `into`, `try_from`) or explicit
`#[specta(type = specta_serde::Phased<SerializeTy, DeserializeTy>)]` overrides.

```rust
use serde::{Deserialize, Serialize};
use serde_with::{OneOrMany, serde_as};
use specta::{Type, Types};
use specta_typescript::Typescript;

#[derive(Type, Serialize, Deserialize)]
#[serde(untagged)]
enum OneOrManyString {
    One(String),
    Many(Vec<String>),
}

#[serde_as]
#[derive(Type, Serialize, Deserialize)]
struct Filters {
    #[serde_as(as = "OneOrMany<_>")]
    #[specta(type = specta_serde::Phased<Vec<String>, OneOrManyString>)]
    tags: Vec<String>,
}

let types = Types::default().register::<Filters>();

let output = Typescript::default()
    .export(&types, specta_serde::format_phases)
    .unwrap();

assert!(output.contains("Filters_Serialize"));
assert!(output.contains("Filters_Deserialize"));
assert!(output.contains("OneOrManyString"));
```
