// Tests for attribute parsing edge cases
// This file tests that various complex attribute patterns are parsed correctly
// by the lower_attr.rs module in specta-macros

use serde::{Deserialize, Serialize};
use specta::Type;

// Test parsing of attributes with format-like strings
// This was previously causing "expected ident" errors
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[allow(dead_code)]
enum WithDocAttributes {
    #[doc = "Format string test: {0}"]
    A(String),

    #[doc = "Multiple placeholders: {line} {msg}"]
    B { line: usize, msg: String },

    #[doc = "Nested braces: {{escaped}}"]
    C,
}

// Test various attribute syntaxes that should be handled gracefully
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
struct WithVariousAttributes {
    // Simple name-value
    #[serde(rename = "field1")]
    a: String,

    // Path-only
    #[serde(flatten)]
    b: String,

    // Complex expression in doc
    #[doc = "This has {curly} {braces} everywhere"]
    c: i32,
}

#[test]
fn test_attribute_parsing() {
    // Just ensure these types can be derived without parse errors
    let mut types = specta::TypeCollection::default();
    let _ = WithDocAttributes::definition(&mut types);
    let _ = WithVariousAttributes::definition(&mut types);
}
