# Specta Swift Implementation Plan

## Overview

This document outlines the implementation plan for `specta-swift`, a language exporter that converts Rust types to Swift code. The implementation follows Specta's established patterns while addressing Swift-specific language features and conventions.

## Current Status

- **Status**: Not implemented (stub code only)
- **Priority**: High - Swift is listed as "coming soon" in Specta's language support table
- **Dependencies**: specta core, specta-serde for validation
- **Target**: Swift 5.0+ with Foundation and Codable support

## Architecture Overview

### Core Components

1. **Swift Exporter Struct** - Main configuration and export interface
2. **Type Conversion System** - Converts `DataType` enum variants to Swift syntax
3. **Primitive Mapping** - Maps Rust primitives to Swift types
4. **Collection Handling** - Arrays, dictionaries, and optionals
5. **Struct/Enum Export** - Complex type definitions
6. **Error Handling** - Swift-specific validation and error reporting

## Implementation Phases

### Phase 1: Project Setup & Core Infrastructure (Week 1)

#### 1.1 Project Structure

- [ ] Set up proper Cargo.toml with dependencies
- [ ] Create module structure: `lib.rs`, `error.rs`, `swift.rs`, `primitives.rs`
- [ ] Add basic documentation and examples
- [ ] Set up CI/CD with GitHub Actions

#### 1.2 Core Types and Configuration

- [ ] Implement `Swift` struct with configuration options
- [ ] Add `IndentStyle`, `NamingConvention`, `OptionalStyle` enums
- [ ] Create builder pattern methods
- [ ] Implement `Default` trait

#### 1.3 Error Handling

- [ ] Define `Error` enum with Swift-specific errors
- [ ] Implement `thiserror` derive for error handling
- [ ] Add error conversion from `specta_serde::Error`

#### 1.4 Basic Type Mapping

- [ ] Implement primitive type conversion (i8 → Int8, String → String, etc.)
- [ ] Add validation for unsupported types (i128, u128, f16)
- [ ] Create primitive mapping tests

### Phase 2: Basic Type Support (Week 2)

#### 2.1 Collection Types

- [ ] Implement array conversion (`Vec<T>` → `[T]`)
- [ ] Implement dictionary conversion (`HashMap<K, V>` → `[K: V]`)
- [ ] Add tuple support (unit, single, multiple elements)
- [ ] Handle nested collections

#### 2.2 Optional Types

- [ ] Implement nullable conversion (`Option<T>` → `T?` or `Optional<T>`)
- [ ] Add configuration for optional style
- [ ] Handle nested optionals

#### 2.3 Literal Types

- [ ] Support literal values for const generics
- [ ] Handle string, numeric, and boolean literals
- [ ] Add validation for unsupported literal types

### Phase 3: Complex Types (Week 3)

#### 3.1 Struct Export

- [ ] Unit structs (`struct Unit;` → `Void`)
- [ ] Tuple structs (`struct Point(f64, f64)` → `(Double, Double)`)
- [ ] Named structs with proper Swift syntax
- [ ] Handle struct tags and metadata
- [ ] Support for flattened fields

#### 3.2 Enum Export

- [ ] Unit variants (`Variant` → `case variant`)
- [ ] Tuple variants (`Variant(String)` → `case variant(String)`)
- [ ] Named variants (`Variant { field: String }` → `case variant(field: String)`)
- [ ] Handle enum representations (untagged, external, internal, adjacent)
- [ ] Support for skipped variants

#### 3.3 Generic Types

- [ ] Generic parameter handling
- [ ] Protocol constraints (`<T: Codable>`)
- [ ] Generic type resolution
- [ ] Nested generics support

### Phase 4: Advanced Features (Week 4)

#### 4.1 Reference Resolution

- [ ] Handle type references and circular dependencies
- [ ] Implement proper type name resolution
- [ ] Support for module-prefixed names
- [ ] Handle inline vs reference types

#### 4.2 Swift-Specific Features

- [ ] Protocol conformance (Codable, CustomStringConvertible, etc.)
- [ ] Naming convention conversion (snake_case → camelCase)
- [ ] Swift keyword escaping
- [ ] Access control (public, internal, private)

#### 4.3 Configuration Options

- [ ] Indentation style configuration
- [ ] Naming convention options
- [ ] Optional style preferences
- [ ] Custom protocol additions
- [ ] Header and import management

### Phase 5: Testing & Documentation (Week 5)

#### 5.1 Test Suite

- [ ] Unit tests for all type conversions
- [ ] Integration tests with real Rust types
- [ ] Snapshot testing for complex types
- [ ] Error case testing
- [ ] Performance benchmarks

#### 5.2 Test Macros

- [ ] `assert_swift!` macro for inline testing
- [ ] `assert_swift_export!` macro for full export testing
- [ ] Helper functions for test setup

#### 5.3 Documentation

- [ ] API documentation with examples
- [ ] Migration guide from other exporters
- [ ] Swift-specific best practices
- [ ] Troubleshooting guide

### Phase 6: Integration & Polish (Week 6)

#### 6.1 Specta Integration

- [ ] Update main Specta documentation
- [ ] Add to language support table
- [ ] Update examples and tutorials
- [ ] Add to CI/CD pipeline

#### 6.2 Performance Optimization

