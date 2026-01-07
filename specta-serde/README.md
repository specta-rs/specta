# specta-serde

A comprehensive serde attribute handling system for [Specta](https://github.com/specta-rs/specta). This crate provides functionality to parse and apply serde attributes like `#[serde(rename = "...")]`, `#[serde(rename_all = "...")]`, and enum representation attributes to `DataType` instances, with separate handling for serialization and deserialization phases.

## Features

- **Comprehensive Attribute Support**: Handles `rename`, `rename_all`, `skip`, `flatten`, `default`, `transparent`, and enum representation attributes
- **Separate Processing Modes**: Distinct handling for serialization and deserialization transformations
- **Enum Representations**: Full support for external, internal, adjacent, untagged, and string enum representations
- **Type-Safe Transformations**: Apply serde semantics while maintaining type safety
- **Integration Ready**: Works seamlessly with existing Specta workflows and TypeScript exports

## Quick Start

Add `specta-serde` to your `Cargo.toml`:

```toml
[dependencies]
specta = { version = "2.0.0-rc.22", features = ["derive"] }
specta-serde = "0.0.9"
serde = { version = "1.0", features = ["derive"] }
```

## Basic Usage

### Processing Type Collections

```rust
use specta::{Type, TypeCollection};
use specta_serde::{process_for_serialization, process_for_deserialization};
use serde::{Serialize, Deserialize};

#[derive(Type, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UserProfile {
    pub user_id: u64,
    pub first_name: String,
    #[serde(rename = "emailAddress")]
    pub email: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let types = TypeCollection::default()
        .register::<UserProfile>();

    // Transform for serialization (Rust -> JSON)
    let ser_types = process_for_serialization(&types)?;
    
    // Transform for deserialization (JSON -> Rust) 
    let de_types = process_for_deserialization(&types)?;

    // Use with your preferred exporter
    Ok(())
}
```

### Individual Type Transformations

```rust
use specta_serde::{SerdeMode, apply_serde_transformations};

// Transform a specific DataType
let transformed = apply_serde_transformations(
    &datatype, 
    SerdeMode::Serialize
)?;
```

## Supported Serde Attributes

### Container Attributes

#### `#[serde(rename_all = "...")]`

Transforms field/variant names according to the specified case convention:

```rust
#[derive(Type, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiResponse {
    pub status_code: u16,     // becomes "statusCode"
    pub error_message: String, // becomes "errorMessage"
}
```

Supported cases:
- `lowercase`
- `UPPERCASE`  
- `PascalCase`
- `camelCase`
- `snake_case`
- `SCREAMING_SNAKE_CASE`
- `kebab-case`
- `SCREAMING-KEBAB-CASE`

#### `#[serde(transparent)]`

For wrapper types that should serialize as their inner type:

```rust
#[derive(Type, Serialize)]
#[serde(transparent)]
pub struct UserId(pub u64);
```

### Field Attributes

#### `#[serde(rename = "...")]`

Rename specific fields:

```rust
#[derive(Type, Serialize)]
pub struct User {
    #[serde(rename = "userId")]
    pub id: u64,
}
```

#### Skip Attributes

Control serialization/deserialization:

```rust
#[derive(Type, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: String,
    #[serde(skip_serializing)]      // Only skip when serializing
    pub secret: String,
    #[serde(skip_deserializing)]    // Only skip when deserializing  
    pub created_at: String,
    #[serde(skip)]                  // Skip in both directions
    pub internal: String,
}
```

#### `#[serde(flatten)]`

Flatten nested structures:

```rust
#[derive(Type, Serialize)]
pub struct User {
    pub name: String,
    #[serde(flatten)]
    pub address: Address,
}
```

#### `#[serde(default)]`

Use default values during deserialization:

```rust
#[derive(Type, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub debug: bool,  // defaults to false if not present
}
```

### Enum Representations

#### External (Default)

```rust
#[derive(Type, Serialize)]
pub enum Status {
    Active,
    Inactive { reason: String },
}
// Serializes as: "Active" or { "Inactive": { "reason": "..." } }
```

#### Internal

```rust
#[derive(Type, Serialize)]
#[serde(tag = "type")]
pub enum Message {
    Text { content: String },
    Image { url: String },
}
// Serializes as: { "type": "Text", "content": "..." }
```

#### Adjacent

```rust
#[derive(Type, Serialize)]
#[serde(tag = "type", content = "data")]
pub enum ApiResponse {
    Success(String),
    Error { code: u32, message: String },
}
// Serializes as: { "type": "Success", "data": "..." }
```

#### Untagged

```rust
#[derive(Type, Serialize)]
#[serde(untagged)]
pub enum Value {
    Integer(i32),
    String(String),
}
// Serializes as the inner value directly
```

#### String Enums

Unit-only enums with `rename_all` become string enums:

```rust
#[derive(Type, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Status {
    NotStarted,    // becomes "not-started"
    InProgress,    // becomes "in-progress" 
    Completed,     // becomes "completed"
}
```

## Advanced Usage

### Processing Modes

The `SerdeMode` enum controls which transformations are applied:

```rust
use specta_serde::SerdeMode;

// For serialization (Rust -> JSON)
let ser_transformed = apply_serde_transformations(
    &datatype, 
    SerdeMode::Serialize
)?;

// For deserialization (JSON -> Rust)
let de_transformed = apply_serde_transformations(
    &datatype, 
    SerdeMode::Deserialize  
)?;
```

This allows different behavior based on direction:
- `skip_serializing` only affects `SerdeMode::Serialize`
- `skip_deserializing` only affects `SerdeMode::Deserialize`
- `skip` affects both modes

### Integration with TypeScript

```rust
use specta_typescript::Typescript;
use specta_serde::process_for_serialization;

let types = TypeCollection::default()
    .register::<MyType>();

// Transform for serialization before export
let transformed_types = process_for_serialization(&types)?;

Typescript::default()
    .with_serde()
    .export_to("./bindings.ts", &transformed_types)?;
```

### Validation

The crate includes validation to ensure serde attributes are used correctly:

```rust
use specta_serde::validate;

let types = TypeCollection::default()
    .register::<MyType>();

// Validate serde usage
validate(&types)?;
```

## Error Handling

The crate defines specific error types for various serde-related issues:

```rust
use specta_serde::Error;

match apply_serde_transformations(&datatype, SerdeMode::Serialize) {
    Ok(transformed) => { /* use transformed type */ },
    Err(Error::InvalidMapKey) => { /* handle invalid map key type */ },
    Err(Error::InvalidUsageOfSkip) => { /* handle invalid skip usage */ },
    Err(Error::InvalidInternallyTaggedEnum) => { /* handle invalid internal enum */ },
    Err(err) => { /* handle other errors */ },
}
```

## Examples

See the [examples directory](./examples/) for comprehensive usage examples, including:

- Basic serde transformations
- Complex enum representations  
- Integration with TypeScript exports
- Error handling patterns

## Contributing

Contributions are welcome! Please see the main [Specta repository](https://github.com/specta-rs/specta) for contribution guidelines.

## License

This project is licensed under the MIT License - see the [LICENSE](../LICENSE) file for details.
