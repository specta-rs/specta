use serde::{Deserialize, Serialize};
use specta::Type;

use crate::ts::assert_ts_inline2;

// Test struct with field alias
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct StructWithAlias {
    #[serde(alias = "bruh")]
    field: String,
}

// Test struct with multiple aliases on same field
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct StructWithMultipleAliases {
    #[serde(alias = "bruh", alias = "alternative", alias = "another")]
    field: String,
}

// Test struct with alias and rename
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub struct StructWithAliasAndRename {
    #[serde(rename = "renamed_field", alias = "bruh")]
    field: String,
}

// Test enum variant with alias
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub enum EnumWithVariantAlias {
    #[serde(alias = "bruh")]
    Variant,
    Other,
}

// Test enum with multiple variant aliases
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub enum EnumWithMultipleVariantAliases {
    #[serde(alias = "bruh", alias = "alternative")]
    Variant,
    Other,
}

// Test enum variant with alias and rename
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
pub enum EnumWithVariantAliasAndRename {
    #[serde(rename = "renamed_variant", alias = "bruh")]
    Variant,
    Other,
}

// Test internally tagged enum with field alias
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type")]
pub enum InternallyTaggedWithAlias {
    A {
        #[serde(alias = "bruh")]
        field: String,
    },
    B {
        other: i32,
    },
}

// Test adjacently tagged enum with field alias
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(tag = "type", content = "data")]
pub enum AdjacentlyTaggedWithAlias {
    A {
        #[serde(alias = "bruh")]
        field: String,
    },
    B {
        other: i32,
    },
}

// Test untagged enum with field alias
#[derive(Type, Serialize, Deserialize)]
#[specta(collect = false)]
#[serde(untagged)]
pub enum UntaggedWithAlias {
    A {
        #[serde(alias = "bruh")]
        field: String,
    },
    B {
        other: i32,
    },
}

#[test]
fn alias() {
    // Note: alias is used during deserialization only, so it should not affect the TypeScript output
    // The TypeScript output should only show the primary field name (or renamed name if using rename)

    insta::assert_snapshot!(assert_ts_inline2::<StructWithAlias>().unwrap(), @r#"{ field: string }"#);
    insta::assert_snapshot!(assert_ts_inline2::<StructWithMultipleAliases>().unwrap(), @r#"{ field: string }"#);
    insta::assert_snapshot!(assert_ts_inline2::<StructWithAliasAndRename>().unwrap(), @r#"{ renamed_field: string }"#);

    insta::assert_snapshot!(assert_ts_inline2::<EnumWithVariantAlias>().unwrap(), @r#""Variant" | "Other""#);
    insta::assert_snapshot!(assert_ts_inline2::<EnumWithMultipleVariantAliases>().unwrap(), @r#""Variant" | "Other""#);
    insta::assert_snapshot!(assert_ts_inline2::<EnumWithVariantAliasAndRename>().unwrap(), @r#""renamed_variant" | "Other""#);

    insta::assert_snapshot!(assert_ts_inline2::<InternallyTaggedWithAlias>().unwrap(), @r#"{ type: "A"; field: string } | { type: "B"; other: number }"#);
    insta::assert_snapshot!(assert_ts_inline2::<AdjacentlyTaggedWithAlias>().unwrap(), @r#"{ type: "A"; data: { field: string } } | { type: "B"; data: { other: number } }"#);
    insta::assert_snapshot!(assert_ts_inline2::<UntaggedWithAlias>().unwrap(), @r#"{ field: string } | { other: number }"#);
}
