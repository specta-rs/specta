# Specta-Serde Implementation Summary

This document provides a comprehensive overview of the implemented serde attribute handling system for Specta.

## Overview

The `specta-serde` crate provides a comprehensive system for parsing and applying serde attributes like `#[serde(rename = "...")]`, `#[serde(rename_all = "...")]`, and enum representation attributes to `DataType` instances. The system supports separate handling for serialization and deserialization phases, ensuring that phase-specific attributes like `skip_serializing` and `skip_deserializing` are properly respected.

## Architecture

### Core Components

1. **Attribute Parsing** (`serde_attrs.rs`)
   - `SerdeAttributes`: Container for parsed serde attributes
   - `SerdeFieldAttributes`: Container for field-specific serde attributes
   - Parsing functions for extracting attributes from `RuntimeAttribute` vectors

2. **Transformation Engine** (`serde_attrs.rs`)
   - `SerdeTransformer`: Main transformation engine
   - `SerdeMode`: Enum for controlling serialization vs deserialization behavior
   - Mode-aware transformation logic

3. **Validation System** (`validate.rs`)
   - Extended validation with serde-aware checks
   - Functions for validating serde usage patterns

4. **Type System Integration** (`lib.rs`)
   - High-level API for processing `TypeCollection` instances
   - Builder integration for creating transformed types

## Key Features

### Supported Serde Attributes

#### Container Attributes
- `#[serde(rename_all = "...")]` - Transform all field/variant names
- `#[serde(transparent)]` - Transparent wrapper types
- `#[serde(tag = "...")]` - Internally tagged enums
- `#[serde(tag = "...", content = "...")]` - Adjacently tagged enums
- `#[serde(untagged)]` - Untagged enums
- `#[serde(default)]` - Default values during deserialization

#### Field Attributes
- `#[serde(rename = "...")]` - Rename specific fields
- `#[serde(skip)]` - Skip in both directions
- `#[serde(skip_serializing)]` - Skip only during serialization
- `#[serde(skip_deserializing)]` - Skip only during deserialization
- `#[serde(flatten)]` - Flatten nested structures
- `#[serde(serialize_with = "...")]` - Custom serialization function
- `#[serde(deserialize_with = "...")]` - Custom deserialization function

### Rename Rules

Full support for all serde rename rules:
- `lowercase`
- `UPPERCASE`
- `PascalCase`
- `camelCase`
- `snake_case`
- `SCREAMING_SNAKE_CASE`
- `kebab-case`
- `SCREAMING-KEBAB-CASE`

### Enum Representations

Complete support for all serde enum representations:
- **External** (default): `"Variant"` or `{ "Variant": data }`
- **Internal**: `{ "type": "Variant", ...fields }`
- **Adjacent**: `{ "type": "Variant", "content": data }`
- **Untagged**: Direct value serialization
- **String**: Unit-only enums with rename_all support

## API Reference

### Core Functions

```rust
// Apply transformations to a single DataType
pub fn apply_serde_transformations(
    datatype: &DataType,
    mode: SerdeMode,
) -> Result<DataType, Error>

// Process entire TypeCollection for serialization
pub fn process_for_serialization(types: &TypeCollection) -> Result<TypeCollection, Error>

// Process entire TypeCollection for deserialization
pub fn process_for_deserialization(types: &TypeCollection) -> Result<TypeCollection, Error>

// Process for both directions
pub fn process_for_both(types: &TypeCollection) -> Result<(TypeCollection, TypeCollection), Error>
```

### SerdeMode Enum

```rust
pub enum SerdeMode {
    Serialize,    // Apply transformations for Rust -> JSON/etc
    Deserialize,  // Apply transformations for JSON/etc -> Rust
}
```

## Implementation Details

### Attribute Parsing

The system parses serde attributes from `RuntimeAttribute` structures created by the Specta macro system. The parsing handles:

1. **Name-value pairs**: `#[serde(rename = "name")]`
2. **Boolean flags**: `#[serde(skip)]`
3. **List structures**: `#[serde(skip_serializing, rename = "name")]`

### Transformation Process

1. **Attribute Extraction**: Parse serde attributes from the DataType
2. **Mode-Aware Filtering**: Apply only relevant attributes for the current mode
3. **Recursive Transformation**: Handle nested types (Lists, Maps, etc.)
4. **Name Transformation**: Apply rename rules to fields and variants
5. **Structure Modification**: Handle special cases like transparent wrappers

### Type Safety

The system maintains full type safety throughout the transformation process:
- All transformations return valid `DataType` instances
- Error handling for invalid attribute combinations
- Validation of serde usage patterns

## Integration Examples

### Basic Usage

```rust
use specta::{Type, TypeCollection};
use specta_serde::{process_for_serialization, SerdeMode, apply_serde_transformations};

// Process entire collection
let types = TypeCollection::default().register::<MyType>();
let ser_types = process_for_serialization(&types)?;

// Process individual type
let transformed = apply_serde_transformations(&datatype, SerdeMode::Serialize)?;
```

### TypeScript Integration

```rust
use specta_typescript::Typescript;
use specta_serde::process_for_serialization;

let types = TypeCollection::default().register::<MyType>();
let transformed_types = process_for_serialization(&types)?;

Typescript::default()
    .with_serde()
    .export_to("./bindings.ts", &transformed_types)?;
```

## Testing

The implementation includes comprehensive tests covering:

### Unit Tests (`serde_attrs.rs`)
- Attribute parsing for all supported patterns
- Rename rule application
- Mode-specific behavior
- Edge cases and error conditions

### Integration Tests (`tests/integration.rs`)
- End-to-end transformation workflows
- TypeCollection processing
- Complex nested type handling
- Transparent wrapper support

## Error Handling

The system provides specific error types for common issues:
- `InvalidMapKey`: Invalid key types for map serialization
- `InvalidUsageOfSkip`: Incorrect skip attribute usage
- `InvalidInternallyTaggedEnum`: Invalid internal enum patterns

## Performance Considerations

- Attribute parsing is performed once per transformation
- Recursive transformation with cycle detection
- Minimal memory allocation during transformation
- Efficient string handling with `Cow<'static, str>`

## Future Enhancements

Potential areas for future development:
1. Support for more complex serde attributes (e.g., `with`, `bound`)
2. Custom serialization format support beyond JSON
3. Compile-time optimization hints
4. Advanced validation rules for complex serde patterns

## Compatibility

- Compatible with Specta 2.0.0-rc.22
- Works with all Specta exporters (TypeScript, JSON Schema, etc.)
- Maintains backward compatibility with existing Specta workflows
- Full integration with the Specta macro system

## Conclusion

This implementation provides a robust, type-safe, and comprehensive solution for handling serde attributes in Specta. It enables seamless integration between Rust's serde serialization framework and Specta's type export system, with proper handling of the nuanced differences between serialization and deserialization phases.