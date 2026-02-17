//! Runtime representation of Rust attributes for type metadata.
//!
//! This module provides types that represent Rust attributes (like `#[serde(...)]` or `#[specta(...)]`)
//! in a runtime-accessible format. These types enable Specta's macro system to capture attribute information
//! during compile time and make it available to exporters (like TypeScript, Swift, OpenAPI) at runtime.
//!
//! # Purpose
//!
//! Rust proc macros can parse attributes using `syn`, but this information is only available at compile time.
//! To allow type exporters to access attribute metadata at runtime, Specta "lowers" parsed attributes into
//! these runtime representations during macro expansion. The macro generates code that constructs these
//! types, embedding the attribute information into the final binary.
//!
//! This is essential for features like:
//! - Honoring `#[serde(rename = "...")]` when exporting to TypeScript
//! - Respecting `#[serde(skip)]` to exclude fields from exports
//! - Processing custom `#[specta(...)]` attributes for exporter-specific behavior
//! - Accessing any other attribute metadata that influences type generation
//!
//! # Design
//!
//! The types mirror Rust's attribute syntax structure as parsed by `syn`:
//! - [`RuntimeAttribute`] represents a complete attribute (e.g., `#[serde(rename = "foo")]`)
//! - [`RuntimeMeta`] represents the metadata kind (path, name-value, or list)
//! - [`RuntimeNestedMeta`] handles nested content within list-style attributes
//! - [`RuntimeLiteral`] represents literal values (strings, integers, bools, floats)
//!
//! All types use owned `String` data rather than static references to support dynamic construction
//! during macro expansion.

use std::hash::{Hash, Hasher};

/// A complete runtime representation of a Rust attribute.
///
/// This type captures both the attribute's path (e.g., `"serde"`) and its associated metadata.
///
/// # Examples from Macro Syntax
///
/// ```ignore
/// // Parsed from: #[serde(rename = "userName")]
/// RuntimeAttribute {
///     path: "serde".to_string(),
///     kind: RuntimeMeta::List(vec![
///         RuntimeNestedMeta::Meta(RuntimeMeta::NameValue {
///             key: "rename".to_string(),
///             value: RuntimeValue::Literal(RuntimeLiteral::Str("userName".to_string())),
///         })
///     ])
/// }
///
/// // Parsed from: #[specta(skip)]
/// RuntimeAttribute {
///     path: "specta".to_string(),
///     kind: RuntimeMeta::List(vec![
///         RuntimeNestedMeta::Meta(RuntimeMeta::Path("skip".to_string()))
///     ])
/// }
/// ```
///
/// # Why It Exists
///
/// Type exporters need to honor serialization attributes to generate accurate type definitions.
/// For example, a TypeScript exporter must know if a field is renamed via `#[serde(rename = "...")]`
/// to generate the correct interface. By capturing attributes at compile time and making them available
/// at runtime, exporters can make intelligent decisions about how to represent types.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RuntimeAttribute {
    /// The attribute path (e.g., `"serde"`, `"specta"`, `"doc"`).
    pub path: String,
    /// The kind of metadata this attribute contains.
    pub kind: RuntimeMeta,
}

