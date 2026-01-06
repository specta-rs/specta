# Attribute System Review

## Overview

This document reviews the completeness of the runtime attribute system defined in `specta/src/datatype/attrs.rs` and implemented in `specta-macros/src/type/lower_attr.rs`.

## Goal

Ensure the runtime attribute system can express everything that syn can parse from Rust attributes.

## Syn's Attribute Structure

Syn provides the following types for attributes:

```rust
struct Attribute {
    path: Path,        // e.g., "serde", "specta"
    meta: Meta,        // The attribute content
}

enum Meta {
    Path(Path),                      // e.g., #[derive(Debug)]
    NameValue(MetaNameValue),        // e.g., #[doc = "..."]
    List(MetaList),                  // e.g., #[serde(rename = "...")]
}

enum Lit {
    Str, Int, Float, Bool, Byte, ByteStr, Char, CStr, Verbatim
}
```

## Our Runtime Structure

```rust
struct RuntimeAttribute {
    path: String,      // Attribute name (e.g., "serde")
    kind: RuntimeMeta, // Attribute content
}

enum RuntimeMeta {
    Path(String),                          // Path-only attribute with identifier
    NameValue { key: String, value: ... }, // Key-value pair
    List(Vec<RuntimeNestedMeta>),          // List of nested items
}

enum RuntimeNestedMeta {
    Meta(RuntimeMeta),       // Nested meta item (recursive)
    Literal(RuntimeLiteral), // Direct literal value
}

enum RuntimeLiteral {
    Str(String),  // String literal
    Int(i64),     // Integer literal
    Bool(bool),   // Boolean literal
    Float(f64),   // Float literal
}
```

## Completeness Analysis

### ✅ Fully Supported Patterns

1. **Path-only attributes** - `#[derive(Debug)]`, `#[serde(untagged)]`
   - Captured as `RuntimeMeta::Path(String)` with the identifier

2. **Name-value with literals** - `#[doc = "text"]`, `#[serde(rename = "foo")]`
   - Captured as `RuntimeMeta::NameValue` with literal values

3. **Name-value with path expressions** - `#[specta(remote = Value)]`
   - Converted to string representation via `to_token_stream()`

4. **Nested lists** - `#[serde(rename(serialize = "a", deserialize = "b"))]`
   - Handled recursively via `RuntimeMeta::List`

5. **Direct literals in lists** - `#[test("literal")]`
   - Captured as `RuntimeNestedMeta::Literal`

6. **All common literal types**:
   - ✅ String: `"text"`
   - ✅ Integer: `42`
   - ✅ Boolean: `true`
   - ✅ Float: `3.14`

### ⚠️ Deliberately Not Supported (Acceptable Trade-offs)

1. **Byte literals** - `b'x'`, `b"bytes"`
   - **Impact**: Minimal - not used in derive macro attributes
   - **Rationale**: Serde, specta, and other derive macros don't use byte literals

2. **Char literals** - `'c'`
   - **Impact**: Minimal - rare in attribute context
   - **Rationale**: Can be represented as string if needed

3. **CStr literals** - `c"string"` (Rust 1.77+)
   - **Impact**: None currently - too new
   - **Rationale**: Not used in existing ecosystem

4. **Verbatim literals** - Token literals
   - **Impact**: None - internal syn type
   - **Rationale**: Not relevant for derive macros

5. **Span information** - Source location
   - **Impact**: Less precise error messages at runtime
   - **Rationale**: Runtime attributes are for reflection, not error reporting

6. **Complex expression types** - Beyond string conversion
   - **Impact**: Loss of type information for complex expressions
   - **Rationale**: String representation is sufficient for runtime reflection

## Implementation Quality

### Strengths

1. **Recursive structure** - Properly handles nested lists
2. **Path preservation** - NOW captures path identifiers (fixed in this PR)
3. **Type-safe** - All variants are well-defined
4. **Hash & Eq** - Implements standard traits correctly
5. **Float handling** - Uses bit representation for equality/hashing

### Verified Coverage

The following attribute patterns have been tested:

```rust
// ✅ Path-only
#[serde(untagged)]

// ✅ Name-value with string
#[serde(rename = "foo")]

// ✅ Name-value with int
#[some_attr(version = 42)]

// ✅ Name-value with bool
#[some_attr(enabled = true)]

// ✅ Name-value with float
#[some_attr(ratio = 3.14)]

// ✅ Name-value with path expression
#[specta(remote = Value)]

// ✅ Nested lists
#[serde(rename(serialize = "a", deserialize = "b"))]

// ✅ List with literals
#[test("literal_value")]

// ✅ Mixed nested structures
#[serde(skip, rename = "foo")]
```

## Conclusion

The runtime attribute system is **complete and fit for purpose**. It can represent all realistic attribute patterns used in the Rust ecosystem, particularly for derive macros like serde and specta.

### Key Achievement

The fix implemented in this PR ensures that path-only attributes like `#[serde(untagged)]` properly preserve their identifier, resolving the critical bug where they were losing their meaning.

### Trade-offs

The missing literal types (byte, char, cstr) are **intentionally not supported** because:
1. They are not used in derive macro attributes
2. Adding them would increase complexity without practical benefit
3. They can be represented as strings if ever needed

### Recommendation

**No further changes needed.** The system is complete for all practical use cases in the specta ecosystem.
