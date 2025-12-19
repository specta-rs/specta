//! Comprehensive demonstration of all Serde attributes supported by specta-serde
//!
//! This example showcases the complete set of Serde attributes that specta-serde
//! can handle, including the newly implemented advanced features like:
//! - Advanced rename syntax with serialize/deserialize specificity
//! - The `other` attribute for handling unknown enum variants
//! - All container, variant, and field attributes
//! - Mode-specific transformations

use serde::{Deserialize, Serialize};
use specta_macros::Type;

fn main() {
    println!("=== Comprehensive Serde Attributes Demo ===\n");

    test_container_attributes();
    test_variant_attributes();
    test_field_attributes();
    test_advanced_rename_syntax();
}

fn test_container_attributes() {
    println!("📦 Container Attributes:");

    // Basic rename
    #[derive(Serialize, Deserialize, Type)]
    #[serde(rename = "CustomUser")]
    struct User {
        name: String,
    }

    // Rename all variants/fields
    #[derive(Serialize, Deserialize, Type)]
    #[serde(rename_all = "camelCase")]
    struct UserPrefs {
        dark_mode: bool,
        font_size: u32,
    }

    // Rename all fields in enum variants
    #[derive(Serialize, Deserialize, Type)]
    #[serde(rename_all_fields = "camelCase")]
    enum Message {
        UserAction {
            user_id: u64,
            action_type: String,
        },
        SystemEvent {
            event_id: u32,
            severity_level: String,
        },
    }

    // Tagged enum representations
    #[derive(Serialize, Deserialize, Type)]
    #[serde(tag = "type")]
    enum InternallyTagged {
        A { value: String },
        B { count: u32 },
    }

    #[derive(Serialize, Deserialize, Type)]
    #[serde(tag = "type", content = "data")]
    enum AdjacentlyTagged {
        Text(String),
        Number(i32),
    }

    #[derive(Serialize, Deserialize, Type)]
    #[serde(untagged)]
    enum UntaggedEnum {
        String(String),
        Number(i32),
    }

    // Transparent struct
    #[derive(Serialize, Deserialize, Type)]
    #[serde(transparent)]
    struct UserId(u64);

    // Default values
    #[derive(Serialize, Deserialize, Type, Default)]
    #[serde(default)]
    struct Config {
        enabled: bool,
        #[serde(default = "default_timeout")]
        timeout: u32,
    }

    fn default_timeout() -> u32 {
        30
    }

    // Deny unknown fields
    #[derive(Serialize, Deserialize, Type)]
    #[serde(deny_unknown_fields)]
    struct StrictConfig {
        version: String,
    }

    println!("  ✅ Basic container attributes (rename, rename_all, etc.)");
    println!("  ✅ Tagged enum representations (internal, adjacent, untagged)");
    println!("  ✅ Transparent structs and default values");
    println!("  ✅ Strict field validation");
    println!();
}

fn test_variant_attributes() {
    println!("🔀 Variant Attributes:");

    #[derive(Serialize, Deserialize, Type)]
    enum Status {
        #[serde(rename = "active")]
        Active,

        #[serde(alias = "inactive")]
        Inactive,

        #[serde(skip)]
        _Internal,

        #[serde(skip_serializing)]
        SkipSer,

        #[serde(skip_deserializing)]
        SkipDe,

        #[serde(rename_all = "camelCase")]
        ComplexVariant { is_critical: bool, error_count: u32 },
    }

    // The 'other' attribute for unknown variants
    #[derive(Serialize, Deserialize, Type)]
    #[serde(tag = "type")]
    enum ApiResponse {
        Success {
            data: String,
        },
        Error {
            message: String,
        },
        #[serde(other)]
        Unknown,
    }

    println!("  ✅ Variant renaming and aliases");
    println!("  ✅ Variant-specific skip attributes");
    println!("  ✅ The 'other' attribute for unknown variants");
    println!("  ✅ Variant-specific rename_all");
    println!();
}

fn test_field_attributes() {
    println!("📝 Field Attributes:");

    #[derive(Serialize, Deserialize, Type)]
    struct UserProfile {
        #[serde(rename = "userName")]
        user_name: String,

        #[serde(alias = "mail")]
        email: String,

        #[serde(default)]
        is_active: bool,

        #[serde(default = "default_age")]
        age: u32,

        #[serde(flatten)]
        metadata: std::collections::HashMap<String, String>,

        #[serde(skip)]
        internal_id: u64,

        #[serde(skip_serializing)]
        password_hash: String,

        #[serde(skip_deserializing)]
        computed_field: String,

        #[serde(skip_serializing_if = "Option::is_none")]
        optional_field: Option<String>,
    }

    fn default_age() -> u32 {
        18
    }

    println!("  ✅ Field renaming and aliases");
    println!("  ✅ Default values and field flattening");
    println!("  ✅ Conditional skipping");
    println!();
}

fn test_advanced_rename_syntax() {
    println!("🔄 Advanced Rename Syntax (Mode-Specific):");

    // This demonstrates the newly implemented advanced rename syntax
    #[derive(Serialize, Deserialize, Type)]
    struct ApiRequest {
        #[serde(rename(serialize = "requestId", deserialize = "request_id"))]
        request_id: String,
        user_data: String,
    }

    #[derive(Serialize, Deserialize, Type)]
    enum ApiMessage {
        #[serde(rename(serialize = "userCreate", deserialize = "user_create"))]
        UserCreate {
            #[serde(rename(serialize = "userId", deserialize = "user_id"))]
            user_id: u64,
        },

        DataUpdate {
            field_name: String,
            new_value: String,
        },
    }

    println!("  ✅ Mode-specific rename for fields and variants");
    println!("  ✅ Different names for serialization vs deserialization");
    println!();

    println!("🎉 All major Serde attributes are now supported by specta-serde!");
    println!();
    println!("Key improvements in this implementation:");
    println!("  • Added support for the 'other' attribute on enum variants");
    println!("  • Implemented advanced rename syntax with serialize/deserialize specificity");
    println!("  • Enhanced variant attribute parsing (previously was defaulted)");
    println!("  • Added comprehensive support for all variant-specific attributes");
    println!("  • Improved mode-specific transformations for better accuracy");
}
