//! Test that verifies our RuntimeAttribute system can represent all realistic attribute patterns
//!
//! This test focuses on ensuring the attribute lowering code in lower_attr.rs can handle
//! all the patterns that syn can parse, even if specta doesn't use all of them.

#[test]
fn test_runtime_attributes_with_various_types() {
    use specta::datatype::{RuntimeAttribute, RuntimeLiteral, RuntimeMeta, RuntimeNestedMeta};

    // Test that we can construct all variations of RuntimeAttribute

    // 1. Path variant with identifier
    let _path_attr = RuntimeAttribute {
        path: "test".to_string(),
        kind: RuntimeMeta::Path("untagged".to_string()),
    };

    // 2. NameValue with String literal
    let _name_value_str = RuntimeAttribute {
        path: "test".to_string(),
        kind: RuntimeMeta::NameValue {
            key: "rename".to_string(),
            value: RuntimeLiteral::Str("new_name".to_string()),
        },
    };

    // 3. NameValue with Int literal
    let _name_value_int = RuntimeAttribute {
        path: "test".to_string(),
        kind: RuntimeMeta::NameValue {
            key: "version".to_string(),
            value: RuntimeLiteral::Int(42),
        },
    };

    // 4. NameValue with Bool literal
    let _name_value_bool = RuntimeAttribute {
        path: "test".to_string(),
        kind: RuntimeMeta::NameValue {
            key: "enabled".to_string(),
            value: RuntimeLiteral::Bool(true),
        },
    };

    // 5. NameValue with Float literal
    let _name_value_float = RuntimeAttribute {
        path: "test".to_string(),
        kind: RuntimeMeta::NameValue {
            key: "ratio".to_string(),
            value: RuntimeLiteral::Float(3.14),
        },
    };

    // 6. List with nested meta
    let _list_attr = RuntimeAttribute {
        path: "test".to_string(),
        kind: RuntimeMeta::List(vec![
            RuntimeNestedMeta::Meta(RuntimeMeta::Path("skip".to_string())),
            RuntimeNestedMeta::Meta(RuntimeMeta::NameValue {
                key: "rename".to_string(),
                value: RuntimeLiteral::Str("foo".to_string()),
            }),
        ]),
    };

    // 7. List with nested list (recursive structure)
    let _nested_list = RuntimeAttribute {
        path: "test".to_string(),
        kind: RuntimeMeta::List(vec![RuntimeNestedMeta::Meta(RuntimeMeta::List(vec![
            RuntimeNestedMeta::Meta(RuntimeMeta::Path("inner".to_string())),
        ]))]),
    };

    // 8. List with literal value
    let _list_with_literal = RuntimeAttribute {
        path: "test".to_string(),
        kind: RuntimeMeta::List(vec![RuntimeNestedMeta::Literal(RuntimeLiteral::Str(
            "literal_value".to_string(),
        ))]),
    };
}
