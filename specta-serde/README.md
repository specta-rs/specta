# Specta Serde

Serde format integration for [Specta](https://github.com/specta-rs/specta).

`specta-serde` provides formatter callbacks you pass to exporters.

## Usage

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

let output_phases = Typescript::default()
    .export(&types, specta_serde::format_phases)
    .unwrap();
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