- [ ] Optimize string building and allocation
- [ ] Reduce memory usage during export
- [ ] Profile and optimize hot paths
- [ ] Add performance tests

#### 6.3 Community & Release

- [ ] Create example projects
- [ ] Write blog post about Swift support
- [ ] Gather community feedback
- [ ] Prepare for initial release

## File Structure

```
specta-swift/
├── Cargo.toml
├── README.md
├── PLAN.md
├── src/
│   ├── lib.rs              # Main library entry point
│   ├── error.rs            # Error types and handling
│   ├── swift.rs            # Swift exporter struct and configuration
│   ├── primitives.rs       # Primitive type conversion
│   ├── collections.rs      # Array, dictionary, tuple handling
│   ├── structs.rs          # Struct export logic
│   ├── enums.rs            # Enum export logic
│   ├── generics.rs         # Generic type handling
│   ├── naming.rs           # Naming convention conversion
│   └── utils.rs            # Utility functions
├── examples/
│   ├── basic.rs            # Basic usage example
│   ├── complex.rs          # Complex types example
│   └── configuration.rs    # Configuration options example
└── tests/
    ├── integration_tests.rs
    ├── snapshots/
    └── test_helpers.rs
```

## API Design

### Main Exporter

```rust
use specta_swift::Swift;

let swift = Swift::new()
    .header("// Generated by Specta")
    .indent(IndentStyle::Spaces(4))
    .naming(NamingConvention::CamelCase)
    .optionals(OptionalStyle::QuestionMark)
    .with_serde()
    .add_protocol("CustomStringConvertible");

let output = swift.export(&types)?;
swift.export_to("./Types.swift", &types)?;
```

### Configuration Options

```rust
pub struct Swift {
    pub header: Cow<'static, str>,
    pub indent: IndentStyle,
    pub naming: NamingConvention,
    pub generics: GenericStyle,
    pub optionals: OptionalStyle,
    pub protocols: Vec<Cow<'static, str>>,
    pub serde: bool,
}
```

## Expected Output Examples

### Simple Struct

```rust
#[derive(Type)]
struct User {
    name: String,
    age: u32,
    active: bool,
}
```

**Swift Output:**

```swift
import Foundation

struct User: Codable {
    let name: String
    let age: UInt32
    let active: Bool
}
```

### Complex Enum

```rust
#[derive(Type)]
enum Status {
    Active,
    Inactive,
    Pending { reason: String },
    Error(String),
}
```

**Swift Output:**

```swift
import Foundation

enum Status: Codable {
    case active
    case inactive
    case pending(reason: String)
    case error(String)
}
```

### Generic Type

```rust
#[derive(Type)]
struct Response<T> {
    data: T,
    success: bool,
}
```

**Swift Output:**

```swift
import Foundation

struct Response<T: Codable>: Codable {
    let data: T
    let success: Bool
}
```

## Testing Strategy

### Test Categories

1. **Unit Tests**: Individual type conversion functions
2. **Integration Tests**: Full export workflow with real types
3. **Snapshot Tests**: Complex type outputs using `insta`
4. **Error Tests**: Invalid input handling
5. **Performance Tests**: Large type collection export

### Test Macros

```rust
// Inline type testing
assert_swift!(String, "String");
assert_swift!(Option<String>, "String?");

// Full export testing
assert_swift_export!(MyStruct, "struct MyStruct: Codable { ... }");

// Error testing
assert_swift!(error; UnsupportedType, "Swift does not support 128-bit integers");
```

## Dependencies

```toml
[dependencies]
specta = { path = "../specta", features = ["derive"] }
specta-serde = { path = "../specta-serde" }
thiserror = "1.0"
serde = { version = "1.0", features = ["derive"] }

[dev-dependencies]
insta = "1.42"
trybuild = "1.0"
```

## Success Criteria

- [ ] All basic Rust types convert to appropriate Swift types
- [ ] Complex types (structs, enums, generics) export correctly
- [ ] Generated Swift code compiles without errors
- [ ] Comprehensive test coverage (>90%)
- [ ] Performance comparable to other language exporters
- [ ] Full documentation and examples
- [ ] Integration with main Specta project

## Risks and Mitigations

### Risk: Swift Language Evolution

- **Mitigation**: Target stable Swift versions, test against multiple versions

### Risk: Complex Generic Constraints

- **Mitigation**: Start with simple cases, iterate on complex scenarios

### Risk: Performance with Large Type Collections

- **Mitigation**: Profile early, optimize string building, consider streaming

### Risk: Swift Naming Conflicts

- **Mitigation**: Implement keyword escaping, validate identifiers

## Timeline

- **Week 1**: Project setup and basic infrastructure
- **Week 2**: Basic type support (primitives, collections, optionals)
- **Week 3**: Complex types (structs, enums, generics)
- **Week 4**: Advanced features and Swift-specific functionality
- **Week 5**: Testing and documentation
- **Week 6**: Integration, polish, and release preparation

## Next Steps

1. **Immediate**: Set up project structure and basic configuration
2. **Short-term**: Implement primitive type mapping and basic tests
3. **Medium-term**: Add struct and enum support with comprehensive testing
4. **Long-term**: Full feature parity with other language exporters

This plan provides a structured approach to implementing `specta-swift` while maintaining consistency with Specta's architecture and ensuring high-quality, well-tested code.
