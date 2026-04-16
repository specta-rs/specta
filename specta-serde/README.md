# Specta Serde

Serde format integration for [Specta](https://github.com/specta-rs/specta).

`specta-serde` provides formatter callbacks you pass to exporters.

## Using with Specta TypeScript

`specta-serde` exposes two formatter tuples for `specta-typescript`:

- `specta_serde::format`: unified shape for both serialize and deserialize.
- `specta_serde::format_phases`: split serialize/deserialize shapes.

Note: if you are looking for `format_phased`, the API name is
`format_phases`.

## `format` (unified shape)

Use `format` when serde behavior is symmetric and one TypeScript type should be
used in both directions.

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

## `format_phases` (split by direction)

Use `format_phases` when the wire format differs between serialization and
deserialization.

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

## Modes

- `format`: unified serialize/deserialize shape.
- `format_phases`: split serialize/deserialize shape for asymmetric serde behavior.

## When to use `format_phases`

Use `format_phases` when serde metadata is directional, for example:

- `#[serde(other)]`
- `#[serde(variant_identifier)]` / `#[serde(field_identifier)]`
- phase-specific renames/conversions
- `#[specta(type = specta_serde::Phased<_, _>)]`