/// The kind of metadata contained in an attribute.
///
/// Rust attributes can take several forms, and this enum captures all of them:
/// - Simple paths (e.g., `untagged` in `#[serde(untagged)]`)
/// - Name-value pairs (e.g., `rename = "value"` in `#[serde(rename = "value")]`)
/// - Lists of nested metadata (e.g., the entire content of `#[serde(rename = "x", skip_serializing)]`)
///
/// # Examples from Macro Syntax
///
/// ```ignore
/// // Path variant - parsed from: #[serde(untagged)]
/// RuntimeMeta::Path("untagged".to_string())
///
/// // NameValue variant - parsed from: #[serde(rename = "userId")]
/// RuntimeMeta::NameValue {
///     key: "rename".to_string(),
///     value: RuntimeValue::Literal(RuntimeLiteral::Str("userId".to_string())),
/// }
///
/// // List variant - parsed from: #[serde(rename = "id", skip_serializing)]
/// RuntimeMeta::List(vec![
///     RuntimeNestedMeta::Meta(RuntimeMeta::NameValue {
///         key: "rename".to_string(),
///         value: RuntimeValue::Literal(RuntimeLiteral::Str("id".to_string())),
///     }),
///     RuntimeNestedMeta::Meta(RuntimeMeta::Path("skip_serializing".to_string())),
/// ])
/// ```
///
/// # Why It Exists
///
/// Attributes have varied syntax in Rust, and exporters need to navigate this structure to find
/// relevant metadata. For instance, a TypeScript exporter might search through a `List` to find
/// a `rename` key, extract its string value, and use that as the property name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuntimeMeta {
    /// A simple path identifier (e.g., `untagged`, `skip`, `flatten`).
    ///
    /// Commonly used for boolean-like flags in attributes.
    Path(String),

    /// A key-value pair (e.g., `rename = "value"`, `default = 42`, `with = module::path`).
    ///
    /// Used when an attribute option needs an associated value.
    NameValue { key: String, value: RuntimeValue },

    /// A list of nested metadata items (e.g., the contents of `#[serde(...)]`).
    ///
    /// Most attributes with parentheses parse as lists, even if they contain a single item.
    List(Vec<RuntimeNestedMeta>),
}

/// Nested metadata within a list-style attribute.
///
/// When attributes contain lists (e.g., `#[serde(rename = "x", skip)]`), each item in the
/// list can be either more metadata (paths, name-values, or nested lists) or a direct literal value.
///
/// # Examples from Macro Syntax
///
/// ```ignore
/// // Meta variant - parsed from: #[serde(rename = "value")]
/// RuntimeNestedMeta::Meta(RuntimeMeta::NameValue {
///     key: "rename".to_string(),
///     value: RuntimeValue::Literal(RuntimeLiteral::Str("value".to_string())),
/// })
///
/// // Literal variant - parsed from: #[custom("raw_string_value")]
/// RuntimeNestedMeta::Literal(RuntimeLiteral::Str("raw_string_value".to_string()))
/// ```
///
/// # Why It Exists
///
/// Some attributes accept direct literal values (e.g., `#[doc = "..."]` or `#[test("name")]`),
/// while others have structured metadata. This enum allows the runtime representation to handle both cases.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuntimeNestedMeta {
    /// Structured metadata (path, name-value, or list).
    Meta(RuntimeMeta),
    /// A direct literal value.
    Literal(RuntimeLiteral),
    /// A non-literal expression captured from attribute syntax.
    Expr(String),
}

/// A value in a name-value attribute pair.
///
/// Rust attributes permit both literals and non-literal expressions in `key = value` positions.
/// This enum keeps those forms distinct so runtime consumers can avoid conflating tokenized
/// expressions with string literals.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RuntimeValue {
    /// A literal value (e.g., `rename = "value"`, `default = true`).
    Literal(RuntimeLiteral),
    /// A non-literal expression (e.g., `with = module::path`, `default = path::to::func`).
    Expr(String),
}

