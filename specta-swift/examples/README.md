# specta-swift Examples

This directory contains comprehensive examples demonstrating ALL the functionality of the `specta-swift` library. Each example focuses on different aspects of the Swift code generation capabilities.

## 📚 Available Examples

### 1. **`basic_types.rs`** - Fundamental Type Mappings

Demonstrates basic Rust to Swift type conversions:

- ✅ Primitive types (i8, u32, f64, bool, char, String)
- ✅ Optional types (Option<T> → T?)
- ✅ Collections (Vec<T> → [T])
- ✅ Nested collections (Vec<Vec<T>> → [[T]])
- ✅ Tuple types ((String, String) → (String, String))
- ✅ Simple enums with different variant types
- ✅ Generic structs with type parameters
- ✅ Complex nested struct relationships

**Run:** `cargo run --example basic_types`  
**Output:** `examples/generated/BasicTypes.swift`

### 2. **`advanced_unions.rs`** - Complex Enum Scenarios

Showcases advanced enum patterns and custom Codable implementations:

- ✅ Complex enums with mixed variant types
- ✅ Generic enums with type parameters
- ✅ Recursive type definitions (Tree<T>)
- ✅ Nested struct references in enum variants
- ✅ String enums with automatic Codable
- ✅ Mixed enums (both simple and complex variants)
- ✅ Custom Codable implementations for complex enums
- ✅ Struct generation for named field variants

**Run:** `cargo run --example advanced_unions`  
**Output:** `examples/generated/AdvancedUnions.swift`

### 3. **`configuration_options.rs`** - All Swift Exporter Settings

Comprehensive demonstration of every configuration option:

- ✅ Custom headers and documentation
- ✅ Naming conventions (PascalCase, camelCase, snake_case)
- ✅ Indentation styles (spaces, tabs, different widths)
- ✅ Generic type styles (protocol constraints vs typealias)
- ✅ Optional type styles (question mark vs Optional<T>)
- ✅ Additional protocol conformance
- ✅ Serde validation settings
- ✅ Combined custom configurations

**Run:** `cargo run --example configuration_options`  
**Output:** `examples/generated/` (multiple configuration files)

### 4. **`special_types.rs`** - Duration and Special Type Handling

Demonstrates special type conversions and helper generation:

- ✅ Duration type mapping to RustDuration helper
- ✅ Automatic helper struct generation
- ✅ timeInterval property for Swift integration
- ✅ Duration fields in structs and enum variants
- ✅ Optional Duration fields
- ✅ Performance metrics with timing information
- ✅ Complex timing-related data structures

**Run:** `cargo run --example special_types`  
**Output:** `examples/generated/SpecialTypes.swift`

### 5. **`string_enums.rs`** - String Enums and Custom Codable

Focuses on enum patterns and Codable implementations:

- ✅ Pure string enums (String, Codable)
- ✅ Mixed enums with both simple and complex variants
- ✅ Custom Codable implementations for complex enums
- ✅ Struct generation for named field variants
- ✅ Generic enum support
- ✅ Proper Swift enum case naming
- ✅ Automatic protocol conformance

**Run:** `cargo run --example string_enums`  
**Output:** `examples/generated/StringEnums.swift`

### 6. **`comprehensive_demo.rs`** - Complete Feature Showcase

The ultimate example demonstrating EVERY feature in a realistic application:

- ✅ All basic and advanced type patterns
- ✅ Complex nested relationships
- ✅ User management with permissions
- ✅ Task management with status tracking
- ✅ File attachment handling
- ✅ Comment and review systems
- ✅ API response patterns
- ✅ System monitoring types
- ✅ Health monitoring and metrics
- ✅ Pagination and metadata

**Run:** `cargo run --example comprehensive_demo`  
**Output:** `examples/generated/ComprehensiveDemo.swift`

### 7. **`simple_usage.rs`** - Quick Start Example

A simple, focused example for getting started quickly:

- ✅ Basic struct and enum definitions
- ✅ Default Swift configuration
- ✅ Custom configuration demonstration
- ✅ File export functionality

**Run:** `cargo run --example simple_usage`  
**Output:** `examples/generated/SimpleTypes.swift`, `examples/generated/CustomTypes.swift`

### 8. **`comments_example.rs`** - Documentation and Comments

Demonstrates comprehensive documentation support:

- ✅ Multi-line type documentation
- ✅ Field-level documentation
- ✅ Complex technical descriptions
- ✅ Swift-compatible doc comments
- ✅ Bullet points and formatting

**Run:** `cargo run --example comments_example`  
**Output:** `examples/generated/CommentsExample.swift`

## 🚀 Quick Start

To run any example:

```bash
cd specta-swift
cargo run --example <example_name>
```

For example:

```bash
cargo run --example basic_types
cargo run --example comprehensive_demo
```

## 📁 Generated Files

Each example generates Swift files in the `examples/generated/` directory that you can inspect:

- `examples/generated/BasicTypes.swift` - From basic_types example
- `examples/generated/AdvancedUnions.swift` - From advanced_unions example
- `examples/generated/SpecialTypes.swift` - From special_types example
- `examples/generated/StringEnums.swift` - From string_enums example
- `examples/generated/ComprehensiveDemo.swift` - From comprehensive_demo example
- `examples/generated/CommentsExample.swift` - From comments_example
- `examples/generated/SimpleTypes.swift` & `examples/generated/CustomTypes.swift` - From simple_usage
- Multiple configuration files from the configuration_options example

## 🔍 Key Features Demonstrated

### Type System Support

- ✅ All Rust primitive types
- ✅ Optional types with proper Swift syntax
- ✅ Collections and nested collections
- ✅ Tuple types
- ✅ Generic types with constraints
- ✅ Complex nested relationships

### Enum Handling

- ✅ Unit variants
- ✅ Tuple variants
- ✅ Named field variants
- ✅ String enums with automatic Codable
- ✅ Mixed enums with custom implementations
- ✅ Generic enums
- ✅ Recursive enum definitions

### Special Types

- ✅ Duration → RustDuration helper
- ✅ Automatic helper struct generation
- ✅ timeInterval property for Swift integration
- ✅ Proper Codable implementations

### Configuration Options

- ✅ All naming conventions
- ✅ All indentation styles
- ✅ Generic type styles
- ✅ Optional type styles
- ✅ Additional protocols
- ✅ Custom headers and documentation

### Code Generation Quality

- ✅ Proper Swift naming conventions
- ✅ Comprehensive Codable implementations
- ✅ Error handling in custom Codable
- ✅ Documentation preservation
- ✅ Clean, readable Swift code

## 💡 Usage Tips

1. **Start with `basic_types`** to understand fundamental mappings
2. **Use `comprehensive_demo`** to see everything in action
3. **Check `configuration_options`** to customize your output
4. **Examine generated `.swift` files** to see the actual output
5. **Use `special_types`** if you work with Duration types
6. **Reference `string_enums`** for enum patterns

## 🎯 Real-World Applications

These examples demonstrate patterns commonly used in:

- 📱 iOS/macOS app development
- 🔄 API client generation
- 📊 Data serialization/deserialization
- 🏗️ Cross-platform type sharing
- 📈 Performance monitoring
- 👥 User management systems
- 📋 Task management applications

---

**Happy coding! 🎉** These examples should give you everything you need to leverage the full power of `specta-swift` in your projects.
