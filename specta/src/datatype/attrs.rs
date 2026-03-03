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
//! - [`Attribute`] represents a complete attribute (e.g., `#[serde(rename = "foo")]`)
//! - [`AttributeMeta`] represents the metadata kind (path, name-value, or list)
//! - [`AttributeNestedMeta`] handles nested content within list-style attributes
//! - [`AttributeLiteral`] represents literal values (strings, integers, bools, floats)
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
/// Attribute {
///     path: "serde".to_string(),
///     kind: AttributeMeta::List(vec![
///         AttributeNestedMeta::Meta(AttributeMeta::NameValue {
///             key: "rename".to_string(),
///             value: AttributeValue::Literal(AttributeLiteral::Str("userName".to_string())),
///         })
///     ])
/// }
///
/// // Parsed from: #[specta(skip)]
/// Attribute {
///     path: "specta".to_string(),
///     kind: AttributeMeta::List(vec![
///         AttributeNestedMeta::Meta(AttributeMeta::Path("skip".to_string()))
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
pub struct Attribute {
    /// The attribute path (e.g., `"serde"`, `"specta"`, `"doc"`).
    pub path: String,
    /// The kind of metadata this attribute contains.
    pub kind: AttributeMeta,
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
/// AttributeMeta::Path("untagged".to_string())
///
/// // NameValue variant - parsed from: #[serde(rename = "userId")]
/// AttributeMeta::NameValue {
///     key: "rename".to_string(),
///     value: AttributeValue::Literal(AttributeLiteral::Str("userId".to_string())),
/// }
///
/// // List variant - parsed from: #[serde(rename = "id", skip_serializing)]
/// AttributeMeta::List(vec![
///     AttributeNestedMeta::Meta(AttributeMeta::NameValue {
///         key: "rename".to_string(),
///         value: AttributeValue::Literal(AttributeLiteral::Str("id".to_string())),
///     }),
///     AttributeNestedMeta::Meta(AttributeMeta::Path("skip_serializing".to_string())),
/// ])
/// ```
///
/// # Why It Exists
///
/// Attributes have varied syntax in Rust, and exporters need to navigate this structure to find
/// relevant metadata. For instance, a TypeScript exporter might search through a `List` to find
/// a `rename` key, extract its string value, and use that as the property name.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttributeMeta {
    /// A simple path identifier (e.g., `untagged`, `skip`, `flatten`).
    ///
    /// Commonly used for boolean-like flags in attributes.
    Path(String),

    /// A key-value pair (e.g., `rename = "value"`, `default = 42`, `with = module::path`).
    ///
    /// Used when an attribute option needs an associated value.
    NameValue {
        /// The option key (for example `rename` or `default`).
        key: String,
        /// The option value associated with [`Self::NameValue::key`].
        value: AttributeValue,
    },

    /// A list of nested metadata items (e.g., the contents of `#[serde(...)]`).
    ///
    /// Most attributes with parentheses parse as lists, even if they contain a single item.
    List(Vec<AttributeNestedMeta>),
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
/// AttributeNestedMeta::Meta(AttributeMeta::NameValue {
///     key: "rename".to_string(),
///     value: AttributeValue::Literal(AttributeLiteral::Str("value".to_string())),
/// })
///
/// // Literal variant - parsed from: #[custom("raw_string_value")]
/// AttributeNestedMeta::Literal(AttributeLiteral::Str("raw_string_value".to_string()))
/// ```
///
/// # Why It Exists
///
/// Some attributes accept direct literal values (e.g., `#[doc = "..."]` or `#[test("name")]`),
/// while others have structured metadata. This enum allows the runtime representation to handle both cases.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttributeNestedMeta {
    /// Structured metadata (path, name-value, or list).
    Meta(AttributeMeta),
    /// A direct literal value.
    Literal(AttributeLiteral),
    /// A non-literal expression captured from attribute syntax.
    Expr(String),
}

/// A value in a name-value attribute pair.
///
/// Rust attributes permit both literals and non-literal expressions in `key = value` positions.
/// This enum keeps those forms distinct so runtime consumers can avoid conflating tokenized
/// expressions with string literals.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AttributeValue {
    /// A literal value (e.g., `rename = "value"`, `default = true`).
    Literal(AttributeLiteral),
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
/// AttributeLiteral::Str("userName".to_string())
///
/// // Parsed from: #[custom(version = 2)]
/// AttributeLiteral::Int(2)
///
/// // Parsed from: #[serde(skip_serializing = false)]
/// AttributeLiteral::Bool(false)
///
/// // Parsed from: #[custom(ratio = 3.14)]
/// AttributeLiteral::Float(3.14)
///
/// // Parsed from: #[custom(byte_val = b'x')]
/// AttributeLiteral::Byte(b'x')
///
/// // Parsed from: #[custom(char_val = 'a')]
/// AttributeLiteral::Char('a')
///
/// // Parsed from: #[custom(bytes = b"hello")]
/// AttributeLiteral::ByteStr(b"hello".to_vec())
///
/// // Parsed from: #[custom(cstr = c"hello")]
/// AttributeLiteral::CStr(b"hello\0".to_vec())
/// ```
///
/// # Why It Exists
///
/// Attribute values aren't always strings - they can be integers (for versions), booleans
/// (for flags), or floats (for numeric configuration). Exporters need to access these values
/// in their original type to make correct decisions.
#[derive(Debug, Clone)]
pub enum AttributeLiteral {
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

// Manual implementation of PartialEq for AttributeLiteral to handle f64
impl PartialEq for AttributeLiteral {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AttributeLiteral::Str(a), AttributeLiteral::Str(b)) => a == b,
            (AttributeLiteral::Int(a), AttributeLiteral::Int(b)) => a == b,
            (AttributeLiteral::Bool(a), AttributeLiteral::Bool(b)) => a == b,
            (AttributeLiteral::Float(a), AttributeLiteral::Float(b)) => a.to_bits() == b.to_bits(),
            (AttributeLiteral::Byte(a), AttributeLiteral::Byte(b)) => a == b,
            (AttributeLiteral::Char(a), AttributeLiteral::Char(b)) => a == b,
            (AttributeLiteral::ByteStr(a), AttributeLiteral::ByteStr(b)) => a == b,
            (AttributeLiteral::CStr(a), AttributeLiteral::CStr(b)) => a == b,
            _ => false,
        }
    }
}

// Manual implementation of Eq for AttributeLiteral
impl Eq for AttributeLiteral {}

// Manual implementation of Hash for AttributeLiteral to handle f64
impl Hash for AttributeLiteral {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            AttributeLiteral::Str(s) => {
                0u8.hash(state);
                s.hash(state);
            }
            AttributeLiteral::Int(i) => {
                1u8.hash(state);
                i.hash(state);
            }
            AttributeLiteral::Bool(b) => {
                2u8.hash(state);
                b.hash(state);
            }
            AttributeLiteral::Float(f) => {
                3u8.hash(state);
                f.to_bits().hash(state);
            }
            AttributeLiteral::Byte(b) => {
                4u8.hash(state);
                b.hash(state);
            }
            AttributeLiteral::Char(c) => {
                5u8.hash(state);
                c.hash(state);
            }
            AttributeLiteral::ByteStr(bs) => {
                6u8.hash(state);
                bs.hash(state);
            }
            AttributeLiteral::CStr(cs) => {
                7u8.hash(state);
                cs.hash(state);
            }
        }
    }
}