/// A literal value that can appear in an attribute.
///
/// Rust attributes can contain various literal types, and this enum captures the ones
/// commonly used in serialization and type metadata.
///
/// # Examples from Macro Syntax
///
/// ```ignore
/// // Parsed from: #[serde(rename = "userName")]
/// RuntimeLiteral::Str("userName".to_string())
///
/// // Parsed from: #[custom(version = 2)]
/// RuntimeLiteral::Int(2)
///
/// // Parsed from: #[serde(skip_serializing = false)]
/// RuntimeLiteral::Bool(false)
///
/// // Parsed from: #[custom(ratio = 3.14)]
/// RuntimeLiteral::Float(3.14)
///
/// // Parsed from: #[custom(byte_val = b'x')]
/// RuntimeLiteral::Byte(b'x')
///
/// // Parsed from: #[custom(char_val = 'a')]
/// RuntimeLiteral::Char('a')
///
/// // Parsed from: #[custom(bytes = b"hello")]
/// RuntimeLiteral::ByteStr(b"hello".to_vec())
///
/// // Parsed from: #[custom(cstr = c"hello")]
/// RuntimeLiteral::CStr(b"hello\0".to_vec())
/// ```
///
/// # Why It Exists
///
/// Attribute values aren't always strings - they can be integers (for versions), booleans
/// (for flags), or floats (for numeric configuration). Exporters need to access these values
/// in their original type to make correct decisions.
#[derive(Debug, Clone)]
pub enum RuntimeLiteral {
    /// A string literal (e.g., `"value"` in `rename = "value"`).
    Str(String),
    /// An integer literal (e.g., `42` in `version = 42`).
    Int(i64),
    /// A boolean literal (e.g., `true` in `skip = true`).
    Bool(bool),
    /// A floating-point literal (e.g., `3.14` in `ratio = 3.14`).
    Float(f64),
    /// A byte literal (e.g., `b'x'` in `byte_val = b'x'`).
    Byte(u8),
    /// A character literal (e.g., `'a'` in `char_val = 'a'`).
    Char(char),
    /// A byte string literal (e.g., `b"hello"` in `bytes = b"hello"`).
    ByteStr(Vec<u8>),
    /// A C-string literal (e.g., `c"hello"` in `cstr = c"hello"`).
    ///
    /// Note: C-string literals require Rust 1.77+. The variant is always available,
    /// but using the `c"..."` syntax requires a recent compiler.
    CStr(Vec<u8>),
}

// Manual implementation of PartialEq for RuntimeLiteral to handle f64
impl PartialEq for RuntimeLiteral {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (RuntimeLiteral::Str(a), RuntimeLiteral::Str(b)) => a == b,
            (RuntimeLiteral::Int(a), RuntimeLiteral::Int(b)) => a == b,
            (RuntimeLiteral::Bool(a), RuntimeLiteral::Bool(b)) => a == b,
            (RuntimeLiteral::Float(a), RuntimeLiteral::Float(b)) => a.to_bits() == b.to_bits(),
            (RuntimeLiteral::Byte(a), RuntimeLiteral::Byte(b)) => a == b,
            (RuntimeLiteral::Char(a), RuntimeLiteral::Char(b)) => a == b,
            (RuntimeLiteral::ByteStr(a), RuntimeLiteral::ByteStr(b)) => a == b,
            (RuntimeLiteral::CStr(a), RuntimeLiteral::CStr(b)) => a == b,
            _ => false,
        }
    }
}

// Manual implementation of Eq for RuntimeLiteral
impl Eq for RuntimeLiteral {}

// Manual implementation of Hash for RuntimeLiteral to handle f64
impl Hash for RuntimeLiteral {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            RuntimeLiteral::Str(s) => {
                0u8.hash(state);
                s.hash(state);
            }
            RuntimeLiteral::Int(i) => {
                1u8.hash(state);
                i.hash(state);
            }
            RuntimeLiteral::Bool(b) => {
                2u8.hash(state);
                b.hash(state);
            }
            RuntimeLiteral::Float(f) => {
                3u8.hash(state);
                f.to_bits().hash(state);
            }
            RuntimeLiteral::Byte(b) => {
                4u8.hash(state);
                b.hash(state);
            }
            RuntimeLiteral::Char(c) => {
                5u8.hash(state);
                c.hash(state);
            }
            RuntimeLiteral::ByteStr(bs) => {
                6u8.hash(state);
                bs.hash(state);
            }
            RuntimeLiteral::CStr(cs) => {
                7u8.hash(state);
                cs.hash(state);
            }
        }
    }
}
