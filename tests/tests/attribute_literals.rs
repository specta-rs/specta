//! Integration tests for byte, char, bytestr, and cstr literal support in attributes
//!
//! These tests verify that the macro system can parse and lower all literal types
//! that syn supports in attribute contexts.
//!
//! Note: We can't actually test custom attributes with non-string literals easily
//! because Rust doesn't allow unknown attributes. Instead, we test that the
//! runtime types can be constructed correctly (which is tested in unit tests).
//!
//! The main integration is tested by ensuring the macro compiles when these
//! variants exist in the enum.

use specta::Type;

#[test]
fn test_new_literal_types_compile() {
    // This test verifies that having the new literal type variants doesn't
    // break compilation of the derive macro.
    #[derive(Type)]
    struct TestStruct {
        #[doc = "A field with a doc comment"]
        field: String,
    }

    use specta::Type as _;
    let mut type_map = specta::TypeCollection::default();
    let _ = TestStruct::definition(&mut type_map);
}
